// taxus-generator/src/routes/slugify.rs

//! URL slug derivation from filenames and heading text.
//!
//! Used verbatim, a file named `My Créative Post.md` produces the URL
//! `/My Créative Post/` — a raw space and non-ASCII bytes flowing into
//! hrefs, the sitemap (where spaces are invalid), and feed links.
//!
//! There is exactly one slug algorithm for node paths (one derivation
//! point per concept): lowercase, whitespace → `-`, non-ASCII
//! transliterated to ASCII, remaining punctuation stripped. This is the
//! behavior taxus has always shipped as its default; the configurable
//! modes from #27 (`safe`/`off`) were removed as unowned knobs. Taxonomy
//! term slugs and the Tera `slugify` filter use their own, similar rules.
//! Phase: parse. See the book's
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
}
