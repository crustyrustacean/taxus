// taxus-generator/src/build/pipeline/wasm.rs

use crate::error::GeneratorError;
use std::path::{Path, PathBuf};
use tracing::debug;

const CLIENT_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/client.js"));
const CLIENT_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/client_bg.wasm"));

/// Result of a successful WASM client build.
pub struct WasmBuildOutput {
    pub js_path: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_size: u64,
}

/// Write the embedded WASM client files to the output directory.
///
/// In dry-run mode no files are written; the returned paths and size are
/// still computed from the embedded constants so stage reporting is unchanged.
pub fn build_wasm_client(
    output_dir: &Path,
    dry_run: bool,
) -> Result<WasmBuildOutput, GeneratorError> {
    let wasm_output_dir = output_dir.join("wasm");
    let js_path = wasm_output_dir.join("client.js");
    let wasm_path = wasm_output_dir.join("client_bg.wasm");

    if dry_run {
        debug!(
            js = %js_path.display(),
            wasm = %wasm_path.display(),
            size = CLIENT_WASM.len(),
            "Dry run - skipping WASM client writes"
        );
        return Ok(WasmBuildOutput {
            wasm_size: CLIENT_WASM.len() as u64,
            js_path,
            wasm_path,
        });
    }

    std::fs::create_dir_all(&wasm_output_dir).map_err(|e| GeneratorError::Io {
        path: wasm_output_dir.clone(),
        source: e,
    })?;

    std::fs::write(&js_path, CLIENT_JS).map_err(|e| GeneratorError::Io {
        path: js_path.clone(),
        source: e,
    })?;

    std::fs::write(&wasm_path, CLIENT_WASM).map_err(|e| GeneratorError::Io {
        path: wasm_path.clone(),
        source: e,
    })?;

    Ok(WasmBuildOutput {
        wasm_size: CLIENT_WASM.len() as u64,
        js_path,
        wasm_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_wasm_client_writes_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let result = build_wasm_client(&output_dir, false).unwrap();

        assert_eq!(result.wasm_size, CLIENT_WASM.len() as u64);
        assert!(output_dir.join("wasm/client.js").exists());
        assert!(output_dir.join("wasm/client_bg.wasm").exists());
    }

    #[test]
    fn test_build_wasm_client_dry_run_writes_nothing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let result = build_wasm_client(&output_dir, true).unwrap();

        // Sizes and paths are still reported from the embedded constants...
        assert_eq!(result.wasm_size, CLIENT_WASM.len() as u64);
        assert!(result.js_path.ends_with("wasm/client.js"));
        assert!(result.wasm_path.ends_with("wasm/client_bg.wasm"));

        // ...but nothing is written, not even the output directory itself
        assert!(!output_dir.exists());
    }
}
