# Yew Static Site Generator (SSG) Template

A Rust-based static site generator built with [Yew](https://yew.rs/), designed for building fast, SEO-friendly websites with the power of WebAssembly.

## Features

- **Static Site Generation**: Pre-rendered HTML pages for optimal performance and SEO
- **Islands Architecture**: Interactive Yew WASM components embedded within Tera-rendered static pages — server-side pre-rendered at build time, hydrated in the browser
- **Yew Components**: Reusable UI components built with Yew's functional component API, shared between the SSG and the WASM client
- **Markdown Content**: Write content in Markdown files with TOML frontmatter
- **Content System**: Pages, sections, and draft support with date-based sorting
- **Blog Features**: Summary/excerpt extraction, reading time, word count, and custom slugs
- **Pagination**: Split large content collections across multiple pages with configurable sorting
- **RSS/Atom Feeds**: Automatic feed generation for content syndication
- **Taxonomies**: Tags, categories, and series for content organization with automatic taxonomy pages
- **Route System**: Automatic route discovery from content directory structure
- **Template System**: Flexible Tera-based templates with inheritance and custom context; `{{ island(component="...", ...) | safe }}` function for embedding islands
- **Asset Processing**: SCSS compilation and static file copying with exclusion patterns
- **Build System**: Unified build pipeline with `SiteBuilder` for orchestrating all stages
- **CLI Interface**: Command-line interface with clap — `build`, `clean`, `init`, `routes`, and `serve` subcommands with rich help text and actionable error messages
- **Development Server**: Hot-reloading local server with WebSocket-based live reload and file watching
- **Structured Logging**: Integrated `tracing` crate for structured, filterable logs with `--verbose` and `--quiet` CLI flags
- **SCSS Styling**: Modern styling with SCSS support
- **Multi-crate Workspace**: Organized code structure with separate crates for client, common, and generator
- **Reusable Library**: The generator is available as a library for programmatic use

## Project Structure

```
yew-ssg/
├── client/                 # WASM hydration client (built by Trunk)
│   ├── src/main.rs         # Island hydration bootstrap (#[wasm_bindgen(start)])
│   ├── index.html          # Trunk entry point
│   └── Trunk.toml          # Trunk build configuration
├── common/                 # Shared Yew components (generator SSR + WASM client)
│   └── src/
│       └── components/
│           └── counter.rs  # Example island component (serializable props)
├── generator/              # Static site generator
│   ├── src/
│   │   ├── lib.rs          # Library entry point and public API re-exports
│   │   ├── config.rs       # SiteConfig, SiteMeta, BuildConfig
│   │   ├── error.rs        # GeneratorError + domain sub-errors
│   │   ├── assets/         # ScssProcessor, StaticCopier
│   │   ├── build/          # SiteBuilder, pipeline stages, BuildReport
│   │   ├── content/        # Page, Section, Frontmatter, ContentSource
│   │   ├── routes/         # RouteDiscovery, RouteRegistry, RouteInfo
│   │   ├── templates/      # TemplateRenderer trait, TeraRenderer, TemplateContext
│   │   ├── init/           # InitScaffolder, InitOptions, InitReport
│   │   ├── serve/          # DevServer, file watcher, WebSocket live reload
│   │   └── bin/main.rs     # CLI binary (build, clean, init, routes, serve subcommands)
│   └── tests/              # Integration tests + fixture sites
└── docs/                   # mdBook documentation
```

## Feature Flags

The generator supports a Cargo feature flag to control whether Yew SSR island support is compiled in:

| Feature | Default | Description |
|---|---|---|
| `islands` | off | Enables Yew SSR + `tokio` + `common` crate compilation; activates `island()` Tera function |

Without the `islands` feature, the generator is a plain Tera + Markdown SSG. The `island()` Tera function is still recognized in templates but produces empty output — no Yew or WASM dependency is included.

```bash
# Plain SSG — no Yew, fastest compile, smallest binary
cargo run -- build --dir my-site

# Islands SSG — Yew SSR pre-renders components at build time
cargo run --features islands -- build --dir my-site
```

Using `just`:

```bash
just build my-site           # plain SSG
just build-islands my-site   # SSG + WASM islands
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

# Initialize with islands support (includes WASM hydration script in templates)
cargo run -- init my-site --islands

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

# Build with debug logging via RUST_LOG
RUST_LOG=debug cargo run -- build

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

### Development Server

Start a local development server with hot reloading:

```bash
# Start server on default port (3000)
cargo run -- serve

# Start with custom port
cargo run -- serve --port 8080

# Start and open browser automatically
cargo run -- serve --open

# Serve from a different directory
cargo run -- serve --site-dir /path/to/site

# Combined options
cargo run -- serve --port 8080 --open --site-dir my-site
```

The development server features:
- **Hot Reloading**: Automatically rebuilds and refreshes the browser when content, templates, styles, or static files change
- **WebSocket Live Reload**: Instant browser refresh via WebSocket connection
- **Error Overlay**: Build errors are displayed directly in the browser
- **Graceful Shutdown**: Press Ctrl+C to stop the server cleanly

Short options are available:
- `-p` for `--port`
- `-s` for `--site-dir`
- `-o` for `--open`

### Logging and Diagnostics

The generator uses structured logging via the `tracing` crate. Control log output with CLI flags or the `RUST_LOG` environment variable:

```bash
# Default: info level (shows build progress and summary)
cargo run -- build

# Verbose: debug level (detailed stage-by-stage output)
cargo run -- build --verbose

# Quiet: error level (only errors are shown)
cargo run -- build --quiet

# Custom: use RUST_LOG for fine-grained control
RUST_LOG=yew_ssg_lib=trace cargo run -- build
RUST_LOG=debug cargo run -- build  # all crates at debug level
```

Log levels:
- `error`: Build failures only
- `warn`: Warnings (e.g., missing optional files)
- `info`: Build progress and summary (default)
- `debug`: Detailed stage information
- `trace`: Very verbose internal diagnostics

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

### Build the WASM Client

After building the static site, compile the Yew WASM hydration bundle with Trunk:

```bash
# Compile and write into the site's dist/wasm/ directory
cd client && trunk build --release --dist ../my-site/dist/wasm
```

Then serve the complete output:

```bash
cd my-site && miniserve dist
# or: python -m http.server 8000 -d dist
```

### Development

For development with Trunk's dev server:

```bash
cd client && trunk serve
```

## Workspace Crates

- **client**: WASM hydration client — finds island mount points in the DOM, deserializes their props, calls `yew::Renderer::hydrate()` on each
- **common**: Shared Yew components — compiled into both the `generator` binary (for SSR) and the `client` WASM bundle (for hydration)
- **generator**: Static site generator library and binary
  - Library (`yew_ssg_lib`): Configuration, content parsing, route discovery, Tera rendering, asset processing, init scaffolding, dev server
  - Binary (`yew-ssg`): CLI with `build`, `clean`, `init`, `routes`, and `serve` subcommands

## Islands Architecture

Yew SSG implements the **Islands Architecture**: Tera renders the static "sea" of HTML; Yew components are pre-rendered server-side into "islands" and then hydrated by the WASM client in the browser.

### How It Works

```
BUILD TIME (generator)                BROWSER
─────────────────────                 ───────
1. Tera renders page shell            4. HTML is immediately visible (no JS needed)
2. island() function calls Yew SSR    5. WASM loads asynchronously
3. SSR HTML + props JSON emitted      6. Yew hydrates mount points → interactive
```

### Embedding an Island in a Template

Use the `island()` Tera function in any `.html` template:

```html
{% block content %}
  {{ page.content | safe }}

  {{ island(component="Counter", initial=5) | safe }}
{% endblock %}
```

The generator renders the component server-side at build time. The output in the static HTML looks like:

```html
<div data-island="Counter" data-props='{"initial":5}'>
  <!-- Pre-rendered by Yew SSR: -->
  <div class="counter"><span>5</span><button>+</button></div>
</div>
```

### Writing an Island Component

Island components live in `common/src/components/`. Their props must be serializable:

```rust
// common/src/components/counter.rs
use serde::{Deserialize, Serialize};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone, Serialize, Deserialize)]
pub struct CounterProps {
    #[prop_or_default]
    pub initial: i32,
}

