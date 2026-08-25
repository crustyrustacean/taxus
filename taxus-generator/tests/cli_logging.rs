//! Regression tests for CLI logging behavior (issue #26).
//!
//! Tracing's global subscriber can only be installed once per process, so
//! library-level tests cannot exercise `init_tracing` variants. These tests
//! spawn the compiled `taxus` binary as a subprocess and assert on the log
//! output it produces.
//!
//! The regression these guard against: setting `RUST_LOG` used to make
//! `init_tracing` return *before installing any subscriber*, silently
//! disabling all logging for `build`/`serve`. Additionally, `RUST_LOG`
//! used to override `--quiet`, and `--verbose` used to be ignored when
//! `RUST_LOG` was set to a lower level.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Path to the compiled taxus binary.
fn taxus_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_taxus"))
}

/// Build a minimal site in a temp dir and return its path.
fn make_site() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let status = Command::new(taxus_bin())
        .arg("init")
        .arg(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to spawn taxus init");
    assert!(status.success(), "taxus init failed");
    dir
}

/// Run `taxus build` in `dir` with the given RUST_LOG value (or None to
/// remove it from the environment) and extra args, returning (code, combined output).
fn run_build(dir: &tempfile::TempDir, rust_log: Option<&str>, args: &[&str]) -> (i32, String) {
    let mut cmd = Command::new(taxus_bin());
    cmd.arg("build")
        .arg("--dir")
        .arg(dir.path())
        .args(args)
        .stdin(Stdio::null());
    match rust_log {
        Some(v) => {
            cmd.env("RUST_LOG", v);
        }
        None => {
            cmd.env_remove("RUST_LOG");
        }
    }
    let out = cmd.output().expect("failed to spawn taxus build");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

/// Strip ANSI escape sequences so level markers can be matched reliably
/// (log output is colored even when piped).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip until the escape sequence terminates (letter, typically 'm')
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Count INFO-level log lines in combined output.
fn info_lines(output: &str) -> usize {
    let clean = strip_ansi(output);
    clean.lines().filter(|l| l.contains(" INFO ")).count()
}

/// Count DEBUG-level log lines in combined output.
fn debug_lines(output: &str) -> usize {
    let clean = strip_ansi(output);
    clean.lines().filter(|l| l.contains(" DEBUG ")).count()
}

#[test]
fn test_build_without_rust_log_logs_info() {
    let site = make_site();
    let (code, out) = run_build(&site, None, &[]);
    assert_eq!(code, 0, "build failed:\n{out}");
    assert!(
        info_lines(&out) > 0,
        "expected INFO logs with no RUST_LOG; got:\n{out}"
    );
    assert_eq!(debug_lines(&out), 0);
}

#[test]
fn test_build_with_rust_log_debug_still_logs() {
    // REGRESSION: RUST_LOG=debug used to produce zero output because no
    // subscriber was installed.
    let site = make_site();
    let (code, out) = run_build(&site, Some("debug"), &[]);
    assert_eq!(code, 0, "build failed:\n{out}");
    assert!(
        debug_lines(&out) > 0,
        "RUST_LOG=debug must produce DEBUG logs; got:\n{out}"
    );
}

#[test]
fn test_build_rust_log_error_suppresses_info() {
    // With no CLI flags, RUST_LOG is honored.
    let site = make_site();
    let (code, out) = run_build(&site, Some("error"), &[]);
    assert_eq!(code, 0, "build failed:\n{out}");
    assert_eq!(info_lines(&out), 0, "RUST_LOG=error must suppress INFO");
}

#[test]
fn test_build_verbose_overrides_rust_log_error() {
    // Explicit CLI flag beats ambient environment.
    let site = make_site();
    let (code, out) = run_build(&site, Some("error"), &["--verbose"]);
    assert_eq!(code, 0, "build failed:\n{out}");
    assert!(
        debug_lines(&out) > 0,
        "--verbose must override RUST_LOG=error; got:\n{out}"
    );
}

#[test]
fn test_build_quiet_overrides_rust_log_debug() {
    // REGRESSION: RUST_LOG=debug used to override --quiet, producing 58 log
    // lines on a quiet run.
    let site = make_site();
    let (code, out) = run_build(&site, Some("debug"), &["--quiet"]);
    assert_eq!(code, 0, "build failed:\n{out}");
    assert_eq!(
        info_lines(&out),
        0,
        "--quiet must override RUST_LOG=debug; got:\n{out}"
    );
    assert_eq!(
        debug_lines(&out),
        0,
        "--quiet must override RUST_LOG=debug; got:\n{out}"
    );
}
