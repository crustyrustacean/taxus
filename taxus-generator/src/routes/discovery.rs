//! Route discovery from content directory.
//!
//! This module provides the [`RouteDiscovery`] type for discovering routes
//! from a content directory structure.

use crate::content::{ContentSource, FilesystemContentSource, Page, split_date_prefix};
use crate::error::{GeneratorError, RouteError};
use crate::routes::slugify::{slugify_path, slugify_segment};
use crate::routes::{RouteInfo, RouteKind, RouteRegistry};
use std::path::{Path, PathBuf};
use taxus_domain::{NodePath, SiteTree, SiteTreeBuilder, Slug, TreeError, UrlPath};
use tracing::{debug, debug_span, info, instrument};
use walkdir::WalkDir;

/// Discovers routes from content directory structure.
pub struct RouteDiscovery {
    content_dir: PathBuf,
}

impl RouteDiscovery {
    /// Create a new route discovery for the given content directory.
    pub fn new<P: Into<PathBuf>>(content_dir: P) -> Self {
        Self {
            content_dir: content_dir.into(),
        }
    }

    /// Discover all routes from the content directory.
    ///
    /// This walks the content directory and creates routes for each `.md` file:
    /// - `_index.md` files become section routes (e.g., `/blog/`)
    /// - Other `.md` files become page routes (e.g., `/about/`)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The content directory cannot be read
    /// - A duplicate route is detected
    #[instrument(skip(self), fields(content_dir = %self.content_dir.display()))]
    pub fn discover(&self) -> Result<RouteRegistry, RouteError> {
        let span = debug_span!("route_discovery", content_dir = %self.content_dir.display());
        let _enter = span.enter();

        let mut registry = RouteRegistry::new();

        // Check if content directory exists
        if !self.content_dir.exists() {
            return Err(RouteError::DiscoveryFailed(format!(
                "Content directory does not exist: {}",
                self.content_dir.display()
            )));
        }

        debug!("Starting route discovery");

        // Walk the content directory
        for entry in WalkDir::new(&self.content_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process .md files
            if path.extension().is_some_and(|ext| ext == "md") {
                // Get relative path from content directory
                if let Ok(relative) = path.strip_prefix(&self.content_dir) {
                    // Convert file path to route
                    if let Some(route) = self.create_route_from_file(relative)? {
                        debug!(path = %route.path, kind = ?route.kind, "Discovered route");
                        registry.register(route)?;
                    }
                }
            }
        }

        info!(route_count = registry.len(), "Route discovery complete");

        Ok(registry)
    }

    /// Discover routes using a ContentSource trait object.
    ///
    /// This is useful for testing with mock content sources.
    #[instrument(skip(self, source), fields(content_dir = %self.content_dir.display()))]
    pub fn discover_from_source<S: ContentSource>(
        &self,
        source: &S,
    ) -> Result<RouteRegistry, RouteError> {
        let span =
            debug_span!("route_discovery_from_source", content_dir = %self.content_dir.display());
        let _enter = span.enter();

        let mut registry = RouteRegistry::new();

        // List all content files from the source
        let files = source
            .list()
            .map_err(|e| RouteError::DiscoveryFailed(e.to_string()))?;

        debug!(file_count = files.len(), "Processing content files");

        for relative in files {
            // Convert file path to route
            if let Some(route) = self.create_route_from_file(&relative)? {
                debug!(path = %route.path, kind = ?route.kind, "Discovered route");
                registry.register(route)?;
            }
        }

        info!(route_count = registry.len(), "Route discovery complete");

        Ok(registry)
    }

