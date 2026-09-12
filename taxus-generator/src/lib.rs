//! # taxus_lib
//!
//! The Taxus generator: the crate that turns a site directory into an
//! output directory.
//!
//! Taxus is a compiler for websites. A build has three phases. **Parse**
//! reads the content directory into the Site Tree, the pure model defined
//! in `taxus_domain`. **Analyse** computes derivations over that tree:
//! listings, taxonomy terms, feed and sitemap entries. **Emit** renders
//! Markdown and templates, processes images and assets, and writes files.
//! This crate does all the reading and writing; the domain crate does
//! none. See the book's
//! [Overview](https://crustyrustacean.github.io/taxus/theory/overview.html) and
//! [Architecture](https://crustyrustacean.github.io/taxus/architecture.html) chapters. Vocabulary is fixed
//! by the [Glossary](https://crustyrustacean.github.io/taxus/theory/glossary.html).
//!
//! # Modules by phase
//!
//! | Phase | Modules |
//! |-------|---------|
//! | parse | [`config`], [`content`], [`routes`] |
//! | analyse and emit | [`build`] (the fifteen stages of [`SiteBuilder::build`]) |
//! | emit | [`templates`], [`images`], [`highlighting`], [`assets`], [`feed`] |
//! | none | [`init`], [`serve`], [`telemetry`], [`error`] |
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
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page};
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
