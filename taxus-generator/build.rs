// taxus-generator/build.rs

#[cfg(feature = "islands")]
use std::env;
#[cfg(feature = "islands")]
use std::path::Path;
#[cfg(feature = "islands")]
use std::process::Command;
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../taxus-client/src");

    #[cfg(feature = "islands")]
    {
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set — must be run by Cargo");
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("CARGO_MANIFEST_DIR has no parent directory");

        build_wasm_client(workspace_root, Path::new(&out_dir));
    }
}

#[cfg(feature = "islands")]
fn build_wasm_client(workspace_root: &Path, out_dir: &Path) {
    let manifest_path = workspace_root.join("taxus-client/Cargo.toml");

    if !manifest_path.exists() {
        panic!(
            "taxus-client not found at {}. \
             Build must run from the taxus workspace.",
            manifest_path.display()
        );
    }

    let wasm_target_dir = workspace_root.join("target/wasm-build");

    let status = Command::new("cargo")
        .args([
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target-dir",
            &wasm_target_dir.display().to_string(),
            "--release",
            "--bin",
            "client",
        ])
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .expect("Failed to run cargo");

    if !status.success() {
        panic!("cargo build for wasm32-unknown-unknown failed");
    }

    let raw_wasm = wasm_target_dir.join("wasm32-unknown-unknown/release/client.wasm");

    wasm_bindgen_cli_support::Bindgen::new()
        .input_path(&raw_wasm)
        .web(true)
        .expect("Failed to configure wasm-bindgen for web target")
        .typescript(false)
        .generate(out_dir)
        .expect("wasm-bindgen failed to generate JS bindings");

    let js_path = out_dir.join("client.js");
    let wasm_path = out_dir.join("client_bg.wasm");

    if !js_path.exists() || !wasm_path.exists() {
        panic!(
            "wasm-bindgen did not produce expected output files in {}",
            out_dir.display()
        );
    }
}
