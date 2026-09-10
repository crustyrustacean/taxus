// taxus-domain/src/identity.rs

//! Identity: slugs, membership paths, and derived URL paths.
//!
//! One derivation point for addresses ([`UrlPath::from_node_path`])
//! invariant 3.

use std::fmt;

/// A single URL path segment, already slugified by the caller.
///
/// The domain does not slugify: the generator owns the one slug algorithm
/// (transliteration, lowercasing, separator collapsing) and frontmatter
/// `slug` overrides are used verbatim. This type only guarantees that the
/// segment can stand alone in a path: it is non-empty, contains no `/`, is
/// neither `.` nor `..`, and has no ASCII control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slug(String);

impl Slug {
    /// Validate and create a slug.
    pub fn new(raw: &str) -> Result<Self, IdentityError> {
        if raw.is_empty() {
            return Err(IdentityError::Empty);
        }
        if raw.contains('/') {
            return Err(IdentityError::Slash { s: raw.to_owned() });
        }
        if raw == "." || raw == ".." {
            return Err(IdentityError::DotSegment { s: raw.to_owned() });
        }
        if raw.bytes().any(|b| b.is_ascii_control()) {
            return Err(IdentityError::ControlCharacter { s: raw.to_owned() });
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Slug {
    type Err = IdentityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Slug::new(s)
    }
}

/// A membership path from the root section to a node, as a sequence of slugs.
///
/// `["blog", "my-post"]` is the page at `/blog/my-post/`; the empty path is
/// the root section. Membership only — this type knows nothing about URLs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct NodePath(Vec<Slug>);

impl NodePath {
    /// The root section's path.
    pub fn root() -> Self {
        Self::default()
    }

    /// Build a path from slug strings, validating each.
    pub fn from_segments<I, S>(segments: I) -> Result<Self, IdentityError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let slugs = segments
            .into_iter()
            .map(|s| Slug::new(s.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(slugs))
    }

    /// Parse a slash-separated path. `""` or `"/"` is the root.
    pub fn parse(raw: &str) -> Result<Self, IdentityError> {
        let trimmed = raw.trim_matches('/');
        if trimmed.is_empty() {
            return Ok(Self::root());
        }
        Self::from_segments(trimmed.split('/'))
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Path of the parent; `None` for the root.
    pub fn parent(&self) -> Option<Self> {
        self.0.split_last().map(|(_, rest)| Self(rest.to_vec()))
    }

    /// Last segment; `None` for the root.
    pub fn last(&self) -> Option<&Slug> {
        self.0.last()
    }

    /// This path with `slug` appended.
    pub fn join(&self, slug: &Slug) -> Self {
        let mut segments = self.0.clone();
        segments.push(slug.clone());
        Self(segments)
    }

    pub fn segments(&self) -> &[Slug] {
        &self.0
    }
}

impl fmt::Display for NodePath {
    /// Segments joined by `/`. The root displays as an empty string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined = self
            .0
            .iter()
            .map(Slug::as_str)
            .collect::<Vec<_>>()
            .join("/");
        f.write_str(&joined)
    }
}

/// The address of a node: derived from its membership path, never stored.
///
/// Derivation (RFC 2 §2.2): `root + "/" + path + "/"`; the root's address
/// is `/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UrlPath(String);

impl UrlPath {
    pub fn from_node_path(path: &NodePath) -> Self {
        if path.is_root() {
            Self("/".to_owned())
        } else {
            Self(format!("/{}/", path))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UrlPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors from slug and path validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("slug must not be empty")]
    Empty,
    #[error("invalid slug `{s}`: must not contain `/`")]
    Slash { s: String },
    #[error("invalid slug `{s}`: `.` and `..` are reserved")]
    DotSegment { s: String },
    #[error("invalid slug `{s}`: must not contain control characters")]
    ControlCharacter { s: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_accepts_any_segment_the_caller_slugified() {
        // Slugification is the generator's job; the domain accepts whatever
        // it produces, including verbatim frontmatter overrides.
        for raw in [
            "my-post-2",
            "a",
            "Ünïcödé",
            "日本語",
            "my_post",
            "My-Post",
            "2026-04-06",
            "-edge-",
            "with space",
        ] {
            assert!(Slug::new(raw).is_ok(), "{raw:?} should be accepted");
        }
    }

    #[test]
    fn slug_rejects_empty_slash_dots_and_controls() {
        assert_eq!(Slug::new(""), Err(IdentityError::Empty));
        assert!(matches!(Slug::new("/"), Err(IdentityError::Slash { .. })));
        assert!(matches!(Slug::new("a/b"), Err(IdentityError::Slash { .. })));
        assert!(matches!(
            Slug::new("."),
            Err(IdentityError::DotSegment { .. })
        ));
        assert!(matches!(
            Slug::new(".."),
            Err(IdentityError::DotSegment { .. })
        ));
        assert!(matches!(
            Slug::new("a\tb"),
            Err(IdentityError::ControlCharacter { .. })
        ));
        assert!(matches!(
            Slug::new("a\u{7f}"),
            Err(IdentityError::ControlCharacter { .. })
        ));
        // `...` is an ordinary (if odd) segment.
        assert!(Slug::new("...").is_ok());
    }

    #[test]
    fn node_path_parse_and_display() {
        let p = NodePath::parse("blog/my-post").unwrap();
        assert_eq!(p.segments().len(), 2);
        assert_eq!(p.to_string(), "blog/my-post");
        assert_eq!(p.parent().map(|p| p.to_string()), Some("blog".into()));
        assert_eq!(p.last().map(Slug::as_str), Some("my-post"));
    }

    #[test]
    fn node_path_root() {
        let root = NodePath::parse("/").unwrap();
        assert!(root.is_root());
        assert!(root.parent().is_none());
        assert_eq!(root.to_string(), "");
    }

    #[test]
    fn node_path_join() {
        let p = NodePath::parse("blog")
            .unwrap()
            .join(&Slug::new("x").unwrap());
        assert_eq!(p.to_string(), "blog/x");
    }

    #[test]
    fn url_path_derivation() {
        assert_eq!(
            UrlPath::from_node_path(&NodePath::parse("blog/my-post").unwrap()).to_string(),
            "/blog/my-post/"
        );
        assert_eq!(UrlPath::from_node_path(&NodePath::root()).to_string(), "/");
    }
}
