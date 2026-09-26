// generator/src/init/scaffold.rs
//
//! Site scaffolding: writes the files named in the [manifest].
//!
//! Nothing here decides *what* a new site contains — that is the manifest's
//! job, and this module only writes what it lists (#111).
use super::manifest;
use super::{InitOptions, InitReport};
use crate::error::{InitError, Result};
use std::path::Path;

/// Scaffolder for creating new site structures.
pub struct InitScaffolder {
    options: InitOptions,
}

impl InitScaffolder {
    /// Create a new scaffolder with the given options.
    pub fn new(options: InitOptions) -> Self {
        Self { options }
    }

    /// Get the options.
    pub fn options(&self) -> &InitOptions {
        &self.options
    }

    /// Scaffold a new site at the given path.
    ///
    /// This creates the directory structure and all default files.
    pub fn scaffold(&self, path: &Path) -> Result<InitReport> {
        // Validate options first
        self.options.validate()?;

        // Create the base directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| InitError::DirectoryCreation {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        let mut report = InitReport::new(path.to_path_buf());

        // Create directory structure
        self.create_directories(path, &mut report)?;

        // Create files
        self.create_files(path, &mut report)?;

        Ok(report)
    }

    /// Create the directory structure.
    ///
    /// The list is [`manifest::DIRECTORIES`](super::manifest::DIRECTORIES).
    fn create_directories(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        for (dir, _) in manifest::DIRECTORIES {
            let dir_path = path.join(dir);
            if !dir_path.exists() {
                std::fs::create_dir_all(&dir_path).map_err(|e| InitError::DirectoryCreation {
                    path: dir_path.clone(),
                    source: e,
                })?;
                report.directories_created += 1;
                report.created_dirs.push(dir_path);
            }
        }

        Ok(())
    }

    /// Create all scaffolded files.
    ///
    /// The file list is [`manifest`], not this function: templates come from
    /// [`manifest::TEMPLATES`] and the rest from [`manifest::OTHER_FILES`],
    /// alongside the two files whose contents are built rather than read
    /// (`site.toml` and `_index.md`, both of which interpolate the site name).
    fn create_files(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        // Templates: contents come from init/templates/ on disk, with the
        // islands-dependent placeholders resolved from the manifest row.
        for template in manifest::TEMPLATES {
            let contents = manifest::resolve(template, self.options.islands);
            self.write_if_absent(
                &path.join("templates").join(template.name),
                contents.as_ref(),
                report,
            )?;
        }

        // site.toml and content/_index.md carry the site name, so they are
        // rendered here rather than read from disk.
        self.create_site_config(path, report)?;
        self.create_index_content(path, report)?;

        // Stylesheets and static files: copied verbatim from init/styles/ and
        // the bytes below.
        self.create_stylesheet(path, report)?;
        self.create_static_files(path, report)?;

        Ok(())
    }

    /// Create the site.toml configuration file.
    fn create_site_config(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let config_path = path.join("site.toml");

        if config_path.exists() {
            return Ok(()); // Don't overwrite existing config
        }

        let islands_setting = if self.options.islands {
            ""
        } else {
            // The scaffold's templates never call island(); say so in
            // the config so the build skips the WASM client (#56).
            "\n# Plain Tera/Markdown site (--no-islands): no WASM hydration.\nislands = false"
        };

        let content = format!(
            r#"[site]
name = "{}"
base_url = "{}"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"{islands_setting}

[images]
# Responsive hero image variants (widths in px).
widths = [400, 800, 1200]
# Output format: "webp", "jpeg" (or "jpg"), or "png".
format = "webp"
# Encoding quality 1-100. Applies to jpeg and webp only; png is lossless
# and ignores it. Changing this re-encodes variants on the next build.
quality = 80
"#,
            self.options.name, self.options.base_url
        );

        std::fs::write(&config_path, content).map_err(|e| InitError::FileWrite {
            path: config_path.clone(),
            source: e,
        })?;

        report.files_created += 1;
        report.created_files.push(config_path);
        Ok(())
    }

