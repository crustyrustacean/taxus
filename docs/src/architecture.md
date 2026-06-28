# Architecture

This page provides a technical overview of Taxus's architecture, including the workspace structure, module organization, build pipeline, and data flow.

## Workspace Structure

Taxus is organized as a multi-crate Cargo workspace with three crates:

```
taxus/
├── taxus-client/    # WASM hydration client
├── taxus-common/    # Shared Yew components
└── taxus-generator/ # SSG library and CLI binary
```

### Crate Responsibilities

| Crate | Role | Output |
|-------|------|--------|
| `taxus-common` | Shared Yew components used by both SSR (generator) and hydration (client) | Library |
| `taxus-generator` | Static site generation: config, content parsing, route discovery, Tera rendering, asset processing | Library (`taxus_lib`) + Binary (`taxus`) |
| `taxus-client` | Browser-side WASM that finds island mount points and hydrates them | WASM bundle (embedded in generator binary at compile time) |

### Data Flow Between Crates

```
┌─────────────┐     SSR at build time      ┌─────────────┐
│   common    │ ──────────────────────────▶│  generator  │
│ (components)│                            │  (CLI/lib)  │
└─────────────┘                            └──────┬──────┘
      │                                           │
      │              Build output                 │
      │         (HTML + props JSON)               │
      │                                           ▼
      │                                    ┌─────────────┐
      │     Hydration at runtime           │   dist/     │
      │ ──────────────────────────────────▶│  (output)   │
      │                                    └─────────────┘
      │                                           │
      ▼                                           ▼
┌─────────────┐                            ┌─────────────┐
│   client    │ ◀─── WASM loads in browser ─│   Browser   │
│   (WASM)    │                            │             │
└─────────────┘                            └─────────────┘
```

## Generator Module Map

The `generator` crate is organized into modules that each own a specific domain:

| Module | Types | Responsibility |
|--------|-------|----------------|
| `config` | `SiteConfig`, `SiteMeta`, `BuildConfig`, `FeedConfig` | Load and validate `site.toml` configuration |
| `content` | `Page`, `Section`, `Frontmatter`, `ContentSource` | Parse Markdown files with TOML frontmatter |
| `routes` | `RouteDiscovery`, `RouteRegistry`, `RouteInfo`, `RouteKind` | Map content files to URL paths |
| `templates` | `TeraRenderer`, `TemplateContext`, `PageContext`, `SectionContext`, `SiteContext` | Render HTML with Tera templates |
| `build` | `SiteBuilder`, `BuildReport`, `ProcessedPage`, `RenderedPage` | Orchestrate the build pipeline (including WASM client writing) |
| `assets` | `ScssProcessor`, `StaticCopier`, `AssetReport` | Compile SCSS, copy static files |
| `feed` | `FeedGenerator`, `FeedEntry`, `FeedConfig` | Generate RSS/Atom feeds |
| `init` | `InitScaffolder`, `InitOptions`, `InitReport` | Scaffold new site directories |
| `serve` | `DevServer`, `FileWatcher`, WebSocket live reload | Development server with hot reload |
| `error` | `GeneratorError` + domain sub-errors | Error handling hierarchy |
| `tracing` | `init()`, `init_with_level()` | Structured logging setup |

### Module Dependencies

```
                ┌──────────┐
                │  config  │
                └────┬─────┘
                     │
         ┌───────────┼───────────┐
         ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐
    │ content │ │  routes │ │  error  │
    └────┬────┘ └────┬────┘ └─────────┘
         │           │
         └─────┬─────┘
               ▼
         ┌───────────┐
         │ templates │
         └─────┬─────┘
               │
         ┌─────┼─────┐
         ▼     ▼     ▼
    ┌────────┐ ┌───────┐ ┌──────┐
    │ assets │ │ build │ │ feed │
    └────────┘ └───┬───┘ └──────┘
                   │
         ┌─────────┼─────────┐
         ▼         ▼         ▼
    ┌────────┐ ┌────────┐ ┌───────┐
    │  init  │ │ serve  │ │tracing│
    └────────┘ └────────┘ └───────┘
```

## Build Pipeline

The `SiteBuilder` orchestrates a 15-stage build pipeline:

