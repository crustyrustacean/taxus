// generator/src/build/pipeline/internal_links.rs

//! Stage 3 (emit): resolve `@/path.md` links to URL paths.
//!
//! An internal link names a content file; the route registry (a
//! projection of the Site Tree) maps it to the document's address, so a
//! renamed or re-slugged page never leaves a broken link behind. A target
//! that names no document fails the build. See the book's
//! [Identity](https://crustyrustacean.github.io/taxus/theory/identity.html) chapter.
//!
//! Code immunity is structural (#6): `pulldown_cmark` reports the exact
//! source range of every code construct — fenced (backtick or tilde, any
//! fence length), indented, and inline spans — and the resolver skips
//! `@/`-pattern matches falling inside any of those ranges. The old
//! `split("```")` heuristic misclassified indented blocks, tilde fences,
//! four-backtick fences, and inline backtick mentions.

use crate::error::GeneratorError;
use crate::routes::RouteRegistry;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Resolve internal links in content.
///
/// Internal links use the syntax `](@/path/to/file.md)` where the path is relative
/// to the content directory root. This function resolves them to the actual URL path.
///
/// Matches inside code — fenced (backtick or tilde, any length), indented,
/// or inline spans — are left untouched: their byte ranges are reported
/// by the Markdown parser and skipped (#6).
///
/// # Errors
///
/// Returns a `BuildError::BrokenInternalLink` if any target path is not found in the registry.
pub fn resolve_internal_links(
    content: &str,
    source_file: &Path,
    registry: &RouteRegistry,
) -> std::result::Result<String, GeneratorError> {
    let code_ranges = code_block_ranges(content);
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while let Some(start) = remaining.find("](@/") {
        // `start` is relative to `remaining`; the ranges are absolute
        // into `content`. Same arithmetic as the old heuristic, minus
        // the heuristic.
        let absolute_start = content.len() - remaining.len() + start;
        if in_ranges(&code_ranges, absolute_start) {
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

/// Byte ranges of every code construct in the document (#6).
///
/// `pulldown_cmark` reports the source range of `Event::Code` (inline
/// spans) and `Start(CodeBlock(..))`/`End(CodeBlock)` (fenced and
/// indented blocks). Ranges are absolute into `content`, sorted by the
/// event order, and non-overlapping.
fn code_block_ranges(content: &str) -> Vec<Range<usize>> {
    let parser = Parser::new_ext(content, Options::all());
    let mut ranges = Vec::new();
    let mut block_start: Option<usize> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Code(_) => ranges.push(range),
            Event::Start(Tag::CodeBlock(_)) => block_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = block_start.take() {
                    ranges.push(start..range.end);
                }
            }
            _ => {}
        }
    }

    ranges
}

/// Whether `pos` (an offset into the full content) falls inside any of
/// the sorted, non-overlapping `ranges`.
fn in_ranges(ranges: &[Range<usize>], pos: usize) -> bool {
    ranges
        .binary_search_by(|r| {
            if pos < r.start {
                std::cmp::Ordering::Greater
            } else if pos >= r.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
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
    // ------------------------------------------------------------------
    // #6: structural code-block immunity — the old ```-split heuristic
    // misclassifies these.
    // ------------------------------------------------------------------

    /// A link in an INDENTED (4-space) code block must not resolve.
    #[test]
    fn test_resolve_internal_links_indented_code_block() {
        let registry = RouteRegistry::new();

        let content = "Here is some text.

    See my [example](@/nope.md) inside.

Done.
";
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(content, source_file, &registry);
        assert!(
            result.is_ok(),
            "an @/ link in an indented code block must not resolve"
        );
        assert!(result.unwrap().contains("](@/nope.md)"));
    }

    /// A link inside INLINE code (single backticks) must not resolve —
    /// the heuristic treats any backtick-bearing text as fence-adjacent.
    #[test]
    fn test_resolve_internal_links_inline_code_span() {
        let registry = RouteRegistry::new();

        let content = "Use the syntax `[x](@/nope.md)` carefully.
";
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(content, source_file, &registry);
        assert!(
            result.is_ok(),
            "an @/ link inside an inline code span must not resolve"
        );
        assert!(result.unwrap().contains("](@/nope.md)"));
    }

    /// Tilde fences are code blocks too.
    #[test]
    fn test_resolve_internal_links_tilde_fence() {
        let registry = RouteRegistry::new();

        let content = "~~~
See [example](@/nope.md) inside.
~~~
";
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(content, source_file, &registry);
        assert!(result.is_ok(), "tilde-fenced code must not resolve");
        assert!(result.unwrap().contains("](@/nope.md)"));
    }

    /// Inline code MENTIONING triple backticks must not toggle the
    /// heuristic's fence state and swallow later real links.
    #[test]
    fn test_resolve_internal_links_backtick_mention_does_not_toggle() {
        let mut registry = RouteRegistry::new();
        use crate::routes::{RouteInfo, RouteKind};
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

        // The phrase "use ``` to fence" in inline code, then a REAL link.
        // Built by concatenation so the triple backticks need no escapes.
        let ticks: String = std::iter::repeat_n('`', 3).collect();
        let content = format!("Use `{ticks}` to fence blocks. Then [about](@/about.md).\n");
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(&content, source_file, &registry);
        assert!(
            result.is_ok(),
            "a real link after an inline backtick mention must resolve: {result:?}"
        );
        let result = result.unwrap();
        assert!(
            result.contains("](/about/)"),
            "the real link should be rewritten, got: {result}"
        );
    }

    /// FOUR-backtick fences contain triple-backtick bodies — the
    /// ```-split heuristic desynchronises on these.
    #[test]
    fn test_resolve_internal_links_four_backtick_fence() {
        let mut registry = RouteRegistry::new();
        use crate::routes::{RouteInfo, RouteKind};
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

        let content = "````
```
[example](@/nope.md)
```
````

Then [about](@/about.md).
";
        let source_file = Path::new("test.md");

        let result = resolve_internal_links(content, source_file, &registry);
        assert!(
            result.is_ok(),
            "the inner nope.md link is inside the four-backtick fence; the real link must resolve"
        );
        let result = result.unwrap();
        assert!(result.contains("](@/nope.md)"), "inner link untouched");
        assert!(result.contains("](/about/)"), "real link rewritten");
    }
}
