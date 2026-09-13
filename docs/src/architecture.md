# Architecture

This page is the map of the code: the crates, the modules, and the build
pipeline stage by stage. The ideas behind the pipeline are in the
[Theory](./theory/overview.md) chapters; the words used here are defined
in the [Glossary](./theory/glossary.md).

## Workspace

Taxus is a Cargo workspace of five crates.

```text
taxus/
├── taxus-domain/    # the Site Tree, identity types, frontmatter, derivations (no I/O)
├── taxus-generator/ # the build: parse, analyse, emit; the `taxus` CLI
├── taxus-common/    # Yew island components and the search index, shared by both sides
├── taxus-client/    # the browser-side WASM that hydrates islands (embedded in the binary)
└── xtask/           # developer task runner (`cargo xtask`)
```

| Crate | Phase | Role | Output |
|-------|-------|------|--------|
| `taxus-domain` | parse, analyse | Defines what a site is (`SiteTree`, `SectionNode`, `PageNode`), how nodes are named (`Slug`, `NodePath`, `UrlPath`), the frontmatter schema, and the pure derivations. Reads no files. | library |
| `taxus-generator` | all three | Fills the tree from disk, calls the derivations, renders Markdown and templates, processes images and assets, writes the output. | library `taxus_lib` and binary `taxus` |
| `taxus-common` | emit | Island components (`Counter`, `SearchBox`) rendered at build time and hydrated in the browser; the search index format. | library |
| `taxus-client` | emit | Finds `[data-island]` mount points in the page and hydrates them; fetches the search index on demand. Compiled to WASM by the generator's build script and embedded with `include_bytes!`. | WASM bundle written to `dist/wasm/` |
| `xtask` | none | `cargo xtask build`, `test`, `lint`, `book`, `release`, and the rest. | binary |

How the crates depend on each other:

```text
 taxus-domain ◄── taxus-generator ──► taxus-common ◄── taxus-client
   (model)          (the build)        (islands,        (hydration,
                         │              search)           compiled to
                         │ build.rs compiles taxus-client   wasm32)
                         │ and embeds client.js + client_bg.wasm
                         ▼
                       dist/
```

## The three phases

Every stage below belongs to one of three phases. **Parse** turns the
filesystem into the Site Tree. **Analyse** computes derivations over the
tree. **Emit** turns the tree, the derivations and the other inputs into
files. The tree is built in stage 1 and is immutable from then on; every
later stage holds it as `&SiteTree`.

## The build pipeline, stage by stage

`SiteBuilder::build` in `taxus-generator/src/build/builder.rs` runs
fifteen stages and logs one line per stage. The list below uses the same
numbers and the same wording as the log. For each stage: which phase it
is, what it reads, what it produces.

**[1/15] Discovering routes** (parse). Reads every `.md` under the content
directory. Produces the `SiteTree` (`RouteDiscovery::discover_tree`) and,
from it, the `RouteRegistry` (`RouteRegistry::from_tree`): one route per
document in tree order. Frontmatter is parsed here, slugs are computed
here, and duplicate paths fail here. An empty registry ends the build with
`NoContent`.

**[2/15] Loading templates** (emit setup). Reads `templates/**/*.html`.
Produces a `TeraRenderer` with the `island()`, `get_section()` and
`get_page()` functions and the `slugify` and `date` filters registered.

