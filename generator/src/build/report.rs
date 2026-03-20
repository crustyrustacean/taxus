//! Build report and statistics.
//!
//! This module provides types for tracking build results and statistics.

use crate::assets::AssetReport;
use std::path::PathBuf;
use std::time::Duration;

/// Report of a completed build.
#[derive(Debug, Clone)]
pub struct BuildReport {
    /// Number of pages rendered
    pub pages_rendered: usize,
    /// Number of sections rendered
    pub sections_rendered: usize,
    /// Number of drafts skipped
    pub drafts_skipped: usize,
    /// Asset processing report
    pub assets: AssetReport,
    /// Build duration
    pub duration: Duration,
    /// Any warnings generated during build
    pub warnings: Vec<String>,
    /// Output directory path
    pub output_dir: PathBuf,
}

impl BuildReport {
    /// Create a new empty build report.
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            pages_rendered: 0,
            sections_rendered: 0,
            drafts_skipped: 0,
            assets: AssetReport::new(),
            duration: Duration::ZERO,
            warnings: Vec::new(),
            output_dir,
        }
    }

    /// Get the total number of files generated (pages + sections + assets).
    pub fn total_files(&self) -> usize {
        self.pages_rendered + self.sections_rendered + self.assets.files_processed
    }

    /// Check if the build had any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty() || self.assets.has_errors()
    }

    /// Add a warning to the report.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Print a summary of the build to stdout.
    pub fn print_summary(&self) {
        let status = if self.has_warnings() {
            format!(
                "⚠  Build completed with warnings  ({:.2}s)",
                self.duration.as_secs_f64()
            )
        } else {
            format!("✓  Build complete  ({:.2}s)", self.duration.as_secs_f64())
        };

        println!("\n{status}");
        println!("─────────────────────────────────");
        println!("  {:<16} {}", "Pages", self.pages_rendered);
        println!("  {:<16} {}", "Sections", self.sections_rendered);
        if self.drafts_skipped > 0 {
            println!("  {:<16} {}", "Drafts skipped", self.drafts_skipped);
        }
        println!("  {:<16} {}", "Assets", self.assets.files_processed);
        println!("  {:<16} {}", "Total files", self.total_files());
        println!("  {:<16} {}", "Output", self.output_dir.display());
        println!("─────────────────────────────────");

        if self.has_warnings() {
            println!("\n  Warnings:");
            for warning in &self.warnings {
                println!("    ⚠  {warning}");
            }
            for error in &self.assets.errors {
                println!("    ⚠  {error}");
            }
        }
    }
}

impl Default for BuildReport {
    fn default() -> Self {
        Self::new(PathBuf::from("dist"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_report_new() {
        let report = BuildReport::new(PathBuf::from("output"));
        assert_eq!(report.pages_rendered, 0);
        assert_eq!(report.sections_rendered, 0);
        assert_eq!(report.drafts_skipped, 0);
        assert_eq!(report.duration, Duration::ZERO);
        assert!(report.warnings.is_empty());
        assert_eq!(report.output_dir, PathBuf::from("output"));
    }

    #[test]
    fn test_build_report_default() {
        let report = BuildReport::default();
        assert_eq!(report.output_dir, PathBuf::from("dist"));
    }

    #[test]
    fn test_build_report_total_files() {
        let mut report = BuildReport::default();
        report.pages_rendered = 5;
        report.sections_rendered = 2;
        report.assets.files_processed = 10;
        assert_eq!(report.total_files(), 17);
    }

    #[test]
    fn test_build_report_has_warnings_false() {
        let report = BuildReport::default();
        assert!(!report.has_warnings());
    }

    #[test]
    fn test_build_report_has_warnings_true() {
        let mut report = BuildReport::default();
        report.add_warning("Test warning");
        assert!(report.has_warnings());
    }

    #[test]
    fn test_build_report_has_warnings_from_assets() {
        let mut report = BuildReport::default();
        report.assets.errors.push("Asset error".to_string());
        assert!(report.has_warnings());
    }

    #[test]
    fn test_build_report_add_warning() {
        let mut report = BuildReport::default();
        report.add_warning("First warning");
        report.add_warning("Second warning".to_string());
        assert_eq!(report.warnings.len(), 2);
        assert_eq!(report.warnings[0], "First warning");
        assert_eq!(report.warnings[1], "Second warning");
    }

    #[test]
    fn test_build_report_print_summary() {
        let mut report = BuildReport::new(PathBuf::from("dist"));
        report.pages_rendered = 5;
        report.sections_rendered = 1;
        report.drafts_skipped = 2;
        report.duration = Duration::from_millis(1500);
        report.assets.files_processed = 3;

        // This just ensures print_summary doesn't panic
        report.print_summary();
    }
}
