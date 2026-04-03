//! Section type for content collections.

use crate::error::{ContentError, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::pagination::{PaginatedSlice, PaginationInfo, Paginator};
use super::{Frontmatter, Page, SortBy};

/// A section containing multiple pages (e.g., a blog).
///
/// Sections are defined by an `_index.md` file in a subdirectory.
#[derive(Debug, Clone)]
pub struct Section {
    /// Section metadata from _index.md frontmatter
    pub frontmatter: Frontmatter,

    /// URL path (e.g., "/blog/")
    pub path: String,

    /// Source directory path relative to content directory
    pub source: PathBuf,

    /// Pages in this section
    pub pages: Vec<Page>,
}

impl Section {
    /// Create a new section from a directory path.
    ///
    /// Looks for `_index.md` in the directory for frontmatter.
    /// If not found, uses default frontmatter.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::content::Section;
    ///
    /// let section = Section::from_dir("content/blog")?;
    /// println!("Section path: {}", section.path);
    /// # Ok::<(), taxus_lib::error::GeneratorError>(())
    /// ```
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        let index_path = dir.join("_index.md");

        let frontmatter = if index_path.exists() {
            let content = std::fs::read_to_string(&index_path).map_err(|e| ContentError::Io {
                path: index_path.clone(),
                source: e,
            })?;

            // Parse frontmatter from _index.md
            Self::parse_frontmatter(&content, &index_path)?
        } else {
            Frontmatter::default()
        };

        let path = Self::dir_to_path(dir);

        Ok(Self {
            frontmatter,
            path,
            source: dir.to_path_buf(),
            pages: Vec::new(),
        })
    }

    /// Parse frontmatter from _index.md content.
    fn parse_frontmatter(content: &str, path: &Path) -> Result<Frontmatter> {
        // Normalize line endings to \n
        let content = content.replace("\r\n", "\n");

        if !content.starts_with("+++\n") {
            return Ok(Frontmatter::default());
        }

        let end = content[4..]
            .find("\n+++\n")
            .map(|i| i + 4)
            .ok_or_else(|| ContentError::UnclosedFrontmatter(path.to_path_buf()))?;

        let fm_str = &content[4..end];
        Frontmatter::from_str(fm_str)
            .map_err(|e| ContentError::InvalidFrontmatter {
                path: path.to_path_buf(),
                source: e,
            })
            .map_err(crate::error::GeneratorError::from)
    }

    /// Convert directory path to URL path.
    fn dir_to_path(dir: &Path) -> String {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

        format!("/{}/", name)
    }

    /// Add a page to this section.
    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    /// Sort pages by date (newest first).
    ///
    /// Pages without dates are placed at the end.
    pub fn sort_by_date(&mut self) {
        self.pages.sort_by(|a, b| {
            let a_date = a.frontmatter.date;
            let b_date = b.frontmatter.date;
            // Reverse order: newest first
            b_date.cmp(&a_date)
        });
    }

    /// Sort pages according to the section's `sort_by` frontmatter setting.
    ///
    /// Defaults to sorting by date (newest first) if not specified.
    pub fn sort_pages(&mut self) {
        match self.frontmatter.sort_by {
            SortBy::Date => self.sort_by_date(),
            SortBy::Weight => self.sort_by_weight(),
            SortBy::Title => self.sort_by_title(),
            SortBy::None => {} // Preserve original order
        }
    }

    /// Sort pages by weight (lowest first).
    ///
    /// Pages without weight are placed at the end.
    pub fn sort_by_weight(&mut self) {
        self.pages.sort_by(|a, b| {
            let a_weight = a.frontmatter.weight;
            let b_weight = b.frontmatter.weight;
            a_weight.cmp(&b_weight)
        });
    }

    /// Sort pages by title (alphabetically).
    pub fn sort_by_title(&mut self) {
        self.pages
            .sort_by(|a, b| a.frontmatter.title.cmp(&b.frontmatter.title));
    }

    /// Check if pagination is enabled for this section.
    pub fn is_paginated(&self) -> bool {
        self.frontmatter.paginate_by > 0
    }

    /// Paginate this section's pages.
    ///
    /// Returns `None` if pagination is not configured for this section.
    /// Returns `Some(Paginator)` if `paginate_by` is set in frontmatter.
    pub fn paginate(&self) -> Option<Paginator> {
        if !self.is_paginated() {
            return None;
        }
        Some(Paginator::new(
            self.pages.clone(),
            self.frontmatter.paginate_by,
            &self.path,
        ))
    }

    /// Get all paginated slices for this section.
    ///
    /// Returns a single slice with all pages if pagination is not configured.
    /// Returns multiple slices if pagination is configured.
    pub fn paginated_slices(&self) -> Vec<PaginatedSlice> {
        match self.paginate() {
            Some(paginator) => paginator.iter().collect(),
            None => {
                // No pagination - return single slice with all pages
                vec![PaginatedSlice {
                    pages: self.pages.clone(),
                    pagination: PaginationInfo::new(
                        1,
                        1,
                        self.pages.len(),
                        self.pages.len(),
                        &self.path,
                    ),
                }]
            }
        }
    }

    /// Get the template name for this section.
    pub fn template(&self) -> &str {
        self.frontmatter.template()
    }

    /// Get the template for paginated pages.
    pub fn paginate_template(&self) -> &str {
        self.frontmatter
            .paginate_template
            .as_deref()
            .unwrap_or_else(|| self.template())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_from_dir_without_index() {
        // Create a temp directory without _index.md
        let temp_dir = tempfile::tempdir().unwrap();
        let section = Section::from_dir(temp_dir.path()).unwrap();

        assert!(section.frontmatter.title.is_empty());
        assert!(section.pages.is_empty());
    }

    #[test]
    fn test_dir_to_path() {
        assert_eq!(Section::dir_to_path(Path::new("content/blog")), "/blog/");
        assert_eq!(Section::dir_to_path(Path::new("content/news")), "/news/");
    }

    #[test]
    fn test_add_page() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut section = Section::from_dir(temp_dir.path()).unwrap();

        let page = Page::from_str("+++\ntitle = \"Test\"\n+++\nContent", "test.md").unwrap();
        section.add_page(page);

        assert_eq!(section.pages.len(), 1);
        assert_eq!(section.pages[0].frontmatter.title, "Test");
    }

    #[test]
    fn test_sort_by_date() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut section = Section::from_dir(temp_dir.path()).unwrap();

        // Add pages with different dates
        let page1 = Page::from_str(
            "+++\ntitle = \"Old\"\ndate = 2024-01-01\n+++\nContent",
            "old.md",
        )
        .unwrap();
        let page2 = Page::from_str(
            "+++\ntitle = \"New\"\ndate = 2024-02-01\n+++\nContent",
            "new.md",
        )
        .unwrap();
        let page3 = Page::from_str("+++\ntitle = \"No Date\"\n+++\nContent", "nodate.md").unwrap();

        section.add_page(page1);
        section.add_page(page2);
        section.add_page(page3);

        section.sort_by_date();

        // Newest should be first
        assert_eq!(section.pages[0].frontmatter.title, "New");
        assert_eq!(section.pages[1].frontmatter.title, "Old");
        assert_eq!(section.pages[2].frontmatter.title, "No Date");
    }

    #[test]
    fn test_section_template() {
        let temp_dir = tempfile::tempdir().unwrap();
        let section = Section::from_dir(temp_dir.path()).unwrap();
        assert_eq!(section.template(), "page.html");
    }

    #[test]
    fn test_section_with_custom_template() {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("_index.md");
        std::fs::write(
            &index_path,
            "+++\ntitle = \"Blog\"\ntemplate = \"section.html\"\n+++\nBlog content",
        )
        .unwrap();

        let section = Section::from_dir(temp_dir.path()).unwrap();
        assert_eq!(section.template(), "section.html");
        assert_eq!(section.frontmatter.title, "Blog");
    }
}
