// taxus-domain/src/lib.rs

//! # taxus-domain
//!
//! The pure data model for Taxus sites: the Site Tree, its nodes, and
//! the derivations computed over them.
//!
//! Taxus is a compiler for websites. This crate is the part that says
//! *what a site is*. The generator crate (`taxus-generator`) reads files
//! into it and writes files out of it. This crate itself does no I/O: no
//! filesystem, no network, no template engine, no Markdown parsing. Every
//! function here can be exercised with a [`SiteTreeBuilder`] and a few
//! string literals.
//!
//! The ideas are explained in the book's Theory chapters:
//!
//! - [Overview](https://crustyrustacean.github.io/taxus/theory/overview.html):
//!   the three phases, parse, analyse, emit
//! - [The Site Tree](https://crustyrustacean.github.io/taxus/theory/site-tree.html):
//!   what a section and a page are, containment versus reachability, why
//!   the tree is immutable
//! - [Identity](https://crustyrustacean.github.io/taxus/theory/identity.html):
//!   content file, slug, node path and URL path
//! - [Derivations](https://crustyrustacean.github.io/taxus/theory/derivations.html):
//!   the pure functions over the tree
//! - [Glossary](https://crustyrustacean.github.io/taxus/theory/glossary.html):
//!   the vocabulary this crate's docs use
//!
//! ## Modules
//!
//! - [`schema`]: the frontmatter, typed. What an author writes between
//!   `+++` lines.
//! - [`identity`]: [`Slug`], [`NodePath`] and [`UrlPath`]. How a node is
//!   named and how its address is derived from that name.
//! - [`tree`]: [`SiteTree`], [`SectionNode`] and [`PageNode`], plus the
//!   [`SiteTreeBuilder`] that assembles them and the ordering listings use.
//! - [`derivation`]: pure functions of the tree: tree order, reachability,
//!   recent pages, aggregation (`pages_from`), taxonomy grouping.
//!
//! ## Invariants
//!
//! 1. **Structure is containment only.** A section's `pages` and
//!    `subsections` are its direct children. Anything deeper is a query
//!    ([`derivation::descendant_pages`]).
//! 2. **Nothing derived is stored.** Listings, orders, groupings and
//!    addresses are recomputed from the tree on request.
//! 3. **Addresses have one derivation point.** A node's URL path comes
//!    from its node path through [`UrlPath::from_node_path`] and nothing
//!    else.
//! 4. **Paths are final when they enter the tree.** The caller applies
//!    slug overrides and strips date prefixes before calling the builder.
//!
//! ## Example
//!
//! ```
//! use std::path::PathBuf;
//! use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder, UrlPath};
//! use taxus_domain::derivation::documents;
//!
//! let mut builder = SiteTreeBuilder::new();
//! builder.add_section(
//!     &NodePath::parse("blog")?,
//!     Some(PathBuf::from("blog/_index.md")),
//!     Frontmatter { title: "Blog".into(), ..Frontmatter::default() },
//!     None,
//! )?;
//! builder.add_page(
//!     &NodePath::parse("blog/project-launch")?,
//!     PathBuf::from("blog/2026-04-03-project-launch.md"),
//!     Frontmatter { title: "Project Launch".into(), ..Frontmatter::default() },
//!     String::from("### Ready, Set, Go!"),
//! )?;
//! let tree = builder.build()?;
//!
//! let post = tree.get_page(&NodePath::parse("blog/project-launch")?).unwrap();
//! assert_eq!(UrlPath::from_node_path(&post.path).as_str(), "/blog/project-launch/");
//! // Tree order: the blog's index file, then its page.
//! assert_eq!(documents(&tree).len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![warn(missing_docs)]

pub mod derivation;
pub mod identity;
pub mod schema;
pub mod tree;

pub use identity::{NodePath, Slug, UrlPath};
pub use schema::{Frontmatter, SortBy};
pub use tree::{PageNode, SectionNode, SiteTree, SiteTreeBuilder, TreeError};
