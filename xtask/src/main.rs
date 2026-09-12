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

use std::path::Path;
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

    /// Run the full CI pipeline locally (fmt, lint, build, test, WASM check,
    /// build get-taxus-org).
    Ci,

    /// Promote CHANGELOG.md's `[Unreleased]` section to the next version
    /// (the workspace version bumped by the given level). Changelog only;
    /// versioning and tagging go through `cargo release`, whose hook runs
    /// `cargo xtask changelog`.
    Release {
        /// Bump level: "major", "minor", or "patch".
        #[arg(long, value_parser = ["major", "minor", "patch"])]
        bump: String,
        /// Dry-run: report what would change without writing CHANGELOG.md.
        #[arg(long)]
        dry_run: bool,
    },

    /// Promote CHANGELOG.md's `[Unreleased]` section to an explicit version.
    /// This is the `cargo release` pre-release hook; a no-op when the
    /// version's section already exists and `[Unreleased]` is empty.
    #[command(disable_version_flag = true)]
    Changelog {
        /// The version being released, e.g. 1.0.0.
        #[arg(long)]
        version: String,
        /// Dry-run: report what would change without writing CHANGELOG.md.
        #[arg(long)]
        dry_run: bool,
    },

    /// Deploy the product site (get-taxus-org) to Cloudflare Pages.
    Deploy {
        /// Cloudflare Pages project name.
        #[arg(long, default_value = "get-taxus-org")]
        project: String,
        /// Deploy to a specific branch. Defaults to the production branch.
        /// Use a different name (e.g. "preview") for a preview deployment.
        #[arg(long)]
        branch: Option<String>,
        /// The Cloudflare Pages production branch name.
        #[arg(long, default_value = "main")]
        prod_branch: String,
        /// Skip the build step and deploy the existing dist/ directory.
        #[arg(long)]
        no_build: bool,
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
        Command::Changelog { version, dry_run } => cmd_changelog(&version, dry_run),
        Command::Deploy {
            project,
            branch,
            prod_branch,
            no_build,
        } => cmd_deploy(&project, branch.as_deref(), &prod_branch, no_build),
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

/// Run a command in the workspace root, print a summary line, and return its exit code.
fn run(label: &str, cmd: &str, args: &[&str]) -> i32 {
    run_in(label, &workspace_root(), cmd, args)
}

/// Run a command in a specific directory, print a summary line, and return its exit code.
fn run_in(label: &str, dir: &Path, cmd: &str, args: &[&str]) -> i32 {
    eprintln!("  {label}");
    let start = Instant::now();

    let mut child = std::process::Command::new(cmd)
        .args(args)
        .current_dir(dir)
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
        Ok(s) if s.success() => (),
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
    // Build each subcommand's argv explicitly rather than inserting into a
    // shared list — order-coupled insert(0)/insert(1) was fragile (#21).
    let mut flags: Vec<String> = Vec::new();
    if release {
        flags.push("--release".into());
    }
    flags.extend(feature_args(feature_flags(features)));
    let flag_refs: Vec<&str> = flags.iter().map(String::as_str).collect();

    if nextest {
        require_tool("cargo-nextest", "Install with: cargo install cargo-nextest");
        let mut nextest_args: Vec<&str> = vec!["run"];
        nextest_args.extend(flag_refs.iter().copied());
        cargo("nextest", &nextest_args)
    } else {
        cargo("test", &flag_refs)
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
    cargo("doc", &args)
}

fn cmd_book(serve: bool) -> i32 {
    require_tool("mdbook", "Install with: cargo install mdbook");
    if serve {
        let label = "mdbook serve";
        run(label, "mdbook", &["serve", "docs/"])
    } else {
        let label = "mdbook build";
        run(label, "mdbook", &["build", "docs/"])
    }
}

fn cmd_audit() -> i32 {
    require_tool("cargo-audit", "Install with: cargo install cargo-audit");
    let label = "cargo audit";
    run(label, "cargo", &["audit"])
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
/// Build the product site with the debug binary into `target/ci-site`,
/// the same `taxus build` the deploy workflow runs, so a change that
/// breaks it fails in CI rather than on the deploy.
fn cmd_build_site() -> i32 {
    let output = workspace_root().join("target").join("ci-site");
    let output = output.to_string_lossy().into_owned();
    let rc = run(
        "taxus build get-taxus-org",
        "cargo",
        &[
            "run",
            "-p",
            "taxus",
            "--",
            "build",
            "--dir",
            "get-taxus-org",
            "--output",
            &output,
        ],
    );
    if rc != 0 {
        return rc;
    }
    for expected in ["index.html", "feed.xml"] {
        if !Path::new(&output).join(expected).is_file() {
            eprintln!("    error: {expected} missing from {output}");
            return 1;
        }
    }
    0
}

fn cmd_ci() -> i32 {
    eprintln!("\n━━━ CI pipeline ━━━\n");

    eprintln!("[1/6] Format check");
    let rc = cmd_fmt(true);
    if rc != 0 {
        return rc;
    }

    eprintln!("[2/6] Clippy");
    let rc = cmd_lint(None, false);
    if rc != 0 {
        return rc;
    }

    eprintln!("[3/6] Build");
    let rc = cmd_build(false, None);
    if rc != 0 {
        return rc;
    }

    eprintln!("[4/6] Test");
    let rc = cmd_test(false, None, false);
    if rc != 0 {
        return rc;
    }

    eprintln!("[5/6] WASM check");
    let rc = cmd_wasm(false);
    if rc != 0 {
        return rc;
    }

    eprintln!("[6/6] Build get-taxus.org");
    let rc = cmd_build_site();
    if rc != 0 {
        return rc;
    }

    eprintln!("\n  ✓ CI pipeline passed\n");
    0
}

/// The workspace version from the root `Cargo.toml` (`[workspace.package]`).
fn workspace_version() -> Result<String, String> {
    let manifest = workspace_root().join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|e| format!("cannot read {}: {e}", manifest.display()))?;
    text.lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("version")
                .map(str::trim_start)
                .and_then(|rest| rest.strip_prefix('='))
                .map(|rest| rest.trim().trim_matches('"').to_string())
        })
        .ok_or_else(|| format!("no `version = \"…\"` line in {}", manifest.display()))
}

