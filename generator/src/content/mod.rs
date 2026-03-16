//! Content loading and parsing.
//!
//! This module provides types for parsing and managing Markdown content files
//! with TOML frontmatter.
//!
//! # Overview
//!
//! - [`Frontmatter`]: Page metadata parsed from TOML
//! - [`Page`]: Individual content file with frontmatter and Markdown
//! - [`Section`]: Collection of pages (e.g., a blog)
//! - [`ContentSource`]: Trait for loading content from various sources
//!
//! # Example
//!
//! ```no_run
//! use generator::content::{Page, ContentSource, FilesystemContentSource};
//! use std::path::PathBuf;
//!
//! // Load a page from a file
//! let page = Page::from_file("content/about.md")?;
//! println!("Title: {}", page.frontmatter.title);
//!
//! // List all content files
//! let source = FilesystemContentSource::new("content");
//! let files = source.list()?;
//! for file in files {
//!     println!("Found: {}", file.display());
//! }
//! # Ok::<(), generator::error::GeneratorError>(())
//! ```

mod frontmatter;
mod page;
mod section;

pub use frontmatter::Frontmatter;
pub use page::Page;
pub use section::Section;

use crate::error::{ContentError, GeneratorError, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Trait for loading content from various sources.
///
/// This trait enables testing with mock content sources and supports
/// different storage backends.
pub trait ContentSource: Send + Sync {
    /// Load content for a given path.
    fn load(&self, path: &Path) -> Result<String>;

    /// Check if content exists at path.
    fn exists(&self, path: &Path) -> bool;

    /// List all content files.
    fn list(&self) -> Result<Vec<PathBuf>>;
}

/// Default filesystem-based content source.
pub struct FilesystemContentSource {
    root: PathBuf,
}

impl FilesystemContentSource {
    /// Create a new filesystem content source.
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }
}

impl ContentSource for FilesystemContentSource {
    fn load(&self, path: &Path) -> Result<String> {
        let full_path = self.root.join(path);
        std::fs::read_to_string(&full_path)
            .map_err(|e| ContentError::Io {
                path: full_path,
                source: e,
            })
            .map_err(GeneratorError::from)
    }

    fn exists(&self, path: &Path) -> bool {
        self.root.join(path).exists()
    }

    fn list(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let relative = path
                    .strip_prefix(&self.root)
                    .map_err(|_| {
                        GeneratorError::from(ContentError::InvalidPath(path.display().to_string()))
                    })?;
                files.push(relative.to_path_buf());
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_content_source_new() {
        let source = FilesystemContentSource::new("content");
        assert_eq!(source.root, PathBuf::from("content"));
    }

    #[test]
    fn test_filesystem_content_source_exists() {
        let source = FilesystemContentSource::new("tests/fixtures/content_site/content");
        assert!(source.exists(&PathBuf::from("_index.md")));
        assert!(!source.exists(&PathBuf::from("nonexistent.md")));
    }
}

/// Mock content source for testing.
#[cfg(test)]
pub struct MockContentSource {
    content: std::collections::HashMap<PathBuf, String>,
}

#[cfg(test)]
impl MockContentSource {
    pub fn new() -> Self {
        Self {
            content: std::collections::HashMap::new(),
        }
    }

    pub fn add(&mut self, path: &str, content: &str) {
        self.content
            .insert(PathBuf::from(path), content.to_string());
    }
}

#[cfg(test)]
impl Default for MockContentSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl ContentSource for MockContentSource {
    fn load(&self, path: &Path) -> Result<String> {
        self.content
            .get(path)
            .cloned()
            .ok_or_else(|| GeneratorError::from(ContentError::NotFound(path.to_path_buf())))
    }

    fn exists(&self, path: &Path) -> bool {
        self.content.contains_key(path)
    }

    fn list(&self) -> Result<Vec<PathBuf>> {
        Ok(self.content.keys().cloned().collect())
    }
}
