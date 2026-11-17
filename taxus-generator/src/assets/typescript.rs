//! Optional TypeScript build step using Microsoft's native `tsc` compiler.
//!
//! TypeScript-enabled Taxus binaries embed Microsoft's native TypeScript 7
//! compiler for their release target, so site builds need no JavaScript runtime
//! or package manager. Binaries without the feature can use an explicit
//! compiler path.

use crate::config::TypeScriptConfig;
use crate::error::AssetError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;

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

        // The build pipeline may pass paths such as `get-taxus-org` while the
        // compiler command runs with that directory as its current directory.
        // Resolve every site/output argument first so paths are not doubled
        // (`get-taxus-org/get-taxus-org/tsconfig.json`).
        let site_root = absolute_path(site_dir)?;
        let output_root = absolute_path(output_dir)?;
        let project = site_root.join("tsconfig.json");
        if !project.is_file() {
            return Err(AssetError::TypeScript(format!(
                "TypeScript is enabled but config file is missing: {}",
                project.display()
            )));
        }

        let compiler = match &config.compiler {
            Some(path) if path.is_absolute() || path.components().count() > 1 => {
                resolve_site_path(&site_root, path)
            }
            Some(path) => path.clone(),
            None => bundled_compiler()?,
        };

        let mut command = Command::new(&compiler);
        command
            .current_dir(&site_root)
            .arg("--pretty")
            .arg("false")
            .arg("--project")
            .arg(&project);

        if dry_run {
            command
                .arg("--noEmit")
                .arg("--rootDir")
                .arg(site_root.join("scripts"));
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

        let js_output = output_root.join("static/ts");
        let root_dir = site_root.join("scripts");
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

fn absolute_path(path: &Path) -> Result<PathBuf, AssetError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let cwd = std::env::current_dir().map_err(|source| AssetError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(cwd.join(path))
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

#[cfg(not(feature = "typescript"))]
fn bundled_compiler() -> Result<PathBuf, AssetError> {
    Err(AssetError::TypeScript(
        "TypeScript is enabled, but this Taxus binary has no bundled compiler; rebuild with `cargo build --features typescript` or set `typescript.compiler` to a compatible Microsoft tsc executable".to_string(),
    ))
}

#[cfg(all(
    feature = "typescript",
    any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )
))]
mod bundled {
    use super::{AssetError, NEXT_TEMP_ID};
    use flate2::read::GzDecoder;
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    use std::fs;
    use std::path::{Component, Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::Ordering;
    use tar::Archive;

    // The compiler is a build-time opt-in. A default Taxus binary contains no
    // TypeScript archive; each target-specific support crate is compiled only with
    // the `typescript` Cargo feature and the matching target.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(super) use taxus_typescript_darwin_arm64 as archive;
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    pub(super) use taxus_typescript_darwin_x64 as archive;
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    pub(super) use taxus_typescript_linux_arm64 as archive;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(super) use taxus_typescript_linux_x64 as archive;
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    pub(super) use taxus_typescript_win32_arm64 as archive;
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    pub(super) use taxus_typescript_win32_x64 as archive;

    pub(super) const CACHE_MANIFEST: &str = ".taxus-manifest";
    const REQUIRED_DECLARATIONS: &[&str] = &["package/lib/lib.d.ts", "package/lib/lib.dom.d.ts"];
    static BUNDLED_COMPILER_LOCK: Mutex<()> = Mutex::new(());

    pub(super) fn bundled_compiler() -> Result<PathBuf, AssetError> {
        bundled_compiler_in(&std::env::temp_dir())
    }

    pub(super) fn bundled_compiler_in(parent: &Path) -> Result<PathBuf, AssetError> {
        let root = parent.join(archive::CACHE_NAME);
        let compiler = root.join(archive::COMPILER_PATH);
        if is_cache_complete(&root, archive::COMPILER_PATH, archive::SHA256) {
            return Ok(compiler);
        }

        fs::create_dir_all(parent).map_err(|source| AssetError::Io {
            path: parent.to_path_buf(),
            source,
        })?;

        let _in_process_guard = BUNDLED_COMPILER_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if is_cache_complete(&root, archive::COMPILER_PATH, archive::SHA256) {
            return Ok(compiler);
        }

        let lock_path = parent.join(format!(".{}.lock", archive::CACHE_NAME));
        let lock_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| AssetError::Io {
                path: lock_path.clone(),
                source,
            })?;
        match lock_file.lock() {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {}
            Err(error) => {
                return Err(AssetError::Io {
                    path: lock_path,
                    source: error,
                });
            }
        }

        if is_cache_complete(&root, archive::COMPILER_PATH, archive::SHA256) {
            return Ok(compiler);
        }

        extract_and_install_compiler(parent, &root)?;
        Ok(compiler)
    }

