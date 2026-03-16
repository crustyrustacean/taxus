# API Reference

This page documents the public API of the generator library.

## Modules

### `config`

Configuration types for loading and representing site configuration.

#### `SiteConfig`

Main configuration type.

```rust
pub struct SiteConfig {
    pub site: SiteMeta,
    pub build: BuildConfig,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `from_file<P: AsRef<Path>>(path: P) -> Result<Self>` | Load configuration from a file |
| `from_dir<P: AsRef<Path>>(dir: P) -> Result<Self>` | Load configuration from a directory |
| `new(name: impl Into<String>, base_url: impl Into<String>) -> Self` | Create a new configuration |
| `validate(&self) -> Result<()>` | Validate the configuration |

#### `SiteMeta`

Site metadata.

```rust
pub struct SiteMeta {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub author: Option<String>,
}
```

#### `BuildConfig`

Build configuration.

```rust
pub struct BuildConfig {
    pub content_dir: PathBuf,
    pub output_dir: PathBuf,
    pub static_dir: PathBuf,
    pub styles_dir: PathBuf,
    pub templates_dir: PathBuf,
}
```

Implements `Default` with sensible defaults.

### `content`

Content types for parsing and managing Markdown files with TOML frontmatter.

#### `Frontmatter`

Page metadata parsed from TOML between `+++` markers.

```rust
pub struct Frontmatter {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<NaiveDate>,
    pub template: Option<String>,
    pub draft: bool,
    pub extra: Option<toml::Value>,
}
```

Implements `Default` with empty values and `draft: false`.

##### Methods

| Method | Description |
|--------|-------------|
| `from_str(s: &str) -> Result<Self, toml::de::Error>` | Parse frontmatter from TOML string |
| `template(&self) -> &str` | Get template name (defaults to "page.html") |

#### `Page`

A single page with frontmatter and Markdown content.

```rust
pub struct Page {
    pub frontmatter: Frontmatter,
    pub path: String,
    pub source: PathBuf,
    pub raw_content: String,
    pub content: Option<String>,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `from_file<P: AsRef<Path>>(path: P) -> Result<Self>` | Parse a page from a Markdown file |
| `from_str(content: &str, source: &str) -> Result<Self>` | Parse a page from a string |
| `template(&self) -> &str` | Get the template name for this page |
| `is_draft(&self) -> bool` | Check if this page is a draft |

#### `Section`

A section containing multiple pages (e.g., a blog).

```rust
pub struct Section {
    pub frontmatter: Frontmatter,
    pub path: String,
    pub source: PathBuf,
    pub pages: Vec<Page>,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `from_dir<P: AsRef<Path>>(dir: P) -> Result<Self>` | Create a section from a directory |
| `add_page(&mut self, page: Page)` | Add a page to this section |
| `sort_by_date(&mut self)` | Sort pages by date (newest first) |
| `template(&self) -> &str` | Get the template name for this section |

#### `ContentSource` Trait

Trait for loading content from various sources.

```rust
pub trait ContentSource: Send + Sync {
    fn load(&self, path: &Path) -> Result<String>;
    fn exists(&self, path: &Path) -> bool;
    fn list(&self) -> Result<Vec<PathBuf>>;
}
```

#### `FilesystemContentSource`

Default filesystem-based content source.

```rust
pub struct FilesystemContentSource {
    root: PathBuf,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new<P: Into<PathBuf>>(root: P) -> Self` | Create a new filesystem content source |

### `error`

Error types for the library.

#### `GeneratorError`

Main error type.

```rust
pub enum GeneratorError {
    Config(ConfigError),
    Content(ContentError),
    Io { path: PathBuf, source: std::io::Error },
}
```

Implements `std::error::Error` and `Display`.

#### `ConfigError`

Configuration errors.

```rust
pub enum ConfigError {
    NotFound(PathBuf),
    Invalid(String),
    Parse(toml::de::Error),
    MissingField { field: &'static str },
}
```

#### `ContentError`

Content-related errors.

```rust
pub enum ContentError {
    NotFound(PathBuf),
    InvalidFrontmatter { path: PathBuf, source: toml::de::Error },
    UnclosedFrontmatter(PathBuf),
    Io { path: PathBuf, source: std::io::Error },
    MissingField { field: &'static str, path: PathBuf },
    InvalidPath(String),
}
```

#### `Result`

Result alias for generator operations.

```rust
pub type Result<T> = std::result::Result<T, GeneratorError>;
```

## Re-exports

The library re-exports commonly used types:

```rust
// Configuration
pub use config::{BuildConfig, SiteConfig, SiteMeta};

// Content
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page, Section};

// Errors
pub use error::{ContentError, GeneratorError, Result};
```

## Usage Examples

### Loading Configuration

```rust
use generator::{SiteConfig, Result};

fn main() -> Result<()> {
    // From a specific file
    let config = SiteConfig::from_file("site.toml")?;
    
    // From a directory
    let config = SiteConfig::from_dir(".")?;
    
    // Programmatic creation
    let config = SiteConfig::new("My Site", "https://example.com");
    
    Ok(())
}
```

### Loading a Page

```rust
use generator::{Page, Result};

fn main() -> Result<()> {
    // Load from file
    let page = Page::from_file("content/about.md")?;
    
    println!("Title: {}", page.frontmatter.title);
    println!("Path: {}", page.path);
    println!("Draft: {}", page.is_draft());
    
    Ok(())
}
```

### Loading a Section

```rust
use generator::{Section, Result};

fn main() -> Result<()> {
    let mut section = Section::from_dir("content/blog")?;
    
    // Add pages and sort by date
    section.sort_by_date();
    
    println!("Section: {}", section.frontmatter.title);
    for page in &section.pages {
        println!("  - {}", page.frontmatter.title);
    }
    
    Ok(())
}
```

### Using ContentSource

```rust
use generator::{ContentSource, FilesystemContentSource, Result};
use std::path::PathBuf;

fn list_content() -> Result<()> {
    let source = FilesystemContentSource::new("content");
    
    // List all markdown files
    for file in source.list()? {
        println!("Found: {}", file.display());
    }
    
    // Check if file exists
    if source.exists(&PathBuf::from("about.md")) {
        let content = source.load(&PathBuf::from("about.md"))?;
        println!("Content length: {} bytes", content.len());
    }
    
    Ok(())
}
```

### Validating Configuration

```rust
use generator::{SiteConfig, ConfigError, GeneratorError};

fn validate_site() -> Result<(), String> {
    let config = SiteConfig::from_dir(".")
        .map_err(|e| format!("Failed to load: {}", e))?;
    
    config.validate()
        .map_err(|e| format!("Invalid config: {}", e))?;
    
    Ok(())
}
```

### Error Handling

```rust
use generator::{SiteConfig, Page, GeneratorError, ConfigError, ContentError};

fn load_site() {
    match SiteConfig::from_dir(".") {
        Ok(config) => {
            println!("Site: {}", config.site.name);
        }
        Err(GeneratorError::Config(ConfigError::NotFound(path))) => {
            eprintln!("Config not found: {}", path.display());
        }
        Err(GeneratorError::Config(ConfigError::Parse(e))) => {
            eprintln!("Parse error: {}", e);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

fn load_page() {
    match Page::from_file("content/about.md") {
        Ok(page) => {
            println!("Title: {}", page.frontmatter.title);
        }
        Err(GeneratorError::Content(ContentError::NotFound(path))) => {
            eprintln!("Page not found: {}", path.display());
        }
        Err(GeneratorError::Content(ContentError::InvalidFrontmatter { path, source })) => {
            eprintln!("Invalid frontmatter in {}: {}", path.display(), source);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

### Accessing Configuration Values

```rust
use generator::SiteConfig;

let config = SiteConfig::from_dir(".")?;

// Site metadata
println!("Name: {}", config.site.name);
println!("URL: {}", config.site.base_url);

if let Some(desc) = &config.site.description {
    println!("Description: {}", desc);
}

// Build configuration
println!("Content: {:?}", config.build.content_dir);
println!("Output: {:?}", config.build.output_dir);
```

## Feature Flags

The library currently has no feature flags. Future versions may add:

- `async`: Async API support
- `cli`: CLI utilities
- `serve`: Development server

## Versioning

The library follows [Semantic Versioning](https://semver.org/):

- **Major**: Breaking API changes
- **Minor**: New features, backward compatible
- **Patch**: Bug fixes, backward compatible

## Stability

The current API is considered **unstable** and may change between versions.

The following are considered stable and will not change in minor versions:

- Error type hierarchy
- `SiteConfig::from_file` and `from_dir` signatures
- `BuildConfig::default()` behavior
- `Page::from_file` and `Page::from_str` signatures
- `Frontmatter::from_str` signature
