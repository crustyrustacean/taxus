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

/// A leaf node: an authored document.
#[derive(Debug, Clone)]
pub struct PageNode {
    /// Membership path from the root (e.g. `["blog", "my-post"]`).
    pub path: NodePath,
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

    /// Published (non-draft) pages, newest first; undated pages last.
    pub fn recent(&self) -> Vec<&PageNode> {
        let mut pages: Vec<&PageNode> = self.iter_pages().filter(|p| !p.is_draft()).collect();
        sort_pages(&mut pages, SortBy::Date);
        pages
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
#[derive(Debug, Default)]
pub struct SiteTreeBuilder {
    sections: Vec<(NodePath, Frontmatter, Option<String>)>,
    pages: Vec<(NodePath, Frontmatter, String)>,
}

impl SiteTreeBuilder {
    /// Start with the root section (default frontmatter, no body).
    pub fn new() -> Self {
        Self {
            sections: vec![(NodePath::root(), Frontmatter::default(), None)],
            pages: Vec::new(),
        }
    }

    /// Set the root section's schema and body.
    pub fn root(mut self, meta: Frontmatter, body: Option<String>) -> Self {
        self.sections[0] = (NodePath::root(), meta, body);
        self
    }

    /// Declare a section (its `_index.md` metadata and body, if any).
    pub fn add_section(
        &mut self,
        path: &NodePath,
        meta: Frontmatter,
        body: Option<String>,
    ) -> Result<(), TreeError> {
        if path.is_root() {
            return Err(TreeError::RootReserved);
        }
        if self.sections.iter().any(|(p, ..)| p == path) {
            return Err(TreeError::Duplicate {
                path: path.to_string(),
            });
        }
        if self.pages.iter().any(|(p, ..)| p == path) {
            return Err(TreeError::Collision {
                path: path.to_string(),
                kind: "page",
            });
        }
        self.sections.push((path.clone(), meta, body));
        Ok(())
    }

    /// Declare a page.
    pub fn add_page(
        &mut self,
        path: &NodePath,
        meta: Frontmatter,
        body: String,
    ) -> Result<(), TreeError> {
        if path.is_root() {
            return Err(TreeError::RootReserved);
        }
        if self.pages.iter().any(|(p, ..)| p == path) {
            return Err(TreeError::Duplicate {
                path: path.to_string(),
            });
        }
        if self.sections.iter().any(|(p, ..)| p == path) {
            return Err(TreeError::Collision {
                path: path.to_string(),
                kind: "section",
            });
        }
        self.pages.push((path.clone(), meta, body));
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
            .filter_map(|(p, ..)| p.parent())
            .chain(self.sections.iter().filter_map(|(p, ..)| p.parent()))
            .collect();
        parents.sort();
        parents.dedup();
        for parent in parents {
            let mut cursor = NodePath::root();
            for slug in parent.segments() {
                cursor = cursor.join(slug);
                if !self.sections.iter().any(|(p, ..)| p == &cursor) {
                    self.sections
                        .push((cursor.clone(), Frontmatter::default(), None));
                }
            }
        }

        let SiteTreeBuilder { sections, pages } = self;

        let mut sections: Vec<SectionNode> = sections
            .into_iter()
            .map(|(path, meta, body)| SectionNode {
                path,
                meta,
                body,
                pages: Vec::new(),
                subsections: Vec::new(),
            })
            .collect();

        // Attach pages to their (guaranteed-existing) parent sections.
        for (path, meta, body) in pages {
            let parent_path = path.parent().expect("non-root page has a parent");
            let parent = sections
                .iter_mut()
                .find(|s| s.path == parent_path)
                .expect("intermediate sections auto-created");
            parent.pages.push(PageNode { path, meta, body });
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
    use crate::identity::Slug;

    #[test]
    fn builds_nested_tree_with_queries() {
        let mut builder = SiteTreeBuilder::new();
        builder
            .add_page(
                &NodePath::parse("about").unwrap(),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();
        builder
            .add_page(
                &NodePath::parse("blog/my-post").unwrap(),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();
        builder
            .add_section(
                &NodePath::parse("blog").unwrap(),
                Frontmatter::default(),
                Some("intro".into()),
            )
            .unwrap();
        builder
            .add_page(
                &NodePath::parse("blog/other").unwrap(),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();

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
    fn auto_creates_intermediate_sections() {
        let mut builder = SiteTreeBuilder::new();
        builder
            .add_page(
                &NodePath::parse("a/b/c/post").unwrap(),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();
        let tree = builder.build().unwrap();
        assert!(
            tree.get_section(&NodePath::parse("a/b/c").unwrap())
                .is_some()
        );
        assert!(
            tree.get_page(&NodePath::parse("a/b/c/post").unwrap())
                .is_some()
        );
    }

    #[test]
    fn rejects_duplicates_and_collisions() {
        let mut builder = SiteTreeBuilder::new();
        let path = NodePath::parse("x").unwrap();
        builder
            .add_page(&path, Frontmatter::default(), String::new())
            .unwrap();
        assert!(matches!(
            builder.add_page(&path, Frontmatter::default(), String::new()),
            Err(TreeError::Duplicate { .. })
        ));
        assert!(matches!(
            builder.add_section(&path, Frontmatter::default(), None),
            Err(TreeError::Collision { .. })
        ));
    }

    #[test]
    fn rejects_root_and_duplicate_sections() {
        let mut builder = SiteTreeBuilder::new();
        assert!(matches!(
            builder.add_page(&NodePath::root(), Frontmatter::default(), String::new()),
            Err(TreeError::RootReserved)
        ));
        let section = NodePath::parse("s").unwrap();
        builder
            .add_section(&section, Frontmatter::default(), None)
            .unwrap();
        assert!(matches!(
            builder.add_section(&section, Frontmatter::default(), None),
            Err(TreeError::Duplicate { .. })
        ));
    }

    #[test]
    fn recent_sorts_by_date_desc_and_filters_drafts() {
        let mut builder = SiteTreeBuilder::new();
        builder
            .add_page(
                &NodePath::parse("old").unwrap(),
                Frontmatter {
                    date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                    ..Frontmatter::default()
                },
                String::new(),
            )
            .unwrap();
        builder
            .add_page(
                &NodePath::parse("new").unwrap(),
                Frontmatter {
                    date: Some(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
                    ..Frontmatter::default()
                },
                String::new(),
            )
            .unwrap();
        builder
            .add_page(
                &NodePath::parse("secret").unwrap(),
                Frontmatter {
                    date: Some(chrono::NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()),
                    draft: true,
                    ..Frontmatter::default()
                },
                String::new(),
            )
            .unwrap();
        builder
            .add_page(
                &NodePath::parse("undated").unwrap(),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();

        let tree = builder.build().unwrap();
        let recent: Vec<&str> = tree
            .recent()
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        assert_eq!(recent, ["new", "old", "undated"]);
    }

    #[test]
    fn children_are_sorted_by_slug() {
        let mut builder = SiteTreeBuilder::new();
        for slug in ["zulu", "alpha", "mike"] {
            builder
                .add_page(
                    &NodePath::parse(slug).unwrap(),
                    Frontmatter::default(),
                    String::new(),
                )
                .unwrap();
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
        builder
            .add_page(
                &NodePath::parse("blog/deep-post").unwrap(),
                Frontmatter::default(),
                String::new(),
            )
            .unwrap();
        let tree = builder.build().unwrap();
        let p = tree
            .get_page(&NodePath::parse("blog/deep-post").unwrap())
            .unwrap();
        assert_eq!(p.path.last().unwrap().as_str(), "deep-post");
        assert_eq!(Slug::new("deep-post").unwrap().as_str(), "deep-post");
    }
}
