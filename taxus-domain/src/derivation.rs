//! Derivations: pure functions of the Site Tree that return a view.
//!
//! Anything you can compute, you don't store. A derivation takes the tree
//! (and plain values such as a `SortBy` or a list of node paths), reads
//! nothing else, writes nothing, and returns borrowed nodes. Each one is a
//! `(source set, filter, order, grouping)`: [`documents`] is the source
//! set in tree order, [`descendant_pages`] is reachability,
//! [`recent`] filters and orders, [`aggregate`] merges, and
//! [`group_by_terms`] groups.
//!
//! Draft filtering is the caller's concern (the generator decides whether
//! drafts participate in a given build), so every derivation that filters
//! drafts takes an explicit `include_drafts`.
//!
//! See the book's
//! [Derivations](https://crustyrustacean.github.io/taxus/theory/derivations.html)
//! chapter, including the rule for deciding whether something is a tree
//! method or a derivation.

use crate::identity::NodePath;
use crate::schema::{Frontmatter, SortBy};
use crate::tree::{PageNode, SectionNode, SiteTree, sort_pages};
use std::collections::BTreeMap;
use std::path::Path;

/// A document: a page, or a section that has an `_index.md`, borrowed
/// from the tree.
///
/// Derivations that range over "everything the site renders" (routes,
/// the sitemap, taxonomies) yield these, so their callers can treat a
/// section's index file and a page alike without two code paths.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder};
/// use taxus_domain::derivation::documents;
///
/// let builder = SiteTreeBuilder::new().root(
///     Some(PathBuf::from("_index.md")),
///     Frontmatter::default(),
///     None,
/// );
/// let tree = builder.build()?;
/// let root = documents(&tree)[0];
/// assert!(root.is_section());
/// assert_eq!(root.path(), &NodePath::root());
/// assert_eq!(root.content_file(), std::path::Path::new("_index.md"));
/// # Ok::<(), taxus_domain::TreeError>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Node<'a> {
    /// A section with an index file.
    Section(&'a SectionNode),
    /// A page.
    Page(&'a PageNode),
}

impl<'a> Node<'a> {
    /// The node path of either kind.
    pub fn path(&self) -> &'a NodePath {
        match self {
            Node::Section(s) => &s.path,
            Node::Page(p) => &p.path,
        }
    }

    /// The frontmatter of either kind.
    pub fn meta(&self) -> &'a Frontmatter {
        match self {
            Node::Section(s) => &s.meta,
            Node::Page(p) => &p.meta,
        }
    }

    /// The source file relative to the content directory. Always present:
    /// [`documents`] only yields sections that have an `_index.md`.
    pub fn content_file(&self) -> &'a Path {
        match self {
            Node::Section(s) => s
                .content_file
                .as_deref()
                .expect("documents() yields only sections with an index file"),
            Node::Page(p) => &p.content_file,
        }
    }

    /// The raw Markdown body of either kind: a page's body, or the
    /// section's `_index.md` body (empty when the section has no index
    /// file, which [`documents`] never yields).
    pub fn body(&self) -> &'a str {
        match self {
            Node::Section(s) => s.body.as_deref().unwrap_or_default(),
            Node::Page(p) => &p.body,
        }
    }

    /// Is this a section (with an index file) rather than a page?
    pub fn is_section(&self) -> bool {
        matches!(self, Node::Section(_))
    }

    /// Is this document a draft?
    pub fn is_draft(&self) -> bool {
        self.meta().draft
    }
}

/// Tree order: every document in the site, in the one canonical order.
///
/// A section's own index (if it has one), then its pages in slug order,
/// then each subsection in slug order, recursively. Drafts are included.
/// Routes are registered in this order and every other derivation starts
/// from it, so anything that keeps input order downstream is
/// deterministic across machines.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder};
/// use taxus_domain::derivation::documents;
///
/// let mut builder = SiteTreeBuilder::new();
/// for path in ["zed", "about", "blog/post"] {
///     builder.add_page(
///         &NodePath::parse(path)?,
///         PathBuf::from(format!("{path}.md")),
///         Frontmatter::default(),
///         String::new(),
///     )?;
/// }
/// let tree = builder.build()?;
/// let order: Vec<String> = documents(&tree).iter().map(|n| n.path().to_string()).collect();
/// // Root pages by slug, then the blog subsection's page.
/// assert_eq!(order, ["about", "zed", "blog/post"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn documents(tree: &SiteTree) -> Vec<Node<'_>> {
    fn walk<'a>(section: &'a SectionNode, out: &mut Vec<Node<'a>>) {
        if section.content_file.is_some() {
            out.push(Node::Section(section));
        }
        out.extend(section.pages.iter().map(Node::Page));
        for sub in &section.subsections {
            walk(sub, out);
        }
    }
    let mut out = Vec::new();
    walk(&tree.root, &mut out);
    out
}

