# Architecture

This page provides a technical overview of Taxus's architecture, including the workspace structure, module organization, build pipeline, and data flow.

## Workspace Structure

Taxus is organized as a multi-crate Cargo workspace with five crates:

```
taxus/
├── taxus-client/    # WASM hydration client
├── taxus-common/    # Shared Yew components
├── taxus-domain/    # Pure data model: the Site Tree and its derivations
├── taxus-generator/ # SSG library and CLI binary
└── xtask/           # Workspace task runner (cargo xtask)
```

### Crate Responsibilities

| Crate | Role | Output |
|-------|------|--------|
| `taxus-common` | Shared Yew components used by both SSR (generator) and hydration (client) | Library |
| `taxus-domain` | The Site Tree (`SiteTree`, `SectionNode`, `PageNode`), identity types (`Slug`, `NodePath`, `UrlPath`), typed frontmatter, and pure derivations over the tree. No I/O. | Library |
| `taxus-generator` | Static site generation: config, content parsing, route discovery, Tera rendering, asset processing | Library (`taxus_lib`) + Binary (`taxus`) |
| `taxus-client` | Browser-side WASM that finds island mount points and hydrates them | WASM bundle (embedded in generator binary at compile time) |
| `xtask` | Workspace task runner wrapping common developer workflows (build, test, lint, release, …) | Binary (`cargo xtask`) |

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
| `config` | `SiteConfig`, `SiteMeta`, `BuildConfig`, `FeedConfig`, `ImageConfig` | Load and validate `site.toml` configuration |
| `content` | `Page`, `Frontmatter`, `ContentSource`, `TaxonomyMap` | Parse Markdown files with TOML frontmatter |
| `routes` | `RouteDiscovery`, `RouteRegistry`, `RouteInfo`, `RouteKind` | Map content files to URL paths |
| `templates` | `TeraRenderer`, `TemplateContext`, `PageContext`, `SectionContext`, `SiteContext` | Render HTML with Tera templates |
| `build` | `SiteBuilder`, `BuildReport`, `ProcessedPage`, `RenderedPage` | Orchestrate the build pipeline (including WASM client writing) |
| `assets` | `ScssProcessor`, `StaticCopier`, `AssetReport` | Compile SCSS, copy static files |
| `images` | `ImageProcessor`, `ImageRegistry`, `ProcessedImage`, `render_picture` | Hero image processing: responsive variants, WebP conversion, `<picture>`/srcset |
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
│  [1/15] Discover routes                                          │
│          └──▶ Walk content/ once, build the SiteTree             │
│          └──▶ Derive RouteRegistry from the tree (from_tree)     │
│                                                                  │
│  [2/15] Load templates                                           │
│          └──▶ Read templates/**/*.html                           │
│          └──▶ Register with Tera (inheritance, island() fn)      │
│                                                                  │
│  [3/15] Process content                                          │
│          └──▶ Parse frontmatter (TOML)                           │
│          └──▶ Convert Markdown → HTML                            │
│          └──▶ Resolve internal links (@/file.md)                 │
│          └──▶ Produce ProcessedPage for each route               │
│                                                                  │
│  [4/15] Process images                                           │
│          └──▶ Generate responsive variants for hero images       │
│          └──▶ Convert to WebP and build <picture> srcset         │
│                                                                  │
│  [5/15] Copy co-located assets                                   │
│          └──▶ Non-.md files in content/ → dist/                  │
│          └──▶ Preserve directory structure                       │
│                                                                  │
│  [6/15] Render pages                                             │
│          └──▶ Apply Tera templates to ProcessedPage              │
│          └──▶ Handle pagination for sections                     │
│          └──▶ Produce RenderedPage (final HTML)                  │
│                                                                  │
│  [7/15] Generate robots.txt                                      │
│          └──▶ If no static/robots.txt exists                     │
│          └──▶ Write default with sitemap reference               │
│                                                                  │
│  [8/15] Generate sitemap.xml                                     │
│          └──▶ List all routes with lastmod dates                 │
│          └──▶ Assign priorities (home: 1.0, sections: 0.8, etc)  │
│                                                                  │
│  [9/15] Generate 404.html                                        │
│          └──▶ Render 404 template if present                     │
│                                                                  │
│  [10/15] Build and render taxonomy pages                         │
│          └──▶ Extract tags, categories, series from pages        │
│          └──▶ Generate /tags/, /tags/slug/, etc                  │
│                                                                  │
│  [11/15] Generate feeds                                          │
│          └──▶ RSS 2.0 (rss_enabled)                              │
│          └──▶ Atom (atom_enabled)                                │
│                                                                  │
│  [12/15] Process assets                                          │
│          └──▶ Compile SCSS → CSS (styles/**/*.scss)              │
│          └──▶ Copy static/ files to dist/static/                 │
│                                                                  │
│  [13/15] Generate search index                                   │
│          └──▶ Build TF-IDF index from page content               │
│          └──▶ Write dist/search_index.bin                        │
│                                                                  │
│  [14/15] Write WASM client                                       │
│          └──▶ Write embedded client.js to dist/wasm/             │
│          └──▶ Write embedded client_bg.wasm to dist/wasm/        │
│                                                                  │
│  [15/15] Write output                                            │
│          └──▶ Write RenderedPage HTML files                      │
│          └──▶ Write taxonomy pages                               │
│          └──▶ Write feed XML files                               │
│          └──▶ Write alias redirects (HTML meta refresh)          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Model