```
┌─────────────────────────────────────────────────────────────────┐
│                      SiteBuilder.build()                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [1/13] Discover routes                                          │
│          └──▶ Walk content/ directory                            │
│          └──▶ Create RouteRegistry (path → content file mapping) │
│                                                                  │
│  [2/13] Load templates                                           │
│          └──▶ Read templates/**/*.html                           │
│          └──▶ Register with Tera (inheritance, island() fn)      │
│                                                                  │
│  [3/13] Process content                                          │
│          └──▶ Parse frontmatter (TOML)                           │
│          └──▶ Convert Markdown → HTML                            │
│          └──▶ Resolve internal links (@/file.md)                 │
│          └──▶ Produce ProcessedPage for each route               │
│                                                                  │
│  [4/13] Copy co-located assets                                   │
│          └──▶ Non-.md files in content/ → dist/                  │
│          └──▶ Preserve directory structure                       │
│                                                                  │
│  [5/13] Render pages                                             │
│          └──▶ Apply Tera templates to ProcessedPage              │
│          └──▶ Handle pagination for sections                     │
│          └──▶ Produce RenderedPage (final HTML)                  │
│                                                                  │
│  [6/13] Generate robots.txt                                      │
│          └──▶ If no static/robots.txt exists                     │
│          └──▶ Write default with sitemap reference               │
│                                                                  │
│  [7/13] Generate sitemap.xml                                     │
│          └──▶ List all routes with lastmod dates                 │
│          └──▶ Assign priorities (home: 1.0, sections: 0.8, etc)  │
│                                                                  │
│  [8/13] Generate 404.html                                        │
│          └──▶ Render 404 template if present                     │
│                                                                  │
│  [9/13] Build and render taxonomy pages                          │
│          └──▶ Extract tags, categories, series from pages        │
│          └──▶ Generate /tags/, /tags/slug/, etc                  │
│                                                                  │
│  [10/13] Generate feeds                                          │
│          └──▶ RSS 2.0 (rss_enabled)                              │
│          └──▶ Atom (atom_enabled)                                │
│                                                                  │
│  [11/13] Process assets                                          │
│          └──▶ Compile SCSS → CSS (styles/**/*.scss)              │
│          └──▶ Copy static/ files to dist/static/                 │
│                                                                  │
│  [12/13] Generate search index                                   │
│          └──▶ Build TF-IDF index from page content               │
│          └──▶ Write dist/search_index.bin                        │
│                                                                  │
│  [13/13] Write WASM client                                       │
│          └──▶ Write embedded client.js to dist/wasm/              │
│          └──▶ Write embedded client_bg.wasm to dist/wasm/         │
│                                                                  │
│  [15/15] Write output                                            │
│          └──▶ Write RenderedPage HTML files                      │
│          └──▶ Write taxonomy pages                               │
│          └──▶ Write feed XML files                               │
│          └──▶ Write alias redirects (HTML meta refresh)          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Key Types in the Pipeline

### RouteRegistry

The first major data structure created. Maps URL paths to content files:

```rust
RouteRegistry {
    "/":           RouteInfo { kind: Section, content_file: "_index.md", ... },
    "/about/":     RouteInfo { kind: Page, content_file: "about.md", ... },
    "/blog/":      RouteInfo { kind: Section, content_file: "blog/_index.md", ... },
    "/blog/post/": RouteInfo { kind: Page, content_file: "blog/post.md", ... },
}
```

### ProcessedPage

The intermediate representation after content parsing:

```rust
ProcessedPage {
    route: RouteInfo,
    page: Page {
        frontmatter: Frontmatter { title, date, tags, ... },
        content: "<p>Rendered HTML from markdown</p>",
        raw_content: "Original markdown text",
    },
}
```

### RenderedPage

The final output after template rendering:

```rust
RenderedPage {
    route: RouteInfo,
    html: "<!DOCTYPE html><html>...</html>",
}
```

## Islands Architecture

The generator pre-renders Yew components at build time:

### Build-Time (SSR)

1. Tera encounters `{{ island(component="Counter", initial=5) | safe }}` in a template
2. The `island()` Tera function calls Yew SSR to render the component
3. Output is wrapped in a mount point with props as JSON:

```html
<div data-island="Counter" data-props='{"initial":5}'>
  <!-- Pre-rendered HTML from Yew SSR -->
  <div class="counter"><span>5</span><button>+</button></div>
</div>
```

### Browser-Time (Hydration)

1. Page loads immediately with pre-rendered HTML (no JavaScript required for initial render)
2. WASM bundle (embedded in the generator binary at compile time) loads asynchronously from `/wasm/`
3. Client finds all `[data-island]` elements
4. For each: deserialize `data-props`, call `yew::Renderer::hydrate()`

See [Islands Architecture](./islands.md) for the full guide.

## Feature Flags

| Feature | Default | Effect |
|---------|---------|--------|
| `lang-rust` | on | Rust syntax highlighting via tree-sitter |

Islands (Yew SSR + WASM hydration) are always enabled. The WASM client
(`taxus-client`) is compiled by `build.rs` at Cargo build time and embedded
in the binary via `include_bytes!`; at site build time it is written to
`dist/wasm/`. No feature flag is required.

```bash
# Build the site (islands are always compiled in)
cargo run -- build --dir my-site
```
