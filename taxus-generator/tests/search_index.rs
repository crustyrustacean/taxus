//! End-to-end coverage for the search index build stage (#25).
//!
//! The destination-first assertion: a page with a custom slug is SERVED at
//! the slug URL, so the search index must contain that URL — not the
//! discovered route path. Before the fix, search results navigated to a
//! 404 for every slug-renamed page.

use taxus_common::search::SearchIndex;
use taxus_lib::build::SiteBuilder;
use taxus_lib::build::pipeline::search::generate_search;
use taxus_lib::build::pipeline::{discover_routes, process_content};
use taxus_lib::config::SiteConfig;
use taxus_lib::highlighting::CodeHighlighter;
use taxus_lib::highlighting::LanguageRegistry;

/// Build the fixture site and return its generated search index, decoded.
fn build_search_index() -> SearchIndex {
    let fixture = std::path::Path::new("tests/fixtures/search_slug_site");
    let config = SiteConfig::from_dir(fixture).expect("fixture config");

    let registry = discover_routes(&config).expect("routes");
    let mut highlighter = CodeHighlighter::new(LanguageRegistry::new(), "hl-");
    let processed =
        process_content(&registry, &config, false, Some(&mut highlighter)).expect("content");

    let generated = generate_search(&processed).expect("search generation");
    SearchIndex::from_bytes(&generated.search_index).expect("index roundtrip")
}

#[test]
fn test_search_index_uses_effective_url_for_custom_slugs() {
    let index = build_search_index();

    let doc = index
        .documents
        .values()
        .find(|d| d.title == "Renamed Entry")
        .expect("fixture page missing from index");

    // The page lives at content/blog/e.md with slug = "renamed-entry".
    // It is served at /renamed-entry/ — the index must point there,
    // not at the stale route path /blog/e/.
    assert_eq!(
        doc.path, "/renamed-entry/",
        "search result for a slug-renamed page must use the served URL"
    );
}

#[test]
fn test_search_index_uses_route_path_for_ordinary_pages() {
    let index = build_search_index();

    let doc = index
        .documents
        .values()
        .find(|d| d.title == "Ordinary Post")
        .expect("fixture page missing from index");

    assert_eq!(
        doc.path, "/blog/ordinary/",
        "ordinary page keeps route path"
    );
}

/// Full-pipeline confirmation: the written file and the built page agree.
///
/// A search hit and the page it navigates to must be the same URL. This
/// builds the site for real and asserts the indexed path exists as output.
#[test]
fn test_search_index_paths_resolve_to_built_output() {
    use std::path::Path;

    let fixture = Path::new("tests/fixtures/search_slug_site");
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut config = SiteConfig::from_dir(fixture).expect("fixture config");
    config.build.output_dir = temp.path().join("dist");

    SiteBuilder::new(config)
        .build()
        .expect("build fixture site");

    // Decode the written index
    let bytes = std::fs::read(temp.path().join("dist/search_index.bin")).expect("written index");
    let index = SearchIndex::from_bytes(&bytes).expect("index roundtrip");

    for doc in index.documents.values() {
        let page_path = temp
            .path()
            .join("dist")
            .join(doc.path.trim_start_matches('/'))
            .join("index.html");
        assert!(
            page_path.exists(),
            "search result {} must resolve to a built page (missing {})",
            doc.path,
            page_path.display()
        );
    }
}
