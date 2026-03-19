# Yew Static Site Generator (SSG) Template

A Rust-based static site generator built with [Yew](https://yew.rs/), designed for building fast, SEO-friendly websites with the power of WebAssembly.

## Features

- **Static Site Generation**: Pre-rendered HTML pages for optimal performance and SEO
- **Yew Components**: Reusable UI components built with Yew's functional component API
- **Markdown Content**: Write content in Markdown files with TOML frontmatter
- **Content System**: Pages, sections, and draft support with date-based sorting
- **Route System**: Automatic route discovery from content directory structure
- **Template System**: Flexible Tera-based templates with inheritance and custom context
- **Asset Processing**: SCSS compilation and static file copying with exclusion patterns
- **Build System**: Unified build pipeline with `SiteBuilder` for orchestrating all stages
- **CLI Interface**: Command-line interface with clap — `build`, `clean`, `init`, and `routes` subcommands with rich help text and actionable error messages
- **SCSS Styling**: Modern styling with SCSS support
- **Multi-crate Workspace**: Organized code structure with separate crates for client, common, and generator
- **Reusable Library**: The generator is available as a library for programmatic use

## Project Structure

```
yew-ssg/
├── client/           # Client-side WebAssembly application
├── common/           # Shared components and code
│   └── src/
│       └── components/  # Reusable page components
├── generator/        # Static site generator
│   ├── src/
│   │   ├── lib.rs    # Library entry point
│   │   ├── config.rs # Configuration types
│   │   ├── error.rs  # Error handling
│   │   ├── assets/   # Asset processing (ScssProcessor, StaticCopier)
│   │   ├── build/    # Build system (SiteBuilder, BuildReport, pipeline)
│   │   ├── content/  # Content parsing (Page, Section, Frontmatter)
│   │   ├── routes/   # Route discovery (RouteDiscovery, RouteRegistry)
│   │   ├── templates/ # Template rendering (TeraRenderer, Context types)
│   │   └── bin/      # CLI binary
│   └── tests/        # Integration tests
├── content/          # Markdown content files
│   └── pages/        # Page content in Markdown
├── static/           # Static assets (images, scripts)
├── styles/           # SCSS stylesheets
└── templates/        # HTML templates
```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- [trunk](https://trunkrs.dev/) for building the client
- A SCSS compiler (e.g., `sass`)

## Getting Started

### Initialize a New Site

Create a new site with the default directory structure:

```bash
# Initialize in the current directory
cargo run -- init

# Initialize in a new directory
cargo run -- init my-site

# Initialize with custom options
cargo run -- init my-site --name "My Site" --base-url "https://example.com"

# Force initialization in a non-empty directory
cargo run -- init my-site --force
```

This creates the following structure:

```
my-site/
├── site.toml               # Site configuration
├── content/
│   └── _index.md           # Home page
├── templates/
│   ├── base.html           # Base HTML layout
│   ├── page.html           # Single-page template
│   └── section.html        # Section/listing template
├── static/
│   ├── scripts.js          # Placeholder scripts file
│   └── favicon.png         # Placeholder favicon
└── styles/
    └── main.scss           # Starter stylesheet
```

### Build the Static Site

```bash
# Build the site
cargo run -- build

# Build with verbose output
cargo run -- build --verbose

# Build silently (errors only)
cargo run -- build --quiet

# Build including draft pages
cargo run -- build --include-drafts

# Dry run (no files written)
cargo run -- build --dry-run

# Clean and rebuild
cargo run -- build --clean

# Build from a different directory
cargo run -- build --dir /path/to/site

# Override the output directory
cargo run -- build --output /tmp/preview
```

### Clean the Output Directory

Remove all generated files without rebuilding:

```bash
cargo run -- clean

# Clean a site in a different directory
cargo run -- clean --dir /path/to/site
```

### Inspect Discovered Routes

List all routes that would be generated from the content directory,
without running a full build. Useful for debugging:

```bash
cargo run -- routes

# Routes for a site in a different directory
cargo run -- routes --dir /path/to/site
```

Example output:

```
Routes for "My Site"
─────────────────────────────────────────────────────
  [section]  /          →  _index.md             →  index.html
  [page]     /about/    →  about.md              →  about/index.html
─────────────────────────────────────────────────────
  Total: 2 routes (1 page, 1 section)
```

### Development

For development with hot-reloading of the client:

```bash
cd client && trunk serve
```

## Workspace Crates

- **client**: The WebAssembly client application built with Yew
- **common**: Shared components and utilities used by both client and generator
- **generator**: A library and binary for static site generation
  - `generator` (library): Reusable SSG library with configuration, error handling, content parsing, and build system
  - `generator` (binary): CLI tool with clap for building sites

## Generator Library

The generator is available as a library for programmatic use:

### Loading Configuration

```rust
use generator::{SiteConfig, Result};

fn main() -> Result<()> {
    // Load configuration from a directory
    let config = SiteConfig::from_dir(".")?;
    
    println!("Building site: {}", config.site.name);
    Ok(())
}
```

### Loading Content

```rust
use generator::{Page, Section, ContentSource, FilesystemContentSource, Result};

fn main() -> Result<()> {
    // Load a page from a file
    let page = Page::from_file("content/about.md")?;
    println!("Title: {}", page.frontmatter.title);
    println!("Is draft: {}", page.is_draft());
    
    // Load a section (e.g., blog)
    let mut section = Section::from_dir("content/blog")?;
    section.sort_by_date();
    
    for page in &section.pages {
        println!("Post: {}", page.frontmatter.title);
    }
    
    // List all content files
    let source = FilesystemContentSource::new("content");
    for file in source.list()? {
        println!("Found: {}", file.display());
    }
    
    Ok(())
}
```

### Route Discovery

```rust
use generator::{RouteDiscovery, RouteRegistry, RouteInfo, RouteKind, Result};

fn main() -> Result<()> {
    // Discover routes from content directory
    let discovery = RouteDiscovery::new("content");
    let registry = discovery.discover()?;
    
    // Query routes
    if let Some(route) = registry.get("/about/") {
        println!("Found route: {:?}", route);
        println!("Content file: {:?}", route.content_file);
        println!("Output file: {:?}", route.output_file);
    }
    
    // Iterate over all pages
    for route in registry.pages() {
        println!("Page: {}", route.path);
    }
    
    // Check route existence
    if registry.contains("/blog/") {
        println!("Blog section exists");
    }
    
    // Count routes
    println!("Total routes: {}", registry.len());
    println!("Pages: {}", registry.pages().count());
    println!("Sections: {}", registry.sections().count());
    
    Ok(())
}
```

### Template Rendering

```rust
use generator::{
    TeraRenderer, TemplateRenderer, TemplateContext,
    SiteContext, PageContext, SectionContext,
    Result
};

fn main() -> Result<()> {
    // Create a template renderer
    let mut renderer = TeraRenderer::new()?;
    
    // Register templates
    renderer.register_template("base.html", r#"
        <html>
            <head><title>{% block title %}{% endblock %}</title></head>
            <body>{% block content %}{% endblock %}</body>
        </html>
    "#)?;
    
    renderer.register_template("page.html", r#"
        {% extends "base.html" %}
        {% block title %}{{ page.title }}{% endblock %}
        {% block content %}{{ page.content | safe }}{% endblock %}
    "#)?;
    
    // Create context
    let site = SiteContext {
        name: "My Site".to_string(),
        base_url: "https://example.com".to_string(),
        description: None,
        author: None,
    };
    
    let page = PageContext {
        title: "Hello World".to_string(),
        description: None,
        path: "/hello/".to_string(),
        content: "<p>Welcome!</p>".to_string(),
        raw_content: "Welcome!".to_string(),
        date: None,
        draft: false,
    };
    
    let ctx = TemplateContext::new(site).with_page(page);
    
    // Render the template
    let html = renderer.render("page.html", &ctx)?;
    println!("{}", html);
    
    Ok(())
}
```

### Asset Processing

```rust
use generator::{
    AssetProcessor, ScssProcessor, StaticCopier,
    Result
};
use std::path::Path;

fn main() -> Result<()> {
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

### Build System

The `SiteBuilder` orchestrates the entire build pipeline:

```rust
use generator::{SiteBuilder, Result};
use std::path::Path;

fn main() -> Result<()> {
    // Build a site from a directory
    let report = SiteBuilder::from_dir(Path::new("."))?
        .verbose(true)
        .include_drafts(false)
        .dry_run(false)
        .build()?;
    
    // Print build summary
    report.print_summary();
    
    println!("Pages rendered: {}", report.pages_rendered);
    println!("Sections rendered: {}", report.sections_rendered);
    println!("Total files: {}", report.total_files());
    println!("Duration: {:.2}s", report.duration.as_secs_f64());
    
    Ok(())
}
```

### Site Initialization

The `InitScaffolder` creates a new site with default structure:

```rust
use generator::{InitOptions, InitScaffolder, Result};
use std::path::Path;

fn main() -> Result<()> {
    // Create options for the new site
    let options = InitOptions::new("My Site", "https://example.com");
    
    // Create the scaffolder
    let scaffolder = InitScaffolder::new(options);
    
    // Scaffold the site
    let report = scaffolder.scaffold(Path::new("my-site"))?;
    
    // Print summary
    report.print_summary();
    
    println!("Directories created: {}", report.directories_created);
    println!("Files created: {}", report.files_created);
    
    Ok(())
}
```

### Configuration

The generator supports configuration via `site.toml`:

```toml
[site]
name = "My Site"
base_url = "https://example.com"
description = "A description"
author = "Author Name"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"
```

### Content Format

Content files use Markdown with TOML frontmatter:

```markdown
+++
title = "My Page"
description = "A brief description"
date = 2024-01-15
template = "custom.html"
draft = false

[extra]
author = "John Doe"
tags = ["rust", "web"]
+++

# Page Content

Your markdown content here.
```

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- [Introduction](docs/src/introduction.md)
- [Getting Started](docs/src/getting-started.md)
- [Configuration](docs/src/configuration.md)
- [Content](docs/src/content.md)
- [API Reference](docs/src/api-reference.md)

Build the documentation with [mdBook](https://rust-lang.github.io/mdBook/):

```bash
cd docs && mdbook serve
```

## License

This project is licensed under the MIT License - see the [License.txt](License.txt) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [Yew](https://yew.rs/) - Rust framework for creating reliable and efficient web applications
- [Trunk](https://trunkrs.dev/) - WASM web application bundler for Rust
