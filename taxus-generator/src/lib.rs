//! Generator Library
//!
//! A reusable static site generator library.
//!
//! # Overview
//!
//! This library provides the core functionality for generating static sites
//! from Markdown content with Yew server-side rendering.
//!
//! # Example
//!
//! ```no_run
//! use taxus_lib::config::SiteConfig;
//! use taxus_lib::content::Page;
//!
//! // Load configuration
//! let config = SiteConfig::from_dir(".")?;
//!
//! // Load a page
//! let page = Page::from_file("content/about.md")?;
//!
//! println!("Building site: {}", config.site.name);
//! println!("Page title: {}", page.frontmatter.title);
//! # Ok::<(), taxus_lib::error::GeneratorError>(())
//! ```

// Module declarations
pub mod assets;
pub mod build;
pub mod config;
pub mod content;
pub mod error;
pub mod feed;
pub mod highlighting;
pub mod images;
pub mod init;
pub mod routes;
pub mod serve;
pub mod telemetry;
pub mod templates;

// Re-exports for convenience
pub use assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
pub use build::{BuildReport, SiteBuilder};
pub use config::{BuildConfig, ImageConfig, SiteConfig, SiteMeta};
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page, Section};
pub use error::{
    AssetError, ContentError, FeedError, GeneratorError, ImageError, InitError, Result, RouteError,
    TemplateError,
};
pub use feed::{FeedConfig, FeedEntry, FeedGenerator};
pub use highlighting::{CodeHighlighter, LanguageRegistry};
pub use images::{ImageProcessor, ImageRegistry, ProcessedImage, render_picture};
pub use init::{InitOptions, InitReport, InitScaffolder};
pub use routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};
pub use templates::{
    HeroContext, PageContext, SectionContext, SiteContext, TemplateContext, TemplateRenderer,
    TeraRenderer,
};
