//! Generator Library
//!
//! A reusable static site generator library for Yew-based projects.
//!
//! # Overview
//!
//! This library provides the core functionality for generating static sites
//! from Markdown content with Yew server-side rendering.
//!
//! # Example
//!
//! ```no_run
//! use generator::config::SiteConfig;
//!
//! // Load configuration
//! let config = SiteConfig::from_dir(".")?;
//!
//! println!("Building site: {}", config.site.name);
//! # Ok::<(), generator::error::GeneratorError>(())
//! ```

// Module declarations
pub mod config;
pub mod error;

// Re-exports for convenience
pub use config::{BuildConfig, SiteConfig, SiteMeta};
pub use error::{GeneratorError, Result};
