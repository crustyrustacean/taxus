//! Integration tests for route discovery.
//!
//! These tests verify the route discovery functionality using test fixtures.

use generator::routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};
use generator::content::FilesystemContentSource;
use std::path::PathBuf;

/// Test discovering routes from the content_site fixture.
#[test]
fn test_discover_content_site() {
    let discovery = RouteDiscovery::new("tests/fixtures/content_site/content");
    let registry = discovery.discover().unwrap();

    // Should find: _index.md, about.md, blog/_index.md, blog/first-post.md, blog/draft-post.md
    assert!(registry.len() >= 4);

    // Check specific routes
    assert!(registry.contains("/"));
    assert!(registry.contains("/about/"));
    assert!(registry.contains("/blog/"));
    assert!(registry.contains("/blog/first-post/"));
}

/// Test that the home route is correctly identified as a section.
#[test]
fn test_discover_content_site_home_route() {
    let discovery = RouteDiscovery::new("tests/fixtures/content_site/content");
    let registry = discovery.discover().unwrap();

    let home = registry.get("/").unwrap();
    assert!(home.is_section());
    assert_eq!(home.content_file, PathBuf::from("_index.md"));
    assert_eq!(home.output_file, PathBuf::from("index.html"));
}

/// Test that page routes are correctly identified.
#[test]
fn test_discover_content_site_page_routes() {
    let discovery = RouteDiscovery::new("tests/fixtures/content_site/content");
    let registry = discovery.discover().unwrap();

    let about = registry.get("/about/").unwrap();
    assert!(about.is_page());
    assert_eq!(about.content_file, PathBuf::from("about.md"));
    assert_eq!(about.output_file, PathBuf::from("about/index.html"));
}

/// Test that nested routes are correctly identified.
#[test]
fn test_discover_content_site_nested_routes() {
    let discovery = RouteDiscovery::new("tests/fixtures/content_site/content");
    let registry = discovery.discover().unwrap();

    // Blog section
    let blog = registry.get("/blog/").unwrap();
    assert!(blog.is_section());
    assert_eq!(blog.content_file, PathBuf::from("blog/_index.md"));
    assert_eq!(blog.output_file, PathBuf::from("blog/index.html"));

    // Blog post
    let post = registry.get("/blog/first-post/").unwrap();
    assert!(post.is_page());
    assert_eq!(post.content_file, PathBuf::from("blog/first-post.md"));
    assert_eq!(post.output_file, PathBuf::from("blog/first-post/index.html"));
}

/// Test counting pages and sections.
#[test]
fn test_discover_content_site_counts() {
    let discovery = RouteDiscovery::new("tests/fixtures/content_site/content");
    let registry = discovery.discover().unwrap();

    // We have: _index.md (section), about.md (page), blog/_index.md (section),
    // blog/first-post.md (page), blog/draft-post.md (page)
    assert_eq!(registry.sections().count(), 2);
    assert!(registry.pages().count() >= 2);
}

/// Test discovering routes from the minimal_site fixture.
#[test]
fn test_discover_minimal_site() {
    let discovery = RouteDiscovery::new("tests/fixtures/minimal_site");
    let registry = discovery.discover().unwrap();

    // Minimal site only has site.toml, no content directory
    // This should either fail or return empty
    // Let's check if it handles missing content gracefully
    assert!(registry.is_empty());
}

/// Test discovering routes using ContentSource trait.
#[test]
fn test_discover_from_source() {
    let source = FilesystemContentSource::new("tests/fixtures/content_site/content");
    let discovery = RouteDiscovery::new("content");
    let registry = discovery.discover_from_source(&source).unwrap();

    assert!(registry.len() >= 4);
    assert!(registry.contains("/"));
    assert!(registry.contains("/about/"));
    assert!(registry.contains("/blog/"));
}

