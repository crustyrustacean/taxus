// taxus-generator/src/build/pipeline/wasm.rs

use crate::error::{GeneratorError, WasmError};
use std::path::{Path, PathBuf};
use std::process::Command;
use wasm_bindgen_cli_support::Bindgen;

/// Result of a successful WASM client build.
pub struct WasmBuildOutput {
    /// Path to the generated JS loader (client.js)
    pub js_path: PathBuf,
    /// Path to the generated WASM binary (client_bg.wasm)
    pub wasm_path: PathBuf,
    /// Size of the WASM binary in bytes
    pub wasm_size: u64,
}

/// Build the taxus-client WASM binary and generate JS bindings.
pub fn build_wasm_client(
    workspace_root: &Path,
    output_dir: &Path,
    release: bool,
) -> Result<WasmBuildOutput, GeneratorError> {
    let wasm_output_dir = output_dir.join("wasm");
    std::fs::create_dir_all(&wasm_output_dir).map_err(|e| GeneratorError::Io {
        path: wasm_output_dir.clone(),
        source: e,
    })?;

    let manifest_path = workspace_root.join("taxus-client/Cargo.toml");
    cargo_build_wasm(&manifest_path, release)?;

    let raw_wasm = find_wasm_artifact(workspace_root, release)?;
    run_bindgen(&raw_wasm, &wasm_output_dir)?;

    let wasm_path = wasm_output_dir.join("client_bg.wasm");
    if release {
        run_wasm_opt(&wasm_path);
    }

    let wasm_size = std::fs::metadata(&wasm_path).map(|m| m.len()).unwrap_or(0);

    Ok(WasmBuildOutput {
        js_path: wasm_output_dir.join("client.js"),
        wasm_path,
        wasm_size,
    })
}

fn cargo_build_wasm(manifest_path: &Path, release: bool) -> Result<(), GeneratorError> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "--target",
        "wasm32-unknown-unknown",
        "--manifest-path",
        &manifest_path.display().to_string(),
        "--bin",
        "client",
    ]);

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().map_err(|e| {
        GeneratorError::Wasm(Box::new(WasmError::BuildFailed(format!(
            "Failed to run cargo: {e}"
        ))))
    })?;

    if !status.success() {
        return Err(GeneratorError::Wasm(Box::new(WasmError::BuildFailed(
            "cargo build for wasm32-unknown-unknown failed".into(),
        ))));
    }

    Ok(())
}

fn find_wasm_artifact(workspace_root: &Path, release: bool) -> Result<PathBuf, GeneratorError> {
    let profile = if release { "release" } else { "debug" };
    let path = workspace_root
        .join("target/wasm32-unknown-unknown")
        .join(profile)
        .join("client.wasm");

    if !path.exists() {
        return Err(GeneratorError::Wasm(Box::new(WasmError::BuildFailed(
            format!("Expected WASM artifact not found at {}", path.display()),
        ))));
    }

    Ok(path)
}

fn run_bindgen(wasm_path: &Path, output_dir: &Path) -> Result<(), GeneratorError> {
    Bindgen::new()
        .input_path(wasm_path)
        .web(true)
        .map_err(|e| {
            GeneratorError::Wasm(Box::new(WasmError::BuildFailed(format!(
                "wasm-bindgen config error: {e}"
            ))))
        })?
        .typescript(false)
        .generate(output_dir)
        .map_err(|e| {
            GeneratorError::Wasm(Box::new(WasmError::BuildFailed(format!(
                "wasm-bindgen failed: {e}"
            ))))
        })?;

    Ok(())
}

fn run_wasm_opt(wasm_path: &Path) {
    let result = Command::new("wasm-opt")
        .args([
            "-Oz",
            "-o",
            &wasm_path.display().to_string(),
            &wasm_path.display().to_string(),
        ])
        .status();

    match result {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: wasm-opt failed, skipping optimization"),
        Err(_) => eprintln!("Warning: wasm-opt not found, skipping optimization"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_wasm_artifact_release() {
        let tmp = TempDir::new().unwrap();
        let wasm_dir = tmp.path().join("target/wasm32-unknown-unknown/release");
        std::fs::create_dir_all(&wasm_dir).unwrap();
        std::fs::write(wasm_dir.join("client.wasm"), b"fake wasm").unwrap();

        let result = find_wasm_artifact(tmp.path(), true);
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("client.wasm"));
    }

    #[test]
    fn find_wasm_artifact_debug() {
        let tmp = TempDir::new().unwrap();
        let wasm_dir = tmp.path().join("target/wasm32-unknown-unknown/debug");
        std::fs::create_dir_all(&wasm_dir).unwrap();
        std::fs::write(wasm_dir.join("client.wasm"), b"fake wasm").unwrap();

        let result = find_wasm_artifact(tmp.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn find_wasm_artifact_missing() {
        let tmp = TempDir::new().unwrap();
        let result = find_wasm_artifact(tmp.path(), true);
        assert!(result.is_err());
    }
}
