// taxus-domain/src/identity.rs

//! Identity: how a node is named and how its address follows from the name.
//!
//! A document has four names. Its **content file** is where it is stored.
//! Its **slug** is one URL segment. Its **node path** is the list of slugs
//! from the root section down to it, and is the source of truth. Its
//! **URL path** is derived from the node path in exactly one place,
//! [`UrlPath::from_node_path`], and nowhere else.
//!
//! The domain does not slugify. The generator owns the one slug algorithm
//! and applies frontmatter `slug` overrides and date-prefix stripping
//! before a path enters the tree; this module only validates that a
//! segment can stand in a path. See the book's
//! [Identity](https://crustyrustacean.github.io/taxus/theory/identity.html)
//! chapter.

use std::fmt;

/// A slug: one URL path segment, already slugified by the caller.
///
/// It exists so that a [`NodePath`] can only ever hold segments that are
/// safe to join with `/`: non-empty, no `/`, neither `.` nor `..`, no
/// ASCII control characters. The domain does not slugify. The generator
/// owns the one slug algorithm (lowercase, ASCII, dashes) and passes
/// frontmatter `slug` overrides through verbatim; this type accepts
/// whatever the generator produced.
///
/// # Example
///
/// ```
/// use taxus_domain::Slug;
///
/// let slug = Slug::new("project-launch")?;
/// assert_eq!(slug.as_str(), "project-launch");
/// assert!(Slug::new("a/b").is_err());
/// assert!(Slug::new("").is_err());
/// # Ok::<(), taxus_domain::identity::IdentityError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slug(String);

impl Slug {
    /// Validate `raw` as a path segment and wrap it; nothing is rewritten.
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

    /// The segment as text.
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

/// A node path: the slugs from the root section down to a node.
///
/// This is a node's name inside the Site Tree and the source of truth
/// for its identity: the builder checks it for duplicates, lookups take
/// it, and the address is derived from it. `["blog", "project-launch"]`
/// names the page under the `blog` section; the empty path names the
/// root section. It says where a node is, not what its URL is: that is
/// [`UrlPath`]'s job.
///
/// # Example
///
/// ```
/// use taxus_domain::NodePath;
///
/// let path = NodePath::parse("blog/project-launch")?;
/// assert_eq!(path.segments().len(), 2);
/// assert_eq!(path.parent().unwrap().to_string(), "blog");
/// assert_eq!(path.last().unwrap().as_str(), "project-launch");
/// assert!(NodePath::parse("/")?.is_root());
/// # Ok::<(), taxus_domain::identity::IdentityError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct NodePath(Vec<Slug>);

impl NodePath {
    /// The root section's path: no segments.
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

    /// Parse a slash-separated path such as `"blog/project-launch"`.
    ///
    /// Leading and trailing slashes are ignored; `""` and `"/"` are the
    /// root. Each segment is validated as a [`Slug`], not slugified.
    pub fn parse(raw: &str) -> Result<Self, IdentityError> {
        let trimmed = raw.trim_matches('/');
        if trimmed.is_empty() {
            return Ok(Self::root());
        }
        Self::from_segments(trimmed.split('/'))
    }

    /// Is this the root section's path?
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

    /// This path with `slug` appended: a child's path.
    pub fn join(&self, slug: &Slug) -> Self {
        let mut segments = self.0.clone();
        segments.push(slug.clone());
        Self(segments)
    }

    /// The slugs from the root down, in order.
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

/// A URL path: the address a document is served at.
///
/// It is derived from a [`NodePath`] by [`UrlPath::from_node_path`] and
/// never stored on a node, so every output that names a document (page
/// links, listings, feeds, the sitemap, the search index, aliases) gets
/// the same address from the same rule. The shape is fixed: `/` for the
/// root, otherwise `/` + the segments joined by `/` + `/`.
///
/// # Example
///
/// ```
/// use taxus_domain::{NodePath, UrlPath};
///
/// let post = NodePath::parse("blog/project-launch")?;
/// assert_eq!(UrlPath::from_node_path(&post).as_str(), "/blog/project-launch/");
/// assert_eq!(UrlPath::from_node_path(&NodePath::root()).as_str(), "/");
/// # Ok::<(), taxus_domain::identity::IdentityError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UrlPath(String);

impl UrlPath {
    /// Derive the address from a node path.
    ///
    /// This is the only place in the workspace that turns a path into an
    /// address. The root is `/`; every other node is `/a/b/`, with a
    /// trailing slash.
    pub fn from_node_path(path: &NodePath) -> Self {
        if path.is_root() {
            Self("/".to_owned())
        } else {
            Self(format!("/{}/", path))
        }
    }

    /// The address as text, e.g. `/blog/project-launch/`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UrlPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a string cannot be a [`Slug`].
///
/// Each variant names one rule a path segment must satisfy. The generator
/// reports these as an invalid route path, naming the content file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    /// The segment was the empty string.
    #[error("slug must not be empty")]
    Empty,
    /// The segment contained a `/`, which would make it two segments.
    #[error("invalid slug `{s}`: must not contain `/`")]
    Slash {
        /// The rejected text.
        s: String,
    },
    /// The segment was `.` or `..`, which paths treat specially.
    #[error("invalid slug `{s}`: `.` and `..` are reserved")]
    DotSegment {
        /// The rejected text.
        s: String,
    },
    /// The segment contained an ASCII control character.
    #[error("invalid slug `{s}`: must not contain control characters")]
    ControlCharacter {
        /// The rejected text.
        s: String,
    },
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
