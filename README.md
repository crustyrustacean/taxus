# Yew Static Site Generator (SSG) Template

A Rust-based static site generator built with [Yew](https://yew.rs/), designed for building fast, SEO-friendly websites with the power of WebAssembly.

## Features

- **Static Site Generation**: Pre-rendered HTML pages for optimal performance and SEO
- **Yew Components**: Reusable UI components built with Yew's functional component API
- **Markdown Content**: Write content in Markdown files with TOML frontmatter
- **Content System**: Pages, sections, and draft support with date-based sorting
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
│   │   ├── content/  # Content parsing (Page, Section, Frontmatter)
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

### Build the Static Site

```bash
cargo run
```

This runs the generator to produce static HTML files.

### Development

For development with hot-reloading of the client:

```bash
cd client && trunk serve
```

## Workspace Crates

- **client**: The WebAssembly client application built with Yew
- **common**: Shared components and utilities used by both client and generator
- **generator**: A library and binary for static site generation
  - `generator` (library): Reusable SSG library with configuration, error handling, and content parsing
  - `generator` (binary): CLI tool that pre-renders pages

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
