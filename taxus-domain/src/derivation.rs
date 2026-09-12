//! Pure derivations over the Site Tree.
//!
//! Each derivation is `(source set, filter, order, grouping)` — computed,
//! never stored. Draft filtering is the caller's concern (the generator
//! decides whether drafts participate in a given build), so every
//! derivation that filters drafts takes an explicit `include_drafts`.

use crate::identity::NodePath;
use crate::schema::{Frontmatter, SortBy};
use crate::tree::{PageNode, SectionNode, SiteTree, sort_pages};
use std::collections::BTreeMap;
use std::path::Path;

/// A rendered document in the tree: a page, or a section that has an
/// `_index.md`. Derivations that range over "everything the site renders"
/// (feeds, the sitemap, taxonomies) yield these.
#[derive(Debug, Clone, Copy)]
pub enum Node<'a> {
    Section(&'a SectionNode),
    Page(&'a PageNode),
}

impl<'a> Node<'a> {
    pub fn path(&self) -> &'a NodePath {
        match self {
            Node::Section(s) => &s.path,
            Node::Page(p) => &p.path,
        }
    }

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

    pub fn is_section(&self) -> bool {
        matches!(self, Node::Section(_))
    }

    pub fn is_draft(&self) -> bool {
        self.meta().draft
    }
}

/// Every rendered document in the site, in **tree order**: a section's
/// own index (if it has one), then its pages in slug order, then each
/// subsection in slug order, recursively. Drafts are included.
///
/// This is the canonical iteration order of a site — the order routes are
/// registered in and the order every projection sees documents in — so
/// anything that keeps input order downstream is deterministic.
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

/// Group documents by the terms `terms_of` reads from their frontmatter —
/// the index behind tags, categories and series.
///
/// Terms come back sorted by name; the documents under a term keep
/// [`documents`] order. A document that lists the same term twice appears
/// twice, as the caller's term-counting expects. Sections with an
/// `_index.md` participate like pages.
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

/// Every page in a section's subtree, depth-first: the section's own pages
/// (slug order), then each subsection's pages in turn.
///
/// Reachability is a query, never a property of the structure: a section's
/// `pages` field holds direct children only, and this is how a listing that
/// wants the whole subtree asks for it.
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

/// Recent pages across the whole site: newest first, undated pages last.
///
/// Drafts are excluded unless `include_drafts` is set — the generator
/// decides per build whether drafts participate.
pub fn recent(tree: &SiteTree, include_drafts: bool) -> Vec<&PageNode> {
    let mut pages: Vec<&PageNode> = tree
        .iter_pages()
        .filter(|p| include_drafts || !p.is_draft())
        .collect();
    sort_pages(&mut pages, SortBy::Date);
    pages
}

/// Aggregation: declared membership beyond containment.
///
/// The receiving section's direct pages, followed by the direct pages of
/// each donor section named in `from` (the `pages_from` frontmatter of the
/// receiver, typically the root `_index.md` listing `["blog"]`), in tree
/// order. Duplicates are removed by path; donors that do not exist are
/// skipped. Ordering is the caller's — pass the result to
/// [`sort_pages`] with the receiver's `sort_by`.
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
