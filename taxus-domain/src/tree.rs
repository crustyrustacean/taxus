// taxus-domain/src/tree.rs

//! The Site Tree: the in-memory model of a site, built once per build.
//!
//! A site is a tree of sections (directories) and pages (documents). This
//! module defines the two node types, [`SectionNode`] and [`PageNode`],
//! the [`SiteTree`] that holds them, the [`SiteTreeBuilder`] that
//! assembles a tree from flat additions, and [`sort_pages`], the one
//! ordering listings use.
//!
//! Invariants:
//! - `pages` and `subsections` are **direct children only**: structure is
//!   containment. Reachability is a query, never a field (see
//!   [`crate::derivation::descendant_pages`]).
//! - Children are kept sorted by slug, so construction is deterministic.
//! - The tree is immutable once built; there is no method that changes a
//!   node after [`SiteTreeBuilder::build`] returns.
//!
//! See the book's
//! [Site Tree](https://crustyrustacean.github.io/taxus/theory/site-tree.html)
//! chapter.

use crate::identity::NodePath;
use crate::schema::{Frontmatter, SortBy};
use std::cmp::Ordering;
use std::path::PathBuf;

/// A page: a leaf of the tree, one authored content file that is not an
/// `_index.md`.
///
/// It holds what was read from disk and nothing computed from it: no
/// rendered HTML, no URL, no summary. Those are derived downstream so
/// that they can never go stale.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, PageNode};
///
/// let post = PageNode {
///     path: NodePath::parse("blog/project-launch")?,
///     content_file: PathBuf::from("blog/2026-04-03-project-launch.md"),
///     meta: Frontmatter { title: "Project Launch".into(), ..Frontmatter::default() },
///     body: String::from("### Ready, Set, Go!"),
/// };
/// assert!(!post.is_draft());
/// assert_eq!(post.path.last().unwrap().as_str(), "project-launch");
/// # Ok::<(), taxus_domain::identity::IdentityError>(())
/// ```
#[derive(Debug, Clone)]
pub struct PageNode {
    /// Membership path from the root (e.g. `["blog", "my-post"]`).
    pub path: NodePath,
    /// Source file, relative to the content directory
    /// (e.g. `blog/2026-04-06-my-post.md`). Storage, not identity: the
    /// membership path is `path`, never this.
    pub content_file: PathBuf,
    /// Schema (frontmatter).
    pub meta: Frontmatter,
    /// Raw Markdown body; rendering to HTML happens in the generator.
    pub body: String,
}

impl PageNode {
    /// Is this page a draft, excluded unless the build includes drafts?
    pub fn is_draft(&self) -> bool {
        self.meta.draft
    }
}

/// A section: a directory inside the content directory, which groups
/// pages and other sections and is itself a document when it has an
/// `_index.md`.
///
/// Every directory is a section, index file or not; one without an
/// `_index.md` has default frontmatter, no body, and nothing to render,
/// but its children are still in the tree. `pages` and `subsections` are
/// direct children only, so a section's listing means "what this
/// directory contains" and nothing deeper.
#[derive(Debug, Clone)]
pub struct SectionNode {
    /// Membership path from the root; the root section's path is empty.
    pub path: NodePath,
    /// The section's `_index.md`, relative to the content directory;
    /// `None` when the directory has no index file (the section still
    /// exists, with default frontmatter).
    pub content_file: Option<PathBuf>,
    /// Schema from `_index.md` (default frontmatter if the file is absent).
    pub meta: Frontmatter,
    /// Body of `_index.md`, if present.
    pub body: Option<String>,
    /// Direct child pages, sorted by slug.
    pub pages: Vec<PageNode>,
    /// Direct child sections, sorted by slug.
    pub subsections: Vec<SectionNode>,
}

