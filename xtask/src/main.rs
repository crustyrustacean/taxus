//! Task runner for the Taxus workspace.
//!
//! Invoke via `cargo xtask <COMMAND>` (aliased in `.cargo/config.toml`).
//!
//! # Design
//!
//! Each workflow that a developer (or CI) runs locally is represented by a
//! subcommand.  The commands deliberately accept **no positional arguments**
//! — every tunable is a named flag — so that they compose well in scripts and
//! Makefiles.

use std::time::Instant;

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "cargo xtask",
    bin_name = "cargo xtask",
    about = "Task runner for the Taxus workspace",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the project.
    Build {
        /// Build in release mode.
        #[arg(long)]
        release: bool,
        /// Space-separated cargo feature flags (e.g. "lang-rust").
        #[arg(long)]
        features: Option<String>,
    },

    /// Run unit and integration tests.
    Test {
        /// Run tests in release mode.
        #[arg(long)]
        release: bool,
        /// Space-separated cargo feature flags.
        #[arg(long)]
        features: Option<String>,
        /// Next test runner (requires cargo-nextest).
        #[arg(long)]
        nextest: bool,
    },

    /// Fast compile check (no codegen).
    Check {
        /// Space-separated cargo feature flags.
        #[arg(long)]
        features: Option<String>,
    },

    /// Lint with Clippy.
    Lint {
        /// Space-separated cargo feature flags.
        #[arg(long)]
        features: Option<String>,
        /// Fix automatically where possible.
        #[arg(long)]
        fix: bool,
    },

    /// Check formatting with rustfmt (does not modify files).
    Fmt {
        /// Write formatting changes in place.
        #[arg(long)]
        check: bool,
    },

    /// Build documentation.
    Doc {
        /// Open the docs in a browser after building.
        #[arg(long)]
        open: bool,
    },

    /// Build the mdbook documentation in `docs/`.
    Book {
        /// Serve the book on a local HTTP server.
        #[arg(long)]
        serve: bool,
    },

    /// Run the security audit (requires cargo-audit).
    Audit,

    /// Build WASM artifacts.
    Wasm {
        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },

    /// Clean build artifacts.
    Clean,

    /// Run the full CI pipeline locally (fmt, lint, test for default + islands).
    Ci,

    /// Prepare a release: generate changelog, tag, verify build.
    Release {
        /// Bump level: "major", "minor", or "patch".
        #[arg(long, value_parser = ["major", "minor", "patch"])]
        bump: String,
        /// Dry-run: print commands without executing them.
        #[arg(long)]
        dry_run: bool,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let exit = match cli.command {
        Command::Build { release, features } => cmd_build(release, features),
        Command::Test {
            release,
            features,
            nextest,
        } => cmd_test(release, features, nextest),
        Command::Check { features } => cmd_check(features),
        Command::Lint { features, fix } => cmd_lint(features, fix),
        Command::Fmt { check } => cmd_fmt(check),
        Command::Doc { open } => cmd_doc(open),
        Command::Book { serve } => cmd_book(serve),
        Command::Audit => cmd_audit(),
        Command::Wasm { release } => cmd_wasm(release),
        Command::Clean => cmd_clean(),
        Command::Ci => cmd_ci(),
        Command::Release { bump, dry_run } => cmd_release(&bump, dry_run),
    };

    std::process::exit(exit);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Workspace root directory (one level up from the xtask crate).
fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should be inside the workspace")
        .to_path_buf()
}

/// Run a command, print a summary line, and return its exit code.
fn run(label: &str, cmd: &str, args: &[&str]) -> i32 {
    eprintln!("  {label}");
    let start = Instant::now();

    let mut child = std::process::Command::new(cmd)
        .args(args)
        .current_dir(workspace_root())
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("    failed to spawn {cmd}: {e}");
            std::process::exit(1);
        });

    let status = child.wait().expect("child process panicked");

    let elapsed = start.elapsed();
    let icon = if status.success() { "✓" } else { "✗" };
    eprintln!("  {icon} {label} ({elapsed:.1?})");

    status.code().unwrap_or(1)
}

/// Run `cargo` with the given subcommand and flags.
fn cargo(subcommand: &str, extra_args: &[&str]) -> i32 {
    let mut args = vec![subcommand];
    args.extend_from_slice(extra_args);
    let label = format!("cargo {subcommand}");
    run(&label, "cargo", &args)
}

/// Build a `--features` flag string from an optional comma/space-separated list.
fn feature_flags(features: Option<String>) -> Vec<String> {
    match features {
        Some(f) if !f.trim().is_empty() => f.split([' ', ',']).map(String::from).collect(),
        _ => Vec::new(),
    }
}

/// Convert feature flags into cargo `--features <f>` arguments.
fn feature_args(features: Vec<String>) -> Vec<String> {
    features
        .into_iter()
        .flat_map(|f| ["--features".into(), f])
        .collect()
}