    /// Build the Site Tree from the content directory.
    ///
    /// Every `.md` file is parsed (frontmatter and body) and placed in the
    /// tree at its final membership path:
    ///
    /// - directory segments are slugified exactly as [`discover`](Self::discover)
    ///   slugifies them;
    /// - `_index.md` declares the section for its directory (the root for
    ///   the content directory itself). Directories without one are still
    ///   sections — the builder creates them with default frontmatter and
    ///   no `content_file`;
    /// - a page's last segment is its frontmatter `slug`, verbatim, if set;
    ///   otherwise the file stem with any `YYYY-MM-DD-` prefix stripped
    ///   ([`split_date_prefix`]) and slugified.
    ///
    /// The last point is the one place the tree and the legacy
    /// [`discover`](Self::discover) registry differ: the registry keys a
    /// page by its filename even when `slug` overrides it. The tree uses
    /// the documented model (URL = section path + slug). A `slug` on an
    /// `_index.md` is not honoured: a section's identity is its directory.
    ///
    /// # Errors
    ///
    /// - the content directory does not exist ([`RouteError::DiscoveryFailed`])
    /// - a file cannot be read or its frontmatter does not parse
    ///   ([`crate::error::ContentError`])
    /// - two files resolve to the same path, or a page and a section do
    ///   ([`RouteError::Duplicate`], same message as the legacy registry)
    /// - a frontmatter `slug` is not a valid path segment
    ///   ([`RouteError::InvalidPath`])
    #[instrument(skip(self), fields(content_dir = %self.content_dir.display()))]
    pub fn discover_tree(&self) -> crate::error::Result<SiteTree> {
        if !self.content_dir.exists() {
            return Err(RouteError::DiscoveryFailed(format!(
                "Content directory does not exist: {}",
                self.content_dir.display()
            ))
            .into());
        }
        let source = FilesystemContentSource::new(&self.content_dir);
        self.discover_tree_from_source(&source)
    }

    /// Build the Site Tree from a [`ContentSource`].
    ///
    /// See [`discover_tree`](Self::discover_tree) for the path rules.
    #[instrument(skip(self, source), fields(content_dir = %self.content_dir.display()))]
    pub fn discover_tree_from_source<S: ContentSource>(
        &self,
        source: &S,
    ) -> crate::error::Result<SiteTree> {
        let span = debug_span!("tree_discovery", content_dir = %self.content_dir.display());
        let _enter = span.enter();

        let files = source.list()?;
        debug!(file_count = files.len(), "Building site tree");

        let mut builder = SiteTreeBuilder::new();
        for relative in files {
            let file_name = relative
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    RouteError::InvalidPath(format!("Invalid file name: {}", relative.display()))
                })?;
            let content = source.load(&relative)?;
            let page = Page::from_str(&content, &relative.to_string_lossy())?;
            let parent = relative.parent().unwrap_or(Path::new(""));
            let parent_path = Self::parent_to_node_path(parent)?;

