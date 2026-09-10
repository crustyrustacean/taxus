// taxus-domain/src/lib.rs

//! # taxus-domain
//!
//! The pure data model for Taxus sites — the Site Tree, its nodes, and the
//! derivations computed over them.
//!
//! This crate is deliberately free of I/O: no filesystem, no network, no
//! template engine, no markdown parsing. It defines *what a site is*;
//! the generator defines *how one is built*.
//!
//! ## Layers
//!
//! - [`schema`]: typed frontmatter — the schema concern
//! - [`identity`]: slugs, membership paths, and the URL paths derived from them
//! - [`tree`]: the Site Tree — [`SectionNode`]/[`PageNode`], construction, queries
//! - [`derivation`]: pure derivations — sorting, aggregation (`pages_from`)
//!
//! ## Invariants
//!
//! - Structure contains only containment: `pages` and `subsections` are
//!   direct children.
//! - Derived structures are recomputed, never stored.
//! - Addresses derive from membership in exactly one place
//!   ([`UrlPath::from_node_path`]).

pub mod derivation;
pub mod identity;
pub mod schema;
pub mod tree;

pub use identity::{NodePath, Slug, UrlPath};
pub use schema::{Frontmatter, SortBy};
pub use tree::{PageNode, SectionNode, SiteTree, SiteTreeBuilder, TreeError};