/// Assert a required external tool is installed.
fn require_tool(name: &str, install_hint: &str) {
    let result = std::process::Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match result {
        Ok(s) if s.success() => return,
        _ => {
            eprintln!("error: `{name}` not found. {install_hint}");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_build(release: bool, features: Option<String>) -> i32 {
    let mut args = Vec::new();
    if release {
        args.push("--release".into());
    }
    args.extend(feature_args(feature_flags(features)));
    cargo(
        "build",
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

fn cmd_test(release: bool, features: Option<String>, nextest: bool) -> i32 {
    let mut args = Vec::new();
    if release {
        args.push("--release".into());
    }
    args.extend(feature_args(feature_flags(features)));

    if nextest {
        require_tool("cargo-nextest", "Install with: cargo install cargo-nextest");
        args.insert(0, "nextest".into());
        args.insert(1, "run".into());
        cargo(
            &args[0],
            &args[1..].iter().map(String::as_str).collect::<Vec<_>>(),
        )
    } else {
        cargo("test", &args.iter().map(String::as_str).collect::<Vec<_>>())
    }
}

fn cmd_check(features: Option<String>) -> i32 {
    let args = feature_args(feature_flags(features));
    cargo(
        "check",
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

fn cmd_lint(features: Option<String>, fix: bool) -> i32 {
    let mut args: Vec<String> = Vec::new();
    if fix {
        args.push("--fix".into());
        // Allow clippy --fix to apply changes automatically
        args.push("--allow-dirty".into());
        args.push("--allow-staged".into());
    }
    args.extend(feature_args(feature_flags(features)));
    args.push("--".into());
    args.push("-D".into());
    args.push("warnings".into());
    cargo(
        "clippy",
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

fn cmd_fmt(check: bool) -> i32 {
    if check {
        cargo("fmt", &["--all", "--", "--check"])
    } else {
        cargo("fmt", &["--all"])
    }
}

fn cmd_doc(open: bool) -> i32 {
    let mut args: Vec<&str> = vec!["--no-deps"];
    if open {
        args.push("--open");
    }
    let code = cargo("doc", &args);
    code
}

fn cmd_book(serve: bool) -> i32 {
    require_tool("mdbook", "Install with: cargo install mdbook");
    if serve {
        let label = "mdbook serve";
        run(&label, "mdbook", &["serve", "docs/"])
    } else {
        let label = "mdbook build";
        run(&label, "mdbook", &["build", "docs/"])
    }
}

fn cmd_audit() -> i32 {
    require_tool("cargo-audit", "Install with: cargo install cargo-audit");
    let label = "cargo audit";
    run(&label, "cargo", &["audit"])
}

fn cmd_wasm(release: bool) -> i32 {
    require_tool("rustup", "Install from https://rustup.rs");

    // Ensure the WASM target is installed.
    let rc = run(
        "ensure wasm32-unknown-unknown target",
        "rustup",
        &["target", "add", "wasm32-unknown-unknown"],
    );
    if rc != 0 {
        return rc;
    }

    let mut base_args: Vec<&str> = vec!["--target", "wasm32-unknown-unknown"];
    if release {
        base_args.push("--release");
    }

    eprintln!("  Checking taxus-common for WASM...");
    let mut args = base_args.clone();
    args.insert(0, "--package");
    args.insert(1, "taxus-common");
    let rc = cargo("check", &args);
    if rc != 0 {
        return rc;
    }

    eprintln!("  Checking taxus-client for WASM...");
    let mut args2 = base_args.clone();
    args2.insert(0, "--package");
    args2.insert(1, "taxus-client");
    cargo("check", &args2)
}

fn cmd_clean() -> i32 {
    cargo("clean", &[])
}

/// Run the full CI pipeline locally (fmt, lint, test, WASM check).
fn cmd_ci() -> i32 {
    eprintln!("\n━━━ CI pipeline ━━━\n");

    eprintln!("[1/5] Format check");
    let rc = cmd_fmt(true);
    if rc != 0 {
        return rc;
    }

    eprintln!("[2/5] Clippy");
    let rc = cmd_lint(None, false);
    if rc != 0 {
        return rc;
    }

    eprintln!("[3/5] Build");
    let rc = cmd_build(false, None);
    if rc != 0 {
        return rc;
    }

    eprintln!("[4/5] Test");
    let rc = cmd_test(false, None, false);
    if rc != 0 {
        return rc;
    }

    eprintln!("[5/5] WASM check");
    let rc = cmd_wasm(false);
    if rc != 0 {
        return rc;
    }

    eprintln!("\n  ✓ CI pipeline passed\n");
    0
}

fn cmd_release(bump: &str, dry_run: bool) -> i32 {
    require_tool("git-cliff", "Install with: cargo install git-cliff");

    let tag = format!("v{bump}");
    let mut args: Vec<&str> = Vec::new();
    args.push("cliff");
    args.push("--unreleased");
    args.push("--tag");
    args.push(&tag);
    args.push("--prepend");
    args.push("CHANGELOG.md");

    let label = if dry_run {
        "git-cliff (dry run)"
    } else {
        "git-cliff — update CHANGELOG.md"
    };

    let mut cmd = std::process::Command::new("cargo");
    cmd.args(&args)
        .current_dir(workspace_root())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if dry_run {
        cmd.arg("--dry-run");
    }

    let status = cmd.status().expect("failed to spawn cargo cliff");
    let icon = if status.success() { "✓" } else { "✗" };
    eprintln!("  {icon} {label}");

    if dry_run {
        eprintln!("\n  Dry run complete. Re-run without --dry-run to write changes.");
    }

    status.code().unwrap_or(1)
}
