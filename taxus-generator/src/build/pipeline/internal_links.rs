// generator/src/build/pipeline/internal_links.rs

use crate::error::GeneratorError;
use crate::routes::RouteRegistry;
use std::path::{Path, PathBuf};

/// Resolve internal links in content.
///
/// Internal links use the syntax `](@/path/to/file.md)` where the path is relative
/// to the content directory root. This function resolves them to the actual URL path.
///
/// # Errors
///
/// Returns a `BuildError::BrokenInternalLink` if any target path is not found in the registry.
pub fn resolve_internal_links(
    content: &str,
    source_file: &Path,
    registry: &RouteRegistry,
) -> std::result::Result<String, GeneratorError> {
    let mut result = String::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("](@/") {
        // Find the opening bracket to capture the link text
        let bracket_pos = remaining[..start].rfind('[');

        let Some(bracket_pos) = bracket_pos else {
            // No opening bracket found, append and continue
            result.push_str(&remaining[..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        // Append content before the link
        result.push_str(&remaining[..bracket_pos]);

        // Extract link text
        let link_text = &remaining[bracket_pos + 1..start];

        // Find the closing parenthesis
        let after_at = start + 4; // Skip "](@/"
        let end_paren = remaining[after_at..].find(')').map(|p| after_at + p);

        let Some(end_paren) = end_paren else {
            // No closing paren found, append and continue
            result.push_str(&remaining[bracket_pos..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        // Extract the target path
        let target_path = &remaining[after_at..end_paren];

        // Look up the target in the registry
        let target_pathbuf = PathBuf::from(target_path);
        let route = registry.find_by_content_file(&target_pathbuf);

        let Some(route) = route else {
            return Err(GeneratorError::BrokenInternalLink {
                file: source_file.display().to_string(),
                target: format!("@/{}", target_path),
            });
        };

        // Append the resolved link
        result.push_str(&format!("[{}]({})", link_text, route.path));

        // Move past this link
        remaining = &remaining[end_paren + 1..];
    }

    // Append any remaining content
    result.push_str(remaining);

    Ok(result)
}

// ============================================
// Internal Link Resolution Tests
// ============================================

#[test]
fn test_resolve_internal_links_valid_link() {
    use crate::routes::{RouteInfo, RouteKind};

    // Create a registry with a route
    let mut registry = RouteRegistry::new();
    registry
        .register(
            RouteInfo::new(
                "/about/".to_string(),
                PathBuf::from("about.md"),
                PathBuf::from("about/index.html"),
                RouteKind::Page,
            )
            .unwrap(),
        )
        .unwrap();

    let content = "See my [about page](@/about.md) for more details.";
    let source_file = Path::new("blog/my-post.md");
    let result = resolve_internal_links(content, source_file, &registry).unwrap();

    assert_eq!(result, "See my [about page](/about/) for more details.");
}

#[test]
fn test_resolve_internal_links_unknown_target() {
    let registry = RouteRegistry::new();

    let content = "See my [about page](@/about.md) for more details.";
    let source_file = Path::new("blog/my-post.md");
    let result = resolve_internal_links(content, source_file, &registry);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, GeneratorError::BrokenInternalLink { .. }));
    if let GeneratorError::BrokenInternalLink { file, target } = err {
        assert_eq!(file, "blog/my-post.md");
        assert_eq!(target, "@/about.md");
    }
}

#[test]
fn test_resolve_internal_links_no_internal_links() {
    let registry = RouteRegistry::new();

    let content = "This is plain text with [a normal link](https://example.com).";
    let source_file = Path::new("test.md");
    let result = resolve_internal_links(content, source_file, &registry).unwrap();

    assert_eq!(
        result,
        "This is plain text with [a normal link](https://example.com)."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_internal_links_multiple_links() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry with multiple routes
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/about/".to_string(),
                    PathBuf::from("about.md"),
                    PathBuf::from("about/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();
        registry
            .register(
                RouteInfo::new(
                    "/blog/first-post/".to_string(),
                    PathBuf::from("blog/first-post.md"),
                    PathBuf::from("blog/first-post/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "See my [about page](@/about.md) and [first post](@/blog/first-post.md).";
        let source_file = Path::new("test.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "See my [about page](/about/) and [first post](/blog/first-post/)."
        );
    }

    #[test]
    fn test_resolve_internal_links_nested_path() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry with a nested route
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/docs/guide/getting-started/".to_string(),
                    PathBuf::from("docs/guide/getting-started.md"),
                    PathBuf::from("docs/guide/getting-started/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "Read the [getting started guide](@/docs/guide/getting-started.md).";
        let source_file = Path::new("index.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "Read the [getting started guide](/docs/guide/getting-started/)."
        );
    }

    #[test]
    fn test_resolve_internal_links_mixed_links() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/about/".to_string(),
                    PathBuf::from("about.md"),
                    PathBuf::from("about/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "Check [external](https://example.com) and [internal](@/about.md) links.";
        let source_file = Path::new("test.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "Check [external](https://example.com) and [internal](/about/) links."
        );
    }
}
