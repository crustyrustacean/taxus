//! Route registry and route information types.
//!
//! This module provides the core types for representing and storing routes.

use crate::error::RouteError;
use std::collections::HashMap;
use std::path::PathBuf;

/// The type of route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteKind {
    /// A single page (e.g., /about/)
    Page,
    /// A section index (e.g., /blog/)
    Section,
}

impl RouteKind {
    /// Returns true if this is a page route.
    pub fn is_page(&self) -> bool {
        matches!(self, RouteKind::Page)
    }

    /// Returns true if this is a section route.
    pub fn is_section(&self) -> bool {
        matches!(self, RouteKind::Section)
    }
}

/// Information about a single route.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteInfo {
    /// URL path (e.g., "/about/")
    pub path: String,
    /// Content file path relative to content directory
    pub content_file: PathBuf,
    /// Output file path relative to output directory
    pub output_file: PathBuf,
    /// Route type
    pub kind: RouteKind,
}

impl RouteInfo {
    /// Create a new route info.
    ///
    /// # Errors
    ///
    /// Returns an error if the path is not valid (must start with `/` and end with `/`,
    /// except for the root path which is just `/`).
    pub fn new(
        path: String,
        content_file: PathBuf,
        output_file: PathBuf,
        kind: RouteKind,
    ) -> Result<Self, RouteError> {
        // Validate path format
        if !Self::is_valid_path(&path) {
            return Err(RouteError::InvalidPath(path));
        }

        Ok(Self {
            path,
            content_file,
            output_file,
            kind,
        })
    }

    /// Check if the path format is valid.
    ///
    /// Valid paths:
    /// - "/" (root)
    /// - "/about/" (trailing slash required for non-root)
    fn is_valid_path(path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        if !path.starts_with('/') {
            return false;
        }
        // Root path is just "/"
        if path == "/" {
            return true;
        }
        // All other paths must end with "/"
        path.ends_with('/')
    }

    /// Returns true if this is a page route.
    pub fn is_page(&self) -> bool {
        self.kind.is_page()
    }

    /// Returns true if this is a section route.
    pub fn is_section(&self) -> bool {
        self.kind.is_section()
    }
}

/// Registry of all routes in the site.
#[derive(Debug, Clone, Default)]
pub struct RouteRegistry {
    routes: HashMap<String, RouteInfo>,
}

