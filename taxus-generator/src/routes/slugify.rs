// taxus-generator/src/routes/slugify.rs

//! Path slugification (#27).
//!
//! URL paths are derived from content filenames. Used verbatim, a file
//! named `My Créative Post.md` produces the URL `/My Créative Post/` —
//! a raw space and non-ASCII bytes flowing into hrefs, the sitemap
//! (where spaces are invalid), and feed links.
//!
//! Three modes, mirroring Zola's `slugify.paths`, chosen in `site.toml`:
//!
//! - `"on"` (default): lowercase, whitespace → `-`, non-ASCII
//!   transliterated to ASCII, remaining punctuation stripped
//! - `"safe"`: non-ASCII preserved (for languages where transliteration
//!   destroys meaning), but lowercase + whitespace normalization still
//!   applied
//! - `"off"`: verbatim — today's behavior, opt-in for sites that
//!   already depend on it
//!
//! Slugs are guaranteed non-empty: an input that slugifies to nothing
//! (e.g. a file named `().md`) falls back to `"page"` so route
//! generation never produces `//` or an empty segment.

use deunicode::deunicode;

/// Slugification mode from `site.toml` (`[build] slugify`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlugMode {
    /// Lowercase, transliterate non-ASCII, strip punctuation. Default.
    #[default]
    On,
    /// Preserve non-ASCII; still normalize case and whitespace.
    Safe,
    /// No transformation at all.
    Off,
}

impl SlugMode {
    /// Parse from a config string. Unknown values fall back to `On`
    /// rather than failing the build: a typo in the mode name should
    /// not take down a site, and `On` is the safest default.
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "safe" => SlugMode::Safe,
            "off" => SlugMode::Off,
            _ => SlugMode::On,
        }
    }
}

/// Slugify a single path segment (a filename stem or directory name).
///
/// Applies the given mode's transformations. The result never contains
/// whitespace and never begins or ends with `-`. Returns `"page"` when
/// everything is stripped — callers use it as a route segment and must
/// never embed an empty one.
pub fn slugify_segment(segment: &str, mode: SlugMode) -> String {
    if mode == SlugMode::Off {
        return segment.to_string();
    }

    let normalized: String = match mode {
        SlugMode::Safe => segment.to_lowercase(),
        _ => deunicode(segment).to_lowercase(),
    };

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
pub fn slugify_path(relative: &str, mode: SlugMode) -> String {
    relative
        .split('/')
        .map(|seg| slugify_segment(seg, mode))
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
    fn test_on_mode_basic() {
        assert_eq!(
            slugify_segment("My Créative Post", SlugMode::On),
            "my-creative-post"
        );
    }

    #[test]
    fn test_on_mode_strips_punctuation() {
        assert_eq!(
            slugify_segment("Hello, World! (v2)", SlugMode::On),
            "hello-world-v2"
        );
    }

    #[test]
    fn test_on_mode_transliterates() {
        // Cyrillic, CJK, and accented text all become ASCII approximations.
        assert_eq!(slugify_segment("Ünïcödé", SlugMode::On), "unicode");
        assert!(!slugify_segment("日本語", SlugMode::On).is_empty());
    }

    #[test]
    fn test_safe_mode_preserves_non_ascii() {
        // Safe keeps the é; On would not.
        assert_eq!(slugify_segment("Café", SlugMode::Safe), "café");
        assert_eq!(slugify_segment("Café", SlugMode::On), "cafe");
    }

    #[test]
    fn test_safe_mode_still_normalizes() {
        assert_eq!(
            slugify_segment("My Créative Post", SlugMode::Safe),
            "my-créative-post"
        );
    }

    #[test]
    fn test_off_mode_verbatim() {
        assert_eq!(
            slugify_segment("My Créative Post", SlugMode::Off),
            "My Créative Post"
        );
    }

    #[test]
    fn test_empty_fallback() {
        assert_eq!(slugify_segment("()", SlugMode::On), "page");
        assert_eq!(slugify_segment("---", SlugMode::On), "page");
        assert_eq!(slugify_segment("", SlugMode::On), "page");
    }

    #[test]
    fn test_no_leading_or_trailing_dashes() {
        assert_eq!(slugify_segment("--Leading", SlugMode::On), "leading");
        assert_eq!(slugify_segment("Trailing--", SlugMode::On), "trailing");
        assert_eq!(slugify_segment("a - b", SlugMode::On), "a-b");
    }

    #[test]
    fn test_underscores_become_dashes() {
        // Filenames commonly use snake_case; URLs conventionally don't.
        assert_eq!(slugify_segment("my_old_post", SlugMode::On), "my-old-post");
    }

    #[test]
    fn test_numeric_segments() {
        assert_eq!(
            slugify_segment("2026-08-25-release", SlugMode::On),
            "2026-08-25-release"
        );
    }

    #[test]
    fn test_path_slugification() {
        assert_eq!(
            slugify_path("blog/My Old Post", SlugMode::On),
            "blog/my-old-post"
        );
    }

    #[test]
    fn test_mode_parsing() {
        assert_eq!(SlugMode::from_config("on"), SlugMode::On);
        assert_eq!(SlugMode::from_config("safe"), SlugMode::Safe);
        assert_eq!(SlugMode::from_config("off"), SlugMode::Off);
        assert_eq!(SlugMode::from_config("OFF"), SlugMode::Off);
        // Unknown values fall back to On
        assert_eq!(SlugMode::from_config("typo"), SlugMode::On);
        assert_eq!(SlugMode::default(), SlugMode::On);
    }
}
