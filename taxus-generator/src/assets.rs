// taxus-generator/src/assets.rs

//! Asset processing module for the generator library.
//!
//! This module provides a trait-based architecture for processing different
//! asset types, with concrete implementations for SCSS compilation and
//! static file copying.
//!
//! # Overview
//!
//! The [`AssetProcessor`] trait defines the interface for processing assets.
//! Implementations handle specific asset types like SCSS files or static files.
//!
//! # Example
//!
//! ```no_run
//! use taxus_lib::assets::{AssetProcessor, ScssProcessor, StaticCopier};
//! use std::path::Path;
//!
//! // Process SCSS files
//! let scss_processor = ScssProcessor::new();
//! let report = scss_processor.process(
//!     Path::new("styles/main.scss"),
//!     Path::new("dist/styles/main.css"),
//!     false
//! ).unwrap();
//!
//! // Copy static files
//! let static_copier = StaticCopier::new();
//! let report = static_copier.process(
//!     Path::new("static"),
//!     Path::new("dist/static"),
//!     false
//! ).unwrap();
//! ```

mod static_files;
mod styles;

pub use static_files::StaticCopier;
pub use styles::ScssProcessor;

use crate::error::AssetError;
use std::path::Path;

/// Trait for processing assets from source to destination.
///
/// Implementations handle specific asset types (SCSS, static files, etc.)
/// and report on the processing results.
pub trait AssetProcessor: Send + Sync {
    /// Process assets from source to destination.
    ///
    /// # Arguments
    ///
    /// * `src` - Source path (file or directory)
    /// * `dest` - Destination path (file or directory)
    /// * `dry_run` - If true, simulate processing without writing any files
    ///
    /// # Returns
    ///
    /// A report containing the number of files processed, skipped, and any errors.
    fn process(&self, src: &Path, dest: &Path, dry_run: bool) -> Result<AssetReport, AssetError>;

    /// Check if this processor handles the given file.
    ///
    /// This is used to determine which processor should handle a particular file.
    fn handles(&self, path: &Path) -> bool;

    /// Get the processor name for logging and error messages.
    fn name(&self) -> &'static str;
}

/// Report of processed assets.
///
/// Contains statistics about the processing operation including
/// files processed, skipped, and any errors encountered.
#[derive(Debug, Default, Clone)]
pub struct AssetReport {
    /// Number of files successfully processed
    pub files_processed: usize,
    /// Number of files skipped (e.g., excluded by pattern)
    pub files_skipped: usize,
    /// Errors encountered during processing
    pub errors: Vec<String>,
}

impl AssetReport {
    /// Create a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a processed file to the report.
    pub fn add_processed(&mut self) {
        self.files_processed += 1;
    }

    /// Add a skipped file to the report.
    pub fn add_skipped(&mut self) {
        self.files_skipped += 1;
    }

    /// Add an error to the report.
    pub fn add_error(&mut self, error: AssetError) {
        self.errors.push(error.to_string());
    }

    /// Check if the report has any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the total number of files (processed + skipped).
    pub fn total_files(&self) -> usize {
        self.files_processed + self.files_skipped
    }

    /// Merge another report into this one.
    pub fn merge(&mut self, other: AssetReport) {
        self.files_processed += other.files_processed;
        self.files_skipped += other.files_skipped;
        self.errors.extend(other.errors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_report_new() {
        let report = AssetReport::new();
        assert_eq!(report.files_processed, 0);
        assert_eq!(report.files_skipped, 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_asset_report_add_processed() {
        let mut report = AssetReport::new();
        report.add_processed();
        report.add_processed();
        assert_eq!(report.files_processed, 2);
    }

    #[test]
    fn test_asset_report_add_skipped() {
        let mut report = AssetReport::new();
        report.add_skipped();
        assert_eq!(report.files_skipped, 1);
    }

    #[test]
    fn test_asset_report_add_error() {
        let mut report = AssetReport::new();
        let error = AssetError::NotFound(std::path::PathBuf::from("test.txt"));
        report.add_error(error);
        assert_eq!(report.errors.len(), 1);
        assert!(report.has_errors());
    }

    #[test]
    fn test_asset_report_total_files() {
        let mut report = AssetReport::new();
        report.add_processed();
        report.add_processed();
        report.add_skipped();
        assert_eq!(report.total_files(), 3);
    }

    #[test]
    fn test_asset_report_merge() {
        let mut report1 = AssetReport::new();
        report1.add_processed();
        report1.add_error(AssetError::NotFound(std::path::PathBuf::from("a.txt")));

        let mut report2 = AssetReport::new();
        report2.add_processed();
        report2.add_skipped();
        report2.add_error(AssetError::Scss("error".to_string()));

        report1.merge(report2);
        assert_eq!(report1.files_processed, 2);
        assert_eq!(report1.files_skipped, 1);
        assert_eq!(report1.errors.len(), 2);
    }

    #[test]
    fn test_asset_report_has_errors() {
        let mut report = AssetReport::new();
        assert!(!report.has_errors());

        report.add_error(AssetError::NotFound(std::path::PathBuf::from("test.txt")));
        assert!(report.has_errors());
    }
}
