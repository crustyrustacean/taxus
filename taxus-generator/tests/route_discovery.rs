//! Integration tests for route discovery.
//!
//! These tests verify the route discovery functionality using test fixtures.

use std::path::PathBuf;
use taxus_lib::content::FilesystemContentSource;
use taxus_lib::routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};

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
    assert_eq!(
        post.output_file,
        PathBuf::from("blog/first-post/index.html")
    );
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
    assert!(
        RouteInfo::new(
            "/".to_string(),
            PathBuf::from("_index.md"),
            PathBuf::from("index.html"),
            RouteKind::Section,
        )
        .is_ok()
    );

    assert!(
        RouteInfo::new(
            "/about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .is_ok()
    );

    assert!(
        RouteInfo::new(
            "/blog/first-post/".to_string(),
            PathBuf::from("blog/first-post.md"),
            PathBuf::from("blog/first-post/index.html"),
            RouteKind::Page,
        )
        .is_ok()
    );

    // Invalid paths
    assert!(
        RouteInfo::new(
            "about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .is_err()
    );

    assert!(
        RouteInfo::new(
            "/about".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .is_err()
    );

    assert!(
        RouteInfo::new(
            "".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .is_err()
    );
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

// ── Site Tree discovery ──────────────────────────────────────────────────────
//
// The tree is now the source of truth; the registry is derived from it.
// These tests hold the two discovery paths to the same answer for every
// fixture that has a content directory, and port the fixture assertions
// above so they run against both.

use std::path::Path;
use taxus_lib::content::Page;

/// Every content directory we ship: fixture sites plus get-taxus-org.
fn content_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir("tests/fixtures")
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("content"))
        .filter(|p| p.is_dir())
        .collect();
    dirs.push(PathBuf::from("../get-taxus-org/content"));
    dirs.sort();
    assert!(
        dirs.len() >= 6,
        "expected several fixture content dirs: {dirs:?}"
    );
    dirs
}

fn sorted_routes(registry: &RouteRegistry) -> Vec<RouteInfo> {
    let mut routes: Vec<RouteInfo> = registry.iter().cloned().collect();
    routes.sort_by(|a, b| a.path.cmp(&b.path));
    routes
}

/// A page's frontmatter `slug`, if it has one.
fn frontmatter_slug(content_dir: &Path, route: &RouteInfo) -> Option<String> {
    if route.kind != RouteKind::Page {
        return None;
    }
    Page::from_file(content_dir.join(&route.content_file))
        .unwrap()
        .frontmatter
        .slug
}

/// `RouteRegistry::from_tree(discover_tree())` agrees with `discover()`:
/// the same routes in the same order once sorted by path.
///
/// The one documented difference is a page with a frontmatter `slug`: the
/// legacy walk keys it by filename (`/blog/e/`), the tree by the documented
/// model, section path + slug (`/blog/renamed-entry/`). For such a page the
/// content file and kind must still agree, and the tree path must be the
/// legacy parent plus the slug.
#[test]
fn test_registry_from_tree_matches_discover_for_every_fixture() {
    for content_dir in content_dirs() {
        let discovery = RouteDiscovery::new(&content_dir);
        let legacy = sorted_routes(&discovery.discover().unwrap());
        let tree = discovery.discover_tree().unwrap();
        let derived = sorted_routes(&RouteRegistry::from_tree(&tree));

        assert_eq!(
            legacy.len(),
            derived.len(),
            "{}: route count differs\nlegacy: {legacy:#?}\nderived: {derived:#?}",
            content_dir.display()
        );

        // Pair by content file: that is the storage identity both share.
        let mut by_file: Vec<(&RouteInfo, &RouteInfo)> = legacy
            .iter()
            .map(|l| {
                let d = derived
                    .iter()
                    .find(|d| d.content_file == l.content_file)
                    .unwrap_or_else(|| {
                        panic!(
                            "{}: no tree route for {}",
                            content_dir.display(),
                            l.content_file.display()
                        )
                    });
                (l, d)
            })
            .collect();
        by_file.sort_by(|a, b| a.0.path.cmp(&b.0.path));

        let mut slug_overrides = 0;
        for (l, d) in by_file {
            match frontmatter_slug(&content_dir, l) {
                None => assert_eq!(d, l, "{}", content_dir.display()),
                Some(slug) => {
                    slug_overrides += 1;
                    assert_eq!(d.kind, l.kind);
                    let parent = l.path.trim_end_matches('/').rsplit_once('/').unwrap().0;
                    assert_eq!(
                        d.path,
                        format!("{parent}/{slug}/"),
                        "{}: slug page keyed by section path + slug",
                        content_dir.display()
                    );
                    assert_eq!(
                        d.output_file,
                        PathBuf::from(d.path.trim_matches('/')).join("index.html")
                    );
                }
            }
        }
        if content_dir.ends_with("search_slug_site/content") {
            assert_eq!(slug_overrides, 1, "fixture has exactly one slug override");
        }
    }
}

/// The content_site assertions from the top of this file, against a registry
/// produced either way.
fn assert_content_site_routes(registry: &RouteRegistry) {
    assert!(registry.len() >= 4);
    assert!(registry.contains("/"));
    assert!(registry.contains("/about/"));
    assert!(registry.contains("/blog/"));
    assert!(registry.contains("/blog/first-post/"));

    let home = registry.get("/").unwrap();
    assert!(home.is_section());
    assert_eq!(home.content_file, PathBuf::from("_index.md"));
    assert_eq!(home.output_file, PathBuf::from("index.html"));

    let about = registry.get("/about/").unwrap();
    assert!(about.is_page());
    assert_eq!(about.content_file, PathBuf::from("about.md"));
    assert_eq!(about.output_file, PathBuf::from("about/index.html"));

    let blog = registry.get("/blog/").unwrap();
    assert!(blog.is_section());
    assert_eq!(blog.content_file, PathBuf::from("blog/_index.md"));
    assert_eq!(blog.output_file, PathBuf::from("blog/index.html"));

    let post = registry.get("/blog/first-post/").unwrap();
    assert!(post.is_page());
    assert_eq!(post.content_file, PathBuf::from("blog/first-post.md"));
    assert_eq!(
        post.output_file,
        PathBuf::from("blog/first-post/index.html")
    );

    assert_eq!(registry.sections().count(), 2);
    assert!(registry.pages().count() >= 2);
}

#[test]
fn test_content_site_routes_legacy_and_tree() {
    let discovery = RouteDiscovery::new("tests/fixtures/content_site/content");
    assert_content_site_routes(&discovery.discover().unwrap());
    let tree = discovery.discover_tree().unwrap();
    assert_content_site_routes(&RouteRegistry::from_tree(&tree));
}

#[test]
fn test_discover_tree_content_site_structure() {
    use taxus_domain::NodePath;

    let tree = RouteDiscovery::new("tests/fixtures/content_site/content")
        .discover_tree()
        .unwrap();

    assert_eq!(tree.root.meta.title, "Home");
    assert_eq!(tree.root.pages.len(), 1, "about.md is the root's only page");
    assert_eq!(tree.root.subsections.len(), 1);

    let blog = tree.get_section(&NodePath::parse("blog").unwrap()).unwrap();
    assert_eq!(blog.meta.title, "Blog");
    let slugs: Vec<&str> = blog
        .pages
        .iter()
        .map(|p| p.path.last().unwrap().as_str())
        .collect();
    assert_eq!(
        slugs,
        ["draft-post", "first-post"],
        "children sorted by slug"
    );
    assert!(blog.pages[0].is_draft());
    assert_eq!(tree.iter_pages().count(), 3);
}

#[test]
fn test_discover_tree_from_source() {
    let source = FilesystemContentSource::new("tests/fixtures/content_site/content");
    let discovery = RouteDiscovery::new("content");
    let tree = discovery.discover_tree_from_source(&source).unwrap();
    let registry = RouteRegistry::from_tree(&tree);
    assert!(registry.len() >= 4);
    assert!(registry.contains("/"));
    assert!(registry.contains("/about/"));
    assert!(registry.contains("/blog/"));
}

#[test]
fn test_discover_tree_minimal_site_has_no_content_dir() {
    let discovery = RouteDiscovery::new("tests/fixtures/minimal_site/content");
    assert!(discovery.discover_tree().is_err());
}