/// `version` bumped by `level` ("major", "minor" or "patch").
fn bump_version(version: &str, level: &str) -> Result<String, String> {
    let parts: Vec<u64> = version
        .split('.')
        .map(|p| p.parse::<u64>())
        .collect::<Result<_, _>>()
        .map_err(|e| format!("cannot parse version `{version}`: {e}"))?;
    let [major, minor, patch] = parts[..] else {
        return Err(format!("version `{version}` is not MAJOR.MINOR.PATCH"));
    };
    Ok(match level {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{major}.{}.0", minor + 1),
        "patch" => format!("{major}.{minor}.{}", patch + 1),
        other => return Err(format!("unknown bump level `{other}`")),
    })
}

/// Today's date in UTC as `YYYY-MM-DD`, from the system clock.
fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Howard Hinnant's civil-from-days; days since 1970-01-01.
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// What [`promote_changelog`] did.
#[derive(Debug, PartialEq, Eq)]
enum Promotion {
    /// `[Unreleased]` became `[version] - date`; a fresh empty
    /// `[Unreleased]` sits above it.
    Promoted,
    /// `[version]` already exists and `[Unreleased]` is empty: nothing to do.
    /// Lets the release hook run more than once without harm.
    AlreadyPromoted,
}

/// Promote the `## [Unreleased]` section of a Keep-a-Changelog file to
/// `## [version] - date`, leaving an empty `## [Unreleased]` above it.
///
/// The changelog is written by hand, one entry per change, as the work
/// lands; releasing only renames the section. Fails when there is no
/// `[Unreleased]` heading, when the section is empty (nothing to release),
/// or when `[version]` already exists with unreleased entries still
/// pending (the version was released and new work has accumulated since).
fn promote_changelog(text: &str, version: &str, date: &str) -> Result<(String, Promotion), String> {
    let unreleased = "## [Unreleased]";
    let versioned = format!("## [{version}]");
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end() == unreleased)
        .ok_or_else(|| format!("no `{unreleased}` heading in CHANGELOG.md"))?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("## "))
        .map_or(lines.len(), |i| start + 1 + i);
    let body_is_empty = lines[start + 1..end].iter().all(|l| l.trim().is_empty());
    let already_released = lines.iter().any(|l| l.starts_with(&versioned));

    match (already_released, body_is_empty) {
        (true, true) => return Ok((text.to_string(), Promotion::AlreadyPromoted)),
        (true, false) => {
            return Err(format!(
                "`{versioned}` already exists and `{unreleased}` has entries; bump to a new version"
            ));
        }
        (false, true) => {
            return Err(format!("`{unreleased}` is empty: nothing to release"));
        }
        (false, false) => {}
    }

    let mut out: Vec<String> = lines[..start].iter().map(|l| l.to_string()).collect();
    out.push(unreleased.to_string());
    out.push(String::new());
    out.push(format!("{versioned} - {date}"));
    out.extend(lines[start + 1..].iter().map(|l| l.to_string()));
    let mut joined = out.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    Ok((joined, Promotion::Promoted))
}

