# API Reference

This page documents the public API of the three library crates:
`taxus-domain` (the model), `taxus_lib` (the generator library, crate
`taxus-generator`) and `taxus-common` (islands and search). Terms are
defined in the [Glossary](./theory/glossary.md). `cargo doc --workspace
--no-deps --open` builds the full rustdoc, which is the authoritative
reference; this page is the map.

## `taxus-domain` Crate

The pure data model. No I/O. See [The Site Tree](./theory/site-tree.md),
[Identity](./theory/identity.md) and [Derivations](./theory/derivations.md).

### `identity` Module

```rust
pub struct Slug(String);      // one validated URL segment
pub struct NodePath(Vec<Slug>); // slugs from the root to a node
pub struct UrlPath(String);   // the derived address, "/blog/my-post/"
pub enum IdentityError { Empty, Slash { s }, DotSegment { s }, ControlCharacter { s } }
```

| Item | Description |
|------|-------------|
| `Slug::new(raw) -> Result<Slug, IdentityError>` | Validate a segment: non-empty, no `/`, not `.` or `..`, no control characters. Does not slugify |
| `Slug::as_str(&self) -> &str` | The segment |
| `NodePath::root() -> NodePath` | The root section's path (empty) |
| `NodePath::parse(raw) -> Result<NodePath, IdentityError>` | From `"blog/my-post"`; `""` and `"/"` are the root |
| `NodePath::from_segments(iter) -> Result<NodePath, IdentityError>` | From slug strings |
| `NodePath::is_root`, `parent`, `last`, `join(&Slug)`, `segments` | Path queries |
| `UrlPath::from_node_path(&NodePath) -> UrlPath` | The one place addresses are derived: `/` for the root, else `/a/b/` |
| `UrlPath::as_str(&self) -> &str` | The address |

### `schema` Module

```rust
pub struct Frontmatter {
    pub title: String,                    // default ""
    pub description: Option<String>,
    pub tagline: Option<String>,
    pub date: Option<NaiveDate>,
    pub template: Option<String>,
    pub draft: bool,
    pub summary: Option<String>,
    pub slug: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub series: Option<String>,
    pub extra: Option<toml::Value>,
    pub sort_by: SortBy,                  // default Date
    pub paginate_by: usize,               // 0 = no pagination
    pub paginate_template: Option<String>,
    pub weight: i32,
    pub pages_from: Vec<String>,          // donor sections, "blog"
    pub updated: Option<NaiveDate>,
    pub hero_image: Option<String>,
    pub hero_alt: Option<String>,
}

pub enum SortBy { Date, Title, Weight, None }
```

| Item | Description |
|------|-------------|
| `Frontmatter::from_str(s) -> Result<Frontmatter, toml::de::Error>` | Parse TOML (via `std::str::FromStr`) |
| `Frontmatter::template(&self) -> &str` | `template`, or `"page.html"` |
| `Frontmatter::extra_as_json(&self) -> HashMap<String, serde_json::Value>` | The `[extra]` table for templates |

### `tree` Module

```rust
pub struct SiteTree { pub root: SectionNode }

pub struct SectionNode {
    pub path: NodePath,
    pub content_file: Option<PathBuf>,   // "blog/_index.md", or None
    pub meta: Frontmatter,
    pub body: Option<String>,
    pub pages: Vec<PageNode>,            // direct children, by slug
    pub subsections: Vec<SectionNode>,   // direct children, by slug
}

pub struct PageNode {
    pub path: NodePath,
    pub content_file: PathBuf,           // "blog/2026-04-03-project-launch.md"
    pub meta: Frontmatter,
    pub body: String,
}

pub enum TreeError { Duplicate { path }, Collision { path, kind }, RootReserved, Identity(IdentityError) }
```

