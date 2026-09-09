//! Identity: slugs, membership paths, and derived URL paths.
//!
//! One derivation point for addresses ([`UrlPath::from_node_path`]) — RFC 2,
//! invariant 3.

use std::fmt;

/// A single URL-safe path segment: lowercase ASCII letters, digits, and
/// hyphens; non-empty; no leading or trailing hyphen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slug(String);

impl Slug {
    /// Validate and create a slug.
    pub fn new(raw: &str) -> Result<Self, IdentityError> {
        if raw.is_empty() {
            return Err(IdentityError::Empty);
        }
        let valid = raw
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !valid {
            return Err(IdentityError::InvalidCharacters { s: raw.to_owned() });
        }
        if raw.starts_with('-') || raw.ends_with('-') {
            return Err(IdentityError::EdgeHyphen { s: raw.to_owned() });
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
    #[error("invalid slug `{s}`: only lowercase ASCII letters, digits, and hyphens are allowed")]
    InvalidCharacters { s: String },
    #[error("invalid slug `{s}`: must not start or end with a hyphen")]
    EdgeHyphen { s: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_accepts_valid() {
        assert!(Slug::new("my-post-2").is_ok());
        assert!(Slug::new("a").is_ok());
    }

    #[test]
    fn slug_rejects_invalid() {
        assert_eq!(Slug::new(""), Err(IdentityError::Empty));
        assert!(Slug::new("My-Post").is_err());
        assert!(Slug::new("my_post").is_err());
        assert!(Slug::new("-post").is_err());
        assert!(Slug::new("post-").is_err());
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
