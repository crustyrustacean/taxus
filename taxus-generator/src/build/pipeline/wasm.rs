// taxus-generator/src/build/pipeline/wasm.rs

use crate::error::GeneratorError;
use std::path::{Path, PathBuf};

#[cfg(feature = "islands")]
const CLIENT_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/client.js"));
#[cfg(feature = "islands")]
const CLIENT_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/client_bg.wasm"));

/// Result of a successful WASM client build.
pub struct WasmBuildOutput {
    pub js_path: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_size: u64,
}

/// Write the embedded WASM client files to the output directory.
pub fn build_wasm_client(output_dir: &Path) -> Result<WasmBuildOutput, GeneratorError> {
    let wasm_output_dir = output_dir.join("wasm");
    std::fs::create_dir_all(&wasm_output_dir).map_err(|e| GeneratorError::Io {
        path: wasm_output_dir.clone(),
        source: e,
    })?;

    let js_path = wasm_output_dir.join("client.js");
    let wasm_path = wasm_output_dir.join("client_bg.wasm");

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