/// Test route registry operations.
#[test]
fn test_registry_operations() {
    let mut registry = RouteRegistry::new();

    // Register routes
    let route1 = RouteInfo::new(
        "/".to_string(),
        PathBuf::from("_index.md"),
        PathBuf::from("index.html"),
        RouteKind::Section,
    )
    .unwrap();

    let route2 = RouteInfo::new(
        "/about/".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .unwrap();

    registry.register(route1).unwrap();
    registry.register(route2).unwrap();

    // Test retrieval
    assert!(registry.contains("/"));
    assert!(registry.contains("/about/"));
    assert!(!registry.contains("/missing/"));

    // Test get
    let retrieved = registry.get("/about/").unwrap();
    assert_eq!(retrieved.path, "/about/");
    assert_eq!(retrieved.content_file, PathBuf::from("about.md"));

    // Test counts
    assert_eq!(registry.len(), 2);
    assert_eq!(registry.pages().count(), 1);
    assert_eq!(registry.sections().count(), 1);
}

/// Test that duplicate routes are rejected.
#[test]
fn test_registry_duplicate_rejection() {
    let mut registry = RouteRegistry::new();

    let route1 = RouteInfo::new(
        "/about/".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .unwrap();

    let route2 = RouteInfo::new(
        "/about/".to_string(),
        PathBuf::from("about-duplicate.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .unwrap();

    registry.register(route1).unwrap();
    let result = registry.register(route2);

    assert!(result.is_err());
}

/// Test route info validation.
#[test]
fn test_route_info_validation() {
    // Valid paths
    assert!(RouteInfo::new(
        "/".to_string(),
        PathBuf::from("_index.md"),
        PathBuf::from("index.html"),
        RouteKind::Section,
    )
    .is_ok());

    assert!(RouteInfo::new(
        "/about/".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .is_ok());

    assert!(RouteInfo::new(
        "/blog/first-post/".to_string(),
        PathBuf::from("blog/first-post.md"),
        PathBuf::from("blog/first-post/index.html"),
        RouteKind::Page,
    )
    .is_ok());

    // Invalid paths
    assert!(RouteInfo::new(
        "about/".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .is_err());

    assert!(RouteInfo::new(
        "/about".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .is_err());

    assert!(RouteInfo::new(
        "".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .is_err());
}

/// Test route kind helper methods.
#[test]
fn test_route_kind_helpers() {
    let page_kind = RouteKind::Page;
    assert!(page_kind.is_page());
    assert!(!page_kind.is_section());

    let section_kind = RouteKind::Section;
    assert!(!section_kind.is_page());
    assert!(section_kind.is_section());
}

/// Test route info helper methods.
#[test]
fn test_route_info_helpers() {
    let page = RouteInfo::new(
        "/about/".to_string(),
        PathBuf::from("about.md"),
        PathBuf::from("about/index.html"),
        RouteKind::Page,
    )
    .unwrap();

    assert!(page.is_page());
    assert!(!page.is_section());

    let section = RouteInfo::new(
        "/blog/".to_string(),
        PathBuf::from("blog/_index.md"),
        PathBuf::from("blog/index.html"),
        RouteKind::Section,
    )
    .unwrap();

    assert!(!section.is_page());
    assert!(section.is_section());
}

/// Test iterator methods on empty registry.
#[test]
fn test_empty_registry_iterators() {
    let registry = RouteRegistry::new();

    assert_eq!(registry.iter().count(), 0);
    assert_eq!(registry.pages().count(), 0);
    assert_eq!(registry.sections().count(), 0);
}

/// Test generate_rust_manifest produces valid output.
#[test]
fn test_generate_rust_manifest() {
    let mut registry = RouteRegistry::new();

    registry
        .register(RouteInfo::new(
            "/".to_string(),
            PathBuf::from("_index.md"),
            PathBuf::from("index.html"),
            RouteKind::Section,
        )
        .unwrap())
        .unwrap();

    registry
        .register(RouteInfo::new(
            "/about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .unwrap())
        .unwrap();

    let manifest = registry.generate_rust_manifest();

    // Check that the manifest contains expected elements
    assert!(manifest.contains("Auto-generated route manifest"));
    assert!(manifest.contains("Routable"));
    assert!(manifest.contains("Route"));
    assert!(manifest.contains("Home"));
    assert!(manifest.contains("About"));
}