/// The Site Tree: one site, fully parsed, as a tree of sections and pages.
///
/// It is built once per build by [`SiteTreeBuilder::build`] and read by
/// every later stage; nothing mutates it afterwards. Because every list,
/// feed and address is derived from this one value, they cannot disagree
/// with each other.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder};
///
/// let mut builder = SiteTreeBuilder::new();
/// builder.add_page(
///     &NodePath::parse("blog/project-launch")?,
///     PathBuf::from("blog/2026-04-03-project-launch.md"),
///     Frontmatter::default(),
///     String::new(),
/// )?;
/// let tree = builder.build()?;
///
/// // `blog` was created for the page even though no _index.md declared it.
/// let blog = tree.get_section(&NodePath::parse("blog")?).unwrap();
/// assert!(blog.content_file.is_none());
/// assert_eq!(blog.pages.len(), 1);
/// assert_eq!(tree.iter_pages().count(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct SiteTree {
    /// The root section: the content directory itself, with node path `[]`.
    pub root: SectionNode,
}

impl SiteTree {
    /// Look up a section by its node path; `None` if no such section.
    pub fn get_section(&self, path: &NodePath) -> Option<&SectionNode> {
        let mut node = &self.root;
        for slug in path.segments() {
            node = node
                .subsections
                .iter()
                .find(|s| s.path.last() == Some(slug))?;
        }
        Some(node)
    }

    /// Look up a page by its node path; `None` if no such page.
    pub fn get_page(&self, path: &NodePath) -> Option<&PageNode> {
        let parent = path.parent()?;
        let section = self.get_section(&parent)?;
        section.pages.iter().find(|p| &p.path == path)
    }

    /// Every page in the site, depth-first from the root, drafts included.
    ///
    /// This is [`crate::derivation::descendant_pages`] applied to the
    /// root; draft filtering is the caller's choice.
    pub fn iter_pages(&self) -> impl Iterator<Item = &PageNode> {
        crate::derivation::descendant_pages(&self.root).into_iter()
    }
}

/// Why a tree could not be built.
///
/// Every variant is a violation of "one node per node path". The builder
/// reports them before any output is written, so a site with two files
/// that resolve to the same address never half-builds.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    /// Two pages, or two sections, were declared at the same node path.
    #[error("duplicate node at `{path}`")]
    Duplicate {
        /// The node path, as `a/b`.
        path: String,
    },
    /// A page and a section were declared at the same node path.
    #[error("`{path}` collides with an existing {kind}")]
    Collision {
        /// The node path, as `a/b`.
        path: String,
        /// What was already there: `"page"` or `"section"`.
        kind: &'static str,
    },
    /// A page or a non-root section was declared at the root path.
    #[error("the root path is reserved")]
    RootReserved,
    /// A segment could not be a slug.
    #[error(transparent)]
    Identity(#[from] crate::identity::IdentityError),
}

/// Assembles a [`SiteTree`] from flat node additions.
///
/// The generator walks the content directory in whatever order the
/// filesystem gives it and calls [`add_page`](Self::add_page) and
/// [`add_section`](Self::add_section) as it goes; the builder turns that
/// flat sequence into a tree with a fixed, slug-sorted shape. Intermediate
/// sections referenced by a path but never declared are created with
/// default frontmatter and no body, so a directory without `_index.md` is
/// still a section.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder};
///
/// let mut builder = SiteTreeBuilder::new().root(
///     Some(PathBuf::from("_index.md")),
///     Frontmatter { title: "Home".into(), ..Frontmatter::default() },
///     Some(String::from("# Welcome")),
/// );
/// builder.add_page(
///     &NodePath::parse("about")?,
///     PathBuf::from("about.md"),
///     Frontmatter::default(),
///     String::new(),
/// )?;
/// let tree = builder.build()?;
/// assert_eq!(tree.root.meta.title, "Home");
/// assert_eq!(tree.root.pages[0].path.to_string(), "about");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Paths are final
///
/// The [`NodePath`] given to [`add_page`](Self::add_page) and
/// [`add_section`](Self::add_section) is the node's **final membership
/// path**: the caller has already slugified every segment, stripped any
/// `YYYY-MM-DD-` filename prefix, and applied a frontmatter `slug` override
/// to the last segment. The builder stores it as-is; [`crate::UrlPath`] is
/// derived from it and from nothing else.
#[derive(Debug, Default)]
pub struct SiteTreeBuilder {
    sections: Vec<PendingSection>,
    pages: Vec<PendingPage>,
}