**[3/15] Processing content** (emit). Reads the tree — the same parse
discovery already made; no file is read from disk a second time — the
registry and the config. Produces one `ProcessedPage` per document in
canonical tree order: internal links resolved against the registry,
Markdown rendered to HTML, headings collected into a table of contents,
code blocks highlighted. Drafts are dropped here unless
`--include-drafts` was passed; skipped documents are counted as they
happen, not inferred by subtraction (#55).

**[4/15] Processing images** (emit). Reads the processed pages and
`[images]` config. For every page with `hero_image`, produces resized
variants under `dist/images/` (or their paths, in dry run) and attaches a
`ProcessedImage` to the page.

**[5/15] Copying co-located assets** (emit). Reads the content directory.
Copies every non-`.md` file to the same relative path under the output
directory.

**[6/15] Rendering pages** (analyse and emit). Reads the processed pages,
the tree and the templates. First fills the tree functions' lookup with
every section and page as a context (`site_lookup`). Then, for each
processed page, builds a `TemplateContext` and runs its template. A
section's `section.pages` is the derivation `aggregate` sorted by
`sort_pages`; a section with `paginate_by` is rendered once per slice.
Produces one `RenderedPage` per output file.

**[7/15] Generating robots.txt** (emit). Reads the config and checks for
`static/robots.txt`. If none exists, produces a default `robots.txt`
pointing at the sitemap and writes it.

**[8/15] Generating 404.html** (emit). Reads the templates. If
`404.html` exists, renders it with the site context and writes it.

**[9/15] Building taxonomy pages** (analyse and emit). Reads the tree,
the processed pages and the templates. `derivation::group_by_terms` fills
a `TaxonomyMap` for tags, categories and series. For each kind whose
templates exist, renders `/tags/` and `/tags/<term>/` and the same for
the other two. Produces `RenderedTaxonomy` values; they are written in
stage 15.

**[10/15] Generating sitemap.xml** (analyse and emit). Reads the rendered
pages, the taxonomy pages and the base URL — the final outputs, not the
tree (#47). The URL set is every `RenderedPage` (which includes the
pagination pages stage 6 emits) plus every taxonomy list and term page;
each entry's date is joined from its processed page by content file.
`<loc>` is XML-escaped. Alias redirects are excluded deliberately: a
redirect is not content, and each targets a URL already in the set.

**[11/15] Generating feeds** (analyse and emit). Reads the tree, the
processed pages and `[feed]` config. `feed_pages` selects dated, non-draft
pages newest first (from `derivation::recent`), scoped by `sections` if
set. Produces the RSS and Atom documents; written in stage 15.

**[12/15] Processing assets** (emit). Reads `styles/` and `static/`.
Compiles SCSS to `dist/css/` and copies static files to `dist/static/`.
The co-located asset report from stage 5 is merged in here.

**[13/15] Generating search index** (emit). Reads the processed pages in
registry order — skipped entirely when `[build] search = false`. Produces
`dist/search_index.bin`: one `SearchDocument` per page with its title,
URL path, truncated summary and taxonomies, plus the TF-IDF postings
over the page's Markdown text with its title and taxonomy terms
repeated as a field boost (never the rendered HTML, whose markup would
pollute the term space).

**[14/15] Writing WASM client** (emit). Reads nothing from the site.
Writes the embedded `client.js` and `client_bg.wasm` to `dist/wasm/` —
skipped when `[build] islands = false` (what `taxus init --no-islands`
writes; a plain Tera/Markdown site ships no hydration code).

**[15/15] Writing output** (emit). Writes every `RenderedPage` to its
output file, then the taxonomy pages, then the feeds, then one redirect
page per `aliases` entry. Produces the `BuildReport`.

In `--dry-run` every stage runs and nothing is written; stage 4 skips
pixel work and stage 12 still compiles SCSS so errors surface.

## Key types along the way

| Type | Made in stage | Holds |
|------|---------------|-------|
| `SiteTree` (`taxus_domain`) | 1 | the parsed site: sections, pages, frontmatter, bodies |
| `RouteRegistry`, `RouteInfo` | 1 | per document: URL path, content file, output file, kind |
| `ProcessedPage` | 3, 4 | route, parsed `Page`, rendered `html_content`, `toc`, `hero_image` |
| `TemplateContext` | 6 | `site`, `page`, `section`, `now`, `extra` for one render |
| `RenderedPage` | 6 | route and final HTML `content` |
| `TaxonomyMap` | 10 | terms per kind, each with its documents' content files |
| `GeneratedFeed`, `GeneratedSitemap`, `GeneratedSearch` | 11, 8, 13 | the bytes of one output file |
| `BuildReport` | 15 | counts, duration, asset report |

Stages join the tree to the processed pages by **content file**: a
`PageNode::content_file` equals a `RouteInfo::content_file` equals a
`Page::source`. That is the one name that survives every stage.

## Generator module map

| Module | Phase | Types | Responsibility |
|--------|-------|-------|----------------|
| `config` | parse | `SiteConfig`, `SiteMeta`, `BuildConfig`, `FeedConfig`, `HighlightConfig`, `ImageConfig`, `MarkdownConfig` | load and validate `site.toml` |
| `content` | parse | `Page`, `Frontmatter` (re-exported from the domain), `ContentSource`, `split_date_prefix`, `TaxonomyMap` | parse one content file; taxonomy map type |
| `routes` | parse | `RouteDiscovery`, `RouteRegistry`, `RouteInfo`, `RouteKind`, `slugify` | build the tree from disk; derive routes; the two slug algorithms (node paths, taxonomy terms) |
| `build` | all | `SiteBuilder`, `BuildReport`, `ProcessedPage`, `RenderedPage`, `pipeline::*` | the fifteen stages |
| `templates` | emit | `TeraRenderer`, `TemplateContext`, `PageContext`, `SectionContext`, `SiteContext`, `PaginationContext`, `TaxonomyTermContext` | render Tera templates; tree functions |
| `images` | emit | `ImageProcessor`, `ProcessedImage`, `ImageRegistry`, `render_picture` | hero image variants and `<picture>` markup |
| `highlighting` | emit | `CodeHighlighter`, `LanguageRegistry` | tree-sitter syntax highlighting |
| `assets` | emit | `ScssProcessor`, `StaticCopier`, `AssetReport` | SCSS and static files |
| `feed` | emit | `FeedGenerator`, `FeedEntry`, `FeedConfig` | RSS and Atom documents |
| `init` | none | `InitScaffolder`, `InitOptions`, `InitReport` | `taxus init` |
| `serve` | none | `DevServer`, `DevServerConfig`, `FileWatcher` | dev server, file watching, live reload |
| `error` | all | `GeneratorError` and the per-module errors | error types |
| `telemetry` | none | `init`, `init_tracing`, `init_with_level` | logging setup |

The `build::pipeline` modules, one per stage or output: `markdown`,
`internal_links`, `pages`, `robots`, `sitemap`, `not_found`, `taxonomy`,
`feeds`, `search`, `wasm`, `alias`.

## Islands

At build time the `island()` Tera function renders a Yew component from
`taxus-common` to HTML and wraps it in a mount point:

```html
<div data-island="Counter" data-props='{"initial":3,"class":""}'>
  <!-- HTML rendered by Yew at build time -->
</div>
```

In the browser the embedded client (`dist/wasm/client.js`) finds every
`[data-island]`, reads `data-props`, and calls `yew::Renderer::hydrate`
on it, attaching event handlers without re-rendering. Pages with no
islands are plain HTML. See [Islands Architecture](./islands.md).

## Feature flags

| Feature | Default | Effect |
|---------|---------|--------|
| `lang-rust` | on | Rust syntax highlighting via tree-sitter |
| `webp-lossy` | on | Lossy WebP hero variants via libwebp; without it WebP is lossless and `images.quality` is ignored for WebP |

Islands are not a feature flag. The WASM client is always compiled and
embedded; `taxus init --no-islands` only leaves the hydration script out
of the scaffolded `base.html`.
