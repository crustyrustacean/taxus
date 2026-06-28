# API Reference

This page documents the public API of the `taxus_lib` generator library and `taxus-common` shared library.

## `taxus-common` Crate

### `search` Module

The search module provides full-text search with TF-IDF ranking.

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

Metadata record for each indexed page.

| Method | Description |
|--------|-------------|
| `new(id, title, path, summary, tags, categories) -> Self` | Create a new document |

#### `SearchIndex`

```rust
pub struct SearchIndex {
    pub documents: Vec<SearchDocument>,
    pub index: HashMap<String, Vec<(u32, f32)>>,
}
```

The main search index structure. The `index` maps word stems to `(document_id, tfidf_score)` pairs.

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create empty index |
| `add_document(&mut self, doc: SearchDocument, content: &str)` | Add a document with its content for indexing |
| `search(&self, query: &str) -> Vec<&SearchDocument>` | Search and return ranked results |
| `finalize(&mut self)` | Apply IDF weighting (call after all documents added) |
| `to_bytes(&self) -> Vec<u8>` | Serialize to binary (postcard format) |
| `from_bytes(bytes: &[u8]) -> Self` | Deserialize from binary |

#### Helper Functions

```rust
pub fn tokenize(text: &str) -> Vec<String>
```

Split text into lowercase tokens. Filters words shorter than 3 characters.

```rust
pub fn stem(tokens: &[String]) -> Vec<String>
```

Apply English Porter stemmer to tokens.

---

## `taxus-generator` Crate

## Re-exports

The library re-exports commonly used types from `lib.rs`:

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

// Build
pub use build::{BuildReport, ProcessedPage, RenderedPage, SiteBuilder};

// Feed
pub use feed::{FeedConfig, FeedEntry, FeedGenerator};

// Init
pub use init::{InitOptions, InitReport, InitScaffolder};

// Routes
pub use routes::{RouteDiscovery, RouteInfo, RouteKind, RouteRegistry};

