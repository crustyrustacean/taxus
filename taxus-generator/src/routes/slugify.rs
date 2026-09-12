// taxus-generator/src/routes/slugify.rs

//! URL slug derivation from filenames and heading text.
//!
//! Used verbatim, a file named `My Créative Post.md` produces the URL
//! `/My Créative Post/` — a raw space and non-ASCII bytes flowing into
//! hrefs, the sitemap (where spaces are invalid), and feed links.
//!
//! This module owns the two slug algorithms, one per concern:
//!
//! - [`slugify_segment`] — **node paths.** Lowercase, whitespace → `-`,
//!   non-ASCII transliterated to ASCII, remaining punctuation stripped.
//!   This is the behavior taxus has always shipped as its default; the
//!   configurable modes from #27 (`safe`/`off`) were removed as unowned
//!   knobs.
//! - [`slugify_term`] — **taxonomy terms.** Lowercase, spaces/underscores
//!   → `-`, punctuation stripped, but non-ASCII letters kept: a term is
//!   display-facing and `Café` should stay `café`, not flatten to `cafe`.
//!
//! Both are one derivation point per concept: templates reach the term
//! rule through the `term_slug` filter, so a tag's term page and the
//! links to it can never disagree. Phase: parse. See the book's
//! [Identity](https://crustyrustacean.github.io/taxus/theory/identity.html) chapter.
//!
//! Slugs are guaranteed non-empty: an input that slugifies to nothing
//! (e.g. a file named `().md`) falls back to `"page"` so route
//! generation never produces `//` or an empty segment.

use deunicode::deunicode;

/// Slugify a single path segment (a filename stem, directory name, or
/// heading text).
///
/// Lowercases, transliterates non-ASCII to ASCII, collapses whitespace
/// and punctuation runs into single dashes, and never begins or ends
/// with `-`. Returns `"page"` when everything is stripped — callers use
/// it as a route segment and must never embed an empty one.
pub fn slugify_segment(segment: &str) -> String {
    let normalized = deunicode(segment).to_lowercase();

    let mut slug = String::with_capacity(normalized.len());
    let mut pending_dash = false;

    for ch in normalized.chars() {
        if ch.is_alphanumeric() {
            // Flush any separator accumulated by preceding whitespace or
            // punctuation before pushing the character.
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch);
        } else {
            // Whitespace or punctuation: act as a separator. Collapses
            // runs of separators into a single dash, never leading.
            pending_dash = true;
        }
    }

    if slug.is_empty() {
        "page".to_string()
    } else {
        slug
    }
}

/// Slugify a whole relative path's worth of segments, preserving
/// directory structure: `blog/My Old Post` → `blog/my-old-post`.
pub fn slugify_path(relative: &str) -> String {
    relative
        .split('/')
        .map(slugify_segment)
        .collect::<Vec<_>>()
        .join("/")
}

/// Slugify a taxonomy term name (a tag, category or series value).
///
/// Lowercases, replaces spaces and underscores with dashes, strips
/// punctuation, collapses dash runs, and trims edges. Unlike
/// [`slugify_segment`] it **keeps non-ASCII letters**: a term is
/// display-facing and `Café` stays `café`. Term pages
/// (`/tags/café/`) and template links (the `term_slug` filter) both
/// derive from this function, so they can never disagree.
///
/// May return the empty string for input that is entirely punctuation
/// (e.g. `"$"`); the caller decides what to do with a term that has no
/// usable slug.
pub fn slugify_term(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .replace([' ', '_'], "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    // Collapse consecutive dashes into one
    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
                prev_dash = true;
            }
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    // Trim leading and trailing dashes
    result.trim_matches('-').to_string()
}

// ============================================
// Slugification Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert_eq!(slugify_segment("My Créative Post"), "my-creative-post");
    }

    #[test]
    fn test_strips_punctuation() {
        assert_eq!(slugify_segment("Hello, World! (v2)"), "hello-world-v2");
    }

    #[test]
    fn test_transliterates() {
        // Cyrillic, CJK, and accented text all become ASCII approximations.
        assert_eq!(slugify_segment("Ünïcödé"), "unicode");
        assert!(!slugify_segment("日本語").is_empty());
    }

    #[test]
    fn test_empty_fallback() {
        assert_eq!(slugify_segment("()"), "page");
        assert_eq!(slugify_segment("---"), "page");
        assert_eq!(slugify_segment(""), "page");
    }

    #[test]
    fn test_no_leading_or_trailing_dashes() {
        assert_eq!(slugify_segment("--Leading"), "leading");
        assert_eq!(slugify_segment("Trailing--"), "trailing");
        assert_eq!(slugify_segment("a - b"), "a-b");
    }

    #[test]
    fn test_underscores_become_dashes() {
        // Filenames commonly use snake_case; URLs conventionally don't.
        assert_eq!(slugify_segment("my_old_post"), "my-old-post");
    }

    #[test]
    fn test_numeric_segments() {
        assert_eq!(slugify_segment("2026-08-25-release"), "2026-08-25-release");
    }

    #[test]
    fn test_path_slugification() {
        assert_eq!(slugify_path("blog/My Old Post"), "blog/my-old-post");
    }

    #[test]
    fn test_term_slug_basic() {
        assert_eq!(slugify_term("Rust"), "rust");
        assert_eq!(slugify_term("Web Development"), "web-development");
        assert_eq!(slugify_term("Hello_World"), "hello-world");
        assert_eq!(slugify_term("Test & Demo!"), "test-demo");
        assert_eq!(slugify_term("Multiple   Spaces"), "multiple-spaces");
    }

    #[test]
    fn test_term_slug_keeps_non_ascii_letters() {
        // The term rule is display-facing: `Café` stays `café`, while the
        // node-path rule transliterates to ASCII (`cafe`). Templates reach
        // this rule through the `term_slug` filter, so term pages and the
        // links to them always agree.
        assert_eq!(slugify_term("Café"), "café");
        assert_eq!(slugify_term("Ünïcödé"), "ünïcödé");
        assert_eq!(slugify_term("日本語"), "日本語");
    }

    #[test]
    fn test_term_slug_contrast_with_segment() {
        // Pin the one behavioral difference between the two algorithms.
        assert_eq!(slugify_segment("Café"), "cafe");
        assert_eq!(slugify_term("Café"), "café");
    }

    #[test]
    fn test_term_slug_edge_cases() {
        assert_eq!(slugify_term("leading"), "leading");
        assert_eq!(slugify_term("-edges-"), "edges");
        assert_eq!(slugify_term("collapse--dashes"), "collapse-dashes");
        // Entirely punctuation: no usable slug. The caller decides.
        assert_eq!(slugify_term("$"), "");
        assert_eq!(slugify_term(""), "");
    }
}
