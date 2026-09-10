//! Pure derivations over the Site Tree.
//!
//! Each derivation is `(source set, filter, order, grouping)` — computed,
//! never stored. Draft filtering is the caller's concern (the generator
//! decides whether drafts participate in a given build).

use crate::identity::NodePath;
use crate::tree::{PageNode, SectionNode, SiteTree, sort_pages};

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
            meta: crate::schema::Frontmatter {
                date: Some(date),
                ..crate::schema::Frontmatter::default()
            },
            body: String::new(),
        }
    }

    fn fixture() -> SiteTree {
        let mut builder = SiteTreeBuilder::new();
        // Root page: direct child of the root, older than anything in blog/.
        builder
            .add_page(
                &NodePath::parse("root-note").unwrap(),
                page("root-note", NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).meta,
                String::new(),
            )
            .unwrap();
        builder
            .add_section(
                &NodePath::parse("blog").unwrap(),
                crate::schema::Frontmatter {
                    sort_by: SortBy::Date,
                    ..crate::schema::Frontmatter::default()
                },
                None,
            )
            .unwrap();
        for (path, y, m) in [("blog/newer", 2026, 9), ("blog/older", 2026, 2)] {
            builder
                .add_page(
                    &NodePath::parse(path).unwrap(),
                    page(path, NaiveDate::from_ymd_opt(y, m, 15).unwrap()).meta,
                    String::new(),
                )
                .unwrap();
        }
        builder.build().unwrap()
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