| Item | Description |
|------|-------------|
| `SiteTree::get_section(&NodePath) -> Option<&SectionNode>` | Lookup by node path |
| `SiteTree::get_page(&NodePath) -> Option<&PageNode>` | Lookup by node path |
| `SiteTree::iter_pages(&self)` | Every page, depth-first, drafts included |
| `PageNode::is_draft(&self) -> bool` | `meta.draft` |
| `SiteTreeBuilder::new() -> SiteTreeBuilder` | Start with a default root |
| `SiteTreeBuilder::root(self, content_file, meta, body) -> Self` | Set the root section's index file |
| `SiteTreeBuilder::add_section(&mut self, &NodePath, content_file, meta, body) -> Result<(), TreeError>` | Declare a section at its final path |
| `SiteTreeBuilder::add_page(&mut self, &NodePath, content_file, meta, body) -> Result<(), TreeError>` | Declare a page at its final path |
| `SiteTreeBuilder::build(self) -> Result<SiteTree, TreeError>` | Assemble; auto-creates intermediate sections; sorts children by slug |
| `sort_pages(&mut [&PageNode], SortBy)` | The ordering listings use: date newest first with undated last, title case-insensitive, weight lowest first, none |

### `derivation` Module

```rust
pub enum Node<'a> { Section(&'a SectionNode), Page(&'a PageNode) }
```

| Item | Description |
|------|-------------|
| `Node::path`, `meta`, `content_file`, `is_section`, `is_draft` | Accessors shared by both kinds |
| `documents(&SiteTree) -> Vec<Node>` | Every document in tree order; drafts included |
| `descendant_pages(&SectionNode) -> Vec<&PageNode>` | Every page below a section, depth-first |
| `recent(&SiteTree, include_drafts) -> Vec<&PageNode>` | All pages, newest first, undated last |
| `aggregate(&SectionNode, &SiteTree, &[NodePath]) -> Vec<&PageNode>` | The receiver's pages plus each donor's direct pages, deduplicated, unsorted |
| `group_by_terms(&SiteTree, include_drafts, terms_of) -> BTreeMap<String, Vec<Node>>` | Documents grouped by the terms a selector reads from frontmatter |

The crate root re-exports `Frontmatter`, `SortBy`, `NodePath`, `Slug`,
`UrlPath`, `PageNode`, `SectionNode`, `SiteTree`, `SiteTreeBuilder` and
`TreeError`.

---

## `taxus-common` Crate

Shared by the generator (build-time rendering) and the client (browser
hydration).

### `components` Module

| Component | Props | Description |
|-----------|-------|-------------|
| `counter::Counter` | `CounterProps { initial: i32, class: String }` | Demonstration counter |
| `search_box::SearchBox` | `SearchBoxProps { placeholder: String, max_results: usize, class: String }` | Client-side search input; see [Search](./search.md) |

### `search` Module

#### `SearchDocument`

```rust
pub struct SearchDocument {
    pub id: u32,
    pub title: String,
    pub path: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}
```

| Method | Description |
|--------|-------------|
| `new(id, title, path, summary, tags, categories) -> Self` | Create a new document |

#### `SearchIndex`

