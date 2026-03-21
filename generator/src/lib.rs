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
//! use yew_ssg_lib::config::SiteConfig;
//! use yew_ssg_lib::content::Page;
//!
//! // Load configuration
//! let config = SiteConfig::from_dir(".")?;
//!
//! // Load a page
//! let page = Page::from_file("content/about.md")?;
//!
//! println!("Building site: {}", config.site.name);
//! println!("Page title: {}", page.frontmatter.title);
//! # Ok::<(), yew_ssg_lib::error::GeneratorError>(())
//! ```

// Module declarations
pub mod assets;
pub mod build;
pub mod config;
pub mod content;
pub mod error;
pub mod init;
pub mod routes;
pub mod templates;
pub mod tracing;

// Re-exports for convenience
pub use assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
pub use build::{BuildReport, SiteBuilder};
pub use config::{BuildConfig, SiteConfig, SiteMeta};
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page, Section};
pub use error::{
    AssetError, BuildError, ContentError, GeneratorError, InitError, Result, RouteError,
    TemplateError,
};
pub use init::{InitOptions, InitReport, InitScaffolder};
pub use routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};
pub use templates::{
    PageContext, SectionContext, SiteContext, TemplateContext, TemplateRenderer, TeraRenderer,
};
