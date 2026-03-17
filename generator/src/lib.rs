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
//! use generator::content::Page;
//!
//! // Load configuration
//! let config = SiteConfig::from_dir(".")?;
//!
//! // Load a page
//! let page = Page::from_file("content/about.md")?;
//!
//! println!("Building site: {}", config.site.name);
//! println!("Page title: {}", page.frontmatter.title);
//! # Ok::<(), generator::error::GeneratorError>(())
//! ```

// Module declarations
pub mod assets;
pub mod build;
pub mod config;
pub mod content;
pub mod error;
pub mod routes;
pub mod templates;

// Re-exports for convenience
pub use assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
pub use build::{BuildReport, SiteBuilder};
pub use config::{BuildConfig, SiteConfig, SiteMeta};
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page, Section};
pub use error::{AssetError, BuildError, ContentError, GeneratorError, Result, RouteError, TemplateError};
pub use routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};
pub use templates::{
    PageContext, SectionContext, SiteContext, TemplateContext, TemplateRenderer, TeraRenderer,
};