#[derive(Debug)]
struct PendingSection {
    path: NodePath,
    content_file: Option<PathBuf>,
    meta: Frontmatter,
    body: Option<String>,
}

#[derive(Debug)]
struct PendingPage {
    path: NodePath,
    content_file: PathBuf,
    meta: Frontmatter,
    body: String,
}

impl SiteTreeBuilder {
    /// Start with the root section (default frontmatter, no body).
    pub fn new() -> Self {
        Self {
            sections: vec![PendingSection {
                path: NodePath::root(),
                content_file: None,
                meta: Frontmatter::default(),
                body: None,
            }],
            pages: Vec::new(),
        }
    }

    /// Set the root section's index file, schema and body.
    pub fn root(
        mut self,
        content_file: Option<PathBuf>,
        meta: Frontmatter,
        body: Option<String>,
    ) -> Self {
        self.sections[0] = PendingSection {
            path: NodePath::root(),
            content_file,
            meta,
            body,
        };
        self
    }

    /// Declare a section (its `_index.md` metadata and body, if any).
    ///
    /// `path` is the section's final membership path (see the type-level
    /// docs): already slugified, with any `slug` override applied.
    /// `content_file` is the `_index.md` relative to the content directory,
    /// or `None` for a directory without one.
    pub fn add_section(
        &mut self,
        path: &NodePath,
        content_file: Option<PathBuf>,
        meta: Frontmatter,
        body: Option<String>,
    ) -> Result<(), TreeError> {
        if path.is_root() {
            return Err(TreeError::RootReserved);
        }
        if self.sections.iter().any(|s| &s.path == path) {
            return Err(TreeError::Duplicate {
                path: path.to_string(),
            });
        }
        if self.pages.iter().any(|p| &p.path == path) {
            return Err(TreeError::Collision {
                path: path.to_string(),
                kind: "page",
            });
        }
        self.sections.push(PendingSection {
            path: path.clone(),
            content_file,
            meta,
            body,
        });
        Ok(())
    }

    /// Declare a page.
    ///
    /// `path` is the page's final membership path (see the type-level
    /// docs): the parent section's path plus the page's slug, where the
    /// slug is the frontmatter `slug` if set, else the slugified,
    /// date-prefix-stripped file stem. `content_file` is the source file
    /// relative to the content directory.
    pub fn add_page(
        &mut self,
        path: &NodePath,
        content_file: PathBuf,
        meta: Frontmatter,
        body: String,
    ) -> Result<(), TreeError> {
        if path.is_root() {
            return Err(TreeError::RootReserved);
        }
        if self.pages.iter().any(|p| &p.path == path) {
            return Err(TreeError::Duplicate {
                path: path.to_string(),
            });
        }
        if self.sections.iter().any(|s| &s.path == path) {
            return Err(TreeError::Collision {
                path: path.to_string(),
                kind: "section",
            });
        }
        self.pages.push(PendingPage {
            path: path.clone(),
            content_file,
            meta,
            body,
        });
        Ok(())
    }