impl RouteRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register a route.
    ///
    /// # Errors
    ///
    /// Returns an error if a route with the same path already exists.
    pub fn register(&mut self, route: RouteInfo) -> Result<(), RouteError> {
        if self.routes.contains_key(&route.path) {
            return Err(RouteError::Duplicate(route.path));
        }
        self.routes.insert(route.path.clone(), route);
        Ok(())
    }

    /// Get route by path.
    pub fn get(&self, path: &str) -> Option<&RouteInfo> {
        self.routes.get(path)
    }

    /// Check if a route exists.
    pub fn contains(&self, path: &str) -> bool {
        self.routes.contains_key(path)
    }

    /// Get the number of routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Iterate over all routes.
    pub fn iter(&self) -> impl Iterator<Item = &RouteInfo> {
        self.routes.values()
    }

    /// Iterate over all page routes.
    pub fn pages(&self) -> impl Iterator<Item = &RouteInfo> {
        self.routes.values().filter(|r| r.is_page())
    }

    /// Iterate over all section routes.
    pub fn sections(&self) -> impl Iterator<Item = &RouteInfo> {
        self.routes.values().filter(|r| r.is_section())
    }

    /// Generate Rust code for client routing (for future use).
    ///
    /// This is a stub implementation that will be expanded in a future phase
    /// to generate code for the client-side router.
    pub fn generate_rust_manifest(&self) -> String {
        let mut output = String::new();
        output.push_str("// Auto-generated route manifest\n");
        output.push_str("use yew_router::Routable;\n\n");
        output.push_str("#[derive(Routable, Clone, PartialEq)]\n");
        output.push_str("pub enum Route {\n");

        for route in self.routes.values() {
            let route_path = if route.path == "/" {
                "".to_string()
            } else {
                route.path.trim_end_matches('/').to_string()
            };
            let variant_name = if route.path == "/" {
                "Home".to_string()
            } else {
                route
                    .path
                    .trim_matches('/')
                    .trim_end_matches('/')
                    .split('/')
                    .map(|s| {
                        let mut chars = s.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            };
            output.push_str(&format!("    #[at(\"{}\")]\n", route_path));
            output.push_str(&format!("    {},\n", variant_name));
        }

        output.push_str("}\n");
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RouteKind tests

    #[test]
    fn test_route_kind_equality() {
        assert_eq!(RouteKind::Page, RouteKind::Page);
        assert_ne!(RouteKind::Page, RouteKind::Section);
    }

    #[test]
    fn test_route_kind_clone() {
        let kind = RouteKind::Page;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    #[test]
    fn test_route_kind_is_page() {
        assert!(RouteKind::Page.is_page());
        assert!(!RouteKind::Section.is_page());
    }

    #[test]
    fn test_route_kind_is_section() {
        assert!(RouteKind::Section.is_section());
        assert!(!RouteKind::Page.is_section());
    }

    // RouteInfo tests

    #[test]
    fn test_route_info_new_page() {
        let route = RouteInfo::new(
            "/about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        assert_eq!(route.path, "/about/");
        assert!(route.is_page());
        assert!(!route.is_section());
    }

    #[test]
    fn test_route_info_new_section() {
        let route = RouteInfo::new(
            "/blog/".to_string(),
            PathBuf::from("blog/_index.md"),
            PathBuf::from("blog/index.html"),
            RouteKind::Section,
        )
        .unwrap();

        assert_eq!(route.path, "/blog/");
        assert!(route.is_section());
        assert!(!route.is_page());
    }

    #[test]
    fn test_route_info_root_path() {
        let route = RouteInfo::new(
            "/".to_string(),
            PathBuf::from("_index.md"),
            PathBuf::from("index.html"),
            RouteKind::Section,
        )
        .unwrap();

        assert_eq!(route.path, "/");
        assert!(route.is_section());
    }

    #[test]
    fn test_route_info_invalid_path_no_leading_slash() {
        let result = RouteInfo::new(
            "about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::InvalidPath(_)));
    }

    #[test]
    fn test_route_info_invalid_path_no_trailing_slash() {
        let result = RouteInfo::new(
            "/about".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::InvalidPath(_)));
    }

    #[test]
    fn test_route_info_invalid_path_empty() {
        let result = RouteInfo::new(
            "".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::InvalidPath(_)));
    }

    // RouteRegistry tests

    #[test]
    fn test_registry_new() {
        let registry = RouteRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = RouteRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = RouteRegistry::new();
        let route = RouteInfo::new(
            "/about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        registry.register(route).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("/about/"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = RouteRegistry::new();
        let route = RouteInfo::new(
            "/about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        registry.register(route).unwrap();
        let retrieved = registry.get("/about/").unwrap();
        assert_eq!(retrieved.path, "/about/");
        assert_eq!(retrieved.content_file, PathBuf::from("about.md"));
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = RouteRegistry::new();
        assert!(registry.get("/missing/").is_none());
    }

    #[test]
    fn test_registry_duplicate() {
        let mut registry = RouteRegistry::new();
        let route1 = RouteInfo::new(
            "/about/".to_string(),
            PathBuf::from("about.md"),
            PathBuf::from("about/index.html"),
            RouteKind::Page,
        )
        .unwrap();
        let route2 = route1.clone();

        registry.register(route1).unwrap();
        let result = registry.register(route2);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RouteError::Duplicate(_)));
    }

    #[test]
    fn test_registry_iterators() {
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

        registry
            .register(RouteInfo::new(
                "/blog/".to_string(),
                PathBuf::from("blog/_index.md"),
                PathBuf::from("blog/index.html"),
                RouteKind::Section,
            )
            .unwrap())
            .unwrap();

        registry
            .register(RouteInfo::new(
                "/blog/first-post/".to_string(),
                PathBuf::from("blog/first-post.md"),
                PathBuf::from("blog/first-post/index.html"),
                RouteKind::Page,
            )
            .unwrap())
            .unwrap();

        assert_eq!(registry.len(), 4);
        assert_eq!(registry.pages().count(), 2);
        assert_eq!(registry.sections().count(), 2);
    }

    #[test]
    fn test_registry_iter() {
        let mut registry = RouteRegistry::new();

        registry
            .register(RouteInfo::new(
                "/about/".to_string(),
                PathBuf::from("about.md"),
                PathBuf::from("about/index.html"),
                RouteKind::Page,
            )
            .unwrap())
            .unwrap();

        registry
            .register(RouteInfo::new(
                "/blog/".to_string(),
                PathBuf::from("blog/_index.md"),
                PathBuf::from("blog/index.html"),
                RouteKind::Section,
            )
            .unwrap())
            .unwrap();

        let paths: Vec<&str> = registry.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"/about/"));
        assert!(paths.contains(&"/blog/"));
    }

    #[test]
    fn test_registry_generate_rust_manifest() {
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
        assert!(manifest.contains("Auto-generated route manifest"));
        assert!(manifest.contains("Route"));
        assert!(manifest.contains("Home"));
        assert!(manifest.contains("About"));
    }
}