    /// Create the content/_index.md file.
    fn create_index_content(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let index_path = path.join("content/_index.md");

        if index_path.exists() {
            return Ok(()); // Don't overwrite existing content
        }

        let content = format!(
            r#"+++
title = "Home"
description = "Welcome to {}"
+++

# Welcome to {}

This is your new static site. Start editing this file to add your content.

## Getting Started

1. Edit `site.toml` to configure your site settings
2. Add new markdown files in the `content/` directory
3. Customize templates in `templates/`
4. Run `taxus serve` to develop your site, with hot-reloading of changes.
5. Run `taxus build` to generate your site
"#,
            self.options.name, self.options.name
        );

        std::fs::write(&index_path, content).map_err(|e| InitError::FileWrite {
            path: index_path.clone(),
            source: e,
        })?;

        report.files_created += 1;
        report.created_files.push(index_path);
        Ok(())
    }

    /// Write a file unless it is already there.
    ///
    /// Never overwrites: `taxus init` into a populated directory is additive,
    /// which is why the CLI warns but does not stop.
    fn write_if_absent(&self, path: &Path, contents: &str, report: &mut InitReport) -> Result<()> {
        if path.exists() {
            return Ok(());
        }
        std::fs::write(path, contents).map_err(|e| InitError::FileWrite {
            path: path.to_path_buf(),
            source: e,
        })?;
        report.files_created += 1;
        report.created_files.push(path.to_path_buf());
        Ok(())
    }

    /// Create the main.scss stylesheet and associated dark/light syntax highlighting stylesheets.
    fn create_stylesheet(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let styles_path_main = path.join("styles/main.scss");
        let styles_path_highlight_dark = path.join("styles/_highlight-dark.scss");
        let styles_path_highlight_light = path.join("styles/_highlight-light.scss");

        let styles_main = include_str!("styles/_main.scss");
        let styles_highlight_dark = include_str!("styles/_highlight-dark.scss");
        let styles_highlight_light = include_str!("styles/_highlight-light.scss");

        if styles_path_main.exists() {
            return Ok(()); // Don't overwrite existing styles
        }

        std::fs::write(&styles_path_main, styles_main).map_err(|e| InitError::FileWrite {
            path: styles_path_main.clone(),
            source: e,
        })?;
        report.files_created += 1;
        report.created_files.push(styles_path_main);

        std::fs::write(&styles_path_highlight_dark, styles_highlight_dark).map_err(|e| {
            InitError::FileWrite {
                path: styles_path_highlight_dark.clone(),
                source: e,
            }
        })?;
        report.files_created += 1;
        report.created_files.push(styles_path_highlight_dark);

        std::fs::write(&styles_path_highlight_light, styles_highlight_light).map_err(|e| {
            InitError::FileWrite {
                path: styles_path_highlight_light.clone(),
                source: e,
            }
        })?;
        report.files_created += 1;
        report.created_files.push(styles_path_highlight_light);
        Ok(())
    }

    /// Create the static files (scripts.js and favicon.png).
    ///
    /// `scripts.js` is read from `init/static/` like the other copied assets.
    /// `favicon.png` stays a byte array: it is binary, and a 1x1 transparent
    /// pixel is a placeholder the user replaces immediately, so a literal in
    /// the source says more than a file on disk would.
    fn create_static_files(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let scripts = include_str!("static/scripts.js");
        self.write_if_absent(&path.join("static/scripts.js"), scripts, report)?;

        // Create favicon.png (a minimal 16x16 PNG)
        let favicon_path = path.join("static/favicon.png");
        if !favicon_path.exists() {
            // Minimal valid PNG: 1x1 transparent pixel
            let favicon_bytes: &[u8] = &[
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
                0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, // 8-bit RGBA
                0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, // IDAT chunk
                0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D,
                0xB4, 0x00, // compressed data
                0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, // IEND chunk
                0x42, 0x60, 0x82,
            ];
            std::fs::write(&favicon_path, favicon_bytes).map_err(|e| InitError::FileWrite {
                path: favicon_path.clone(),
                source: e,
            })?;
            report.files_created += 1;
            report.created_files.push(favicon_path);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GeneratorError;
    use tempfile::TempDir;

    fn test_options() -> InitOptions {
        InitOptions::new("Test Site", "https://test.example.com")
    }

    #[test]
    fn test_scaffolder_new() {
        let options = test_options();
        let scaffolder = InitScaffolder::new(options.clone());
        assert_eq!(scaffolder.options().name, options.name);
    }

    #[test]
    fn test_scaffold_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        let report = scaffolder.scaffold(temp_dir.path()).unwrap();

        assert!(temp_dir.path().join("content").exists());
        assert!(temp_dir.path().join("templates").exists());
        assert!(temp_dir.path().join("static").exists());
        assert!(temp_dir.path().join("styles").exists());
        assert_eq!(report.directories_created, 4);
    }

    #[test]
    fn test_scaffold_creates_site_config() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        scaffolder.scaffold(temp_dir.path()).unwrap();

        let config_path = temp_dir.path().join("site.toml");
        assert!(config_path.exists());

        let content = std::fs::read_to_string(config_path).unwrap();
        assert!(content.contains("Test Site"));
        assert!(content.contains("https://test.example.com"));
    }