    /// Assemble the tree.
    ///
    /// Intermediate sections are auto-created with default frontmatter;
    /// children are ordered by slug. Pages attach to their parents first
    /// (all sections present), then sections attach deepest-first so a
    /// parent is always still available when its children come up.
    pub fn build(mut self) -> Result<SiteTree, TreeError> {
        // Auto-create intermediate sections for every referenced parent.
        let mut parents: Vec<NodePath> = self
            .pages
            .iter()
            .filter_map(|p| p.path.parent())
            .chain(self.sections.iter().filter_map(|s| s.path.parent()))
            .collect();
        parents.sort();
        parents.dedup();
        for parent in parents {
            let mut cursor = NodePath::root();
            for slug in parent.segments() {
                cursor = cursor.join(slug);
                if !self.sections.iter().any(|s| s.path == cursor) {
                    self.sections.push(PendingSection {
                        path: cursor.clone(),
                        content_file: None,
                        meta: Frontmatter::default(),
                        body: None,
                    });
                }
            }
        }

        let SiteTreeBuilder { sections, pages } = self;

        let mut sections: Vec<SectionNode> = sections
            .into_iter()
            .map(|s| SectionNode {
                path: s.path,
                content_file: s.content_file,
                meta: s.meta,
                body: s.body,
                pages: Vec::new(),
                subsections: Vec::new(),
            })
            .collect();

        // Attach pages to their (guaranteed-existing) parent sections.
        for p in pages {
            let parent_path = p.path.parent().expect("non-root page has a parent");
            let parent = sections
                .iter_mut()
                .find(|s| s.path == parent_path)
                .expect("intermediate sections auto-created");
            parent.pages.push(PageNode {
                path: p.path,
                content_file: p.content_file,
                meta: p.meta,
                body: p.body,
            });
        }

        // Attach sections deepest-first; children sorted by slug.
        sections.sort_by_key(|s| s.path.segments().len());
        while sections.len() > 1 {
            let mut node = sections.pop().expect("non-empty");
            node.pages.sort_by(|a, b| a.path.last().cmp(&b.path.last()));
            node.subsections
                .sort_by(|a, b| a.path.last().cmp(&b.path.last()));
            let parent_path = node.path.parent().expect("non-root section has a parent");
            let parent = sections
                .iter_mut()
                .find(|s| s.path == parent_path)
                .expect("parent is shallower and still in the list");
            parent.subsections.push(node);
        }

        let mut root = sections.pop().ok_or(TreeError::RootReserved)?;
        root.pages.sort_by(|a, b| a.path.last().cmp(&b.path.last()));
        root.subsections
            .sort_by(|a, b| a.path.last().cmp(&b.path.last()));
        Ok(SiteTree { root })
    }
}

