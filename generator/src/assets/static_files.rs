//! Static file copier for assets that need no processing.
//!
//! This module provides the [`StaticCopier`] implementation for copying
//! static files (images, fonts, scripts, etc.) to the output directory.

use crate::assets::{AssetProcessor, AssetReport};
use crate::error::AssetError;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Static file copier for assets that need no processing.
///
/// This processor copies files from source to destination preserving
/// directory structure. It supports exclusion patterns to skip
/// files that should be handled by other processors.
///
/// # Example
///
/// ```no_run
/// use generator::assets::StaticCopier;
/// use generator::assets::AssetProcessor;
/// use std::path::Path;
///
/// let copier = StaticCopier::with_exclusions(vec!["*.scss".to_string()]);
/// let report = copier.process(
///     Path::new("static"),
///     Path::new("dist/static")
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct StaticCopier {
    /// File patterns to exclude from copying
    exclude_patterns: Vec<String>,
}

impl Default for StaticCopier {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticCopier {
    /// Create a new static file copier with default settings.
    pub fn new() -> Self {
        Self {
            exclude_patterns: Vec::new(),
        }
    }

    /// Create a copier with exclusion patterns.
    ///
    /// Files matching any of the patterns will be skipped.
    /// Patterns support glob syntax: `*.scss`, `**/*.sass`, etc.
    pub fn with_exclusions(patterns: Vec<String>) -> Self {
        Self {
            exclude_patterns: patterns,
        }
    }

    /// Check if a file should be excluded based on patterns.
    fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().map(|n| n.to_string_lossy());

        for pattern in &self.exclude_patterns {
            // Simple glob matching for common patterns
            if pattern.starts_with("*.") {
                // Extension pattern like "*.scss"
                if let Some(name) = &file_name
                    && name.ends_with(&pattern[1..])
                {
                    return true;
                }
            } else if pattern.starts_with("**/*.") {
                // Recursive extension pattern like "**/*.scss"
                let ext = &pattern[3..]; // "**/*" -> ".*.scss" -> ".scss"
                if path_str.ends_with(&ext[1..]) {
                    return true;
                }
            } else if path_str == *pattern || file_name.as_deref() == Some(pattern.as_str()) {
                // Exact match
                return true;
            }
        }
        false
    }

    /// Copy a single file from source to destination.
    fn copy_file(&self, src: &Path, dest: &Path) -> Result<(), AssetError> {
        // Create parent directories if needed
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| AssetError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Copy the file
        fs::copy(src, dest).map_err(|e| AssetError::CopyFailed {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    /// Process a directory recursively.
    fn process_directory(
        &self,
        src_dir: &Path,
        dest_dir: &Path,
        report: &mut AssetReport,
    ) -> Result<(), AssetError> {
        for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Check exclusion
            if self.is_excluded(path) {
                report.add_skipped();
                continue;
            }

            // Calculate relative path and destination
            let relative = path
                .strip_prefix(src_dir)
                .map_err(|_| AssetError::CopyFailed {
                    src: path.to_path_buf(),
                    dest: dest_dir.to_path_buf(),
                    reason: "Failed to calculate relative path".to_string(),
                })?;

            let dest_path = dest_dir.join(relative);

            // Copy the file
            match self.copy_file(path, &dest_path) {
                Ok(()) => report.add_processed(),
                Err(e) => report.add_error(e),
            }
        }

        Ok(())
    }
}

impl AssetProcessor for StaticCopier {
    fn process(&self, src: &Path, dest: &Path) -> Result<AssetReport, AssetError> {
        let mut report = AssetReport::new();

        // Check if source exists
        if !src.exists() {
            return Err(AssetError::NotFound(src.to_path_buf()));
        }

        if src.is_dir() {
            // Process directory recursively
            self.process_directory(src, dest, &mut report)?;
        } else {
            // Process single file
            if self.is_excluded(src) {
                report.add_skipped();
            } else {
                self.copy_file(src, dest)?;
                report.add_processed();
            }
        }

        Ok(report)
    }

    fn handles(&self, _path: &Path) -> bool {
        // StaticCopier handles all files (but respects exclusions)
        true
    }

    fn name(&self) -> &'static str {
        "static"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_static_copier_new() {
        let copier = StaticCopier::new();
        assert!(copier.exclude_patterns.is_empty());
    }

    #[test]
    fn test_static_copier_with_exclusions() {
        let copier =
            StaticCopier::with_exclusions(vec!["*.scss".to_string(), "*.sass".to_string()]);
        assert_eq!(copier.exclude_patterns.len(), 2);
    }

    #[test]
    fn test_static_copier_handles_all() {
        let copier = StaticCopier::new();
        assert!(copier.handles(Path::new("image.png")));
        assert!(copier.handles(Path::new("script.js")));
        assert!(copier.handles(Path::new("style.css")));
        assert!(copier.handles(Path::new("data.json")));
    }

    #[test]
    fn test_static_copier_is_excluded_extension() {
        let copier = StaticCopier::with_exclusions(vec!["*.scss".to_string()]);

        assert!(copier.is_excluded(Path::new("styles/main.scss")));
        assert!(!copier.is_excluded(Path::new("theme.sass"))); // .sass does not match .scss pattern
        assert!(!copier.is_excluded(Path::new("styles/main.css")));
    }

    #[test]
    fn test_static_copier_is_excluded_recursive() {
        let copier = StaticCopier::with_exclusions(vec!["**/*.scss".to_string()]);

        assert!(copier.is_excluded(Path::new("main.scss")));
        assert!(copier.is_excluded(Path::new("styles/main.scss")));
        assert!(copier.is_excluded(Path::new("styles/partials/_vars.scss")));
        assert!(!copier.is_excluded(Path::new("styles/main.css")));
    }

    #[test]
    fn test_static_copier_process_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("test.txt");
        let dest_path = temp_dir.path().join("output/test.txt");

        // Create test file
        fs::write(&src_path, "test content").unwrap();

        let copier = StaticCopier::new();
        let result = copier.process(&src_path, &dest_path);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.files_processed, 1);
        assert_eq!(report.files_skipped, 0);

        // Check file was copied
        assert!(dest_path.exists());
        let content = fs::read_to_string(&dest_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_static_copier_process_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("static");
        let dest_dir = temp_dir.path().join("output");

        // Create test files
        fs::create_dir_all(src_dir.join("images")).unwrap();
        fs::create_dir_all(src_dir.join("scripts")).unwrap();
        fs::write(src_dir.join("images/logo.png"), "png data").unwrap();
        fs::write(src_dir.join("scripts/app.js"), "js code").unwrap();
        fs::write(src_dir.join("favicon.ico"), "ico data").unwrap();

        let copier = StaticCopier::new();
        let result = copier.process(&src_dir, &dest_dir);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.files_processed, 3);
        assert_eq!(report.files_skipped, 0);

        // Check files were copied with structure preserved
        assert!(dest_dir.join("images/logo.png").exists());
        assert!(dest_dir.join("scripts/app.js").exists());
        assert!(dest_dir.join("favicon.ico").exists());
    }

