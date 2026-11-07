//! The stable starting point for library consumers.
//!
//! This module intentionally re-exports the small set of types needed to build
//! a site without importing Taxus's internal pipeline modules. More specialized
//! APIs remain available through their documented modules.

pub use crate::assets::AssetReport;
pub use crate::build::{BuildReport, SiteBuilder};
pub use crate::config::{BuildConfig, ImageConfig, SiteConfig, SiteMeta, TypeScriptConfig};
pub use crate::content::{ContentSource, FilesystemContentSource, Frontmatter, Page};
pub use crate::error::{GeneratorError, Result};
pub use crate::feed::{FeedConfig, FeedEntry};