/// Promote `[Unreleased]` to `version` in the workspace CHANGELOG.md.
fn cmd_changelog(version: &str, dry_run: bool) -> i32 {
    let path = workspace_root().join("CHANGELOG.md");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let date = today_utc();
    match promote_changelog(&text, version, &date) {
        Ok((_, Promotion::AlreadyPromoted)) => {
            eprintln!("  ✓ CHANGELOG.md already has [{version}] and no unreleased entries");
            0
        }
        Ok((updated, Promotion::Promoted)) => {
            let entries = updated
                .lines()
                .skip_while(|l| !l.starts_with(&format!("## [{version}]")))
                .skip(1)
                .take_while(|l| !l.starts_with("## "))
                .filter(|l| l.starts_with("- "))
                .count();
            if dry_run {
                eprintln!(
                    "  ✓ would promote [Unreleased] ({entries} entries) to [{version}] - {date} (dry run)"
                );
                return 0;
            }
            if let Err(e) = std::fs::write(&path, updated) {
                eprintln!("error: cannot write {}: {e}", path.display());
                return 1;
            }
            eprintln!("  ✓ promoted [Unreleased] ({entries} entries) to [{version}] - {date}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

/// `cargo xtask release --bump <level>`: promote `[Unreleased]` to the
/// workspace version bumped by `level`.
fn cmd_release(bump: &str, dry_run: bool) -> i32 {
    let current = match workspace_version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let next = match bump_version(&current, bump) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    eprintln!("\n━━━ Changelog for {next} ({bump} bump from {current}) ━━━\n");
    let rc = cmd_changelog(&next, dry_run);
    if rc == 0 {
        eprintln!(
            "\n  Versioning and tagging: `cargo release {bump} --execute --no-confirm` runs this\n  step itself, bumps every crate to {next}, commits and tags v{next}."
        );
    }
    rc
}

/// Deploy the get-taxus-org product site to Cloudflare Pages via `wrangler`.
///
/// Builds the site with `taxus build` (unless `--no-build` is given), then
/// uploads `get-taxus-org/dist` with `npx wrangler pages deploy`. Production is
/// the default (wrangler is invoked with `--branch <prod_branch>`); pass
/// `--branch <name>` for a preview deployment on a different branch.
fn cmd_deploy(project: &str, branch: Option<&str>, prod_branch: &str, no_build: bool) -> i32 {
    let site_dir = workspace_root().join("get-taxus-org");
    let dist_dir = site_dir.join("dist");

    if !site_dir.is_dir() {
        eprintln!("error: get-taxus-org/ not found at {}", site_dir.display());
        return 1;
    }

    require_tool(
        "taxus",
        "Install with: cargo install --path taxus-generator --root ~/.local",
    );
    require_tool("npx", "Install Node.js from https://nodejs.org");

    if !no_build {
        eprintln!("\n━━━ Build product site ━━━\n");
        let rc = run_in("taxus build", &site_dir, "taxus", &["build"]);
        if rc != 0 {
            return rc;
        }
    }

    if !dist_dir.is_dir() {
        eprintln!(
            "error: {} does not exist. Run without --no-build first.",
            dist_dir.display()
        );
        return 1;
    }

    let resolved_branch = branch.unwrap_or(prod_branch);
    let is_prod = resolved_branch == prod_branch;
    let target = if is_prod {
        "production".to_string()
    } else {
        format!("preview branch '{resolved_branch}'")
    };

    eprintln!("\n━━━ Deploy to Cloudflare Pages ━━━\n");
    eprintln!("  project : {project}");
    eprintln!("  target  : {target}");
    eprintln!("  dist    : {}\n", dist_dir.display());

    let args: Vec<&str> = vec![
        "--yes",
        "wrangler@latest",
        "pages",
        "deploy",
        "dist",
        "--project-name",
        project,
        "--branch",
        resolved_branch,
        "--commit-dirty=true",
    ];

    run_in("wrangler pages deploy", &site_dir, "npx", &args)
}

#[cfg(test)]
mod tests {
    use super::{Promotion, bump_version, promote_changelog, today_utc};

    const CHANGELOG: &str = "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n### Added\n\n- one\n- two\n\n## [0.7.0] - 2026-09-07\n\n### Fixed\n\n- old\n";

    #[test]
    fn promote_renames_unreleased_and_leaves_an_empty_one() {
        let (out, what) = promote_changelog(CHANGELOG, "1.0.0", "2026-09-12").unwrap();
        assert_eq!(what, Promotion::Promoted);
        assert_eq!(
            out,
            "# Changelog\n\nIntro.\n\n## [Unreleased]\n\n## [1.0.0] - 2026-09-12\n\n### Added\n\n- one\n- two\n\n## [0.7.0] - 2026-09-07\n\n### Fixed\n\n- old\n"
        );
    }

    #[test]
    fn promote_is_a_no_op_once_done() {
        let (once, _) = promote_changelog(CHANGELOG, "1.0.0", "2026-09-12").unwrap();
        let (twice, what) = promote_changelog(&once, "1.0.0", "2026-09-13").unwrap();
        assert_eq!(what, Promotion::AlreadyPromoted);
        assert_eq!(twice, once);
    }

    #[test]
    fn promote_rejects_empty_missing_and_stale() {
        let empty = "# Changelog\n\n## [Unreleased]\n\n## [0.7.0] - 2026-09-07\n\n- old\n";
        assert!(
            promote_changelog(empty, "1.0.0", "2026-09-12")
                .unwrap_err()
                .contains("empty")
        );
        assert!(
            promote_changelog("# Changelog\n\n## [0.7.0]\n", "1.0.0", "2026-09-12")
                .unwrap_err()
                .contains("no `## [Unreleased]`")
        );
        // 1.0.0 was released and new entries accumulated: bump again.
        let (released, _) = promote_changelog(CHANGELOG, "1.0.0", "2026-09-12").unwrap();
        let stale = released.replacen("## [Unreleased]\n", "## [Unreleased]\n\n- newer\n", 1);
        assert!(
            promote_changelog(&stale, "1.0.0", "2026-09-13")
                .unwrap_err()
                .contains("already exists")
        );
    }

    #[test]
    fn today_is_iso_date() {
        let today = today_utc();
        assert_eq!(today.len(), 10, "{today}");
        assert_eq!(&today[4..5], "-");
        assert_eq!(&today[7..8], "-");
        assert!(today.starts_with("20"), "{today}");
    }

    #[test]
    fn bump_levels() {
        assert_eq!(bump_version("0.7.0", "major").unwrap(), "1.0.0");
        assert_eq!(bump_version("0.7.0", "minor").unwrap(), "0.8.0");
        assert_eq!(bump_version("0.7.3", "patch").unwrap(), "0.7.4");
        assert_eq!(bump_version("1.2.3", "major").unwrap(), "2.0.0");
        assert!(bump_version("1.2", "patch").is_err());
        assert!(bump_version("1.2.x", "patch").is_err());
        assert!(bump_version("1.2.3", "huge").is_err());
    }
}
