//! Storage-only ("flattened") content directory patterns.
//!
//! A `[build] flatten` pattern names content directories that exist only
//! to organise files on disk. A matching directory never becomes a
//! section or a URL segment: a page inside it belongs to the nearest
//! non-flattened ancestor. Patterns are interpreted by
//! [`RouteDiscovery`](crate::routes::RouteDiscovery) before any path
//! enters the Site Tree; the tree never sees a flattened segment.
//!
//! # Syntax
//!
//! - a content-relative directory path, `/`-separated (`\` is accepted
//!   and normalised to `/`);
//! - `*` matches exactly one path segment (`blog/*` matches `blog/2026`,
//!   not `blog` or `blog/2026/04`); there is no `**`;
//! - any other segment is literal (`blog/drafts`), compared
//!   case-sensitively against the raw, pre-slugify directory name.

use std::path::Path;

/// One segment of a [`FlattenPattern`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// `*`: any single segment.
    Any,
    /// A literal directory name.
    Literal(String),
}

/// A parsed `[build] flatten` pattern. See the [module docs](self).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlattenPattern {
    raw: String,
    segments: Vec<Segment>,
}

impl FlattenPattern {
    /// Parse a pattern.
    ///
    /// # Errors
    ///
    /// A human-readable message if the pattern is empty, starts with `/`,
    /// has an empty segment (`blog//x`, a trailing `/`), a `.` or `..`
    /// segment, or a `*` anywhere other than as a whole segment (`**`,
    /// `20*`).
    pub fn parse(pattern: &str) -> Result<FlattenPattern, String> {
        let normalised = pattern.replace('\\', "/");
        if normalised.is_empty() {
            return Err("pattern is empty".to_string());
        }
        if normalised.starts_with('/') {
            return Err(
                "pattern must be relative to the content directory (no leading `/`)".into(),
            );
        }
        let segments = normalised
            .split('/')
            .map(|seg| match seg {
                "" => Err("pattern has an empty segment (`//` or a trailing `/`)".to_string()),
                "." | ".." => Err(format!("pattern may not contain a `{seg}` segment")),
                "*" => Ok(Segment::Any),
                s if s.contains('*') => Err(format!(
                    "`{s}`: `*` must be a whole segment and matches exactly one directory \
                     (`**` and partial wildcards are not supported)"
                )),
                s => Ok(Segment::Literal(s.to_string())),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FlattenPattern {
            raw: pattern.to_string(),
            segments,
        })
    }

    /// Parse every pattern, stopping at the first invalid one.
    ///
    /// # Errors
    ///
    /// `"invalid build.flatten pattern \"<pattern>\": <reason>"`.
    pub fn parse_all(patterns: &[String]) -> Result<Vec<FlattenPattern>, String> {
        patterns
            .iter()
            .map(|p| {
                Self::parse(p).map_err(|e| format!("invalid build.flatten pattern \"{p}\": {e}"))
            })
            .collect()
    }

    /// Whether the content-relative directory `dir` matches this pattern.
    ///
    /// Compared component by component, so a native Windows path
    /// (`blog\2026`) matches the same as `blog/2026`. The segment counts
    /// must be equal: `blog/*` does not match `blog` or `blog/2026/04`.
    pub fn matches(&self, dir: &Path) -> bool {
        let mut components = dir.components();
        for segment in &self.segments {
            let Some(component) = components.next() else {
                return false;
            };
            let std::path::Component::Normal(name) = component else {
                return false;
            };
            if let Segment::Literal(lit) = segment
                && name.to_str() != Some(lit.as_str())
            {
                return false;
            }
        }
        components.next().is_none()
    }

    /// The pattern as written in `site.toml`.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl std::fmt::Display for FlattenPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> FlattenPattern {
        FlattenPattern::parse(s).unwrap()
    }

    #[test]
    fn valid_patterns_parse() {
        for s in [
            "blog/*",
            "blog/drafts",
            "*",
            "blog/*/*",
            "a b/*",
            "*/archive",
        ] {
            let pat = p(s);
            assert_eq!(pat.as_str(), s);
        }
    }

    #[test]
    fn invalid_patterns_are_rejected() {
        for s in [
            "",
            "/blog",
            "/",
            "blog//x",
            "blog/",
            "../blog",
            "blog/..",
            "./blog",
            "blog/.",
            "**",
            "blog/**",
            "blog/20*",
            "*x",
            "blog\\\\x",
        ] {
            assert!(
                FlattenPattern::parse(s).is_err(),
                "{s:?} should be rejected"
            );
        }
    }

    #[test]
    fn star_matches_exactly_one_segment() {
        let pat = p("blog/*");
        assert!(pat.matches(Path::new("blog/2026")));
        assert!(pat.matches(Path::new("blog/drafts")));
        assert!(!pat.matches(Path::new("blog")));
        assert!(!pat.matches(Path::new("blog/2026/04")));
        assert!(!pat.matches(Path::new("notes/2026")));
        assert!(!pat.matches(Path::new("")));
    }

    #[test]
    fn literal_patterns_match_only_themselves_case_sensitively() {
        let pat = p("blog/drafts");
        assert!(pat.matches(Path::new("blog/drafts")));
        assert!(!pat.matches(Path::new("blog/2026")));
        assert!(!pat.matches(Path::new("blog/Drafts")));
        assert!(!pat.matches(Path::new("Blog/drafts")));
    }

    #[test]
    fn matches_against_raw_unslugified_names() {
        assert!(p("Blog Posts/*").matches(Path::new("Blog Posts/2026")));
        assert!(!p("blog-posts/*").matches(Path::new("Blog Posts/2026")));
    }

    #[test]
    fn backslashes_in_pattern_are_normalised() {
        let pat = p("blog\\*");
        assert_eq!(pat.as_str(), "blog\\*");
        assert!(pat.matches(Path::new("blog/2026")));
    }

    #[test]
    fn matches_native_joined_paths() {
        // `Path::join` uses `\` on Windows.
        let dir = PathBuf::from("blog").join("2026");
        assert!(p("blog/*").matches(&dir));
        #[cfg(windows)]
        assert!(p("blog/*").matches(Path::new(r"blog\2026")));
    }

    #[test]
    fn parse_all_names_the_offending_pattern() {
        let ok = FlattenPattern::parse_all(&["blog/*".into(), "notes".into()]).unwrap();
        assert_eq!(ok.len(), 2);
        let err = FlattenPattern::parse_all(&["blog/*".into(), "blog/**".into()]).unwrap_err();
        assert!(err.contains("build.flatten"), "{err}");
        assert!(err.contains("\"blog/**\""), "{err}");
    }
}
