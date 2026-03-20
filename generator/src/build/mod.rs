//! Build system for generating static sites.
//!
//! This module provides the main build orchestration for the static site generator.
//! It coordinates configuration loading, content parsing, route discovery, template
//! rendering, and asset processing into a unified build pipeline.
//!
//! # Overview
//!
//! - [`SiteBuilder`]: Main entry point for building a site
//! - [`BuildReport`]: Statistics and results from a build
//! - [`pipeline`]: Individual build stage functions
//!
//! # Example
//!
//! ```no_run
//! use yew_ssg_lib::build::SiteBuilder;
//! use std::path::Path;
//!
//! // Build from a directory containing site.toml
//! let report = SiteBuilder::from_dir(Path::new("."))?
//!     .verbose(true)
//!     .build()?;
//!
//! report.print_summary();
//! # Ok::<(), yew_ssg_lib::error::GeneratorError>(())
//! ```

mod builder;
pub mod pipeline;
mod report;

pub use builder::SiteBuilder;
pub use pipeline::{ProcessedPage, RenderedPage};
pub use report::BuildReport;