    pub(super) fn extract_and_install_compiler(
        parent: &Path,
        root: &Path,
    ) -> Result<(), AssetError> {
        const MAX_INSTALL_ATTEMPTS: usize = 8;

        let digest = format!("{:x}", Sha256::digest(archive::ARCHIVE));
        if digest != archive::SHA256 {
            return Err(AssetError::TypeScript(format!(
                "bundled compiler archive failed SHA-256 verification: {digest}"
            )));
        }

        let staging = create_unique_dir(parent, ".taxus-tsc")?;
        let decoder = GzDecoder::new(archive::ARCHIVE);
        let mut tar = Archive::new(decoder);
        if let Err(error) = tar.unpack(&staging) {
            let _ = fs::remove_dir_all(&staging);
            return Err(AssetError::TypeScript(format!(
                "could not extract bundled compiler: {error}"
            )));
        }

        if !has_required_compiler_files(&staging, archive::COMPILER_PATH) {
            let staged_compiler = staging.join(archive::COMPILER_PATH);
            let _ = fs::remove_dir_all(&staging);
            return Err(AssetError::TypeScript(format!(
                "bundled compiler archive did not contain {} and its standard libraries",
                staged_compiler.display()
            )));
        }

        if let Err(error) = write_cache_manifest(&staging, archive::SHA256) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }

        if !is_cache_complete(&staging, archive::COMPILER_PATH, archive::SHA256) {
            let _ = fs::remove_dir_all(&staging);
            return Err(AssetError::TypeScript(
                "staged bundled compiler cache failed completeness verification".to_string(),
            ));
        }

        for attempt in 0..MAX_INSTALL_ATTEMPTS {
            if is_cache_complete(root, archive::COMPILER_PATH, archive::SHA256) {
                let _ = fs::remove_dir_all(&staging);
                return Ok(());
            }

            if root.symlink_metadata().is_ok() {
                let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
                let stale = parent.join(format!(".taxus-tsc-stale-{}-{id}", std::process::id()));
                match fs::rename(root, &stale) {
                    Ok(()) => remove_path_best_effort(&stale),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        if is_cache_complete(root, archive::COMPILER_PATH, archive::SHA256) {
                            let _ = fs::remove_dir_all(&staging);
                            return Ok(());
                        }
                        if attempt + 1 < MAX_INSTALL_ATTEMPTS {
                            std::thread::sleep(std::time::Duration::from_millis(20));
                            continue;
                        }
                        if let Err(remove_error) = remove_path(root) {
                            let _ = fs::remove_dir_all(&staging);
                            return Err(AssetError::Io {
                                path: root.to_path_buf(),
                                source: if remove_error.kind() == std::io::ErrorKind::NotFound {
                                    error
                                } else {
                                    remove_error
                                },
                            });
                        }
                    }
                }
            }

