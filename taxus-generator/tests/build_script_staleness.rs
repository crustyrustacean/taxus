//! #45: the embedded WASM client must rebuild when `taxus-common` changes.
//!
//! `taxus-client` depends on `taxus-common` (every island component lives
//! there), but the generator's `build.rs` historically declared
//! `rerun-if-changed` only for the client's sources. Editing taxus-common
//! recompiled the SSR code while the embedded `client.js`/`client_bg.wasm`
//! stayed stale — hydration silently disagreeing with the server.
//!
//! This test cannot run inside `cargo test` (it must invoke `cargo build`
//! for the generator and inspect cargo's fingerprint output, which nests
//! another cargo invocation). It is written as a plain integration test
//! that shells out and is skipped unless `TAXUS_TEST_BUILD_SCRIPT` is set
//! (CI and `cargo xtask ci` set it; a plain `cargo test` skips in ~0 ms).

use std::path::Path;
use std::process::Command;

fn env_flag(name: &str) -> bool {
    // cargo sets CARGO_* for integration tests; avoid accidentally
    // enabling under plain `cargo test`.
    std::env::var(name).is_ok_and(|v| v != "0")
}

#[test]
fn common_change_reruns_generator_build_script() {
    if !env_flag("TAXUS_TEST_BUILD_SCRIPT") || env_flag("CARGO_CFG_TEST") {
        eprintln!("skipping: set TAXUS_TEST_BUILD_SCRIPT=1 to run build-script staleness test");
        return;
    }

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("generator manifest has a parent")
        .to_path_buf();

    // A marker appended to a doc comment in taxus-common: changes the
    // crate's compiled output, is harmless, and is trivially revertible.
    let common_lib = workspace_root.join("taxus-common/src/lib.rs");
    let original = std::fs::read_to_string(&common_lib).expect("taxus-common/src/lib.rs readable");
    let marker = format!("\n// staleness probe {}\n", std::process::id());
    if original.contains(&marker) {
        panic!("probe marker already present; refusing to double-apply");
    }

    std::fs::write(&common_lib, original.clone() + &marker).expect("probe applied");

    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "-p", "taxus"])
        .current_dir(&workspace_root)
        .output()
        .expect("cargo build runs");

    // Revert FIRST so a failed assertion doesn't leave the probe behind.
    std::fs::write(&common_lib, &original).expect("probe reverted");

    assert!(
        output.status.success(),
        "cargo build -p taxus failed after touching taxus-common"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Cargo prints its progress lines on stderr.
    let progress = format!("{stdout}{stderr}");
    assert!(
        progress.contains("Compiling taxus v"),
        "touching taxus-common did not recompile the generator — the embedded \
         WASM would be stale (#45). cargo said:\n{progress}"
    );
}
