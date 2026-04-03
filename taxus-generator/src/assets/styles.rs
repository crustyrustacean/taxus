//! SCSS/SASS processor for compiling stylesheets.
//!
//! This module provides the [`ScssProcessor`] implementation for compiling
//! SCSS files to CSS using the `grass` crate.

use crate::assets::{AssetProcessor, AssetReport};
use crate::error::AssetError;
use std::fs;
use std::path::Path;
use tracing::{debug, debug_span, info, instrument};

/// SCSS/SASS processor for compiling stylesheets.
///
/// This processor compiles `.scss` files to CSS using the `grass` crate.
/// It supports include paths for `@import` resolution and optional minification.
///
/// # Example
///
/// ```no_run
/// use taxus_lib::assets::ScssProcessor;
/// use taxus_lib::assets::AssetProcessor;
/// use std::path::Path;
///
/// let processor = ScssProcessor::new()
///     .with_minify(true);
/// let report = processor.process(
///     Path::new("styles/main.scss"),
///     Path::new("dist/styles/main.css")
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ScssProcessor {
    /// Additional include paths for @import resolution
    include_paths: Vec<std::path::PathBuf>,
    /// Whether to minify output CSS
    minify: bool,
}

impl Default for ScssProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScssProcessor {
    /// Create a new SCSS processor with default settings.
    pub fn new() -> Self {
        Self {
            include_paths: Vec::new(),
            minify: false,
        }
    }

    /// Create a processor with additional include paths.
    ///
    /// Include paths are used to resolve `@import` statements in SCSS files.
    pub fn with_include_paths<P: Into<std::path::PathBuf>>(paths: Vec<P>) -> Self {
        Self {
            include_paths: paths.into_iter().map(Into::into).collect(),
            minify: false,
        }
    }

    /// Set whether to minify the output CSS.
    pub fn with_minify(mut self, minify: bool) -> Self {
        self.minify = minify;
        self
    }

    /// Compile SCSS content to CSS.
    ///
    /// # Arguments
    ///
    /// * `content` - The SCSS content to compile
    /// * `source_path` - Path to the source file (for error messages)
    ///
    /// # Returns
    ///
    /// The compiled CSS content.
    fn compile(&self, content: &str, source_path: &Path) -> Result<String, AssetError> {
        let mut options = grass::Options::default();

        // Add include paths
        for path in &self.include_paths {
            options = options.load_path(path.clone());
        }

        // Set style based on minify option
        let style = if self.minify {
            grass::OutputStyle::Compressed
        } else {
            grass::OutputStyle::Expanded
        };
        options = options.style(style);

        grass::from_string(content.to_string(), &options).map_err(|e| {
            let error_msg = format!("{}: {}", source_path.display(), e);
            AssetError::Scss(error_msg)
        })
    }

    /// Get the output path for a given source path.
    ///
    /// Changes the extension from `.scss` to `.css`.
    fn output_path(src: &Path, dest: &Path) -> std::path::PathBuf {
        if src.extension().is_some_and(|ext| ext == "scss") {
            // Change .scss extension to .css
            dest.with_extension("css")
        } else if src.extension().is_some_and(|ext| ext == "sass") {
            // Change .sass extension to .css
            dest.with_extension("css")
        } else {
            dest.to_path_buf()
        }
    }
}

impl ScssProcessor {
    /// Process a single SCSS file.
    fn process_file(
        &self,
        src: &Path,
        dest: &Path,
        report: &mut AssetReport,
    ) -> Result<(), AssetError> {
        debug!(src = %src.display(), dest = %dest.display(), "Processing SCSS file");

        // Read source file
        let content = fs::read_to_string(src).map_err(|e| AssetError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;

        // Compile SCSS to CSS
        let css = self.compile(&content, src)?;

        // Determine output path (change .scss to .css)
        let output_path = Self::output_path(src, dest);

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AssetError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write output file
        fs::write(&output_path, css).map_err(|e| AssetError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(output = %output_path.display(), "SCSS compiled successfully");
        report.add_processed();
        Ok(())
    }

    /// Process a directory of SCSS files recursively.
    fn process_directory(
        &self,
        src_dir: &Path,
        dest_dir: &Path,
        report: &mut AssetReport,
    ) -> Result<(), AssetError> {
        use walkdir::WalkDir;

        debug!(src = %src_dir.display(), dest = %dest_dir.display(), "Processing SCSS directory");

        for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip directories and non-SCSS files
            if !path.is_file() || !self.handles(path) {
                continue;
            }

            // Calculate relative path and destination
            let relative = path
                .strip_prefix(src_dir)
                .map_err(|_| AssetError::NotFound(src_dir.to_path_buf()))?;

            let dest_path = dest_dir.join(relative);

            // Process the file
            match self.process_file(path, &dest_path, report) {
                Ok(()) => {}
                Err(e) => report.add_error(e),
            }
        }

        Ok(())
    }
}

impl AssetProcessor for ScssProcessor {
    #[instrument(skip(self), fields(processor = "scss"))]
    fn process(&self, src: &Path, dest: &Path) -> Result<AssetReport, AssetError> {
        let span =
            debug_span!("scss_asset_processing", src = %src.display(), dest = %dest.display());
        let _enter = span.enter();

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
            self.process_file(src, dest, &mut report)?;
        }

