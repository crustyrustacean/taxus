// generator/src/telemetry.rs

//! Telemetry initialization for the generator.
//!
//! This module provides structured logging via the `tracing` crate, replacing
//! the previous `println!`/`eprintln!` based output. Log levels are controlled
//! by the `RUST_LOG` environment variable or CLI flags.
//!
//! # Log Levels
//!
//! | Level | Usage |
//! |-------|-------|
//! | `ERROR` | Build failures, critical errors |
//! | `WARN` | Warnings, suggestions |
//! | `INFO` | Stage headers, summaries |
//! | `DEBUG` | Verbose progress details |
//! | `TRACE` | Fine-grained operations |
//!
//! # Environment Variable
//!
//! The `RUST_LOG` environment variable controls the level for commands that
//! don't take `--verbose`/`--quiet` flags (or when neither flag is given).
//! CLI flags take precedence over the environment when present.
//!
//! ```bash
//! # Default (info and above)
//! RUST_LOG=info taxus build
//!
//! # Verbose equivalent
//! RUST_LOG=debug taxus build
//!
//! # Everything
//! RUST_LOG=trace taxus build
//!
//! # Only errors
//! RUST_LOG=error taxus build
//!
//! # Module-specific
//! RUST_LOG=taxus_lib::build=debug taxus build
//! ```

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the tracing subscriber with environment filter.
///
/// The log level is controlled by the `RUST_LOG` environment variable.
/// If not set, defaults to `info` level.
///
/// # Example
///
/// ```rust,ignore
/// // Call early in main
/// taxus_lib::telemetry::init();
/// ```
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize tracing based on CLI flags.
///
/// Precedence (highest wins):
///
/// 1. `--quiet`   → error level
/// 2. `--verbose` → debug level
/// 3. `RUST_LOG` environment variable (module directives honored)
/// 4. `info` (default)
///
/// A subscriber is always installed; flags never cause logging to be
/// silently disabled.
pub fn init_tracing(verbose: bool, quiet: bool) {
    if quiet {
        init_with_level("error");
    } else if verbose {
        init_with_level("debug");
    } else {
        // No flags: honor RUST_LOG if present, else default to info.
        init();
    }
}

/// Initialize tracing with a specific level override.
///
/// This is useful for CLI flags like `--verbose` or `--quiet`.
/// The given level always applies — CLI flags take precedence over the
/// `RUST_LOG` environment variable (explicit user intent on the command
/// line wins over ambient environment). Callers wanting `RUST_LOG` to be
/// honored should use [`init`] instead.
///
/// # Arguments
///
/// * `level` - The log level to use (e.g., "error", "warn", "info", "debug", "trace")
///
/// # Example
///
/// ```rust,ignore
/// // For --quiet flag
/// taxus_lib::telemetry::init_with_level("error");
///
/// // For --verbose flag
/// taxus_lib::telemetry::init_with_level("debug");
/// ```
pub fn init_with_level(level: &str) {
    let filter = EnvFilter::new(level);

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