            match fs::rename(&staging, root) {
                Ok(()) => {
                    if is_cache_complete(root, archive::COMPILER_PATH, archive::SHA256) {
                        return Ok(());
                    }
                    if attempt + 1 == MAX_INSTALL_ATTEMPTS {
                        return Err(AssetError::TypeScript(format!(
                            "extracted compiler cache at {} is incomplete",
                            root.display()
                        )));
                    }
                }
                Err(error) => {
                    if is_cache_complete(root, archive::COMPILER_PATH, archive::SHA256) {
                        let _ = fs::remove_dir_all(&staging);
                        return Ok(());
                    }
                    if attempt + 1 < MAX_INSTALL_ATTEMPTS {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        continue;
                    }
                    let _ = fs::remove_dir_all(&staging);
                    return Err(AssetError::Io {
                        path: root.to_path_buf(),
                        source: error,
                    });
                }
            }
        }

        let _ = fs::remove_dir_all(&staging);
        Err(AssetError::TypeScript(format!(
            "could not install bundled compiler cache at {}",
            root.display()
        )))
    }

    pub(super) fn is_cache_complete(
        root: &Path,
        compiler_rel: &str,
        expected_sha256: &str,
    ) -> bool {
        if !has_required_compiler_files(root, compiler_rel) {
            return false;
        }

        let Ok(manifest) = fs::read_to_string(root.join(CACHE_MANIFEST)) else {
            return false;
        };
        let mut lines = manifest.lines();
        let Some(header) = lines.next() else {
            return false;
        };
        if header != format!("sha256:{expected_sha256}") {
            return false;
        }

        let mut saw_compiler = false;
        let mut saw_lib_d_ts = false;
        let mut saw_lib_dom_d_ts = false;
        let mut entry_count = 0usize;

        for line in lines {
            if line.is_empty() {
                continue;
            }
            let Some((size_str, rel_path)) = line.split_once('\t') else {
                return false;
            };
            let Ok(expected_size) = size_str.parse::<u64>() else {
                return false;
            };
            let Some(full_path) = join_safe_relative(root, rel_path) else {
                return false;
            };
            let Ok(meta) = fs::metadata(&full_path) else {
                return false;
            };
            if !meta.is_file() || meta.len() != expected_size {
                return false;
            }
            if rel_path == compiler_rel {
                saw_compiler = true;
            } else if rel_path == "package/lib/lib.d.ts" {
                saw_lib_d_ts = true;
            } else if rel_path == "package/lib/lib.dom.d.ts" {
                saw_lib_dom_d_ts = true;
            }
            entry_count += 1;
        }

        entry_count > 0 && saw_compiler && saw_lib_d_ts && saw_lib_dom_d_ts
    }

    fn create_unique_dir(parent: &Path, prefix: &str) -> Result<PathBuf, AssetError> {
        loop {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let dir = parent.join(format!("{prefix}-{}-{id}", std::process::id()));
            match fs::create_dir(&dir) {
                Ok(()) => return Ok(dir),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(AssetError::Io { path: dir, source }),
            }
        }
    }

    fn remove_path(path: &Path) -> std::io::Result<()> {
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
            Ok(_) => fs::remove_file(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn remove_path_best_effort(path: &Path) {
        let _ = remove_path(path);
    }

    fn has_required_compiler_files(root: &Path, compiler_rel: &str) -> bool {
        let Some(compiler) = join_safe_relative(root, compiler_rel) else {
            return false;
        };
        if !compiler.is_file() {
            return false;
        }
        REQUIRED_DECLARATIONS
            .iter()
            .all(|rel| join_safe_relative(root, rel).is_some_and(|path| path.is_file()))
    }

    fn write_cache_manifest(staging: &Path, expected_sha256: &str) -> Result<(), AssetError> {
        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(staging) {
            let entry = entry.map_err(|error| {
                let path = error.path().unwrap_or(staging).to_path_buf();
                let source = error
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walkdir error"));
                AssetError::Io { path, source }
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(staging).map_err(|error| {
                AssetError::TypeScript(format!(
                    "staged compiler entry escaped staging directory {}: {error}",
                    staging.display()
                ))
            })?;
            let rel_str = rel
                .components()
                .map(|component| match component {
                    Component::Normal(part) => part.to_str().ok_or_else(|| {
                        AssetError::TypeScript(format!(
                            "staged compiler entry has non-UTF-8 path: {}",
                            entry.path().display()
                        ))
                    }),
                    _ => Err(AssetError::TypeScript(format!(
                        "unexpected path component in staged compiler: {}",
                        entry.path().display()
                    ))),
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("/");
            if rel_str == CACHE_MANIFEST {
                continue;
            }
            let meta = entry.metadata().map_err(|error| {
                let path = error.path().unwrap_or(entry.path()).to_path_buf();
                let source = error
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("metadata error"));
                AssetError::Io { path, source }
            })?;
            entries.push((rel_str, meta.len()));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut content = format!("sha256:{expected_sha256}\n");
        for (rel_str, len) in entries {
            let _ = writeln!(content, "{len}\t{rel_str}");
        }

        let manifest_path = staging.join(CACHE_MANIFEST);
        fs::write(&manifest_path, content).map_err(|source| AssetError::Io {
            path: manifest_path,
            source,
        })
    }

    fn join_safe_relative(root: &Path, rel_path: &str) -> Option<PathBuf> {
        if rel_path.is_empty() || rel_path.contains('\0') || rel_path.contains('\\') {
            return None;
        }
        let mut path = root.to_path_buf();
        for segment in rel_path.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return None;
            }
            path.push(segment);
        }
        Some(path)
    }
}

#[cfg(all(
    feature = "typescript",
    any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )
))]
fn bundled_compiler() -> Result<PathBuf, AssetError> {
    bundled::bundled_compiler()
}

