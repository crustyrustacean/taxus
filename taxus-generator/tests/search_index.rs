//! End-to-end coverage for the search index build stage (#25).
//!
//! The destination-first assertion: a page with a custom slug is SERVED at
//! section path + slug, so the search index must contain that URL — not the
//! filename path. Before the fix, search results navigated to a 404 for
//! every slug-renamed page.

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
    let tree = taxus_lib::build::pipeline::discover_tree(&config).expect("tree");
    let mut highlighter = CodeHighlighter::new(LanguageRegistry::new(), "hl-");
    let (processed, _skipped) =
        process_content(&tree, &registry, &config, false, Some(&mut highlighter)).expect("content");

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
    // It is served at section path + slug, /blog/renamed-entry/ — the
    // index must point there, not at the filename path /blog/e/ nor at a
    // root-level /renamed-entry/.
    assert_eq!(
        doc.path, "/blog/renamed-entry/",
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

/// #43: the index is built from Markdown text, not rendered HTML. The
/// fixture's HTML would contribute `hl-keyword`, `span`, `class` and the
/// fenced-code language tag `rust` as terms — none of which may be in
/// the index. The title and tags, which used to be unindexed, must be.
#[test]
fn test_search_index_contains_markdown_text_not_html_noise() {
    let index = build_search_index();

    for stem in index.index.keys() {
        assert!(
            !["span", "class", "hl", "hl_keyword", "hl_keywor", "dhl"].contains(&stem.as_str()),
            "markup leaked into the index: {stem}"
        );
    }

    // Highlighter class prefix "hl-" tokenizes to "hl" + suffix segments;
    // with the raw-HTML index those terms existed. With Markdown text
    // they cannot: they are not words in the body.
    assert!(
        !index.index.contains_key("hl"),
        "the highlighter class prefix must not be an index term"
    );
}

/// #43: a query that matches only the title (the word appears nowhere in
/// any body) must return the page.
#[test]
fn test_search_index_finds_title_only_match() {
    let index = build_search_index();

    // "Ordinary" appears in the title of Ordinary Post and in no body.
    let results = index.search("ordinary");
    assert!(
        results.iter().any(|d| d.title == "Ordinary Post"),
        "a title-only match must be found"
    );
}

/// #43: tags are indexed — a query for a tag returns the tagged page.
#[test]
fn test_search_index_finds_tag_match() {
    let index = build_search_index();

    let results = index.search("zephyr");
    assert!(
        results.iter().any(|d| d.title == "Renamed Entry"),
        "a tag match must find the tagged page"
    );
}

/// #56: with `[build] islands = false` and `search = false`, the build
/// writes neither the WASM client nor the search index. A plain
/// Tera/Markdown site ships exactly its pages — not several hundred KB
/// of hydration code it never loads.
#[test]
fn test_no_islands_site_skips_wasm_and_search_output() {
    use std::path::Path;

    let fixture = Path::new("tests/fixtures/no_islands_site");
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut config = SiteConfig::from_dir(fixture).expect("fixture config");
    config.build.output_dir = temp.path().join("dist");

    assert!(!config.build.islands, "fixture sets islands = false");
    assert!(!config.build.search, "fixture sets search = false");

    let report = SiteBuilder::new(config)
        .build()
        .expect("build fixture site");
    assert!(
        report.pages_rendered >= 1,
        "the pages themselves still build"
    );

    assert!(
        !temp.path().join("dist/search_index.bin").exists(),
        "search index must not be written when build.search = false"
    );
    assert!(
        !temp.path().join("dist/wasm").exists(),
        "WASM client directory must not be written when build.islands = false"
    );
}
