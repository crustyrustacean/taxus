//! Tracing initialization for the generator.
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
//! ```bash
//! # Default (info and above)
//! RUST_LOG=info yew-ssg build
//!
//! # Verbose equivalent
//! RUST_LOG=debug yew-ssg build
//!
//! # Everything
//! RUST_LOG=trace yew-ssg build
//!
//! # Only errors
//! RUST_LOG=error yew-ssg build
//!
//! # Module-specific
//! RUST_LOG=yew_ssg_lib::build=debug yew-ssg build
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
/// yew_ssg_lib::tracing::init();
/// ```
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize tracing with a specific level override.
///
/// This is useful for CLI flags like `--verbose` or `--quiet`.
/// The environment variable `RUST_LOG` takes precedence if set.
///
/// # Arguments
///
/// * `level` - The log level to use (e.g., "error", "warn", "info", "debug", "trace")
///
/// # Example
///
/// ```rust,ignore
/// // For --quiet flag
/// yew_ssg_lib::tracing::init_with_level("error");
///
/// // For --verbose flag
/// yew_ssg_lib::tracing::init_with_level("debug");
/// ```
pub fn init_with_level(level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