```rust
pub struct SearchIndex {
    pub documents: BTreeMap<u32, SearchDocument>,
    pub index: HashMap<String, Vec<(u32, f32)>>,
}
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create empty index |
| `add_document(&mut self, doc: SearchDocument, content: &str)` | Add a document with its content for indexing |
| `search(&self, query: &str) -> Vec<&SearchDocument>` | Search and return ranked results |
| `finalize(&mut self)` | Apply IDF weighting (call after all documents added) |
| `to_bytes(&self) -> Result<Vec<u8>, postcard::Error>` | Serialize to binary (postcard format) |
| `from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error>` | Deserialize from binary |

#### Helper Functions

```rust
pub fn tokenize(text: &str) -> Vec<String>  // lowercase tokens, words shorter than 3 characters dropped
pub fn stem(tokens: &[String]) -> Vec<String> // English Porter stemmer
```

---

## `taxus-generator` Crate (`taxus_lib`)

### Re-exports

```rust
pub use config::{BuildConfig, ImageConfig, SiteConfig, SiteMeta};
pub use content::{ContentSource, FilesystemContentSource, Frontmatter, Page};
pub use templates::{HeroContext, PageContext, SectionContext, SiteContext, TemplateContext, TemplateRenderer, TeraRenderer};
pub use assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
pub use build::{BuildReport, SiteBuilder};
pub use feed::{FeedConfig, FeedEntry, FeedGenerator};
pub use highlighting::{CodeHighlighter, LanguageRegistry};
pub use images::{ImageProcessor, ImageRegistry, ProcessedImage, render_picture};
pub use init::{InitOptions, InitReport, InitScaffolder};
pub use routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};
pub use error::{AssetError, ContentError, FeedError, GeneratorError, ImageError, InitError, Result, RouteError, TemplateError};
```

## `config` Module

### `SiteConfig`

```rust
pub struct SiteConfig {
    pub site: SiteMeta,
    pub build: BuildConfig,
    pub feed: FeedConfig,
    pub highlight: HighlightConfig,
    pub images: ImageConfig,
    pub markdown: MarkdownConfig,
    pub base_dir: PathBuf,
}
```

| Method | Description |
|--------|-------------|
| `from_file(path: P) -> Result<Self>` | Load from file |
| `from_dir(dir: P) -> Result<Self>` | Load from directory (looks for `site.toml`) |
| `new(name, base_url) -> Self` | Create programmatically |
| `validate(&self) -> Result<()>` | Validate `site.name`, `site.base_url`, `images.quality`, `images.format` |

### `SiteMeta`

```rust
pub struct SiteMeta {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub author: Option<String>,
}
```

### `BuildConfig`

```rust
pub struct BuildConfig {
    pub content_dir: PathBuf,    // default: "content"
    pub output_dir: PathBuf,     // default: "dist"
    pub static_dir: PathBuf,     // default: "static"
    pub styles_dir: PathBuf,     // default: "styles"
    pub templates_dir: PathBuf,  // default: "templates"
}
```

| Method | Description |
|--------|-------------|
| `resolve_paths(&mut self, base_dir: &Path)` | Make relative directories absolute against the site directory |

### `FeedConfig`

```rust
pub struct FeedConfig {
    pub rss_enabled: bool,       // default: true
    pub atom_enabled: bool,      // default: false
    pub limit: usize,            // default: 20 (0 is treated as 20)
    pub full_content: bool,      // default: false
    pub title: Option<String>,
    pub rss_path: Option<String>,   // default file: feed.xml
    pub atom_path: Option<String>,  // default file: feed.atom
    pub sections: Vec<String>,   // default: [] (whole site)
}
```

### `HighlightConfig`, `ImageConfig`, `MarkdownConfig`

```rust
pub struct HighlightConfig { pub enabled: bool /* true */, pub class_prefix: String /* "hl-" */ }
pub struct ImageConfig { pub widths: Vec<u32>, pub quality: u8, pub format: String, pub output_dir: PathBuf }
pub struct MarkdownConfig { pub insert_anchor_links: bool }
```

| Method | Description |
|--------|-------------|
| `ImageConfig::normalize(&mut self)` | `"jpg"` becomes `"jpeg"` |
| `ImageConfig::validate(&self) -> Result<()>` | Quality 1..=100; format `webp`, `jpeg`, `jpg` or `png` |

## `content` Module

`Frontmatter` and `SortBy` are re-exported from `taxus-domain`.

### `Page`

One parsed content file (page or index file).

```rust
pub struct Page {
    pub frontmatter: Frontmatter, // page metadata
    pub raw_content: String,      // the Markdown body
}
```

A `Page` is the parsed form of one tree node's document — exactly what
discovery read from disk, nothing computed. The build constructs them
from the tree in stage 3; the served URL lives on
[`ProcessedPage`](#processedpage), never here.

| Method | Description |
|--------|-------------|
| `from_file(path: P) -> Result<Self>` | Load from a Markdown file |
| `from_str(content: &str, source: &str) -> Result<Self>` | Parse from string (what discovery uses) |
| `template(&self) -> &str` | `template`, or `"page.html"` |
| `is_draft(&self) -> bool` | Check if draft |
| `summary(&self) -> String` | Frontmatter `summary`, else text before `<!-- more -->`, else first paragraph |
| `word_count(&self) -> usize`, `reading_time(&self) -> usize` | From the body; 200 words per minute, rounded up |
| `aliases`, `tags`, `categories`, `series` | Frontmatter accessors |

### `split_date_prefix`

```rust
pub fn split_date_prefix(stem: &str) -> (&str, Option<NaiveDate>)
```

Strips a valid `YYYY-MM-DD-` prefix from a file stem and returns the
date; a stem that is only a date is returned unchanged.

### Sections

Sections are not a `content` type. A directory is a `taxus_domain::SectionNode`
in the `SiteTree` built by `RouteDiscovery::discover_tree`; see
[The Site Tree](./theory/site-tree.md).

### `TaxonomyKind`, `TaxonomyTerm`, `TaxonomyMap`

```rust
pub enum TaxonomyKind { Tag, Category, Series }
pub struct TaxonomyTerm { pub kind, pub name: String, pub slug: String, pub page_count: usize, pub page_paths: Vec<String> /* content files */ }
pub struct TaxonomyMap { /* terms per kind */ }
```

| Method | Description |
|--------|-------------|
| `TaxonomyKind::path_prefix(&self) -> &str` | `"tags"`, `"categories"`, `"series"` |
| `TaxonomyKind::plural_name(&self) -> &str` | `"Tags"`, `"Categories"`, `"Series"` |
| `TaxonomyTerm::url_path(&self) -> String` | `/tags/rust/` |
| `TaxonomyMap::add_term(&mut self, kind, name, content_file)` | File a document under a term |
| `TaxonomyMap::tags()`, `categories()`, `series()` | Terms of a kind, sorted by name |
| `TaxonomyMap::get_tag(slug)`, `get_category(slug)`, `get_series(slug)` | Lookup by term slug |

### `ContentSource` Trait

```rust
pub trait ContentSource: Send + Sync {
    fn load(&self, path: &Path) -> Result<String>;
    fn exists(&self, path: &Path) -> bool;
    fn list(&self) -> Result<Vec<PathBuf>>;
}
```

### `FilesystemContentSource`

| Method | Description |
|--------|-------------|
| `new(root: P) -> Self` | Create with root directory |

## `routes` Module

### `RouteKind`

```rust
pub enum RouteKind { Page, Section }
```

### `RouteInfo`

```rust
pub struct RouteInfo {
    pub path: String,          // URL path, "/blog/my-post/"
    pub content_file: PathBuf,
    pub output_file: PathBuf,  // "blog/my-post/index.html"
    pub kind: RouteKind,
}
```

### `RouteRegistry`

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create empty registry |
| `from_tree(tree: &SiteTree) -> Self` | Derive the registry from a Site Tree (one route per document, in tree order) |
| `register(&mut self, route: RouteInfo)` | Register a route |
| `get(&self, path: &str) -> Option<&RouteInfo>` | Get by URL path |
| `contains(&self, path: &str) -> bool` | Check existence |
| `len(&self) -> usize`, `is_empty(&self) -> bool` | Count routes |
| `iter(&self)`, `pages(&self)`, `sections(&self)` | Iterate in registration order |
| `find_by_content_file(&self, &Path) -> Option<&RouteInfo>` | Lookup by content file |

### `RouteDiscovery`

| Method | Description |
|--------|-------------|
| `new(content_dir: P) -> Self` | Create with content directory |
| `discover_tree(&self) -> Result<SiteTree>` | Walk the content directory and build the Site Tree (what `SiteBuilder::build` uses) |
| `discover_tree_from_source(&self, source: &impl ContentSource) -> Result<SiteTree>` | Same, from a `ContentSource` |
| `discover(&self) -> Result<RouteRegistry, RouteError>` | Legacy file walk: routes keyed by filename, frontmatter not read |
| `discover_from_source(&self, source: &impl ContentSource) -> Result<RouteRegistry, RouteError>` | Legacy walk from a `ContentSource` |

### `slugify`

```rust
pub fn slugify_segment(segment: &str) -> String  // "My Créative Post" -> "my-creative-post" (node paths; ASCII)
pub fn slugify_path(relative: &str) -> String    // "blog/My Old Post" -> "blog/my-old-post"
pub fn slugify_term(name: &str) -> String        // "Café" -> "café" (taxonomy terms; keeps non-ASCII letters)
```

## `templates` Module

### `TemplateRenderer` Trait

```rust
pub trait TemplateRenderer: Send + Sync {
    fn render(&self, template: &str, context: &TemplateContext) -> Result<String, TemplateError>;
    fn register_template(&mut self, name: &str, content: &str) -> Result<(), TemplateError>;
    fn has_template(&self, name: &str) -> bool;
    fn load_templates(&mut self, dir: &Path) -> Result<(), TemplateError>;
}
```

### `TeraRenderer`

| Method | Description |
|--------|-------------|
| `new() -> Result<Self, TemplateError>` | Empty renderer with `island()`, `get_section()`, `get_page()`, `slugify`, `term_slug`, `slug` and `date` registered |
| `from_dir(dir: P) -> Result<Self, TemplateError>` | Create and load `**/*.html` from a directory |
| `set_site_lookup(&self, sections, pages)` | What `get_section` and `get_page` resolve to; the render stage fills it |

### `TemplateContext`

```rust
pub struct TemplateContext {
    pub page: Option<PageContext>,
    pub section: Option<SectionContext>,
    pub site: SiteContext,
    pub now: NowContext,
    pub extra: HashMap<String, serde_json::Value>,
}
```

| Method | Description |
|--------|-------------|
| `new(site: SiteContext) -> Self` | Create with site |
| `with_page(self, page: PageContext) -> Self` | Add page |
| `with_section(self, section: SectionContext) -> Self` | Add section |
| `with_extra(self, extra: HashMap) -> Self` | Add extra |

### `PageContext`

```rust
pub struct PageContext {
    pub title: String,
    pub description: Option<String>,
    pub tagline: Option<String>,
    pub path: String,             // URL path
    pub permalink: String,        // base_url + path
    pub content: String,          // rendered HTML
    pub raw_content: String,
    pub date: Option<String>,     // ISO 8601
    pub draft: bool,
    pub summary: String,
    pub word_count: usize,
    pub reading_time: usize,
    pub toc: Vec<TocEntry>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub series: Option<String>,
    pub weight: i32,
    pub hero: Option<HeroContext>,
}
```

### `HeroContext`

```rust
pub struct HeroContext { pub src: String, pub srcset: String, pub width: u32, pub height: u32, pub alt: String, pub mime_type: String }
```

### `SectionContext` and `SubsectionContext`

```rust
pub struct SectionContext {
    pub title: String,
    pub description: Option<String>,
    pub path: String,
    pub permalink: String,
    pub content: Option<String>,
    pub toc: Vec<TocEntry>,
    pub pages: Vec<PageContext>,          // the listing
    pub pagination: Option<PaginationContext>,
    pub subsections: Vec<SubsectionContext>,
}

pub struct SubsectionContext { pub title: String, pub description: Option<String>, pub path: String, pub permalink: String }
```

### `PaginationContext`

```rust
pub struct PaginationContext {
    pub current: usize,
    pub total: usize,
    pub per_page: usize,
    pub total_items: usize,
    pub prev: Option<String>,
    pub next: Option<String>,
    pub first: String,
    pub last: String,
}
```

| Method | Description |
|--------|-------------|
| `is_first(&self)`, `is_last(&self)` | Position checks |
| `page_range(&self) -> Vec<Option<usize>>` | Page numbers for navigation, `None` for gaps |

### `TaxonomyListContext` and `TaxonomyTermContext`

```rust
pub struct TaxonomyListContext { pub kind: String, pub path: String, pub terms: Vec<TaxonomyTermContext> }
pub struct TaxonomyTermContext { pub kind: String, pub name: String, pub slug: String, pub path: String, pub page_count: usize, pub pages: Vec<PageContext> }
```

Both are passed to templates as `extra.taxonomy`.

### `SiteContext` and `NowContext`

```rust
pub struct SiteContext { pub name: String, pub base_url: String, pub description: Option<String>, pub author: Option<String> }
pub struct NowContext { pub year: i32 }
```

### `compute_permalink`

```rust
pub fn compute_permalink(base_url: &str, path: &str) -> String
```

## `build` Module

### `SiteBuilder`

| Method | Description |
|--------|-------------|
| `from_dir(dir: &Path) -> Result<Self>` | Create from directory |
| `new(config: SiteConfig) -> Self` | Create from config |
| `dry_run(self, bool) -> Self` | Set dry-run mode |
| `verbose(self, bool) -> Self` | No-op kept for API compatibility; verbosity is tracing `debug` level (#54) |
| `include_drafts(self, bool) -> Self` | Include drafts |
| `output_dir(self, dir: impl Into<PathBuf>) -> Self` | Override the output directory |
| `build(self) -> Result<BuildReport>` | Run the fifteen-stage pipeline (see [Architecture](./architecture.md)) |
| `clean(self) -> Result<()>` | Clean output directory |
| `config(&self) -> &SiteConfig` | The configuration |

### `BuildReport`

```rust
pub struct BuildReport {
    pub pages_rendered: usize,
    pub sections_rendered: usize,
    pub drafts_skipped: usize,
    pub sitemap_urls: usize,
    pub assets: AssetReport,
    pub duration: Duration,
    pub warnings: Vec<String>,
    pub output_dir: PathBuf,
}
```

| Method | Description |
|--------|-------------|
| `print_summary(&self)` | Print summary |
| `total_files(&self) -> usize` | Pages plus sections plus assets |
| `has_warnings(&self)`, `has_errors(&self)`, `is_failure(&self)` | Status checks |
| `add_warning(&mut self, warning)` | Record a warning |

### `ProcessedPage` and `RenderedPage`

```rust
pub struct ProcessedPage {
    pub route: RouteInfo,
    pub page: Page,
    pub html_content: String,
    pub toc: Vec<TocEntry>,
    pub hero_image: Option<ProcessedImage>,
}

pub struct RenderedPage {
    pub route: RouteInfo,
    pub content: String,
}
```

| Method | Description |
|--------|-------------|
| `ProcessedPage::effective_url_path(&self) -> String` | The served URL path (the route's path, derived from the tree) |

### `build::pipeline` functions

| Function | Stage | Description |
|----------|-------|-------------|
| `load_config(dir) -> Result<SiteConfig>` | setup | Load `site.toml` |
| `discover_tree(&SiteConfig) -> Result<SiteTree>` | 1 | Build the Site Tree |
| `discover_routes(&SiteConfig) -> Result<RouteRegistry>` | 1 | The tree projected to routes |
| `load_templates(&SiteConfig) -> Result<TeraRenderer>` | 2 | Load templates |
| `process_content(&SiteTree, &RouteRegistry, &SiteConfig, include_drafts, highlighter) -> Result<(Vec<ProcessedPage>, usize)>` | 3 | Render Markdown from the tree; returns the pages and the observed skip count (#55) |
| `process_images(&mut [ProcessedPage], &SiteConfig, dry_run) -> Result<ImageRegistry>` | 4 | Hero image variants |
| `copy_colocated_assets(content_dir, output_dir, dry_run) -> Result<AssetReport>` | 5 | Copy non-`.md` files |
| `pages::render_pages(&[ProcessedPage], &SiteTree, &TeraRenderer, &SiteContext) -> Result<Vec<RenderedPage>>` | 6 | Run templates |
| `robots::generate_robots`, `write_robots` | 7 | `robots.txt` |
| `not_found::generate_404`, `write_404` | 8 | `404.html` |
| `taxonomy::build_taxonomy_map(&SiteTree)`, `render_taxonomy_pages`, `write_taxonomy_pages` | 9 | Taxonomy pages |
| `sitemap::generate_sitemap(&[RenderedPage], &[RenderedTaxonomy], &[ProcessedPage], &SiteConfig)`, `write_sitemap` | 10 | `sitemap.xml` from final outputs |
| `feeds::feed_pages(&SiteTree, &[String])`, `generate_feeds`, `write_feeds` | 11 | Feeds |
| `process_assets(&SiteConfig, output_dir, dry_run) -> Result<AssetReport>` | 12 | SCSS and static files |
| `search::generate_search(&[ProcessedPage]) -> Result<GeneratedSearch>`, `write_search_index` | 13 | `search_index.bin` |
| `wasm::build_wasm_client(output_dir, dry_run) -> Result<WasmBuildOutput>` | 14 | Write `dist/wasm/` |
| `write_output(&[RenderedPage], output_dir, dry_run)`, `alias::write_aliases` | 15 | Write files |
| `clean_output(output_dir)` | | Remove the output directory |
| `markdown::markdown_to_html_with_toc(markdown, highlighter, &MarkdownOptions) -> (String, Vec<TocEntry>)` | 3 | Markdown rendering |
| `internal_links::resolve_internal_links(content, source_file, &RouteRegistry)` | 3 | `@/` links |
| `render_island_counter(CounterProps)`, `render_search_box(SearchBoxProps)` | 6 | Island SSR helpers |

## `assets` Module

### `AssetProcessor` Trait

```rust
pub trait AssetProcessor: Send + Sync {
    fn process(&self, src: &Path, dest: &Path, dry_run: bool) -> Result<AssetReport, AssetError>;
    fn handles(&self, path: &Path) -> bool;
    fn name(&self) -> &'static str;
}
```

### `ScssProcessor`

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create with defaults |
| `with_include_paths(paths: Vec<P>) -> Self` | Set include paths |
| `with_minify(self, bool) -> Self` | Set minify |

### `StaticCopier`

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create with defaults |
| `with_exclusions(patterns: Vec<String>) -> Self` | Set exclusions |

### `AssetReport`

```rust
pub struct AssetReport {
    pub files_processed: usize,
    pub files_skipped: usize,
    pub errors: Vec<String>,
}
```

| Method | Description |
|--------|-------------|
| `merge(&mut self, other: AssetReport)` | Merge reports |
| `has_errors(&self) -> bool`, `total_files(&self) -> usize` | Status |

## `images` Module

| Item | Description |
|------|-------------|
| `ImageProcessor::new(ImageConfig, output_dir)` | Create a processor |
| `ImageProcessor::process(&self, source, alt) -> Result<ProcessedImage>` | Generate variants (cached by content hash and quality) |
| `ImageProcessor::process_dry(&self, source, alt) -> Result<ProcessedImage>` | Paths only, no pixel work |
| `ImageProcessor::quality_ignored_for_webp(&self) -> bool` | True when built without `webp-lossy` and the format is WebP |
| `ProcessedImage::srcset`, `fallback_src`, `mime_type`, `url_path(&ImageVariant)` | What `HeroContext` is built from |
| `ImageRegistry` | Processed images keyed by source path |
| `render_picture(&ProcessedImage, alt, loading) -> String` | A `<picture>` element |
| `LOSSY_WEBP_AVAILABLE: bool` | Whether the `webp-lossy` feature is compiled in |

## `highlighting` Module

| Item | Description |
|------|-------------|
| `LanguageRegistry::new()`, `get(name)`, `iter()` | Registered tree-sitter grammars (`rust`, alias `rs`) |
| `CodeHighlighter::new(LanguageRegistry, class_prefix)` | Create a highlighter |
| `CodeHighlighter::highlight(&mut self, code, language) -> HighlightResult` | Highlight one block; unknown languages are escaped plain text |

## `feed` Module

```rust
pub struct FeedConfig { pub title, pub description, pub base_url, pub author, pub author_email, pub language, pub limit: Option<usize>, pub full_content, pub filename }
pub struct FeedEntry { pub title, pub url, pub summary, pub content: Option<String>, pub date: DateTime<Utc>, pub updated, pub author, pub author_email, pub tags }
```

| Item | Description |
|------|-------------|
| `FeedEntry::from_page(&Page, url) -> FeedEntry` | Summary is `summary`, else `description`, else `Page::summary()`; `url` is the served (effective) URL, supplied by the caller |
| `FeedGenerator::new(FeedConfig)` | Create a generator |
| `FeedGenerator::generate_rss(&[Page])`, `generate_atom(&[Page])` | Drafts dropped, newest first, truncated to `limit`; URLs derive from `page.path`, so prefer the `_from_entries` forms for pages with custom slugs |
| `FeedGenerator::generate_rss_from_entries(Vec<FeedEntry>)`, `generate_atom_from_entries(Vec<FeedEntry>)` | The pipeline form: entries carry their own effective URLs; newest first, truncated to `limit` |
| `FeedGenerator::rss_filename()`, `atom_filename()` | `feed.xml`, `feed.atom` |
| `escape_xml(&str) -> String` | XML escaping |

## `init` Module

### `InitOptions`

```rust
pub struct InitOptions {
    pub name: String,
    pub base_url: String,
    pub force: bool,
    pub islands: bool,
}
```

| Method | Description |
|--------|-------------|
| `new(name, base_url) -> Self` | Create options |
| `with_force(self, bool) -> Self` | Set force |
| `with_islands(self, bool) -> Self`, `without_islands(self) -> Self` | Islands on or off |
| `validate(&self) -> Result<(), InitError>` | Non-empty name; base URL starts with `http://` or `https://` |

### `InitScaffolder`

| Method | Description |
|--------|-------------|
| `new(options: InitOptions) -> Self` | Create scaffolder |
| `scaffold(&self, path: &Path) -> Result<InitReport>` | Scaffold site |

### `InitReport`

```rust
pub struct InitReport {
    pub path: PathBuf,
    pub directories_created: usize,
    pub files_created: usize,
    pub created_dirs: Vec<PathBuf>,
    pub created_files: Vec<PathBuf>,
}
```

### Helpers

```rust
pub fn is_directory_empty(path: &Path) -> Result<bool>
pub fn derive_site_name(path: &Path) -> String
```

## `serve` Module

### `DevServer`

| Method | Description |
|--------|-------------|
| `new(config: DevServerConfig, rebuild: RebuildFn) -> Self` | Create server; `RebuildFn` is `Arc<dyn Fn() -> Result<(), String> + Send + Sync>` |
| `run(&self) -> Result<()>` | Start server (async) |
| `host(&self) -> IpAddr`, `port(&self) -> u16` | Bind address |

### `DevServerConfig`

```rust
pub struct DevServerConfig {
    pub host: IpAddr,            // default: 127.0.0.1
    pub port: u16,               // default: 3000
    pub output_dir: PathBuf,
    pub site_dir: PathBuf,
    // ...
}
```

| Method | Description |
|--------|-------------|
| `default() -> Self` | Create with defaults |
| `with_host(self, host: IpAddr) -> Self` | Set bind address |
| `with_port(self, port: u16) -> Self` | Set port |
| `with_output_dir(self, dir: PathBuf) -> Self` | Set output dir |
| `with_site_dir(self, dir: PathBuf) -> Self` | Set site dir |
| `with_include_drafts(self, bool) -> Self` | Mirrored into every rebuild (#40) |
| `with_open(self, bool) -> Self` | Open a browser after starting |

Also exported: `browsable_url(SocketAddr) -> String`, `FileWatcher`,
`WatchEvent`, `ChangeType`, `ReloadEvent`, `WebSocketMessage`,
`inject_live_reload_script`, `LIVE_RELOAD_SCRIPT`.

## `error` Module

### `GeneratorError`

```rust
pub enum GeneratorError {
    Config(Box<ConfigError>),
    Content(Box<ContentError>),
    Template(Box<TemplateError>),
    Asset(Box<AssetError>),
    Route(Box<RouteError>),
    Init(Box<InitError>),
    Serve(Box<ServeError>),
    Feed(Box<FeedError>),
    Image(Box<ImageError>),
    Search(Box<SearchError>),
    Io { path: PathBuf, source: std::io::Error },
    NoContent,
    BrokenInternalLink { file: String, target: String },
    PageRenderFailed { path: String, source: TemplateError },
}
```

| Type | Description |
|------|-------------|
| `ConfigError` | Configuration errors (not found, parse, missing field, invalid value) |
| `ContentError` | Content errors (not found, frontmatter, IO) |
| `TemplateError` | Template errors (not found, render, syntax) |
| `AssetError` | Asset errors (SCSS, copy) |
| `RouteError` | Route errors (not found, duplicate, invalid path, discovery failed) |
| `FeedError` | Feed generation errors |
| `ImageError` | Image processing errors |
| `InitError` | Initialization errors (invalid name or URL, file write, cancelled) |
| `ServeError` | Server errors (port in use, WebSocket) |
| `SearchError` | Search index serialization errors |
| `WasmError` | WASM client write errors (not wrapped by `GeneratorError`) |

### `Result`

```rust
pub type Result<T> = std::result::Result<T, GeneratorError>;
```

## `telemetry` Module

| Function | Description |
|----------|-------------|
| `init()` | Initialize with `RUST_LOG` env var |
| `init_tracing(verbose: bool, quiet: bool)` | Initialize from CLI flags, falling back to `RUST_LOG` |
| `init_with_level(level: &str)` | Initialize with specific level |
