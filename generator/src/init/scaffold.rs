//! Site scaffolding functionality.

use std::path::Path;

use crate::error::{InitError, Result};

use super::{InitOptions, InitReport};

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
    fn create_directories(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let directories = [
            "content",
            "templates",
            "static",
            "styles",
        ];

        for dir in &directories {
            let dir_path = path.join(dir);
            if !dir_path.exists() {
                std::fs::create_dir_all(&dir_path).map_err(|e| InitError::DirectoryCreation {
                    path: dir_path.clone(),
                    source: e,
                })?;
                report.directories_created += 1;
            }
        }

        Ok(())
    }

    /// Create all default files.
    fn create_files(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        // Create site.toml
        self.create_site_config(path, report)?;

        // Create content/_index.md
        self.create_index_content(path, report)?;

        // Create templates
        self.create_templates(path, report)?;

        // Create styles/main.scss
        self.create_stylesheet(path, report)?;

        // Create static/.gitkeep
        self.create_gitkeep(path, report)?;

        Ok(())
    }

    /// Create the site.toml configuration file.
    fn create_site_config(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let config_path = path.join("site.toml");
        
        if config_path.exists() {
            return Ok(()); // Don't overwrite existing config
        }

        let content = format!(
            r#"[site]
name = "{}"
base_url = "{}"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"
"#,
            self.options.name,
            self.options.base_url
        );

        std::fs::write(&config_path, content).map_err(|e| InitError::FileWrite {
            path: config_path.clone(),
            source: e,
        })?;

        report.files_created += 1;
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
4. Run `yew-ssg build` to generate your site
"#,
            self.options.name,
            self.options.name
        );

        std::fs::write(&index_path, content).map_err(|e| InitError::FileWrite {
            path: index_path.clone(),
            source: e,
        })?;

        report.files_created += 1;
        Ok(())
    }

    /// Create the default templates.
    fn create_templates(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        // Create base.html
        let base_path = path.join("templates/base.html");
        if !base_path.exists() {
            let content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ site.name }}{% endblock %}</title>
    <link rel="stylesheet" href="/css/styles.css">
</head>
<body>
    <header>
        <h1>{{ site.name }}</h1>
        <nav>
            <a href="/">Home</a>
        </nav>
    </header>
    <main>
        {% block content %}{% endblock %}
    </main>
    <footer>
        <p>&copy; {{ "now" | date(format="%Y") }} {{ site.name }}</p>
    </footer>
</body>
</html>
"#;
            std::fs::write(&base_path, content).map_err(|e| InitError::FileWrite {
                path: base_path.clone(),
                source: e,
            })?;
            report.files_created += 1;
        }

        // Create page.html
        let page_path = path.join("templates/page.html");
        if !page_path.exists() {
            let content = r#"{% extends "base.html" %}

{% block title %}{{ page.title }} - {{ site.name }}{% endblock %}

{% block content %}
<article>
    <h1>{{ page.title }}</h1>
    {% if page.description %}
    <p class="description">{{ page.description }}</p>
    {% endif %}
    {{ page.content | safe }}
</article>
{% endblock %}
"#;
            std::fs::write(&page_path, content).map_err(|e| InitError::FileWrite {
                path: page_path.clone(),
                source: e,
            })?;
            report.files_created += 1;
        }

        // Create section.html
        let section_path = path.join("templates/section.html");
        if !section_path.exists() {
            let content = r#"{% extends "base.html" %}

{% block title %}{{ section.title }} - {{ site.name }}{% endblock %}

{% block content %}
<section>
    <h1>{{ section.title }}</h1>
    {% if section.description %}
    <p class="description">{{ section.description }}</p>
    {% endif %}
    
    {% if section.pages %}
    <ul class="page-list">
        {% for page in section.pages %}
        <li>
            <a href="{{ page.path }}">
                <h2>{{ page.title }}</h2>
                {% if page.description %}
                <p>{{ page.description }}</p>
                {% endif %}
            </a>
        </li>
        {% endfor %}
    </ul>
    {% endif %}
    
    {{ section.content | safe }}
</section>
{% endblock %}
"#;
            std::fs::write(&section_path, content).map_err(|e| InitError::FileWrite {
                path: section_path.clone(),
                source: e,
            })?;
            report.files_created += 1;
        }

        Ok(())
    }

    /// Create the main.scss stylesheet.
    fn create_stylesheet(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let styles_path = path.join("styles/main.scss");
        
        if styles_path.exists() {
            return Ok(()); // Don't overwrite existing styles
        }

        let content = r#"// Basic site styles
* {
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    line-height: 1.6;
    margin: 0;
    padding: 0;
}

header {
    background: #f5f5f5;
    padding: 1rem;
    
    h1 {
        margin: 0;
    }
    
    nav {
        margin-top: 0.5rem;
        
        a {
            margin-right: 1rem;
            text-decoration: none;
            color: #0066cc;
            
            &:hover {
                text-decoration: underline;
            }
        }
    }
}

main {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
    
    h1 {
        margin-top: 0;
    }
}

article {
    .description {
        color: #666;
        font-style: italic;
    }
}

.page-list {
    list-style: none;
    padding: 0;
    
    li {
        margin-bottom: 1.5rem;
        padding-bottom: 1.5rem;
        border-bottom: 1px solid #eee;
        
        &:last-child {
            border-bottom: none;
        }
        
        a {
            text-decoration: none;
            color: inherit;
            
            &:hover h2 {
                color: #0066cc;
            }
        }
        
        h2 {
            margin: 0 0 0.5rem 0;
            color: #333;
        }
        
        p {
            margin: 0;
            color: #666;
        }
    }
}

footer {
    text-align: center;
    padding: 1rem;
    background: #f5f5f5;
    margin-top: 2rem;
}
"#;

        std::fs::write(&styles_path, content).map_err(|e| InitError::FileWrite {
            path: styles_path.clone(),
            source: e,
        })?;

        report.files_created += 1;
        Ok(())
    }

    /// Create the static/.gitkeep file.
    fn create_gitkeep(&self, path: &Path, report: &mut InitReport) -> Result<()> {
        let gitkeep_path = path.join("static/.gitkeep");
        
        if gitkeep_path.exists() {
            return Ok(());
        }

        std::fs::write(&gitkeep_path, "").map_err(|e| InitError::FileWrite {
            path: gitkeep_path.clone(),
            source: e,
        })?;

        report.files_created += 1;
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
    }

    #[test]
    fn test_scaffold_creates_gitkeep() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());
        
        scaffolder.scaffold(temp_dir.path()).unwrap();
        
        assert!(temp_dir.path().join("static/.gitkeep").exists());
    }

    #[test]
    fn test_scaffold_report_counts() {
        let temp_dir = TempDir::new().unwrap();
        let scaffolder = InitScaffolder::new(test_options());
        
        let report = scaffolder.scaffold(temp_dir.path()).unwrap();
        
        // 4 directories + 7 files (site.toml, _index.md, base.html, page.html, section.html, main.scss, .gitkeep)
        assert_eq!(report.directories_created, 4);
        assert_eq!(report.files_created, 7);
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
        assert!(matches!(result.unwrap_err(), GeneratorError::Init(InitError::InvalidName(_))));
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