            if file_name == "_index.md" {
                if parent_path.is_root() {
                    debug!(path = "/", kind = "section", "Discovered node");
                    builder = builder.root(
                        Some(relative.clone()),
                        page.frontmatter,
                        Some(page.raw_content),
                    );
                } else {
                    debug!(path = %parent_path, kind = "section", "Discovered node");
                    builder
                        .add_section(
                            &parent_path,
                            Some(relative.clone()),
                            page.frontmatter,
                            Some(page.raw_content),
                        )
                        .map_err(|e| Self::map_tree_error(e, &parent_path))?;
                }
            } else {
                let stem = file_name.strip_suffix(".md").ok_or_else(|| {
                    RouteError::InvalidPath(format!("Not a markdown file: {}", file_name))
                })?;
                let slug = match &page.frontmatter.slug {
                    Some(custom) => Slug::new(custom).map_err(|e| {
                        RouteError::InvalidPath(format!("{}: {e}", relative.display()))
                    })?,
                    None => Slug::new(&slugify_segment(split_date_prefix(stem).0))
                        .expect("slugify_segment output is a valid segment"),
                };
                let path = parent_path.join(&slug);
                debug!(path = %path, kind = "page", "Discovered node");
                builder
                    .add_page(&path, relative.clone(), page.frontmatter, page.raw_content)
                    .map_err(|e| Self::map_tree_error(e, &path))?;
            }
        }

        let tree = builder
            .build()
            .map_err(|e| Self::map_tree_error(e, &NodePath::root()))?;
        info!(page_count = tree.iter_pages().count(), "Site tree built");
        Ok(tree)
    }

    /// Slugify each directory segment into a membership path, exactly as
    /// [`parent_to_url_path`](Self::parent_to_url_path) does for routes.
    fn parent_to_node_path(parent: &Path) -> crate::error::Result<NodePath> {
        if parent.as_os_str().is_empty() {
            return Ok(NodePath::root());
        }
        NodePath::parse(&slugify_path(&parent_segments(parent)))
            .map_err(|e| RouteError::InvalidPath(e.to_string()).into())
    }

    /// Translate a tree construction error into the [`RouteError`] the
    /// legacy registry would have produced for the same input, so error
    /// messages do not change.
    fn map_tree_error(err: TreeError, path: &NodePath) -> GeneratorError {
        let url = UrlPath::from_node_path(path).to_string();
        match err {
            // A page and a section at the same path were one `Duplicate`
            // route in the registry too.
            TreeError::Duplicate { .. } | TreeError::Collision { .. } => {
                RouteError::Duplicate(url).into()
            }
            TreeError::RootReserved => RouteError::InvalidPath(url).into(),
            TreeError::Identity(e) => RouteError::InvalidPath(e.to_string()).into(),
        }
    }

    /// Create a RouteInfo from a content file path.
    fn create_route_from_file(&self, relative: &Path) -> Result<Option<RouteInfo>, RouteError> {
        let file_name = relative
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                RouteError::InvalidPath(format!("Invalid file name: {}", relative.display()))
            })?;

        // Determine route kind based on file name
        let (kind, url_path, output_file) = if file_name == "_index.md" {
            // Section index
            let parent = relative.parent().unwrap_or(Path::new(""));
            let url_path = self.parent_to_url_path(parent);
            let output_file = self.parent_to_output_file(parent);
            (RouteKind::Section, url_path, output_file)
        } else {
            // Regular page
            let stem = file_name
                .strip_suffix(".md")
                .ok_or_else(|| {
                    RouteError::InvalidPath(format!("Not a markdown file: {}", file_name))
                })?
                .to_string();

            // Strip a `YYYY-MM-DD-` date prefix so dates never leak into
            // URLs or output paths (#67).
            let (stem, _) = crate::content::split_date_prefix(&stem);

            let parent = relative.parent().unwrap_or(Path::new(""));
            let url_path = self.page_to_url_path(parent, stem);
            let output_file = self.page_to_output_file(parent, stem);
            (RouteKind::Page, url_path, output_file)
        };

        RouteInfo::new(url_path, relative.to_path_buf(), output_file, kind).map(Some)
    }

    /// Convert a parent directory path to a URL path for sections.
    ///
    /// Examples:
    /// - "" -> "/"
    /// - "blog" -> "/blog/"
    /// - "blog/tech" -> "/blog/tech/"
    ///
    /// Each directory segment is slugified.
    fn parent_to_url_path(&self, parent: &Path) -> String {
        if parent.as_os_str().is_empty() {
            "/".to_string()
        } else {
            format!("/{}/", slugify_path(&parent_segments(parent)))
        }
    }

    /// Convert a parent directory path to an output file path for sections.
    ///
    /// Examples:
    /// - "" -> "index.html"
    /// - "blog" -> "blog/index.html"
    /// - "blog/tech" -> "blog/tech/index.html"
    fn parent_to_output_file(&self, parent: &Path) -> PathBuf {
        if parent.as_os_str().is_empty() {
            PathBuf::from("index.html")
        } else {
            PathBuf::from(slugify_path(&parent_segments(parent))).join("index.html")
        }
    }

    /// Convert a page path to a URL path.
    ///
    /// Examples:
    /// - ("", "about") -> "/about/"
    /// - ("blog", "First Post") -> "/blog/first-post/"
    ///
    /// Both the parent directory segments and the page stem are
    /// slugified.
    fn page_to_url_path(&self, parent: &Path, stem: &str) -> String {
        if parent.as_os_str().is_empty() {
            format!("/{}/", slugify_path(stem))
        } else {
            format!(
                "/{}/",
                slugify_path(&format!("{}/{}", parent_segments(parent), stem))
            )
        }
    }

    /// Convert a page path to an output file path.
    ///
    /// Examples:
    /// - ("", "about") -> "about/index.html"
    /// - ("blog", "first-post") -> "blog/first-post/index.html"
    fn page_to_output_file(&self, parent: &Path, stem: &str) -> PathBuf {
        if parent.as_os_str().is_empty() {
            PathBuf::from(slugify_path(stem)).join("index.html")
        } else {
            PathBuf::from(slugify_path(&format!(
                "{}/{}",
                parent_segments(parent),
                stem
            )))
            .join("index.html")
        }
    }
}

