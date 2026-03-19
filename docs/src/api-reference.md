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

### `routes`

Route discovery and management types for mapping content files to URL paths.

#### `RouteKind`

Enum distinguishing between page and section routes.

```rust
pub enum RouteKind {
    /// A single page (e.g., /about/)
    Page,
    /// A section index (e.g., /blog/)
    Section,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `is_page(&self) -> bool` | Returns true if this is a page route |
| `is_section(&self) -> bool` | Returns true if this is a section route |

#### `RouteInfo`

Information about a single route.

```rust
pub struct RouteInfo {
    /// URL path (e.g., "/about/")
    pub path: String,
    /// Content file path relative to content directory
    pub content_file: PathBuf,
    /// Output file path relative to output directory
    pub output_file: PathBuf,
    /// Route type
    pub kind: RouteKind,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new(path: String, content_file: PathBuf, output_file: PathBuf, kind: RouteKind) -> Result<Self, RouteError>` | Create a new route info with path validation |
| `is_page(&self) -> bool` | Returns true if this is a page route |
| `is_section(&self) -> bool` | Returns true if this is a section route |

#### `RouteRegistry`

Registry of all routes in the site.

```rust
pub struct RouteRegistry {
    routes: HashMap<String, RouteInfo>,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create a new empty registry |
| `register(&mut self, route: RouteInfo) -> Result<(), RouteError>` | Register a route (fails on duplicate) |
| `get(&self, path: &str) -> Option<&RouteInfo>` | Get route by path |
| `contains(&self, path: &str) -> bool` | Check if a route exists |
| `len(&self) -> usize` | Get the number of routes |
| `is_empty(&self) -> bool` | Check if the registry is empty |
| `iter(&self) -> impl Iterator<Item = &RouteInfo>` | Iterate over all routes |
| `pages(&self) -> impl Iterator<Item = &RouteInfo>` | Iterate over all page routes |
| `sections(&self) -> impl Iterator<Item = &RouteInfo>` | Iterate over all section routes |
| `generate_rust_manifest(&self) -> String` | Generate Rust code for client routing |

#### `RouteDiscovery`

Discovers routes from content directory structure.

```rust
pub struct RouteDiscovery {
    content_dir: PathBuf,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new<P: Into<PathBuf>>(content_dir: P) -> Self` | Create a new route discovery |
| `discover(&self) -> Result<RouteRegistry, RouteError>` | Discover all routes from content directory |
| `discover_from_source<S: ContentSource>(&self, source: &S) -> Result<RouteRegistry, RouteError>` | Discover routes using ContentSource trait |

##### Path Conversion Logic

| Content File | URL Path | Output File | Route Kind |
|--------------|----------|-------------|------------|
| `_index.md` | `/` | `index.html` | Section |
| `about.md` | `/about/` | `about/index.html` | Page |
| `blog/_index.md` | `/blog/` | `blog/index.html` | Section |
| `blog/first-post.md` | `/blog/first-post/` | `blog/first-post/index.html` | Page |

### `error`

Error types for the library.

#### `GeneratorError`

Main error type.

```rust
pub enum GeneratorError {
    Config(ConfigError),
    Content(ContentError),
    Template(TemplateError),
    Asset(AssetError),
    Route(RouteError),
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

#### `TemplateError`

Template-related errors.

```rust
pub enum TemplateError {
    NotFound(String),
    Render(String),
    Syntax { template: String, message: String },
    Io { path: PathBuf, source: std::io::Error },
    DirNotFound(PathBuf),
}
```

#### `AssetError`

Asset-related errors.

```rust
pub enum AssetError {
    NotFound(PathBuf),
    Scss(String),
    Io { path: PathBuf, source: std::io::Error },
    CopyFailed { src: PathBuf, dest: PathBuf, reason: String },
}
```

#### `RouteError`

Route-related errors.

```rust
pub enum RouteError {
    NotFound(String),
    Duplicate(String),
    InvalidPath(String),
    DiscoveryFailed(String),
}
```

#### `Result`

Result alias for generator operations.

```rust
pub type Result<T> = std::result::Result<T, GeneratorError>;
```

### `assets`

Asset processing types for SCSS compilation and static file copying.

#### `AssetProcessor` Trait

Trait for processing assets from source to destination.

```rust
pub trait AssetProcessor: Send + Sync {
    fn process(&self, src: &Path, dest: &Path) -> Result<AssetReport, AssetError>;
    fn handles(&self, path: &Path) -> bool;
    fn name(&self) -> &'static str;
}
```

#### `ScssProcessor`

SCSS/SASS processor for compiling stylesheets.

```rust
pub struct ScssProcessor {
    include_paths: Vec<PathBuf>,
    minify: bool,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create a new SCSS processor with default settings |
| `with_include_paths(paths: Vec<PathBuf>) -> Self` | Create processor with include paths for `@import` |
| `with_minify(minify: bool) -> Self` | Enable or disable CSS minification |

#### `StaticCopier`

Static file copier for assets that need no processing.

```rust
pub struct StaticCopier {
    exclude_patterns: Vec<String>,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create a new static file copier |
| `with_exclusions(patterns: Vec<String>) -> Self` | Create copier with exclusion patterns |

#### `AssetReport`

Report of processed assets.

```rust
pub struct AssetReport {
    pub files_processed: usize,
    pub files_skipped: usize,
    pub errors: Vec<String>,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create a new empty report |
| `add_processed(&mut self)` | Add a processed file to the report |
| `add_skipped(&mut self)` | Add a skipped file to the report |
| `add_error(&mut self, error: AssetError)` | Add an error to the report |
| `has_errors(&self) -> bool` | Check if the report has any errors |
| `total_files(&self) -> usize` | Get total files (processed + skipped) |
| `merge(&mut self, other: AssetReport)` | Merge another report into this one |

### `templates`

Template rendering types for Tera-based templates.

#### `TemplateRenderer` Trait

Trait for template rendering backends.

```rust
pub trait TemplateRenderer: Send + Sync {
    fn render(&self, template: &str, context: &TemplateContext) -> Result<String, TemplateError>;
    fn register_template(&mut self, name: &str, content: &str) -> Result<(), TemplateError>;
    fn has_template(&self, name: &str) -> bool;
    fn load_templates(&mut self, dir: &Path) -> Result<(), TemplateError>;
}
```

#### `TeraRenderer`

Tera-based template renderer.

```rust
pub struct TeraRenderer { /* ... */ }
```

##### Methods

| Method | Description |
|--------|-------------|
| `new() -> Result<Self, TemplateError>` | Create a new empty renderer |
| `from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, TemplateError>` | Create renderer and load templates from directory |

#### `TemplateContext`

Context for template rendering containing all available variables.

```rust
pub struct TemplateContext {
    pub page: Option<PageContext>,
    pub section: Option<SectionContext>,
    pub site: SiteContext,
    pub extra: HashMap<String, serde_json::Value>,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new(site: SiteContext) -> Self` | Create a new context with site defaults |
| `with_page(self, page: PageContext) -> Self` | Add page context |
| `with_section(self, section: SectionContext) -> Self` | Add section context |
| `with_extra(self, extra: HashMap<String, JsonValue>) -> Self` | Add extra variables |

#### `PageContext`

Page-specific context for templates.

```rust
pub struct PageContext {
    pub title: String,
    pub description: Option<String>,
    pub path: String,
    pub content: String,
    pub raw_content: String,
    pub date: Option<String>,
    pub draft: bool,
}
```

#### `SectionContext`

Section-specific context for templates.

```rust
pub struct SectionContext {
    pub title: String,
    pub path: String,
    pub pages: Vec<PageContext>,
}
```

#### `SiteContext`

Site-wide context for templates.

```rust
pub struct SiteContext {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub author: Option<String>,
}
```

### `init`

Site initialization types for scaffolding new sites.

#### `InitOptions`

Configuration for site initialization.

```rust
pub struct InitOptions {
    pub name: String,
    pub base_url: String,
    pub force: bool,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new(name: impl Into<String>, base_url: impl Into<String>) -> Self` | Create new options |
| `with_force(self, force: bool) -> Self` | Set force flag |
| `validate(&self) -> std::result::Result<(), InitError>` | Validate options |

#### `InitScaffolder`

Main entry point for creating new sites.

```rust
pub struct InitScaffolder { /* ... */ }
```

##### Methods

| Method | Description |
|--------|-------------|
| `new(options: InitOptions) -> Self` | Create scaffolder with options |
| `scaffold(&self, path: &Path) -> Result<InitReport>` | Create directory structure and files |

#### `InitReport`

Statistics and results from initialization.

```rust
pub struct InitReport {
    pub path: PathBuf,
    pub directories_created: usize,
    pub files_created: usize,
    pub created_dirs: Vec<PathBuf>,   // paths of directories created
    pub created_files: Vec<PathBuf>,  // paths of files created
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `new(path: PathBuf) -> Self` | Create a new report |
| `print_summary(&self)` | Print initialization summary |

#### `DefaultTemplates`

Default template content for new sites.

##### Methods

| Method | Description |
|--------|-------------|
| `base_html() -> &'static str` | Get base.html template |
| `page_html() -> &'static str` | Get page.html template |
| `section_html() -> &'static str` | Get section.html template |
| `main_scss() -> &'static str` | Get main.scss content |
| `site_toml(name: &str, base_url: &str) -> String` | Generate site.toml content |
| `index_md(site_name: &str) -> String` | Generate _index.md content |

## Re-exports

The library re-exports commonly used types:

```rust
// Configuration
pub use config::{BuildConfig, SiteConfig, SiteMeta};

// Content
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page, Section};

// Templates
pub use templates::{
    PageContext, SectionContext, SiteContext, TemplateContext,
    TemplateRenderer, TeraRenderer,
};

// Assets
pub use assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};

// Init
pub use init::{DefaultTemplates, InitOptions, InitReport, InitScaffolder};

// Errors
pub use error::{AssetError, BuildError, ContentError, GeneratorError, InitError, Result, RouteError, TemplateError};
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

### Template Rendering

```rust
use generator::{
    TeraRenderer, TemplateRenderer, TemplateContext,
    SiteContext, PageContext, Result
};

fn render_page() -> Result<()> {
    // Create renderer and load templates
    let mut renderer = TeraRenderer::new()?;
    renderer.register_template("page.html", r#"
        <article>
            <h1>{{ page.title }}</h1>
            {{ page.content | safe }}
        </article>
    "#)?;
    
    // Create context
    let site = SiteContext {
        name: "My Site".to_string(),
        base_url: "https://example.com".to_string(),
        description: None,
        author: None,
    };
    
    let page = PageContext {
        title: "Hello".to_string(),
        description: None,
        path: "/hello/".to_string(),
        content: "<p>World</p>".to_string(),
        raw_content: "World".to_string(),
        date: None,
        draft: false,
    };
    
    let ctx = TemplateContext::new(site).with_page(page);
    
    // Render
    let html = renderer.render("page.html", &ctx)?;
    println!("{}", html);
    
    Ok(())
}
```

### Loading Templates from Directory

```rust
use generator::{TeraRenderer, TemplateRenderer, Result};

fn load_templates() -> Result<()> {
    // Load all .html templates from directory
    let renderer = TeraRenderer::from_dir("templates")?;
    
    // Check if template exists
    if renderer.has_template("base.html") {
        println!("Base template loaded");
    }
    
    Ok(())
}
```

### Template Inheritance

```rust
use generator::{TeraRenderer, TemplateRenderer, TemplateContext, SiteContext, Result};

fn render_with_inheritance() -> Result<()> {
    let mut renderer = TeraRenderer::new()?;
    
    // Base template with blocks
    renderer.register_template("base.html", r#"
        <!DOCTYPE html>
        <html>
        <head>{% block title %}{% endblock %}</head>
        <body>{% block content %}{% endblock %}</body>
        </html>
    "#)?;
    
    // Child template extending base
    renderer.register_template("page.html", r#"
        {% extends "base.html" %}
        {% block title %}{{ page.title }}{% endblock %}
        {% block content %}{{ page.content | safe }}{% endblock %}
    "#)?;
    
    let site = SiteContext {
        name: "My Site".to_string(),
        base_url: "https://example.com".to_string(),
        description: None,
        author: None,
    };
    
    // Render child template
    let ctx = TemplateContext::new(site);
    let html = renderer.render("page.html", &ctx)?;
    
    Ok(())
}
```

### Asset Processing

```rust
use generator::{AssetProcessor, ScssProcessor, StaticCopier, Result};
use std::path::Path;

fn process_assets() -> Result<()> {
    // Compile SCSS to CSS
    let scss_processor = ScssProcessor::with_include_paths(
        vec![Path::new("styles").to_path_buf()]
    ).with_minify(true);
    
    let report = scss_processor.process(
        Path::new("styles/main.scss"),
        Path::new("dist/styles/main.css")
    )?;
    println!("Processed {} SCSS files", report.files_processed);
    
    // Copy static files with exclusions
    let static_copier = StaticCopier::with_exclusions(
        vec!["*.scss".to_string()]
    );
    
    let report = static_copier.process(
        Path::new("static"),
        Path::new("dist/static")
    )?;
    println!("Copied {} static files", report.files_processed);
    
    Ok(())
}
```

## CLI Reference

The `yew-ssg` binary provides four subcommands.

### `yew-ssg build`

Build the static site from content and templates.

```
yew-ssg build [OPTIONS]

Options:
  -d, --dir <PATH>       Root directory (must contain site.toml) [default: .]
  -v, --verbose          Print detailed progress for each build stage
  -q, --quiet            Suppress all output except errors
      --include-drafts   Include pages marked draft = true
      --dry-run          Simulate without writing files
      --clean            Remove output directory before building
  -o, --output <PATH>    Override the output directory from site.toml
  -h, --help             Print help
```

### `yew-ssg clean`

Remove all generated files from the output directory.

```
yew-ssg clean [OPTIONS]

Options:
  -d, --dir <PATH>   Root directory (must contain site.toml) [default: .]
  -h, --help         Print help
```

### `yew-ssg init`

Initialize a new site with a default directory structure.

```
yew-ssg init [OPTIONS] [PATH]

Arguments:
  [PATH]   Directory to initialize [default: .]

Options:
  -n, --name <NAME>       Site name used in templates and site.toml
  -u, --base-url <URL>    Base URL (must start with http:// or https://)
  -f, --force             Initialize even if directory is not empty
  -h, --help              Print help
```

Files created by `init`:

| Path | Description |
|------|-------------|
| `site.toml` | Site configuration |
| `content/_index.md` | Home page content |
| `templates/base.html` | Base HTML layout |
| `templates/page.html` | Single-page template |
| `templates/section.html` | Section/listing template |
| `styles/main.scss` | Starter stylesheet |
| `static/scripts.js` | Placeholder scripts |
| `static/favicon.png` | Placeholder favicon |

### `yew-ssg routes`

List all routes discovered from the content directory without building.

```
yew-ssg routes [OPTIONS]

Options:
  -d, --dir <PATH>   Root directory (must contain site.toml) [default: .]
  -h, --help         Print help
```

Example output:

```
Routes for "My Site"
─────────────────────────────────────────────────────
  [section]  /             _index.md              index.html
  [page]     /about/       about.md               about/index.html
  [section]  /blog/        blog/_index.md         blog/index.html
  [page]     /blog/hello/  blog/hello-world.md    blog/hello/index.html
─────────────────────────────────────────────────────
  Total: 4 routes (2 pages, 2 sections)
```

### Error Hints

When a command fails, the CLI prints an actionable hint alongside the error:

| Error | Hint |
|-------|------|
| `site.toml` not found | Run `yew-ssg init` or use `--dir` |
| No content found | Add `.md` files to `content/`, start with `content/_index.md` |
| Template not found | Check that `templates/` contains `base.html` and `page.html` |

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
