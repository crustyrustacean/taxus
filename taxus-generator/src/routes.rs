// taxus-generator/src/routes.rs

//! Route discovery and management.
//!
//! This module provides types for discovering and managing routes in a static site.
//! Routes are derived from the content directory structure and map content files
//! to URL paths and output files.
//!
//! # Overview
//!
//! - [`RouteKind`]: Enum distinguishing between pages and sections
//! - [`RouteInfo`]: Information about a single route
//! - [`RouteRegistry`]: Collection of all routes with query methods
//! - [`RouteDiscovery`]: Discovers routes from content directory
//!
//! # Example
//!
//! ```no_run
//! use taxus_lib::routes::{RouteDiscovery, RouteRegistry, RouteInfo, RouteKind};
//!
//! // Discover routes from content directory
//! let discovery = RouteDiscovery::new("content");
//! let registry = discovery.discover()?;
//!
//! // Query routes
//! if let Some(route) = registry.get("/about/") {
//!     println!("Found route: {:?}", route);
//!     println!("Content file: {:?}", route.content_file);
//!     println!("Output file: {:?}", route.output_file);
//! }
//!
//! // Iterate over all pages
//! for route in registry.pages() {
//!     println!("Page: {}", route.path);
//! }
//!
//! // Check route existence
//! if registry.contains("/blog/") {
//!     println!("Blog section exists");
//! }
//! # Ok::<(), taxus_lib::error::GeneratorError>(())
//! ```

mod discovery;
mod registry;

pub use discovery::RouteDiscovery;
pub use registry::{RouteInfo, RouteKind, RouteRegistry};
