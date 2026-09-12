//! Section listings are derived from the Site Tree.
//!
//! Uses the `section_listing_site` fixture:
//!
//! - `docs/` has `sort_by = "weight"` with three pages whose alphabetical
//!   order (alpha, beta, gamma) is the reverse of their weight order
//!   (beta = 1, gamma = 2, alpha = 3) — the #5 regression test.
//! - `blog/` has a direct child (`top-level`, January) and a nested
//!   section `blog/2026/` with one page (`post`, February).
//! - the root `_index.md` declares `pages_from = ["blog", "blog/2026"]`,
//!   the declared-membership bridge that replaces the old prefix scan (#70).
//! - `notes/` (default `sort_by = "date"`) mixes two dated pages with an
//!   undated one.
//! - `glossary/` has `sort_by = "title"` with titles whose byte order
//!   (`Banana`, `apple`, `cherry`) differs from their case-insensitive order.

use std::fs;
use std::path::Path;
use taxus_lib::build::SiteBuilder;

fn build_fixture() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    SiteBuilder::from_dir(Path::new("tests/fixtures/section_listing_site"))
        .unwrap()
        .output_dir(tmp.path())
        .build()
        .unwrap();
    tmp
}

/// The `href` targets of the section's page list, in document order.
fn listed_links(output_dir: &Path, page: &str) -> Vec<String> {
    let html = fs::read_to_string(output_dir.join(page)).unwrap();
    let list = html
        .split("<ul class=\"page-list\">")
        .nth(1)
        .expect("page list present")
        .split("</ul>")
        .next()
        .unwrap();
    list.split("href=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .collect()
}

#[test]
fn weight_sorted_section_lists_pages_by_weight() {
    let out = build_fixture();
    assert_eq!(
        listed_links(out.path(), "docs/index.html"),
        ["/docs/beta/", "/docs/gamma/", "/docs/alpha/"],
        "sort_by = \"weight\" orders by weight, not title (#5)"
    );
    // `weight` is visible to templates.
    let html = fs::read_to_string(out.path().join("docs/index.html")).unwrap();
    assert!(html.contains("Beta</a> (weight 1)"), "{html}");
    assert!(html.contains("Alpha</a> (weight 3)"), "{html}");
}

#[test]
fn section_lists_direct_children_only() {
    let out = build_fixture();
    // `blog/` lists its own page, not its grandchild in `blog/2026/` (#70).
    assert_eq!(
        listed_links(out.path(), "blog/index.html"),
        ["/blog/top-level/"]
    );
    assert_eq!(
        listed_links(out.path(), "blog/2026/index.html"),
        ["/blog/2026/post/"]
    );
}

#[test]
fn root_lists_pages_from_donor_sections() {
    // The root has no pages of its own; `pages_from = ["blog", "blog/2026"]`
    // pulls in each donor's direct pages, then the root's sort_by (date,
    // newest first) orders them.
    let out = build_fixture();
    assert_eq!(
        listed_links(out.path(), "index.html"),
        ["/blog/2026/post/", "/blog/top-level/"]
    );
}

#[test]
fn date_sort_is_newest_first_with_undated_last() {
    let out = build_fixture();
    assert_eq!(
        listed_links(out.path(), "notes/index.html"),
        ["/notes/newer/", "/notes/older/", "/notes/undated/"]
    );
}

#[test]
fn title_sort_is_case_insensitive() {
    let out = build_fixture();
    assert_eq!(
        listed_links(out.path(), "glossary/index.html"),
        ["/glossary/apple/", "/glossary/banana/", "/glossary/cherry/"]
    );
}