    #[test]
    fn test_static_copier_process_with_exclusions() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("assets");
        let dest_dir = temp_dir.path().join("output");

        // Create test files
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("style.scss"), "$color: blue;").unwrap();
        fs::write(src_dir.join("script.js"), "console.log('hi');").unwrap();

        let copier = StaticCopier::with_exclusions(vec!["*.scss".to_string()]);
        let result = copier.process(&src_dir, &dest_dir);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.files_processed, 1); // Only script.js
        assert_eq!(report.files_skipped, 1); // style.scss

        // Check correct files
        assert!(!dest_dir.join("style.scss").exists());
        assert!(dest_dir.join("script.js").exists());
    }

    #[test]
    fn test_static_copier_process_not_found() {
        let copier = StaticCopier::new();
        let result = copier.process(Path::new("nonexistent"), Path::new("output"));

        assert!(result.is_err());
        match result.unwrap_err() {
            AssetError::NotFound(path) => assert!(path.ends_with("nonexistent")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_static_copier_copy_binary_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("image.png");
        let dest_path = temp_dir.path().join("output/image.png");

        // Create binary file
        let binary_data: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header
        fs::write(&src_path, &binary_data).unwrap();

        let copier = StaticCopier::new();
        let result = copier.process(&src_path, &dest_path);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.files_processed, 1);

        // Check binary content was preserved
        let copied = fs::read(&dest_path).unwrap();
        assert_eq!(copied, binary_data);
    }

    #[test]
    fn test_static_copier_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("static");
        let dest_dir = temp_dir.path().join("output");

        // Create nested structure
        fs::create_dir_all(src_dir.join("a/b/c")).unwrap();
        fs::write(src_dir.join("a/b/c/deep.txt"), "deep file").unwrap();
        fs::write(src_dir.join("a/top.txt"), "top file").unwrap();

        let copier = StaticCopier::new();
        let result = copier.process(&src_dir, &dest_dir);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.files_processed, 2);

        // Check nested structure preserved
        assert!(dest_dir.join("a/b/c/deep.txt").exists());
        assert!(dest_dir.join("a/top.txt").exists());
    }

    #[test]
    fn test_static_copier_name() {
        let copier = StaticCopier::new();
        assert_eq!(copier.name(), "static");
    }
}