/// Taxonomy grouping: documents grouped by the terms `terms_of` reads
/// from their frontmatter. This is the index behind tags, categories and
/// series.
///
/// Terms come back sorted by name; the documents under a term keep
/// [`documents`] order. A document that lists the same term twice appears
/// twice, as the caller's term-counting expects. Sections with an
/// `_index.md` participate like pages. Nothing is stored: add a tag to
/// one page and the term exists on the next call; remove it and the term
/// is gone.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder};
/// use taxus_domain::derivation::group_by_terms;
///
/// let mut builder = SiteTreeBuilder::new();
/// builder.add_page(
///     &NodePath::parse("blog/project-launch")?,
///     PathBuf::from("blog/2026-04-03-project-launch.md"),
///     Frontmatter { tags: vec!["rust".into(), "ssg".into()], ..Frontmatter::default() },
///     String::new(),
/// )?;
/// let tree = builder.build()?;
/// let tags = group_by_terms(&tree, false, |m| m.tags.iter().map(String::as_str).collect());
/// assert_eq!(tags.keys().collect::<Vec<_>>(), ["rust", "ssg"]);
/// assert_eq!(tags["rust"].len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn group_by_terms<'a, F>(
    tree: &'a SiteTree,
    include_drafts: bool,
    terms_of: F,
) -> BTreeMap<String, Vec<Node<'a>>>
where
    F: Fn(&'a Frontmatter) -> Vec<&'a str>,
{
    let mut groups: BTreeMap<String, Vec<Node<'a>>> = BTreeMap::new();
    for node in documents(tree) {
        if node.is_draft() && !include_drafts {
            continue;
        }
        for term in terms_of(node.meta()) {
            groups.entry(term.to_owned()).or_default().push(node);
        }
    }
    groups
}

/// Reachability: every page in a section's subtree, depth-first.
///
/// The section's own pages (slug order), then each subsection's pages in
/// turn. Reachability is a query, never a field: a section's `pages`
/// holds direct children only, and this is how a caller that wants the
/// whole subtree asks for it.
pub fn descendant_pages(section: &SectionNode) -> Vec<&PageNode> {
    fn walk<'a>(section: &'a SectionNode, out: &mut Vec<&'a PageNode>) {
        out.extend(section.pages.iter());
        for sub in &section.subsections {
            walk(sub, out);
        }
    }
    let mut out = Vec::new();
    walk(section, &mut out);
    out
}

/// Recent pages: every page in the site, newest first, undated last.
///
/// This is what a feed starts from. Section index files are not pages
/// and are not included. Drafts are excluded unless `include_drafts` is
/// set; the generator decides per build whether drafts participate.
pub fn recent(tree: &SiteTree, include_drafts: bool) -> Vec<&PageNode> {
    let mut pages: Vec<&PageNode> = tree
        .iter_pages()
        .filter(|p| include_drafts || !p.is_draft())
        .collect();
    sort_pages(&mut pages, SortBy::Date);
    pages
}