/// A content-relative directory as `/`-joined segments, whatever the
/// platform's separator.
///
/// `Path::display()` prints the native separator, and on Windows
/// `blog\\2026` would reach [`slugify_path`] as one segment and become
/// `blog-2026`. Joining the components keeps the segment boundaries.
fn parent_segments(parent: &Path) -> String {
    parent
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Internal helper function for testing path conversion.
/// Converts a content file path to (url_path, output_file, route_kind).
#[cfg(test)]
fn convert_path(path: &str) -> (String, PathBuf, RouteKind) {
    let discovery = RouteDiscovery::new("content");
    let relative = PathBuf::from(path);

    let route = discovery
        .create_route_from_file(&relative)
        .unwrap()
        .unwrap();
    (route.path, route.output_file, route.kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Path::join` uses the native separator, so on Windows this is the
    /// `blog\\2026` case that used to slugify into `blog-2026`.
    #[test]
    fn test_parent_segments_join_with_forward_slash() {
        let nested = Path::new("blog").join("2026").join("archive");
        assert_eq!(parent_segments(&nested), "blog/2026/archive");
        assert_eq!(parent_segments(Path::new("blog")), "blog");
        assert_eq!(parent_segments(Path::new("")), "");
        assert_eq!(
            RouteDiscovery::parent_to_node_path(&Path::new("blog").join("2026"))
                .unwrap()
                .to_string(),
            "blog/2026"
        );
    }
    use crate::content::MockContentSource;

    // Path conversion tests

    #[test]
    fn test_path_conversion_index() {
        // _index.md -> /, index.html, Section
        let (path, output, kind) = convert_path("_index.md");
        assert_eq!(path, "/");
        assert_eq!(output, PathBuf::from("index.html"));
        assert_eq!(kind, RouteKind::Section);
    }

    #[test]
    fn test_path_conversion_page() {
        // about.md -> /about/, about/index.html, Page
        let (path, output, kind) = convert_path("about.md");
        assert_eq!(path, "/about/");
        assert_eq!(output, PathBuf::from("about/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_nested_section() {
        // blog/_index.md -> /blog/, blog/index.html, Section
        let (path, output, kind) = convert_path("blog/_index.md");
        assert_eq!(path, "/blog/");
        assert_eq!(output, PathBuf::from("blog/index.html"));
        assert_eq!(kind, RouteKind::Section);
    }

    #[test]
    fn test_path_conversion_nested_page() {
        // blog/first-post.md -> /blog/first-post/, blog/first-post/index.html, Page
        let (path, output, kind) = convert_path("blog/first-post.md");
        assert_eq!(path, "/blog/first-post/");
        assert_eq!(output, PathBuf::from("blog/first-post/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_dated_page() {
        // blog/2026-04-06-my-post.md -> /blog/my-post/ (#67)
        let (path, output, kind) = convert_path("blog/2026-04-06-my-post.md");
        assert_eq!(path, "/blog/my-post/");
        assert_eq!(output, PathBuf::from("blog/my-post/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_dated_page_at_root() {
        // 2026-04-06-my-post.md -> /my-post/, my-post/index.html (#67)
        let (path, output, kind) = convert_path("2026-04-06-my-post.md");
        assert_eq!(path, "/my-post/");
        assert_eq!(output, PathBuf::from("my-post/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_pure_date_stem_untouched() {
        // A stem that is only a date is left alone (nothing to strip into).
        let (path, output, _kind) = convert_path("2026-04-06.md");
        assert_eq!(path, "/2026-04-06/");
        assert_eq!(output, PathBuf::from("2026-04-06/index.html"));
    }

    #[test]
    fn test_path_conversion_deeply_nested() {
        // blog/tech/first-post.md -> /blog/tech/first-post/, blog/tech/first-post/index.html, Page
        let (path, output, kind) = convert_path("blog/tech/first-post.md");
        assert_eq!(path, "/blog/tech/first-post/");
        assert_eq!(output, PathBuf::from("blog/tech/first-post/index.html"));
        assert_eq!(kind, RouteKind::Page);
    }

    #[test]
    fn test_path_conversion_deeply_nested_section() {
        // blog/tech/_index.md -> /blog/tech/, blog/tech/index.html, Section
        let (path, output, kind) = convert_path("blog/tech/_index.md");
        assert_eq!(path, "/blog/tech/");
        assert_eq!(output, PathBuf::from("blog/tech/index.html"));
        assert_eq!(kind, RouteKind::Section);
    }

    // RouteDiscovery tests

    #[test]
    fn test_discovery_new() {
        let discovery = RouteDiscovery::new("content");
        assert_eq!(discovery.content_dir, PathBuf::from("content"));
    }

    #[test]
    fn test_discover_from_source() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.contains("/"));
        assert!(registry.contains("/about/"));
    }

    #[test]
    fn test_discover_from_source_with_nested() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");
        source.add("blog/_index.md", "+++\ntitle = \"Blog\"\n+++\n");
        source.add("blog/first-post.md", "+++\ntitle = \"First Post\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert_eq!(registry.len(), 4);
        assert!(registry.contains("/"));
        assert!(registry.contains("/about/"));
        assert!(registry.contains("/blog/"));
        assert!(registry.contains("/blog/first-post/"));
    }

    #[test]
    fn test_discover_from_source_empty() {
        let source = MockContentSource::new();

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert!(registry.is_empty());
    }

    #[test]
    fn test_discover_from_source_pages_and_sections() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");
        source.add("blog/_index.md", "+++\ntitle = \"Blog\"\n+++\n");
        source.add("blog/first-post.md", "+++\ntitle = \"First Post\"\n+++\n");
        source.add("blog/second-post.md", "+++\ntitle = \"Second Post\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let registry = discovery.discover_from_source(&source).unwrap();

        assert_eq!(registry.pages().count(), 3);
        assert_eq!(registry.sections().count(), 2);
    }

    // Tree discovery tests

    fn tree_of(files: &[(&str, &str)]) -> SiteTree {
        let mut source = MockContentSource::new();
        for (path, content) in files {
            source.add(path, content);
        }
        RouteDiscovery::new("content")
            .discover_tree_from_source(&source)
            .unwrap()
    }

    fn tree_error(files: &[(&str, &str)]) -> String {
        let mut source = MockContentSource::new();
        for (path, content) in files {
            source.add(path, content);
        }
        RouteDiscovery::new("content")
            .discover_tree_from_source(&source)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn test_tree_sections_and_pages() {
        let tree = tree_of(&[
            ("_index.md", "+++\ntitle = \"Home\"\n+++\nWelcome"),
            ("about.md", "+++\ntitle = \"About\"\n+++\n"),
            ("blog/_index.md", "+++\ntitle = \"Blog\"\n+++\n"),
            (
                "blog/First Post.md",
                "+++\ntitle = \"First Post\"\n+++\nBody",
            ),
        ]);

        assert_eq!(tree.root.meta.title, "Home");
        assert_eq!(tree.root.body.as_deref(), Some("Welcome"));
        assert_eq!(
            tree.root.content_file.as_deref(),
            Some(Path::new("_index.md"))
        );
        assert_eq!(tree.root.pages.len(), 1);
        assert_eq!(tree.root.pages[0].path.to_string(), "about");

        let blog = tree.get_section(&NodePath::parse("blog").unwrap()).unwrap();
        assert_eq!(blog.meta.title, "Blog");
        assert_eq!(
            blog.content_file.as_deref(),
            Some(Path::new("blog/_index.md"))
        );
        assert_eq!(blog.pages.len(), 1);
        // Stem slugified exactly like the legacy route.
        assert_eq!(blog.pages[0].path.to_string(), "blog/first-post");
        assert_eq!(
            blog.pages[0].content_file,
            PathBuf::from("blog/First Post.md")
        );
        assert_eq!(blog.pages[0].body, "Body");
    }

    #[test]
    fn test_tree_directory_without_index_is_a_section() {
        let tree = tree_of(&[("docs/Guide.md", "+++\ntitle = \"Guide\"\n+++\n")]);
        let docs = tree.get_section(&NodePath::parse("docs").unwrap()).unwrap();
        assert!(docs.content_file.is_none());
        assert!(docs.body.is_none());
        assert_eq!(docs.pages[0].path.to_string(), "docs/guide");
    }

    #[test]
    fn test_tree_strips_date_prefix_and_defaults_date() {
        let tree = tree_of(&[("blog/2026-04-06-my-post.md", "+++\ntitle = \"Post\"\n+++\n")]);
        let post = tree
            .get_page(&NodePath::parse("blog/my-post").unwrap())
            .expect("dated prefix is stripped from the path");
        assert_eq!(
            post.content_file,
            PathBuf::from("blog/2026-04-06-my-post.md")
        );
        assert_eq!(
            post.meta.date,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 6),
            "filename date is the default publication date (#67)"
        );
    }

    #[test]
    fn test_tree_applies_frontmatter_slug_within_section() {
        let tree = tree_of(&[(
            "blog/e.md",
            "+++\ntitle = \"E\"\nslug = \"Renamed Entry\"\n+++\n",
        )]);
        // Verbatim: the domain accepts what the author wrote.
        let page = tree
            .get_page(&NodePath::parse("blog/Renamed Entry").unwrap())
            .expect("slug override is the final segment");
        assert_eq!(page.content_file, PathBuf::from("blog/e.md"));
        assert!(tree.get_page(&NodePath::parse("blog/e").unwrap()).is_none());
    }

    #[test]
    fn test_tree_duplicate_error_matches_registry() {
        // Two files slugify to the same path.
        let err = tree_error(&[
            ("blog/my-post.md", "+++\ntitle = \"A\"\n+++\n"),
            ("blog/My Post.md", "+++\ntitle = \"B\"\n+++\n"),
        ]);
        assert_eq!(err, "Route error: Duplicate route: /blog/my-post/");
    }

    #[test]
    fn test_tree_page_section_collision_is_a_duplicate() {
        let err = tree_error(&[
            ("blog.md", "+++\ntitle = \"A\"\n+++\n"),
            ("blog/_index.md", "+++\ntitle = \"B\"\n+++\n"),
        ]);
        assert_eq!(err, "Route error: Duplicate route: /blog/");
    }

    #[test]
    fn test_tree_rejects_slug_with_slash() {
        let err = tree_error(&[("a.md", "+++\ntitle = \"A\"\nslug = \"x/y\"\n+++\n")]);
        assert!(err.contains("Invalid route path"), "{err}");
        assert!(err.contains("a.md"), "{err}");
    }

    #[test]
    fn test_tree_reports_bad_frontmatter_with_relative_path() {
        let err = tree_error(&[("blog/bad.md", "+++\ntitle = \n+++\n")]);
        assert!(err.contains("Invalid frontmatter in blog/bad.md"), "{err}");
    }

    #[test]
    fn test_discover_tree_missing_content_dir() {
        let err = RouteDiscovery::new("definitely/not/here")
            .discover_tree()
            .unwrap_err()
            .to_string();
        assert!(err.contains("Content directory does not exist"), "{err}");
    }

    #[test]
    fn test_registry_from_tree_matches_legacy_for_source() {
        let mut source = MockContentSource::new();
        source.add("_index.md", "+++\ntitle = \"Home\"\n+++\n");
        source.add("about.md", "+++\ntitle = \"About\"\n+++\n");
        source.add("blog/_index.md", "+++\ntitle = \"Blog\"\n+++\n");
        source.add(
            "blog/2026-04-06-first-post.md",
            "+++\ntitle = \"First\"\n+++\n",
        );
        source.add("docs/Guide.md", "+++\ntitle = \"Guide\"\n+++\n");

        let discovery = RouteDiscovery::new("content");
        let legacy = discovery.discover_from_source(&source).unwrap();
        let tree = discovery.discover_tree_from_source(&source).unwrap();
        let derived = RouteRegistry::from_tree(&tree);

        let sorted = |r: &RouteRegistry| {
            let mut v: Vec<RouteInfo> = r.iter().cloned().collect();
            v.sort_by(|a, b| a.path.cmp(&b.path));
            v
        };
        assert_eq!(sorted(&derived), sorted(&legacy));
        // `docs/` has no _index.md: a section in the tree, no route in either.
        assert!(!derived.contains("/docs/"));
        assert!(derived.contains("/docs/guide/"));
    }

    // Helper method tests

    #[test]
    fn test_parent_to_url_path_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_url_path(Path::new(""));
        assert_eq!(path, "/");
    }

    #[test]
    fn test_parent_to_url_path_single() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_url_path(Path::new("blog"));
        assert_eq!(path, "/blog/");
    }

    #[test]
    fn test_parent_to_url_path_nested() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_url_path(Path::new("blog/tech"));
        assert_eq!(path, "/blog/tech/");
    }

    #[test]
    fn test_parent_to_output_file_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_output_file(Path::new(""));
        assert_eq!(path, PathBuf::from("index.html"));
    }

    #[test]
    fn test_parent_to_output_file_single() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.parent_to_output_file(Path::new("blog"));
        assert_eq!(path, PathBuf::from("blog/index.html"));
    }

    #[test]
    fn test_page_to_url_path_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_url_path(Path::new(""), "about");
        assert_eq!(path, "/about/");
    }

    #[test]
    fn test_page_to_url_path_nested() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_url_path(Path::new("blog"), "first-post");
        assert_eq!(path, "/blog/first-post/");
    }

    #[test]
    fn test_page_to_output_file_root() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_output_file(Path::new(""), "about");
        assert_eq!(path, PathBuf::from("about/index.html"));
    }

    #[test]
    fn test_page_to_output_file_nested() {
        let discovery = RouteDiscovery::new("content");
        let path = discovery.page_to_output_file(Path::new("blog"), "first-post");
        assert_eq!(path, PathBuf::from("blog/first-post/index.html"));
    }
}
