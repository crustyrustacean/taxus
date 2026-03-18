//! Route discovery from content directory.
//!
//! This module provides the [`RouteDiscovery`] type for discovering routes
//! from a content directory structure.

use crate::content::ContentSource;
use crate::error::RouteError;
use crate::routes::{RouteInfo, RouteKind, RouteRegistry};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discovers routes from content directory structure.
pub struct RouteDiscovery {
    content_dir: PathBuf,
}

impl RouteDiscovery {
    /// Create a new route discovery for the given content directory.
    pub fn new<P: Into<PathBuf>>(content_dir: P) -> Self {
        Self {
            content_dir: content_dir.into(),
        }
    }

    /// Discover all routes from the content directory.
    ///
    /// This walks the content directory and creates routes for each `.md` file:
    /// - `_index.md` files become section routes (e.g., `/blog/`)
    /// - Other `.md` files become page routes (e.g., `/about/`)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The content directory cannot be read
    /// - A duplicate route is detected
    pub fn discover(&self) -> Result<RouteRegistry, RouteError> {
        let mut registry = RouteRegistry::new();

        // Check if content directory exists
        if !self.content_dir.exists() {
            return Err(RouteError::DiscoveryFailed(format!(
                "Content directory does not exist: {}",
                self.content_dir.display()
            )));
        }

        // Walk the content directory
        for entry in WalkDir::new(&self.content_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process .md files
            if path.extension().is_some_and(|ext| ext == "md") {
                // Get relative path from content directory
                if let Ok(relative) = path.strip_prefix(&self.content_dir) {
                    // Convert file path to route
                    if let Some(route) = self.create_route_from_file(relative)? {
                        registry.register(route)?;
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Discover routes using a ContentSource trait object.
    ///
    /// This is useful for testing with mock content sources.
    pub fn discover_from_source<S: ContentSource>(
        &self,
        source: &S,
    ) -> Result<RouteRegistry, RouteError> {
        let mut registry = RouteRegistry::new();

        // List all content files from the source
        let files = source
            .list()
            .map_err(|e| RouteError::DiscoveryFailed(e.to_string()))?;

        for relative in files {
            // Convert file path to route
            if let Some(route) = self.create_route_from_file(&relative)? {
                registry.register(route)?;
            }
        }

        Ok(registry)
    }

    /// Create a RouteInfo from a content file path.
    fn create_route_from_file(&self, relative: &Path) -> Result<Option<RouteInfo>, RouteError> {
        let file_name = relative
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                RouteError::InvalidPath(format!("Invalid file name: {}", relative.display()))
            })?;

        // Determine route kind based on file name
        let (kind, url_path, output_file) = if file_name == "_index.md" {
            // Section index
            let parent = relative.parent().unwrap_or(Path::new(""));
            let url_path = self.parent_to_url_path(parent);
            let output_file = self.parent_to_output_file(parent);
            (RouteKind::Section, url_path, output_file)
        } else {
            // Regular page
            let stem = file_name
                .strip_suffix(".md")
                .ok_or_else(|| {
                    RouteError::InvalidPath(format!("Not a markdown file: {}", file_name))
                })?
                .to_string();

            let parent = relative.parent().unwrap_or(Path::new(""));
            let url_path = self.page_to_url_path(parent, &stem);
            let output_file = self.page_to_output_file(parent, &stem);
            (RouteKind::Page, url_path, output_file)
        };

        RouteInfo::new(url_path, relative.to_path_buf(), output_file, kind).map(Some)
    }

    /// Convert a parent directory path to a URL path for sections.
    ///
    /// Examples:
    /// - "" -> "/"
    /// - "blog" -> "/blog/"
    /// - "blog/tech" -> "/blog/tech/"
    fn parent_to_url_path(&self, parent: &Path) -> String {
        if parent.as_os_str().is_empty() {
            "/".to_string()
        } else {
            format!("/{}/", parent.display())
        }
    }

    /// Convert a parent directory path to an output file path for sections.
    ///
    /// Examples:
    /// - "" -> "index.html"
    /// - "blog" -> "blog/index.html"
    /// - "blog/tech" -> "blog/tech/index.html"
    fn parent_to_output_file(&self, parent: &Path) -> PathBuf {
        if parent.as_os_str().is_empty() {
            PathBuf::from("index.html")
        } else {
            parent.join("index.html")
        }
    }

    /// Convert a page path to a URL path.
    ///
    /// Examples:
    /// - ("", "about") -> "/about/"
    /// - ("blog", "first-post") -> "/blog/first-post/"
    fn page_to_url_path(&self, parent: &Path, stem: &str) -> String {
        if parent.as_os_str().is_empty() {
            format!("/{}/", stem)
        } else {
            format!("/{}/{}/", parent.display(), stem)
        }
    }

    /// Convert a page path to an output file path.
    ///
    /// Examples:
    /// - ("", "about") -> "about/index.html"
    /// - ("blog", "first-post") -> "blog/first-post/index.html"
    fn page_to_output_file(&self, parent: &Path, stem: &str) -> PathBuf {
        if parent.as_os_str().is_empty() {
            PathBuf::from(stem).join("index.html")
        } else {
            parent.join(stem).join("index.html")
        }
    }
}

/// Internal helper function for testing path conversion.
/// Converts a content file path to (url_path, output_file, route_kind).
#[cfg(test)]
fn convert_path(path: &str) -> (String, PathBuf, RouteKind) {
    let discovery = RouteDiscovery::new("content");
    let relative = PathBuf::from(path);

    let route = discovery
        .create_route_from_file(&relative)
        .unwrap()
        .unwrap();
    (route.path, route.output_file, route.kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::MockContentSource;

    // Path conversion tests

    #[test]
    fn test_path_conversion_index() {
        // _index.md -> /, index.html, Section
        let (path, output, kind) = convert_path("_index.md");
        assert_eq!(path, "/");
        assert_eq!(output, PathBuf::from("index.html"));
        assert_eq!(kind, RouteKind::Section);
    }

    #[test]
    fn test_path_conversion_page() {
        // about.md -> /about/, about/index.html, Page
        let (path, output, kind) = convert_path("about.md");
        assert_eq!(path, "/about/");
        assert_eq!(output, PathBuf::from("about/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_nested_section() {
        // blog/_index.md -> /blog/, blog/index.html, Section
        let (path, output, kind) = convert_path("blog/_index.md");
        assert_eq!(path, "/blog/");
        assert_eq!(output, PathBuf::from("blog/index.html"));
        assert_eq!(kind, RouteKind::Section);
    }

    #[test]
    fn test_path_conversion_nested_page() {
        // blog/first-post.md -> /blog/first-post/, blog/first-post/index.html, Page
        let (path, output, kind) = convert_path("blog/first-post.md");
        assert_eq!(path, "/blog/first-post/");
        assert_eq!(output, PathBuf::from("blog/first-post/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_deeply_nested() {
        // blog/tech/first-post.md -> /blog/tech/first-post/, blog/tech/first-post/index.html, Page
        let (path, output, kind) = convert_path("blog/tech/first-post.md");
        assert_eq!(path, "/blog/tech/first-post/");
        assert_eq!(output, PathBuf::from("blog/tech/first-post/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_deeply_nested_section() {
        // blog/tech/_index.md -> /blog/tech/, blog/tech/index.html, Section
        let (path, output, kind) = convert_path("blog/tech/_index.md");
        assert_eq!(path, "/blog/tech/");
        assert_eq!(output, PathBuf::from("blog/tech/index.html"));
        assert_eq!(kind, RouteKind::Section);
    }

    // RouteDiscovery tests

    #[test]
    fn test_discovery_new() {
        let discovery = RouteDiscovery::new("content");
        assert_eq!(discovery.content_dir, PathBuf::from("content"));
    }

    #[test]
    fn test_discover_from_source() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.contains("/"));
        assert!(registry.contains("/about/"));
    }

    #[test]
    fn test_discover_from_source_with_nested() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");
        source.add("blog/_index.md", "+++\ntitle = \"Blog\"\n+++\n");
        source.add("blog/first-post.md", "+++\ntitle = \"First Post\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert_eq!(registry.len(), 4);
        assert!(registry.contains("/"));
        assert!(registry.contains("/about/"));
        assert!(registry.contains("/blog/"));
        assert!(registry.contains("/blog/first-post/"));
    }

    #[test]
    fn test_discover_from_source_empty() {
        let source = MockContentSource::new();

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert!(registry.is_empty());
    }

    #[test]
    fn test_discover_from_source_pages_and_sections() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");
        source.add("blog/_index.md", "+++\ntitle = \"Blog\"\n+++\n");
        source.add("blog/first-post.md", "+++\ntitle = \"First Post\"\n+++\n");
        source.add("blog/second-post.md", "+++\ntitle = \"Second Post\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert_eq!(registry.pages().count(), 3);
        assert_eq!(registry.sections().count(), 2);
    }

    // Helper method tests

    #[test]
    fn test_parent_to_url_path_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_url_path(Path::new(""));
        assert_eq!(path, "/");
    }

    #[test]
    fn test_parent_to_url_path_single() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_url_path(Path::new("blog"));
        assert_eq!(path, "/blog/");
    }

    #[test]
    fn test_parent_to_url_path_nested() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_url_path(Path::new("blog/tech"));
        assert_eq!(path, "/blog/tech/");
    }

    #[test]
    fn test_parent_to_output_file_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_output_file(Path::new(""));
        assert_eq!(path, PathBuf::from("index.html"));
    }

    #[test]
    fn test_parent_to_output_file_single() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_output_file(Path::new("blog"));
        assert_eq!(path, PathBuf::from("blog/index.html"));
    }

    #[test]
    fn test_page_to_url_path_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_url_path(Path::new(""), "about");
        assert_eq!(path, "/about/");
    }

    #[test]
    fn test_page_to_url_path_nested() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_url_path(Path::new("blog"), "first-post");
        assert_eq!(path, "/blog/first-post/");
    }

    #[test]
    fn test_page_to_output_file_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_output_file(Path::new(""), "about");
        assert_eq!(path, PathBuf::from("about/index.html"));
    }

    #[test]
    fn test_page_to_output_file_nested() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_output_file(Path::new("blog"), "first-post");
        assert_eq!(path, PathBuf::from("blog/first-post/index.html"));
    }
}
