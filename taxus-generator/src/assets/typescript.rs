//! Optional TypeScript build step using Microsoft's native `tsc` compiler.
//!
//! The feasibility spike embeds Microsoft's native TypeScript 7 compiler for
//! Linux x64, so site builds need no JavaScript runtime or package manager.
//! Other platforms can provide an explicit compiler path until matching
//! native archives are bundled.

use crate::config::TypeScriptConfig;
use crate::error::AssetError;
use flate2::read::GzDecoder;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tar::Archive;
use tracing::info;

const BUNDLED_TSC: &[u8] = include_bytes!("../../vendor/typescript-linux-x64-v7.0.2.tgz");
const BUNDLED_SHA256: &str = "7ecad6f67377e831856367ab062ef394f21506a611405bf8ac0ff039348637d3";
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// TypeScript compiler integration.
pub struct TypeScriptProcessor;

impl TypeScriptProcessor {
    /// Type-check and emit configured TypeScript into the generated
    /// `static/ts/` directory. Returns the number of emitted files.
    pub fn process(
        config: &TypeScriptConfig,
        site_dir: &Path,
        output_dir: &Path,
        dry_run: bool,
    ) -> Result<usize, AssetError> {
        if !config.enabled {
            return Ok(0);
        }

        let project = site_dir.join("tsconfig.json");
        if !project.is_file() {
            return Err(AssetError::TypeScript(format!(
                "TypeScript is enabled but config file is missing: {}",
                project.display()
            )));
        }

        let compiler = match &config.compiler {
            Some(path) if path.is_absolute() || path.components().count() > 1 => {
                resolve_site_path(site_dir, path)
            }
            Some(path) => path.clone(),
            None => bundled_compiler()?,
        };

        let mut command = Command::new(&compiler);
        command
            .current_dir(site_dir)
            .arg("--pretty")
            .arg("false")
            .arg("--project")
            .arg(&project);

        if dry_run {
            command
                .arg("--noEmit")
                .arg("--rootDir")
                .arg(site_dir.join("scripts"));
            info!(compiler = %compiler.display(), project = %project.display(), "Type-checking TypeScript");
            let status = command.status().map_err(|error| {
                AssetError::TypeScript(format!(
                    "could not execute compiler {}: {error}",
                    compiler.display()
                ))
            })?;
            if !status.success() {
                return Err(AssetError::TypeScript(format!(
                    "compiler exited with {status} (project {})",
                    project.display()
                )));
            }
            return Ok(0);
        }

        let js_output = output_dir.join("static/ts");
        let root_dir = site_dir.join("scripts");
        let staging = make_staging_dir(&js_output)?;
        command
            // Override tsconfig's noEmit setting: a normal build must emit,
            // but only after checking has succeeded.
            .arg("--noEmit")
            .arg("false")
            .arg("--noEmitOnError")
            .arg("--rootDir")
            .arg(&root_dir)
            .arg("--outDir")
            .arg(&staging);

        info!(compiler = %compiler.display(), project = %project.display(), "Checking and compiling TypeScript");
        let status = match command.status() {
            Ok(status) => status,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging);
                return Err(AssetError::TypeScript(format!(
                    "could not execute compiler {}: {error}",
                    compiler.display()
                )));
            }
        };
        if !status.success() {
            let _ = fs::remove_dir_all(&staging);
            return Err(AssetError::TypeScript(format!(
                "compiler exited with {status} (project {})",
                project.display()
            )));
        }

        let emitted = count_files(&staging);
        replace_compiled_output(&staging, &js_output)?;
        Ok(emitted)
    }
}

fn resolve_site_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Stage under the same parent so final directory rename remains same-filesystem.
fn make_staging_dir(output: &Path) -> Result<PathBuf, AssetError> {
    let parent = output.parent().ok_or_else(|| {
        AssetError::TypeScript(format!(
            "output directory has no parent: {}",
            output.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| AssetError::Io {
        path: parent.to_path_buf(),
        source: error,
    })?;

    loop {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let staging = parent.join(format!(".taxus-ts-stage-{}-{id}", std::process::id()));
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(AssetError::Io {
                    path: staging,
                    source: error,
                });
            }
        }
    }
}

fn count_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .count()
}

/// Replace output only after a successful compiler exit; restore the old bundle
/// if the final rename fails. A failed type check never touches the last good JS.
fn replace_compiled_output(staging: &Path, output: &Path) -> Result<(), AssetError> {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let backup = output.with_extension(format!("taxus-ts-backup-{}-{id}", std::process::id()));
    let had_output = output.exists();
    if had_output {
        if backup.exists() {
            fs::remove_dir_all(&backup).map_err(|error| AssetError::Io {
                path: backup.clone(),
                source: error,
            })?;
        }
        fs::rename(output, &backup).map_err(|error| AssetError::Io {
            path: output.to_path_buf(),
            source: error,
        })?;
    }

    if let Err(error) = fs::rename(staging, output) {
        if had_output {
            let _ = fs::rename(&backup, output);
        }
        let _ = fs::remove_dir_all(staging);
        return Err(AssetError::Io {
            path: output.to_path_buf(),
            source: error,
        });
    }

    if had_output {
        fs::remove_dir_all(&backup).map_err(|error| AssetError::Io {
            path: backup,
            source: error,
        })?;
    }
    Ok(())
}

