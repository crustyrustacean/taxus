//! Pure derivations over the Site Tree.
//!
//! Each derivation is `(source set, filter, order, grouping)` — computed,
//! never stored. Draft filtering is the caller's concern (the generator
//! decides whether drafts participate in a given build), so every
//! derivation that filters drafts takes an explicit `include_drafts`.

use crate::identity::NodePath;
use crate::schema::SortBy;
use crate::tree::{PageNode, SectionNode, SiteTree, sort_pages};

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
/// Merge the receiving section's direct pages with the pages of the named
/// donor sections (e.g. `pages_from = ["blog"]` on the root `_index.md`).
/// Duplicates are removed by path; the merged listing is sorted per the
/// receiving section's `sort_by`.
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
    sort_pages(&mut merged, section.meta.sort_by);
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
    fn aggregate_merges_and_sorts_by_receiver() {
        let tree = fixture();
        let root = tree.get_section(&NodePath::root()).unwrap();
        let merged = aggregate(root, &tree, &[NodePath::parse("blog").unwrap()]);
        let paths: Vec<&str> = merged
            .iter()
            .map(|p| p.path.last().unwrap().as_str())
            .collect();
        // Newest first across both sources; the January root note is last.
        assert_eq!(paths, ["newer", "older", "root-note"]);
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