// Errors
pub use error::{
    AssetError, ContentError, FeedError, GeneratorError, InitError,
    Result, RouteError, TemplateError,
};
```

## `config` Module

### `SiteConfig`

```rust
pub struct SiteConfig {
    pub site: SiteMeta,
    pub build: BuildConfig,
    pub feed: FeedConfig,
    pub base_dir: PathBuf,
}
```

| Method | Description |
|--------|-------------|
| `from_file(path: P) -> Result<Self>` | Load from file |
| `from_dir(dir: P) -> Result<Self>` | Load from directory (looks for `site.toml`) |
| `new(name, base_url) -> Self` | Create programmatically |
| `validate(&self) -> Result<()>` | Validate required fields |

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

### `FeedConfig`

```rust
pub struct FeedConfig {
    pub rss_enabled: bool,       // default: true
    pub atom_enabled: bool,      // default: false
    pub limit: usize,            // default: 0 (all)
    pub full_content: bool,      // default: false
    pub title: Option<String>,
    pub rss_path: Option<String>,
    pub atom_path: Option<String>,
}
```

## `content` Module

### `Frontmatter`

```rust
pub struct Frontmatter {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<NaiveDate>,
    pub updated: Option<NaiveDate>,
    pub template: Option<String>,
    pub draft: bool,
    pub summary: Option<String>,
    pub slug: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub series: Option<String>,
    pub extra: Option<toml::Value>,
    pub sort_by: SortBy,           // default: Date
    pub paginate_by: usize,
    pub paginate_template: Option<String>,
    pub weight: i32,
}
```

| Method | Description |
|--------|-------------|
| `from_str(s: &str) -> Result<Self, toml::de::Error>` | Parse from TOML |
| `template(&self) -> &str` | Get template (default: `"page.html"`) |

### `Page`

```rust
pub struct Page {
    pub frontmatter: Frontmatter,
    pub path: String,
    pub source: PathBuf,
    pub raw_content: String,
    pub content: Option<String>,
}
```

| Method | Description |
|--------|-------------|
| `from_file(path: P) -> Result<Self>` | Load from Markdown file |
| `from_str(content: &str, source: &str) -> Result<Self>` | Parse from string |
| `is_draft(&self) -> bool` | Check if draft |
| `url_path(&self) -> String` | Get URL path |
| `aliases(&self) -> &Vec<String>` | Get redirect aliases |

### `Section`

```rust
pub struct Section {
    pub frontmatter: Frontmatter,
    pub path: String,
    pub source: PathBuf,
    pub content: Option<String>,
    pub pages: Vec<Page>,
}
```

| Method | Description |
|--------|-------------|
| `from_dir(dir: P) -> Result<Self>` | Load from directory |
| `add_page(&mut self, page: Page)` | Add a page |
| `sort_by_date(&mut self)` | Sort by date (newest first) |

### `ContentSource` Trait

```rust
pub trait ContentSource: Send + Sync {
    fn load(&self, path: &Path) -> Result<String>;
    fn exists(&self, path: &Path) -> bool;
    fn list(&self) -> Result<Vec<PathBuf>>;
}
```

### `FilesystemContentSource`

```rust
pub struct FilesystemContentSource { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new(root: P) -> Self` | Create with root directory |

## `routes` Module

### `RouteKind`

```rust
pub enum RouteKind {
    Page,
    Section,
}
```

### `RouteInfo`

```rust
pub struct RouteInfo {
    pub path: String,
    pub content_file: PathBuf,
    pub output_file: PathBuf,
    pub kind: RouteKind,
}
```

### `RouteRegistry`

```rust
pub struct RouteRegistry { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create empty registry |
| `register(&mut self, route: RouteInfo)` | Register a route |
| `get(&self, path: &str) -> Option<&RouteInfo>` | Get by path |
| `contains(&self, path: &str) -> bool` | Check existence |
| `len(&self) -> usize` | Count routes |
| `iter(&self) -> impl Iterator<Item = &RouteInfo>` | Iterate all |
| `pages(&self) -> impl Iterator<Item = &RouteInfo>` | Iterate pages |
| `sections(&self) -> impl Iterator<Item = &RouteInfo>` | Iterate sections |

### `RouteDiscovery`

```rust
pub struct RouteDiscovery { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new(content_dir: P) -> Self` | Create with content directory |
| `discover(&self) -> Result<RouteRegistry>` | Discover all routes |

## `templates` Module

### `TemplateRenderer` Trait

```rust
pub trait TemplateRenderer: Send + Sync {
    fn render(&self, template: &str, context: &TemplateContext) -> Result<String>;
    fn register_template(&mut self, name: &str, content: &str) -> Result<()>;
    fn has_template(&self, name: &str) -> bool;
    fn load_templates(&mut self, dir: &Path) -> Result<()>;
}
```

### `TeraRenderer`

```rust
pub struct TeraRenderer { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new() -> Result<Self>` | Create empty renderer |
| `from_dir(dir: P) -> Result<Self>` | Create and load from directory |

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
    pub path: String,
    pub permalink: String,
    pub content: String,
    pub raw_content: String,
    pub date: Option<String>,
    pub draft: bool,
    pub summary: String,
    pub word_count: usize,
    pub reading_time: usize,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub series: Option<String>,
}
```

### `SectionContext`

```rust
pub struct SectionContext {
    pub title: String,
    pub description: Option<String>,
    pub path: String,
    pub content: Option<String>,
    pub pages: Vec<PageContext>,
    pub pagination: Option<PaginationContext>,
}
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

### `SiteContext`

```rust
pub struct SiteContext {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub author: Option<String>,
}
```

### `NowContext`

```rust
pub struct NowContext {
    pub year: i32,
}
```

## `build` Module

### `SiteBuilder`

```rust
pub struct SiteBuilder {
    config: SiteConfig,
    dry_run: bool,
    verbose: bool,
    include_drafts: bool,
}
```

| Method | Description |
|--------|-------------|
| `from_dir(dir: &Path) -> Result<Self>` | Create from directory |
| `new(config: SiteConfig) -> Self` | Create from config |
| `dry_run(self, bool) -> Self` | Set dry-run mode |
| `verbose(self, bool) -> Self` | Set verbose mode |
| `include_drafts(self, bool) -> Self` | Include drafts |
| `build(self) -> Result<BuildReport>` | Run build pipeline |
| `clean(self) -> Result<()>` | Clean output directory |

### `build::pipeline::search` Module

#### `GeneratedSearch`

```rust
pub struct GeneratedSearch {
    pub search_index: Vec<u8>,
}
```

Container for the serialized search index.

| Function | Description |
|----------|-------------|
| `generate_search(pages: &[ProcessedPage]) -> Result<GeneratedSearch>` | Create search index from processed pages |
| `write_search_index(generated: &GeneratedSearch, output_dir: &Path, dry_run: bool) -> Result<()>` | Write `search_index.bin` to output |

### `build::pipeline::wasm` Module

The WASM client is compiled at Cargo build time by `taxus-generator/build.rs` and embedded into the binary. At site build time, the embedded files are written to the output directory.

#### `WasmBuildOutput`

```rust
pub struct WasmBuildOutput {
    pub js_path: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_size: u64,
}
```

Result of writing the embedded WASM client files.

| Function | Description |
|----------|-------------|
| `build_wasm_client(output_dir: &Path) -> Result<WasmBuildOutput>` | Write embedded `client.js` and `client_bg.wasm` to `dist/wasm/` |

### `BuildReport`

```rust
pub struct BuildReport {
    pub output_dir: PathBuf,
    pub pages_rendered: usize,
    pub sections_rendered: usize,
    pub drafts_skipped: usize,
    pub sitemap_urls: usize,
    pub assets: AssetReport,
    pub duration: Duration,
}
```

| Method | Description |
|--------|-------------|
| `print_summary(&self)` | Print summary |
| `has_warnings(&self) -> bool` | Check for warnings |

### `ProcessedPage`

```rust
pub struct ProcessedPage {
    pub route: RouteInfo,
    pub page: Page,
}
```

### `RenderedPage`

```rust
pub struct RenderedPage {
    pub route: RouteInfo,
    pub html: String,
}
```

## `assets` Module

### `AssetProcessor` Trait

```rust
pub trait AssetProcessor: Send + Sync {
    fn process(&self, src: &Path, dest: &Path) -> Result<AssetReport>;
    fn handles(&self, path: &Path) -> bool;
    fn name(&self) -> &'static str;
}
```

### `ScssProcessor`

```rust
pub struct ScssProcessor {
    include_paths: Vec<PathBuf>,
    minify: bool,
}
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create with defaults |
| `with_include_paths(paths: Vec<PathBuf>) -> Self` | Set include paths |
| `with_minify(bool) -> Self` | Set minify |

### `StaticCopier`

```rust
pub struct StaticCopier {
    exclude_patterns: Vec<String>,
}
```

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
| `with_force(bool) -> Self` | Set force |
| `with_islands(bool) -> Self` | Set islands |

### `InitScaffolder`

```rust
pub struct InitScaffolder { /* ... */ }
```

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

## `serve` Module

### `DevServer`

```rust
pub struct DevServer { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new(config: DevServerConfig) -> Self` | Create server |
| `run(&self) -> Result<()>` | Start server (async) |

### `DevServerConfig`

```rust
pub struct DevServerConfig {
    pub site_dir: PathBuf,
    pub port: u16,
    pub output_dir: PathBuf,
}
```

| Method | Description |
|--------|-------------|
| `default() -> Self` | Create with defaults |
| `with_port(self, port: u16) -> Self` | Set port |
| `with_output_dir(self, dir: PathBuf) -> Self` | Set output dir |
| `with_site_dir(self, dir: PathBuf) -> Self` | Set site dir |

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
    Io { path: PathBuf, source: std::io::Error },
    NoContent,
    BrokenInternalLink { file: String, target: String },
    PageRenderFailed { path: String, source: TemplateError },
}
```

| Type | Description |
|------|-------------|
| `ConfigError` | Configuration errors (not found, parse, missing field) |
| `ContentError` | Content errors (not found, frontmatter, IO) |
| `TemplateError` | Template errors (not found, render, syntax) |
| `AssetError` | Asset errors (SCSS, copy) |
| `RouteError` | Route errors (not found, duplicate, invalid) |
| `FeedError` | Feed generation errors |
| `ImageError` | Image processing errors |
| `InitError` | Initialization errors (cancelled) |
| `ServeError` | Server errors (port in use, WebSocket) |
| `WasmError` | WASM build errors (tool missing, build failed) |

### `Result`

```rust
pub type Result<T> = std::result::Result<T, GeneratorError>;
```

## `tracing` Module

| Function | Description |
|----------|-------------|
| `init()` | Initialize with `RUST_LOG` env var |
| `init_with_level(level: &str)` | Initialize with specific level |