#[function_component(Counter)]
pub fn counter(props: &CounterProps) -> Html {
    let count = use_state(|| props.initial);
    let on_click = {
        let count = count.clone();
        Callback::from(move |_| count.set(*count + 1))
    };
    html! {
        <div class="counter">
            <span>{ *count }</span>
            <button onclick={on_click}>{ "+" }</button>
        </div>
    }
}
```

### Registering a New Island

Two registries must be updated in sync:

1. **Generator SSR registry** — in `generator/src/templates/renderer.rs`, add a match arm to the `island()` Tera function
2. **Client hydration registry** — in `client/src/main.rs`, add a match arm to `hydrate_island()`

### Two-Tier Interactivity

| Tier | Technology | When to use |
|---|---|---|
| 1 — General | `static/scripts.js` | DOM manipulation, toggles, analytics, lightweight events |
| 2 — Performance | Yew WASM island | Heavy computation, complex reactive state, data-intensive UI |

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
        summary: String::new(),
        word_count: 1,
        reading_time: 1,
        tags: vec![],
        categories: vec![],
        series: None,
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

### Pagination

Enable pagination in a section's `_index.md`:

```markdown
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
paginate_template = "section.html"
+++
```

Pagination fields:
- `sort_by`: Sort order (`date`, `weight`, or `title`)
- `paginate_by`: Items per page
- `paginate_template`: Template for pagination pages (optional)

In templates, access pagination via `section.pagination`:

```html
{% if section.pagination %}
<nav class="pagination">
  {% if section.pagination.prev %}
  <a href="{{ section.pagination.prev }}">← Previous</a>
  {% endif %}
  <span>Page {{ section.pagination.current }} of {{ section.pagination.total }}</span>
  {% if section.pagination.next %}
  <a href="{{ section.pagination.next }}">Next →</a>
  {% endif %}
</nav>
{% endif %}
```

Pagination URLs:
- First page: `/blog/`
- Subsequent pages: `/blog/page/2/`, `/blog/page/3/`, etc.

### RSS/Atom Feeds

Configure feeds in `site.toml`:

```toml
[feed]
rss_enabled = true
atom_enabled = true
limit = 20
full_content = false
title = "My Site Feed"
rss_path = "rss.xml"
atom_path = "atom.xml"
```

Feed configuration:
- `rss_enabled`: Generate RSS 2.0 feed (default: true)
- `atom_enabled`: Generate Atom feed (default: true)
- `limit`: Maximum entries in feed (default: 20)
- `full_content`: Include full content vs summary (default: false)
- `title`: Custom feed title (defaults to site name)
- `rss_path`: RSS feed path (default: `rss.xml`)
- `atom_path`: Atom feed path (default: `atom.xml`)

Feeds are automatically generated during build and placed in the output directory.

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