    #[test]
    fn test_scaffold_creates_index_content() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        scaffolder.scaffold(temp_dir.path()).unwrap();

        let index_path = temp_dir.path().join("content/_index.md");
        assert!(index_path.exists());

        let content = std::fs::read_to_string(index_path).unwrap();
        assert!(content.contains("+++"));
        assert!(content.contains("title = \"Home\""));
    }

    #[test]
    fn test_scaffold_creates_templates() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        scaffolder.scaffold(temp_dir.path()).unwrap();

        assert!(temp_dir.path().join("templates/base.html").exists());
        assert!(temp_dir.path().join("templates/page.html").exists());
        assert!(temp_dir.path().join("templates/section.html").exists());
        assert!(temp_dir.path().join("templates/404.html").exists());
        assert!(temp_dir.path().join("templates/tags.html").exists());
        assert!(temp_dir.path().join("templates/categories.html").exists());
        assert!(temp_dir.path().join("templates/series.html").exists());
        assert!(temp_dir.path().join("templates/tags_term.html").exists());
        assert!(
            temp_dir
                .path()
                .join("templates/categories_term.html")
                .exists()
        );
        assert!(temp_dir.path().join("templates/series_term.html").exists());
    }

    #[test]
    fn test_scaffold_creates_stylesheet() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        scaffolder.scaffold(temp_dir.path()).unwrap();

        let styles_path = temp_dir.path().join("styles/main.scss");
        assert!(styles_path.exists());

        let content = std::fs::read_to_string(styles_path).unwrap();
        assert!(content.contains("box-sizing"));
        assert!(content.contains("highlight-light"));
    }

    #[test]
    fn test_scaffold_creates_highlight_themes() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        scaffolder.scaffold(temp_dir.path()).unwrap();

        let dark = temp_dir.path().join("styles/_highlight-dark.scss");
        let light = temp_dir.path().join("styles/_highlight-light.scss");

        assert!(dark.exists());
        assert!(light.exists());

        let dark_content = std::fs::read_to_string(dark).unwrap();
        assert!(dark_content.contains("hl-keyword"));

        let light_content = std::fs::read_to_string(light).unwrap();
        assert!(light_content.contains("hl-keyword"));
    }

    #[test]
    fn test_scaffold_creates_static_files() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        scaffolder.scaffold(temp_dir.path()).unwrap();

        assert!(temp_dir.path().join("static/scripts.js").exists());
        assert!(temp_dir.path().join("static/favicon.png").exists());
    }

    #[test]
    fn test_scaffold_report_counts() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        let report = scaffolder.scaffold(temp_dir.path()).unwrap();

        // 4 directories + 12 files (site.toml, _index.md, base.html, page.html, section.html, 404.html, tags_term.html, categories_term.html, series_term.html, main.scss, scripts.js, favicon.png)
        // Note: _highlight-dark.scss and _highlight-light.scss are also created but counted as part of the stylesheet set
        assert_eq!(report.directories_created, 4);
        assert_eq!(report.files_created, 17);
    }

    #[test]
    fn test_scaffold_does_not_overwrite_config() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());

        // Create an existing config
        let config_path = temp_dir.path().join("site.toml");
        std::fs::write(&config_path, "existing content").unwrap();

        scaffolder.scaffold(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(config_path).unwrap();
        assert_eq!(content, "existing content");
    }

    #[test]
    fn test_scaffold_validates_options() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_options = InitOptions::new("", "https://test.example.com");
        let scaffolder = InitScaffolder::new(invalid_options);

        let result = scaffolder.scaffold(temp_dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            GeneratorError::Init(inner) if matches!(*inner, InitError::InvalidName(_))
        ));
    }

    #[test]
    fn test_scaffold_creates_base_directory() {
        let temp_dir = TempDir::new().unwrap();
        let new_site_path = temp_dir.path().join("new-site");

        let scaffolder = InitScaffolder::new(test_options());
        let report = scaffolder.scaffold(&new_site_path).unwrap();

        assert!(new_site_path.exists());
        assert_eq!(report.path, new_site_path);
    }
}