fn bundled_compiler() -> Result<PathBuf, AssetError> {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Err(AssetError::TypeScript(
            "the bundled TypeScript compiler PoC currently supports Linux x64 only; set `typescript.compiler` to a compatible Microsoft tsc executable on this platform".to_string(),
        ));
    }

    let root = std::env::temp_dir().join("taxus-typescript-7.0.2-linux-x64");
    let compiler = root.join("package/lib/tsc");
    let declarations = root.join("package/lib/lib.dom.d.ts");
    if compiler.is_file() && declarations.is_file() {
        return Ok(compiler);
    }

    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(BUNDLED_TSC));
    if digest != BUNDLED_SHA256 {
        return Err(AssetError::TypeScript(format!(
            "bundled compiler archive failed SHA-256 verification: {digest}"
        )));
    }

    let parent = root.parent().expect("temp compiler path has parent");
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(".taxus-tsc-{}-{id}", std::process::id()));
    fs::create_dir(&staging).map_err(|error| AssetError::Io {
        path: staging.clone(),
        source: error,
    })?;

    let decoder = GzDecoder::new(BUNDLED_TSC);
    let mut archive = Archive::new(decoder);
    if let Err(error) = archive.unpack(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(AssetError::TypeScript(format!(
            "could not extract bundled compiler: {error}"
        )));
    }

    let staged_compiler = staging.join("package/lib/tsc");
    let staged_declarations = staging.join("package/lib/lib.dom.d.ts");
    if !staged_compiler.is_file() || !staged_declarations.is_file() {
        let _ = fs::remove_dir_all(&staging);
        return Err(AssetError::TypeScript(format!(
            "bundled compiler archive did not contain {} and its standard libraries",
            staged_compiler.display()
        )));
    }

    if root.exists() {
        let _ = fs::remove_dir_all(&staging);
        if compiler.is_file() && declarations.is_file() {
            return Ok(compiler);
        }
        return Err(AssetError::TypeScript(format!(
            "another process created an incomplete compiler cache at {}",
            root.display()
        )));
    }
    if let Err(error) = fs::rename(&staging, &root) {
        let _ = fs::remove_dir_all(&staging);
        if compiler.is_file() && declarations.is_file() {
            return Ok(compiler);
        }
        return Err(AssetError::Io {
            path: root.clone(),
            source: error,
        });
    }
    Ok(compiler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TypeScriptConfig;
    use tempfile::TempDir;

    #[test]
    fn disabled_is_a_noop_without_a_tsconfig() {
        let temp = TempDir::new().unwrap();
        assert_eq!(
            TypeScriptProcessor::process(
                &TypeScriptConfig::default(),
                temp.path(),
                &temp.path().join("dist"),
                false,
            )
            .unwrap(),
            0
        );
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn bundled_compiler_checks_emits_and_preserves_last_good_output_on_error() {
        let temp = TempDir::new().unwrap();
        let site = temp.path().join("site");
        let output = temp.path().join("dist");
        fs::create_dir_all(site.join("scripts")).unwrap();
        fs::write(
            site.join("tsconfig.json"),
            r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "strict": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "noEmit": true
  },
  "include": ["scripts/**/*.ts"]
}"#,
        )
        .unwrap();
        let entry = site.join("scripts/main.ts");
        fs::write(
            &entry,
            "const button: HTMLButtonElement | null = document.querySelector('#save');\nbutton?.click();\n",
        )
        .unwrap();
        let config = TypeScriptConfig {
            enabled: true,
            compiler: None,
        };

        assert_eq!(
            TypeScriptProcessor::process(&config, &site, &output, false).unwrap(),
            1
        );
        let js = output.join("static/ts/main.js");
        let emitted = fs::read_to_string(&js).unwrap();
        assert!(emitted.contains("document.querySelector"));
        assert!(!emitted.contains("HTMLButtonElement"));

        fs::write(&entry, "const broken: number = 'type error';\n").unwrap();
        assert!(TypeScriptProcessor::process(&config, &site, &output, false).is_err());
        assert_eq!(fs::read_to_string(js).unwrap(), emitted);

        fs::write(&entry, "const checked: number = 42;\n").unwrap();
        assert_eq!(
            TypeScriptProcessor::process(&config, &site, &output, true).unwrap(),
            0
        );
    }
}
