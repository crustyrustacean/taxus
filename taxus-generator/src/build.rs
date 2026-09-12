// taxus-generator/src/build.rs

//! The build: all three phases, parse, analyse and emit, in fifteen stages.
//!
//! [`SiteBuilder::build`] builds the Site Tree once (stage 1), then runs
//! every other stage against that immutable tree. The stage list, with
//! what each stage reads and produces, is in the book's
//! [Architecture](https://crustyrustacean.github.io/taxus/architecture.html) chapter; the model behind it is
//! the [Theory](https://crustyrustacean.github.io/taxus/theory/overview.html) section.
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
//! use taxus_lib::build::SiteBuilder;
//! use std::path::Path;
//!
//! // Build from a directory containing site.toml
//! let report = SiteBuilder::from_dir(Path::new("."))?
//!     .verbose(true)
//!     .build()?;
//!
//! report.print_summary();
//! # Ok::<(), taxus_lib::error::GeneratorError>(())
//! ```

mod builder;
pub mod pipeline;
mod report;
mod ssr;

pub use builder::SiteBuilder;
pub use pipeline::{ProcessedPage, RenderedPage};
pub use report::BuildReport;
