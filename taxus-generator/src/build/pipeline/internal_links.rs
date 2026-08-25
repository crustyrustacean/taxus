// generator/src/build/pipeline/internal_links.rs

use crate::error::GeneratorError;
use crate::routes::RouteRegistry;
use std::path::{Path, PathBuf};

/// Resolve internal links in content.
///
/// Internal links use the syntax `](@/path/to/file.md)` where the path is relative
/// to the content directory root. This function resolves them to the actual URL path.
///
/// Code blocks (triple backticks) are skipped to avoid processing example code.
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
        if is_inside_code_block(content, remaining, start) {
            let after_close = start + 4;
            result.push_str(&remaining[..after_close]);
            remaining = &remaining[after_close..];
            continue;
        }

        let bracket_pos = remaining[..start].rfind('[');

        let Some(bracket_pos) = bracket_pos else {
            result.push_str(&remaining[..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        result.push_str(&remaining[..bracket_pos]);

        let link_text = &remaining[bracket_pos + 1..start];

        let after_at = start + 4;
        let end_paren = remaining[after_at..].find(')').map(|p| after_at + p);

        let Some(end_paren) = end_paren else {
            result.push_str(&remaining[bracket_pos..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        let target_path = &remaining[after_at..end_paren];

        let target_pathbuf = PathBuf::from(target_path);
        let route = registry.find_by_content_file(&target_pathbuf);

        let Some(route) = route else {
            return Err(GeneratorError::BrokenInternalLink {
                file: source_file.display().to_string(),
                target: format!("@/{}", target_path),
            });
        };

        result.push_str(&format!(
            "[{}]({})",
            link_text,
            encode_destination(&route.path)
        ));

        remaining = &remaining[end_paren + 1..];
    }

    result.push_str(remaining);

    Ok(result)
}

/// Encode a URL path so the emitted markdown link destination is always
/// parseable by CommonMark (#33).
///
/// Destinations containing raw spaces are not valid CommonMark link
/// targets, so pulldown-cmark refuses to parse them and renders the
/// entire `[text](dest)` construct as literal text — a silent failure
/// with no warning. CommonMark's escaping rules: wrap the destination in
/// `<...>` when it contains whitespace (angle brackets allow spaces
/// except unescaped `<`, `>`, and line breaks), and percent-encode any
/// literal `<`/`>` within.
fn encode_destination(path: &str) -> String {
    if !path.chars().any(|c| c.is_whitespace()) {
        // No spaces: valid as a bare destination. Escape backslashes so
        // pulldown-cmark does not treat them as markdown escapes.
        return path.replace('\\', "\\\\");
    }

    let encoded = path
        .replace('\\', "\\\\")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace('\n', "%0A")
        .replace('\r', "%0D");

    format!("<{encoded}>")
}

fn is_inside_code_block(full_content: &str, remaining: &str, pos: usize) -> bool {
    let offset = full_content.len() - remaining.len();
    let absolute_pos = offset + pos;

    let mut in_code_block = false;
    let mut byte_idx = 0;

    for chunk in full_content.split("```") {
        if byte_idx >= absolute_pos {
            break;
        }
        if byte_idx + chunk.len() >= absolute_pos {
            return in_code_block;
        }
        in_code_block = !in_code_block;
        byte_idx += chunk.len() + 3;
    }

    in_code_block
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

    #[test]
    fn test_resolve_internal_links_inside_code_block() {
        let registry = RouteRegistry::new();

        let content = r#"Here is some text.

```markdown
See my [about page](@/about.md) for more.
```

And [real link](@/about.md) outside.
"#;
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(content, source_file, &registry);
        assert!(
            result.is_err(),
            "Should error on real link outside code block"
        );
    }

    #[test]
    fn test_resolve_internal_links_only_in_code_block() {
        let registry = RouteRegistry::new();

        let content = r#"Here is some text.

```markdown
See my [example](@/path/to/page.md) for more.
```

No other links.
"#;
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(content, source_file, &registry);
        assert!(
            result.is_ok(),
            "Should not error when internal link is only in code block"
        );
        let result = result.unwrap();
        assert!(result.contains("](@/path/to/page.md)"));
    }

    #[test]
    fn test_resolve_internal_links_spaced_target_emits_parseable_markdown() {
        use crate::routes::{RouteInfo, RouteKind};

        // #33: a route path containing spaces must still parse as a link
        // after rewriting. Before the fix, the emitted
        // [text](/My Post/) was invalid CommonMark and rendered as
        // literal text with no warning.
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/My Creative Post/".to_string(),
                    PathBuf::from("My Creative Post.md"),
                    PathBuf::from("My Creative Post/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "See my [post](@/My Creative Post.md).";
        let source_file = Path::new("index.md");
        let rewritten = resolve_internal_links(content, source_file, &registry).unwrap();

        // The rewritten markdown must actually parse as a link — this is
        // the destination-first assertion: not "contains the path" but
        // "renders as an <a> to the path". Spaces in the angle-bracketed
        // destination are percent-encoded by pulldown-cmark, which is the
        // correct URL form.
        let html = crate::build::pipeline::markdown::markdown_to_html(&rewritten, None);
        assert!(
            html.contains(r#"href="/My%20Creative%20Post/""#),
            "rewritten markdown must render as an anchor to the route; got: {html}"
        );
    }
}