#[cfg(all(
    feature = "typescript",
    not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    ))
))]
fn bundled_compiler() -> Result<PathBuf, AssetError> {
    Err(AssetError::TypeScript(
        "TypeScript is enabled, but no bundled compiler is available for this target; set `typescript.compiler` to a compatible Microsoft tsc executable".to_string(),
    ))
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

    #[cfg(not(feature = "typescript"))]
    #[test]
    fn bundled_compiler_is_opt_in() {
        let temp = TempDir::new().unwrap();
        let site = temp.path().join("site");
        fs::create_dir_all(site.join("scripts")).unwrap();
        fs::write(site.join("tsconfig.json"), "{}").unwrap();
        let config = TypeScriptConfig {
            enabled: true,
            compiler: None,
        };

        let error = TypeScriptProcessor::process(&config, &site, &temp.path().join("dist"), false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("--features typescript"));
    }

    #[cfg(all(
        feature = "typescript",
        any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )
    ))]
    #[test]
    fn bundled_compiler_replaces_cache_missing_lib_d_ts() {
        let temp = TempDir::new().unwrap();
        let compiler = bundled::bundled_compiler_in(temp.path()).unwrap();
        let root = temp.path().join(bundled::archive::CACHE_NAME);
        let lib_d_ts = root.join("package/lib/lib.d.ts");
        let lib_dom_d_ts = root.join("package/lib/lib.dom.d.ts");
        assert!(compiler.is_file());
        assert!(lib_d_ts.is_file());
        assert!(lib_dom_d_ts.is_file());

        fs::remove_file(&lib_d_ts).unwrap();
        assert!(!bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
        assert!(compiler.is_file());
        assert!(lib_dom_d_ts.is_file());

        let repaired = bundled::bundled_compiler_in(temp.path()).unwrap();
        assert_eq!(repaired, compiler);
        assert!(lib_d_ts.is_file());
        assert!(bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
    }

    #[cfg(all(
        feature = "typescript",
        any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )
    ))]
    #[test]
    fn bundled_compiler_replaces_cache_missing_other_declaration_file() {
        let temp = TempDir::new().unwrap();
        let compiler = bundled::bundled_compiler_in(temp.path()).unwrap();
        let root = temp.path().join(bundled::archive::CACHE_NAME);
        let lib_d_ts = root.join("package/lib/lib.d.ts");
        let lib_dom_d_ts = root.join("package/lib/lib.dom.d.ts");
        let other_lib = root.join("package/lib/lib.es2015.core.d.ts");
        assert!(other_lib.is_file());

        fs::remove_file(&other_lib).unwrap();
        assert!(compiler.is_file());
        assert!(lib_d_ts.is_file());
        assert!(lib_dom_d_ts.is_file());
        assert!(!bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));

        let repaired = bundled::bundled_compiler_in(temp.path()).unwrap();
        assert_eq!(repaired, compiler);
        assert!(other_lib.is_file());
        assert!(bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
    }

    #[cfg(all(
        feature = "typescript",
        any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )
    ))]
    #[test]
    fn bundled_compiler_replaces_cache_missing_sentinel_or_manifest_or_truncated() {
        let temp = TempDir::new().unwrap();
        let compiler = bundled::bundled_compiler_in(temp.path()).unwrap();
        let root = temp.path().join(bundled::archive::CACHE_NAME);
        let lib_dom_d_ts = root.join("package/lib/lib.dom.d.ts");

        // Missing lib.dom.d.ts sentinel
        fs::remove_file(&lib_dom_d_ts).unwrap();
        assert!(!bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
        bundled::bundled_compiler_in(temp.path()).unwrap();
        assert!(lib_dom_d_ts.is_file());

        // Missing compiler sentinel
        fs::remove_file(&compiler).unwrap();
        assert!(!bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
        bundled::bundled_compiler_in(temp.path()).unwrap();
        assert!(compiler.is_file());

        // Truncated declaration file
        let lib_d_ts = root.join("package/lib/lib.d.ts");
        fs::write(&lib_d_ts, b"").unwrap();
        assert!(!bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
        bundled::bundled_compiler_in(temp.path()).unwrap();
        assert!(fs::metadata(&lib_d_ts).unwrap().len() > 0);

        // Pre-existing legacy/unmanifested incomplete cache with both old sentinels
        fs::remove_dir_all(&root).unwrap();
        fs::create_dir_all(root.join("package/lib")).unwrap();
        fs::write(&compiler, b"not-a-real-compiler").unwrap();
        fs::write(&lib_dom_d_ts, b"// incomplete").unwrap();
        assert!(!bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
        bundled::bundled_compiler_in(temp.path()).unwrap();
        assert!(root.join(bundled::CACHE_MANIFEST).is_file());
        assert!(lib_d_ts.is_file());
        assert!(bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
    }

    #[cfg(all(
        feature = "typescript",
        any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )
    ))]
    #[test]
    fn bundled_compiler_handles_concurrent_extraction_and_repair() {
        // Test locked concurrent calls via bundled_compiler_in on an empty dir.
        let temp = TempDir::new().unwrap();
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| scope.spawn(|| bundled::bundled_compiler_in(temp.path())))
                .collect();
            for handle in handles {
                let path = handle.join().unwrap().unwrap();
                assert!(path.is_file());
            }
        });

        let root = temp.path().join(bundled::archive::CACHE_NAME);
        assert!(bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));

        // Simulate an incomplete cache and exercise direct multi-process-like
        // racing calls to extract_and_install_compiler without the in-process mutex.
        fs::remove_file(root.join("package/lib/lib.d.ts")).unwrap();
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| scope.spawn(|| bundled::extract_and_install_compiler(temp.path(), &root)))
                .collect();
            for handle in handles {
                handle.join().unwrap().unwrap();
            }
        });

        assert!(bundled::is_cache_complete(
            &root,
            bundled::archive::COMPILER_PATH,
            bundled::archive::SHA256,
        ));
    }

    #[cfg(all(feature = "typescript", target_os = "linux", target_arch = "x86_64"))]
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
    "noEmit": true,
    "sourceMap": true
  },
  "include": ["scripts/**/*.ts"]
}"#,
        )
        .unwrap();
        let entry = site.join("scripts/main.ts");
        let helper = site.join("scripts/greeting.ts");
        fs::write(
            &helper,
            "export const greeting: string = 'hello from TypeScript';\n",
        )
        .unwrap();
        fs::write(
            &entry,
            "import { greeting } from './greeting.js';\nconst button: HTMLButtonElement | null = document.querySelector('#save');\nbutton?.setAttribute('aria-label', greeting);\n",
        )
        .unwrap();
        let config = TypeScriptConfig {
            enabled: true,
            compiler: None,
        };

        assert_eq!(
            TypeScriptProcessor::process(&config, &site, &output, false).unwrap(),
            4
        );
        let js = output.join("static/ts/main.js");
        let helper_js = output.join("static/ts/greeting.js");
        let emitted = fs::read_to_string(&js).unwrap();
        assert!(emitted.contains("./greeting.js"));
        assert!(emitted.contains("document.querySelector"));
        assert!(!emitted.contains("HTMLButtonElement"));
        assert!(
            fs::read_to_string(helper_js)
                .unwrap()
                .contains("hello from TypeScript")
        );
        assert!(output.join("static/ts/main.js.map").is_file());
        assert!(emitted.contains("sourceMappingURL=main.js.map"));

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