The build has one source of truth: the **Site Tree**, defined in
`taxus-domain` and built once, at the start of `SiteBuilder::build()`, by
`RouteDiscovery::discover_tree()`. The tree is immutable after that; every
later stage only queries it. See [Content Model](./content-model.md) for the
ideas behind it — this section is the map of the types.

### Node types

```rust
SiteTree { root: SectionNode }

SectionNode {                       // a directory
    path: NodePath,                 // membership path, e.g. ["blog"]; root is []
    content_file: Option<PathBuf>,  // its _index.md, or None if the directory has none
    meta: Frontmatter,              // from _index.md (defaults when absent)
    body: Option<String>,
    pages: Vec<PageNode>,           // direct children only, sorted by slug
    subsections: Vec<SectionNode>,  // direct children only, sorted by slug
}

PageNode {                          // a document
    path: NodePath,                 // e.g. ["blog", "my-post"]
    content_file: PathBuf,          // e.g. "blog/2026-04-06-my-post.md" (storage, not identity)
    meta: Frontmatter,
    body: String,
}
```

A `NodePath` is a list of `Slug`s. The generator computes it: directory
segments are slugified, a page's last segment is its frontmatter `slug`
(verbatim) or its slugified, date-prefix-stripped file stem. The domain
does not slugify; a `Slug` only has to be a usable path segment. A node's
address is derived from its path in exactly one place,
`UrlPath::from_node_path` (`/blog/my-post/`; the root is `/`).

`SiteTreeBuilder` assembles the tree from flat `add_page` / `add_section`
calls, auto-creates intermediate sections that have no `_index.md`, and
rejects two nodes at the same path (`TreeError::Duplicate`, or `Collision`
for a page and a section) — those surface as the same `Duplicate route`
error the route registry has always reported.

### Navigation

Queries on `SiteTree` (all cheap, all read-only):

| Method | Returns |
|--------|---------|
| `get_section(&NodePath)` | the section at that path |
| `get_page(&NodePath)` | the page at that path |
| `iter_pages()` | every page in the site, depth-first, drafts included |

### Derivations

Everything computed *from* the tree lives in `taxus_domain::derivation` as
pure functions: `(tree or section) -> Vec<&PageNode>`, never stored back.

| Function | Meaning |
|----------|---------|
| `documents(&SiteTree)` | every rendered document (`Node::Page` or `Node::Section` with an `_index.md`) in tree order: section index, pages by slug, subsections by slug, recursively. The canonical iteration order of a site |
| `group_by_terms(&SiteTree, include_drafts, terms_of)` | documents grouped by the terms a selector reads from frontmatter — the index behind tags, categories and series; terms sorted by name, documents in tree order |
| `descendant_pages(&SectionNode)` | every page in a section's subtree, depth-first |
| `recent(&SiteTree, include_drafts)` | all pages, newest first, undated last |
| `aggregate(&SectionNode, &SiteTree, &[NodePath])` | a section's pages merged with the pages of donor sections (`pages_from`), deduplicated, sorted by the receiver's `sort_by` |
| `tree::sort_pages(&mut [&PageNode], SortBy)` | the ordering used by listings |

Draft filtering is always the caller's decision (the generator knows
whether `--drafts` was passed), so derivations take it as a parameter.

### What is wired today

- **Routes** are derived: `RouteRegistry::from_tree` produces one
  `RouteInfo` per document from `documents`, in that order, and the
  registry iterates in registration order. The registry is a projection
  of the tree, not a second source, and every stage that walks it sees
  documents in tree order.
- **Section listings** (`build/pipeline/pages.rs`) look the section up in
  the tree and list `descendant_pages` of that node — every page under the
  section, as the previous URL-prefix scan did — sorted by the section's
  `sort_by`. `sort_by = "weight"` now works and `weight` is exposed on
  `page` in templates. Date and title ordering keep their historical
  comparators (undated pages first, byte-order titles) until they are
  moved to the domain's in a dedicated change.
- **Taxonomies** (`build/pipeline/taxonomy.rs`) are `group_by_terms`
  over the tree for tags, categories and series. **Feeds** and the
  **sitemap** take their membership from `documents` and join the
  rendered HTML by content file. All three are deterministic: where trunk
  ordered tied sort keys by `HashMap` iteration, they now follow tree
  order.
- Pagination slices the section listing above. The search index still
  walks the processed pages, in registry (tree) order.
- There is one frontmatter parser, `Page::from_str`; `content::Section`
  and its duplicate were removed once the tree replaced them.

The rule for new code: **the tree is immutable after `build()` starts; a
stage that needs structure queries the tree, and a stage that needs a new
projection adds a pure function to `taxus_domain::derivation`.**

## Key Types in the Pipeline

### RouteRegistry

Derived from the Site Tree. Maps URL paths to content files:

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
