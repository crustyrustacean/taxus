// taxus-domain/src/tree.rs

//! The Site Tree: the in-memory model of a site's containment structure.
//!
//! Invariants:
//! - `pages` and `subsections` are **direct children only** — structure
//!   contains only containment. Reachability is a query, never a property
//!   of the structure (see [`crate::derivation`]).
//! - Children are kept sorted by slug, so construction is deterministic.

use crate::identity::NodePath;
use crate::schema::{Frontmatter, SortBy};
use std::cmp::Ordering;
use std::path::PathBuf;

/// A leaf node: an authored document.
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
    /// Raw markdown body (Document Tree extraction happens downstream).
    pub body: String,
}

impl PageNode {
    /// Is this page excluded from release builds?
    pub fn is_draft(&self) -> bool {
        self.meta.draft
    }
}

/// An inner node: a directory that groups pages and subsections, and is
/// itself a document (its `_index.md`, if present).
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

/// A fully constructed site tree.
#[derive(Debug, Clone)]
pub struct SiteTree {
    pub root: SectionNode,
}

impl SiteTree {
    /// Look up a section by membership path.
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

    /// Look up a page by membership path.
    pub fn get_page(&self, path: &NodePath) -> Option<&PageNode> {
        let parent = path.parent()?;
        let section = self.get_section(&parent)?;
        section.pages.iter().find(|p| &p.path == path)
    }

    /// Every page in the site, depth-first, including drafts.
    pub fn iter_pages(&self) -> impl Iterator<Item = &PageNode> {
        fn walk<'a>(section: &'a SectionNode, out: &mut Vec<&'a PageNode>) {
            out.extend(section.pages.iter());
            for sub in &section.subsections {
                walk(sub, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out.into_iter()
    }
}

/// Errors from tree construction.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    #[error("duplicate node at `{path}`")]
    Duplicate { path: String },
    #[error("`{path}` collides with an existing {kind}")]
    Collision { path: String, kind: &'static str },
    #[error("the root path is reserved")]
    RootReserved,
    #[error(transparent)]
    Identity(#[from] crate::identity::IdentityError),
}

/// Assembles a [`SiteTree`] from flat node additions.
///
/// Intermediate sections referenced by a page or section path but never
/// declared are created with default frontmatter and no body — matching the
/// generator's "sections without `_index.md` are still sections" behavior.
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

/// Deterministic ordering for derived listings.
///
/// Date: newest first, undated last. Title: case-insensitive ascending.
/// Weight: lowest first. None: preserve order.
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
}