/// Aggregation: the pages a section lists, including ones it does not
/// contain, as declared by its `pages_from`.
///
/// The receiving section's direct pages, followed by the direct pages of
/// each donor section named in `from` (the `pages_from` frontmatter of the
/// receiver, typically the root `_index.md` listing `["blog"]`), in tree
/// order. Duplicates are removed by path; donors that do not exist are
/// skipped. Ordering is the caller's: pass the result to [`sort_pages`]
/// with the receiver's `sort_by`. This is how a home page shows recent
/// posts without listings ever reaching past direct children on their own.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use taxus_domain::{Frontmatter, NodePath, SiteTreeBuilder};
/// use taxus_domain::derivation::aggregate;
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
/// // The root contains no pages of its own...
/// assert!(tree.root.pages.is_empty());
/// // ...but with `pages_from = ["blog"]` it lists the blog's direct pages.
/// let listed = aggregate(&tree.root, &tree, &[NodePath::parse("blog")?]);
/// assert_eq!(listed[0].path.to_string(), "blog/project-launch");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn aggregate<'a>(
    section: &'a SectionNode,
    tree: &'a SiteTree,
    from: &[NodePath],
) -> Vec<&'a PageNode> {
    let mut merged: Vec<&'a PageNode> = section.pages.iter().collect();
    for source in from {
        if let Some(donor) = tree.get_section(source) {
            for page in &donor.pages {
                if !merged.iter().any(|p| p.path == page.path) {
                    merged.push(page);
                }
            }
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Slug;
    use crate::schema::SortBy;
    use crate::tree::SiteTreeBuilder;
    use chrono::NaiveDate;

    fn page(path: &str, date: NaiveDate) -> PageNode {
        PageNode {
            path: NodePath::parse(path).unwrap(),
            content_file: std::path::PathBuf::from(format!("{path}.md")),
            meta: crate::schema::Frontmatter {
                date: Some(date),
                ..crate::schema::Frontmatter::default()
            },
            body: String::new(),
        }
    }

    fn add(builder: &mut SiteTreeBuilder, path: &str, meta: crate::schema::Frontmatter) {
        builder
            .add_page(
                &NodePath::parse(path).unwrap(),
                std::path::PathBuf::from(format!("{path}.md")),
                meta,
                String::new(),
            )
            .unwrap();
    }

    fn fixture() -> SiteTree {
        let mut builder = SiteTreeBuilder::new();
        // Root page: direct child of the root, older than anything in blog/.
        add(
            &mut builder,
            "root-note",
            page("root-note", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).meta,
        );
        builder
            .add_section(
                &NodePath::parse("blog").unwrap(),
                None,
                crate::schema::Frontmatter {
                    sort_by: SortBy::Date,
                    ..crate::schema::Frontmatter::default()
                },
                None,
            )
            .unwrap();
        for (path, y, m) in [("blog/newer", 2026, 9), ("blog/older", 2026, 2)] {
            add(
                &mut builder,
                path,
                page(path, NaiveDate::from_ymd_opt(y, m, 15).unwrap()).meta,
            );
        }
        builder.build().unwrap()
    }

    #[test]
    fn descendant_pages_walks_the_subtree_depth_first() {
        let mut builder = SiteTreeBuilder::new();
        for path in [
            "top",
            "blog/b-post",
            "blog/a-post",
            "blog/2026/nested",
            "docs/guide",
        ] {
            add(&mut builder, path, crate::schema::Frontmatter::default());
        }
        let tree = builder.build().unwrap();

        let names = |section: &SectionNode| -> Vec<String> {
            descendant_pages(section)
                .iter()
                .map(|p| p.path.to_string())
                .collect()
        };
        let blog = tree.get_section(&NodePath::parse("blog").unwrap()).unwrap();
        assert_eq!(
            names(blog),
            ["blog/a-post", "blog/b-post", "blog/2026/nested"]
        );
        // Direct children stay direct children.
        assert_eq!(blog.pages.len(), 2);
        assert_eq!(
            names(&tree.root),
            [
                "top",
                "blog/a-post",
                "blog/b-post",
                "blog/2026/nested",
                "docs/guide"
            ]
        );
    }

    #[test]
    fn documents_yields_tree_order_including_indexed_sections() {
        let mut builder = SiteTreeBuilder::new().root(
            Some(std::path::PathBuf::from("_index.md")),
            crate::schema::Frontmatter::default(),
            None,
        );
        builder
            .add_section(
                &NodePath::parse("blog").unwrap(),
                Some(std::path::PathBuf::from("blog/_index.md")),
                crate::schema::Frontmatter::default(),
                None,
            )
            .unwrap();
        for path in ["zed", "about", "blog/b", "blog/a", "docs/guide"] {
            add(&mut builder, path, crate::schema::Frontmatter::default());
        }
        let tree = builder.build().unwrap();

        let files: Vec<String> = documents(&tree)
            .iter()
            .map(|n| n.content_file().display().to_string())
            .collect();
        // Root index, root pages by slug, blog index, blog pages by slug,
        // then docs/ (no index file, so no document of its own).
        assert_eq!(
            files,
            [
                "_index.md",
                "about.md",
                "zed.md",
                "blog/_index.md",
                "blog/a.md",
                "blog/b.md",
                "docs/guide.md"
            ]
        );
        assert!(documents(&tree)[0].is_section());
        assert_eq!(documents(&tree)[0].path(), &NodePath::root());
    }

    #[test]
    fn group_by_terms_sorts_terms_and_keeps_tree_order() {
        let tagged = |tags: &[&str], draft: bool| crate::schema::Frontmatter {
            tags: tags.iter().map(|t| t.to_string()).collect(),
            draft,
            ..crate::schema::Frontmatter::default()
        };
        let mut builder = SiteTreeBuilder::new();
        builder
            .add_section(
                &NodePath::parse("blog").unwrap(),
                Some(std::path::PathBuf::from("blog/_index.md")),
                tagged(&["rust"], false),
                None,
            )
            .unwrap();
        add(&mut builder, "blog/z", tagged(&["web", "rust"], false));
        add(&mut builder, "blog/a", tagged(&["rust"], false));
        add(&mut builder, "blog/secret", tagged(&["rust"], true));
        let tree = builder.build().unwrap();

        let groups = group_by_terms(&tree, false, |m| {
            m.tags.iter().map(String::as_str).collect()
        });
        assert_eq!(groups.keys().collect::<Vec<_>>(), ["rust", "web"]);
        let rust: Vec<String> = groups["rust"]
            .iter()
            .map(|n| n.content_file().display().to_string())
            .collect();
        // The section index participates; the draft does not.
        assert_eq!(rust, ["blog/_index.md", "blog/a.md", "blog/z.md"]);

        let with_drafts =
            group_by_terms(&tree, true, |m| m.tags.iter().map(String::as_str).collect());
        assert_eq!(with_drafts["rust"].len(), 4);
    }

    #[test]
    fn recent_sorts_by_date_desc_and_filters_drafts() {
        let mut builder = SiteTreeBuilder::new();
        let dated = |y, m, draft| crate::schema::Frontmatter {
            date: Some(NaiveDate::from_ymd_opt(y, m, 1).unwrap()),
            draft,
            ..crate::schema::Frontmatter::default()
        };
        add(&mut builder, "old", dated(2026, 1, false));
        add(&mut builder, "new", dated(2026, 9, false));
        add(&mut builder, "secret", dated(2026, 12, true));
        add(
            &mut builder,
            "undated",
            crate::schema::Frontmatter::default(),
        );
        let tree = builder.build().unwrap();

        fn names(pages: Vec<&PageNode>) -> Vec<&str> {
            pages
                .iter()
                .map(|p| p.path.last().unwrap().as_str())
                .collect()
        }
        assert_eq!(names(recent(&tree, false)), ["new", "old", "undated"]);
        assert_eq!(
            names(recent(&tree, true)),
            ["secret", "new", "old", "undated"]
        );
    }

    #[test]
    fn aggregate_merges_receiver_then_donors_in_tree_order() {
        let tree = fixture();
        let root = tree.get_section(&NodePath::root()).unwrap();
        let mut merged = aggregate(root, &tree, &[NodePath::parse("blog").unwrap()]);
        let paths: Vec<&str> = merged
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        // Membership only: the receiver's own page, then the donor's pages
        // in slug order. Ordering is the caller's.
        assert_eq!(paths, ["root-note", "newer", "older"]);

        sort_pages(&mut merged, root.meta.sort_by);
        let sorted: Vec<&str> = merged
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        assert_eq!(sorted, ["newer", "older", "root-note"]);
    }

    #[test]
    fn aggregate_donor_pages_are_direct_children_only() {
        let mut builder = SiteTreeBuilder::new();
        add(
            &mut builder,
            "blog/top",
            crate::schema::Frontmatter::default(),
        );
        add(
            &mut builder,
            "blog/2026/nested",
            crate::schema::Frontmatter::default(),
        );
        let tree = builder.build().unwrap();
        let root = tree.get_section(&NodePath::root()).unwrap();
        let merged = aggregate(root, &tree, &[NodePath::parse("blog").unwrap()]);
        let paths: Vec<String> = merged.iter().map(|p| p.path.to_string()).collect();
        assert_eq!(paths, ["blog/top"]);
    }

    #[test]
    fn aggregate_dedupes() {
        let tree = fixture();
        let root = tree.get_section(&NodePath::root()).unwrap();
        let merged = aggregate(
            root,
            &tree,
            &[
                NodePath::root(),
                NodePath::parse("blog").unwrap(),
                NodePath::parse("missing").unwrap(),
            ],
        );
        let mut paths: Vec<&str> = merged
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        paths.sort();
        assert_eq!(paths, ["newer", "older", "root-note"]);
    }

    #[test]
    fn slug_helper_round_trips() {
        let tree = fixture();
        let blog = tree.get_section(&NodePath::parse("blog").unwrap()).unwrap();
        assert_eq!(blog.path.last().map(Slug::as_str), Some("blog"));
    }
}
