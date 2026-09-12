// taxus-generator/src/routes.rs

//! Route discovery and the route registry (parse phase).
//!
//! [`RouteDiscovery::discover_tree`] walks the content directory and builds
//! the Site Tree; [`RouteRegistry::from_tree`] projects the tree to one
//! [`RouteInfo`] per document (URL path, content file, output file, kind).
//! The registry is a view of the tree, never a second source. See the
//! book's [Identity](https://crustyrustacean.github.io/taxus/theory/identity.html) and
//! [Site Tree](https://crustyrustacean.github.io/taxus/theory/site-tree.html) chapters.
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
//! // Build the Site Tree from the content directory, then derive the routes
//! let discovery = RouteDiscovery::new("content");
//! let tree = discovery.discover_tree()?;
//! let registry = RouteRegistry::from_tree(&tree);
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
pub mod slugify;

pub use discovery::RouteDiscovery;
pub use registry::{RouteInfo, RouteKind, RouteRegistry};
pub use slugify::{slugify_path, slugify_segment};