/// Sorting: put a list of pages in the order a section's `sort_by` asks for.
///
/// This is the one ordering rule every listing, feed and `get_section`
/// result shares, so two views of the same section can never disagree
/// about order. Date: newest first, undated last. Title: case-insensitive
/// ascending. Weight: lowest first. None: keep the input order. The sort
/// is stable, so ties keep tree (slug) order.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, PageNode, SortBy};
/// use taxus_domain::tree::sort_pages;
///
/// let page = |slug: &str, weight: i32| PageNode {
///     path: NodePath::parse(slug).unwrap(),
///     content_file: PathBuf::from(format!("{slug}.md")),
///     meta: Frontmatter { weight, ..Frontmatter::default() },
///     body: String::new(),
/// };
/// let (a, b) = (page("a", 2), page("b", 1));
/// let mut pages = vec![&a, &b];
/// sort_pages(&mut pages, SortBy::Weight);
/// assert_eq!(pages[0].path.to_string(), "b");
/// ```
pub fn sort_pages(pages: &mut [&PageNode], by: SortBy) {
    match by {
        SortBy::None => {}
        SortBy::Date => pages.sort_by(|a, b| match (&a.meta.date, &b.meta.date) {
            (Some(a), Some(b)) => b.cmp(a),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }),
        SortBy::Title => pages.sort_by(|a, b| {
            a.meta
                .title
                .to_lowercase()
                .cmp(&b.meta.title.to_lowercase())
        }),
        SortBy::Weight => pages.sort_by_key(|p| p.meta.weight),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{Slug, UrlPath};
    use std::path::Path;

    fn add_page(builder: &mut SiteTreeBuilder, path: &str) {
        add_page_with(builder, path, Frontmatter::default());
    }

    fn add_page_with(builder: &mut SiteTreeBuilder, path: &str, meta: Frontmatter) {
        builder
            .add_page(
                &NodePath::parse(path).unwrap(),
                PathBuf::from(format!("{path}.md")),
                meta,
                String::new(),
            )
            .unwrap();
    }

    fn add_section(builder: &mut SiteTreeBuilder, path: &str, body: Option<&str>) {
        builder
            .add_section(
                &NodePath::parse(path).unwrap(),
                Some(PathBuf::from(format!("{path}/_index.md"))),
                Frontmatter::default(),
                body.map(str::to_owned),
            )
            .unwrap();
    }

    #[test]
    fn builds_nested_tree_with_queries() {
        let mut builder = SiteTreeBuilder::new();
        add_page(&mut builder, "about");
        add_page(&mut builder, "blog/my-post");
        add_section(&mut builder, "blog", Some("intro"));
        add_page(&mut builder, "blog/other");

        let tree = builder.build().unwrap();

        assert_eq!(
            tree.get_section(&NodePath::parse("blog").unwrap())
                .unwrap()
                .body
                .as_deref(),
            Some("intro")
        );
        assert!(
            tree.get_page(&NodePath::parse("blog/my-post").unwrap())
                .is_some()
        );
        assert!(
            tree.get_page(&NodePath::parse("blog/missing").unwrap())
                .is_none()
        );
        assert!(
            tree.get_section(&NodePath::parse("missing").unwrap())
                .is_none()
        );

        let blog = tree.get_section(&NodePath::parse("blog").unwrap()).unwrap();
        assert_eq!(blog.pages.len(), 2);
        // "about" is a direct child of the root; blog posts are not.
        assert_eq!(tree.root.pages.len(), 1);
        assert_eq!(tree.root.pages[0].path.last().unwrap().as_str(), "about");
        assert_eq!(tree.iter_pages().count(), 3);
    }

    #[test]
    fn nodes_carry_their_content_file() {
        let mut builder = SiteTreeBuilder::new().root(
            Some(PathBuf::from("_index.md")),
            Frontmatter::default(),
            None,
        );
        add_section(&mut builder, "blog", None);
        builder
            .add_page(
                &NodePath::parse("blog/my-post").unwrap(),
                PathBuf::from("blog/2026-04-06-my-post.md"),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();
        let tree = builder.build().unwrap();

        assert_eq!(
            tree.root.content_file.as_deref(),
            Some(Path::new("_index.md"))
        );
        let blog = tree.get_section(&NodePath::parse("blog").unwrap()).unwrap();
        assert_eq!(
            blog.content_file.as_deref(),
            Some(Path::new("blog/_index.md"))
        );
        let post = tree
            .get_page(&NodePath::parse("blog/my-post").unwrap())
            .unwrap();
        // Storage keeps the date prefix; the membership path does not.
        assert_eq!(post.content_file, Path::new("blog/2026-04-06-my-post.md"));
        assert_eq!(post.path.to_string(), "blog/my-post");
    }

    #[test]
    fn auto_creates_intermediate_sections() {
        let mut builder = SiteTreeBuilder::new();
        add_page(&mut builder, "a/b/c/post");
        let tree = builder.build().unwrap();
        let c = tree
            .get_section(&NodePath::parse("a/b/c").unwrap())
            .expect("intermediate section exists");
        // Auto-created sections have no index file and default metadata.
        assert!(c.content_file.is_none());
        assert!(c.body.is_none());
        assert!(
            tree.get_page(&NodePath::parse("a/b/c/post").unwrap())
                .is_some()
        );
    }

    #[test]
    fn path_is_final_slug_override_is_the_callers_job() {
        // The caller resolved `slug = "custom-slug"` on a file whose title
        // (and stem) say otherwise; the builder trusts the path it is given
        // and the address derives from that path alone.
        let mut builder = SiteTreeBuilder::new();
        add_page_with(
            &mut builder,
            "blog/custom-slug",
            Frontmatter {
                title: "Something Else Entirely".into(),
                slug: Some("custom-slug".into()),
                ..Frontmatter::default()
            },
        );
        let tree = builder.build().unwrap();
        let page = tree
            .get_page(&NodePath::parse("blog/custom-slug").unwrap())
            .unwrap();
        assert_eq!(page.meta.title, "Something Else Entirely");
        assert_eq!(
            UrlPath::from_node_path(&page.path).as_str(),
            "/blog/custom-slug/"
        );
    }

    #[test]
    fn rejects_duplicates_and_collisions() {
        let mut builder = SiteTreeBuilder::new();
        let path = NodePath::parse("x").unwrap();
        builder
            .add_page(
                &path,
                PathBuf::from("x.md"),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();
        assert!(matches!(
            builder.add_page(
                &path,
                PathBuf::from("x-again.md"),
                Frontmatter::default(),
                String::new()
            ),
            Err(TreeError::Duplicate { .. })
        ));
        assert!(matches!(
            builder.add_section(&path, None, Frontmatter::default(), None),
            Err(TreeError::Collision { .. })
        ));
    }

    #[test]
    fn rejects_root_and_duplicate_sections() {
        let mut builder = SiteTreeBuilder::new();
        assert!(matches!(
            builder.add_page(
                &NodePath::root(),
                PathBuf::from("_index.md"),
                Frontmatter::default(),
                String::new()
            ),
            Err(TreeError::RootReserved)
        ));
        let section = NodePath::parse("s").unwrap();
        builder
            .add_section(&section, None, Frontmatter::default(), None)
            .unwrap();
        assert!(matches!(
            builder.add_section(&section, None, Frontmatter::default(), None),
            Err(TreeError::Duplicate { .. })
        ));
    }

    #[test]
    fn children_are_sorted_by_slug() {
        let mut builder = SiteTreeBuilder::new();
        for slug in ["zulu", "alpha", "mike"] {
            add_page(&mut builder, slug);
        }
        let tree = builder.build().unwrap();
        let slugs: Vec<&str> = tree
            .root
            .pages
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        assert_eq!(slugs, ["alpha", "mike", "zulu"]);
    }

    #[test]
    fn slug_derives_from_path() {
        let mut builder = SiteTreeBuilder::new();
        add_page(&mut builder, "blog/deep-post");
        let tree = builder.build().unwrap();
        let p = tree
            .get_page(&NodePath::parse("blog/deep-post").unwrap())
            .unwrap();
        assert_eq!(p.path.last().unwrap().as_str(), "deep-post");
        assert_eq!(Slug::new("deep-post").unwrap().as_str(), "deep-post");
    }

    #[test]
    fn sort_pages_orders_by_weight_then_keeps_input_order() {
        let mut builder = SiteTreeBuilder::new();
        for (slug, weight) in [("a", 3), ("b", 1), ("c", 2), ("d", 1)] {
            add_page_with(
                &mut builder,
                slug,
                Frontmatter {
                    weight,
                    ..Frontmatter::default()
                },
            );
        }
        let tree = builder.build().unwrap();
        let mut pages: Vec<&PageNode> = tree.root.pages.iter().collect();
        sort_pages(&mut pages, SortBy::Weight);
        let slugs: Vec<&str> = pages
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        // Stable: b and d tie on weight and keep their slug order.
        assert_eq!(slugs, ["b", "d", "c", "a"]);
    }

    fn slugs_sorted_by(tree: &SiteTree, by: SortBy) -> Vec<String> {
        let mut pages: Vec<&PageNode> = tree.root.pages.iter().collect();
        sort_pages(&mut pages, by);
        pages
            .iter()
            .map(|p| p.path.last().unwrap().as_str().to_string())
            .collect()
    }

    #[test]
    fn sort_pages_by_date_is_newest_first_undated_last() {
        let mut builder = SiteTreeBuilder::new();
        for (slug, date) in [
            ("undated", None),
            ("older", Some("2026-01-01")),
            ("newer", Some("2026-03-01")),
        ] {
            add_page_with(
                &mut builder,
                slug,
                Frontmatter {
                    date: date.map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap()),
                    ..Frontmatter::default()
                },
            );
        }
        let tree = builder.build().unwrap();
        assert_eq!(
            slugs_sorted_by(&tree, SortBy::Date),
            ["newer", "older", "undated"]
        );
    }

    #[test]
    fn sort_pages_by_title_is_case_insensitive() {
        let mut builder = SiteTreeBuilder::new();
        for (slug, title) in [("c", "cherry"), ("b", "Banana"), ("a", "apple")] {
            add_page_with(
                &mut builder,
                slug,
                Frontmatter {
                    title: title.to_string(),
                    ..Frontmatter::default()
                },
            );
        }
        let tree = builder.build().unwrap();
        // Byte order would put "Banana" before "apple".
        assert_eq!(slugs_sorted_by(&tree, SortBy::Title), ["a", "b", "c"]);
    }
}