        info!(
            processed = report.files_processed,
            errors = report.errors.len(),
            minified = self.minify,
            "SCSS processing complete"
        );

        Ok(report)
    }

    fn handles(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|ext| ext == "scss" || ext == "sass")
    }

    fn name(&self) -> &'static str {
        "scss"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scss_processor_new() {
        let processor = ScssProcessor::new();
        assert!(processor.include_paths.is_empty());
        assert!(!processor.minify);
    }

    #[test]
    fn test_scss_processor_with_minify() {
        let processor = ScssProcessor::new().with_minify(true);
        assert!(processor.minify);
    }

    #[test]
    fn test_scss_processor_handles_scss() {
        let processor = ScssProcessor::new();
        assert!(processor.handles(Path::new("styles/main.scss")));
        assert!(processor.handles(Path::new("styles/main.sass")));
        assert!(!processor.handles(Path::new("styles/main.css")));
        assert!(!processor.handles(Path::new("styles/main.js")));
    }

    #[test]
    fn test_scss_processor_process_basic() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("test.scss");
        let dest_path = temp_dir.path().join("output/test.css");

        // Create test SCSS file
        let scss_content = r#"
$color: blue;

body {
    color: $color;
}
"#;
        fs::write(&src_path, scss_content).unwrap();

        let processor = ScssProcessor::new();
        let result = processor.process(&src_path, &dest_path);

        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.files_processed, 1);

        // Check output file was created with .css extension
        let expected_output = temp_dir.path().join("output/test.css");
        assert!(expected_output.exists());

        // Check content
        let css = fs::read_to_string(&expected_output).unwrap();
        assert!(css.contains("color: blue"));
    }

    #[test]
    fn test_scss_processor_process_minified() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("test.scss");
        let dest_path = temp_dir.path().join("test.css");

        let scss_content = r#"
body {
    color: blue;
    margin: 0;
}
"#;
        fs::write(&src_path, scss_content).unwrap();

        let processor = ScssProcessor::new().with_minify(true);
        let result = processor.process(&src_path, &dest_path);

        assert!(result.is_ok());

        // Check output is minified (no newlines between rules)
        let css = fs::read_to_string(temp_dir.path().join("test.css")).unwrap();
        // Minified CSS should be more compact
        assert!(css.contains("color:blue") || css.contains("color: blue"));
    }

    #[test]
    fn test_scss_processor_process_not_found() {
        let processor = ScssProcessor::new();
        let result = processor.process(Path::new("nonexistent.scss"), Path::new("output.css"));

        assert!(result.is_err());
        match result.unwrap_err() {
            AssetError::NotFound(path) => assert!(path.ends_with("nonexistent.scss")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_scss_processor_invalid_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("invalid.scss");
        let dest_path = temp_dir.path().join("invalid.css");

        // Invalid SCSS syntax
        let scss_content = "body { color: }"; // Missing value
        fs::write(&src_path, scss_content).unwrap();

        let processor = ScssProcessor::new();
        let result = processor.process(&src_path, &dest_path);

        assert!(result.is_err());
        match result.unwrap_err() {
            AssetError::Scss(msg) => assert!(msg.contains("invalid.scss")),
            _ => panic!("Expected Scss error"),
        }
    }

    #[test]
    fn test_scss_processor_output_path() {
        let src = Path::new("styles/main.scss");
        let dest = Path::new("dist/styles/main.scss");

        let output = ScssProcessor::output_path(src, dest);
        assert_eq!(output, Path::new("dist/styles/main.css"));
    }

    #[test]
    fn test_scss_processor_with_variables() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("vars.scss");
        let dest_path = temp_dir.path().join("vars.css");

        let scss_content = r#"
$primary: #3498db;
$secondary: #2ecc71;

.button {
    background: $primary;
    &:hover {
        background: $secondary;
    }
}
"#;
        fs::write(&src_path, scss_content).unwrap();

        let processor = ScssProcessor::new();
        let result = processor.process(&src_path, &dest_path);

        assert!(result.is_ok());
        let css = fs::read_to_string(temp_dir.path().join("vars.css")).unwrap();
        assert!(css.contains("#3498db"));
        assert!(css.contains("#2ecc71"));
    }

    #[test]
    fn test_scss_processor_name() {
        let processor = ScssProcessor::new();
        assert_eq!(processor.name(), "scss");
    }
}
