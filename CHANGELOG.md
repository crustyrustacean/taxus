# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.36] - 2026-03-29

### Added

- **pagination**: Wire paginator into build pipeline

### Documentation

- Fix issues with CHANGELOG.md
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.35] - 2026-03-29

### Added

#### Release Automation

- **git-cliff Integration**: Automated changelog generation with conventional commits
  - `cliff.toml` configuration for changelog formatting
  - Groups commits by type: Added, Fixed, Changed, Documentation, Miscellaneous
  - Automatically generates version entries from commit messages

- **release.toml**: Release workflow configuration
  - Consolidated release process configuration
  - Disabled crates.io publishing (publish = false)

### Changed

- **Release Workflow**: Fixed git-cliff workdir path for correct changelog generation
- **Cleanup**: Removed duplicate `generator/CHANGELOG.md` file

## [0.1.34] - 2026-03-28

### Added

#### 404 Page Handling

- **404.html Template**: New template generated during `yew-ssg init`
  - Renders a user-friendly 404 page with site styling
  - Available at `/404.html` in the output directory
  - Uses the same base template as other pages

- **Development Server 404 Support**: `serve` command now serves 404.html for unknown routes
  - Falls back to 404.html content with 404 status code
  - Works with the live reload WebSocket connection
  - Provides better development experience when testing navigation

### Fixed

- **Section Content Rendering**: Section templates now properly render content
  - Added `description` field to `SectionContext`
  - Added `content` field to `SectionContext` for rendered markdown content
  - Section `_index.md` files now correctly populate template variables

- **Output Directory Creation**: `robots.txt`, `sitemap.xml`, and feed files now properly create output directories
  - `write_robots()`, `write_sitemap()`, and `write_feeds()` now call `fs::create_dir_all()`
  - Previously failed if the output directory didn't exist before these stages

### Changed

- **Clippy Lint Fixes**: Resolved all clippy warnings across the codebase
  - Updated dependencies to latest versions
  - Improved code quality and idiomatic Rust patterns

- **Template Variable Rendering**: Refined template context serialization for cleaner output

## [0.1.29] - 2026-03-22

### Added

#### Page Permalinks

- **`page.permalink` Field**: Pre-computed absolute URL in template context
  - Combines `site.base_url` and `page.path` with proper slash handling
  - No double slashes or missing slashes (e.g., `https://example.com/about/`)
  - Available in all templates via `{{ page.permalink }}`
  - Useful for canonical tags: `<link rel="canonical" href="{{ page.permalink }}" />`
  - Useful for Open Graph meta tags: `<meta property="og:url" content="{{ page.permalink }}" />`

- **`compute_permalink` Function**: Public helper for URL construction
  - Handles edge cases: trailing slashes on base_url, leading slashes on path
  - Exported from `yew_ssg_lib::templates` module

### Changed

- **Feed Generation**: Now uses `compute_permalink` for consistent URL building
- **Sitemap Generation**: Now uses `compute_permalink` for consistent URL building
- **Documentation**: Updated templates.md and api-reference.md with permalink field

## [0.1.28] - 2026-03-22

### Added

#### Internal Links

- **Internal Link Resolution**: Reference other pages by content file path with build-time validation
  - `@/path/to/file.md` syntax in markdown links
  - Links are resolved to actual URL paths at build time (e.g., `@/about.md` → `/about/`)
  - Build fails with clear error if target doesn't exist
  - Works with nested paths (e.g., `@/blog/first-post.md`)

- **Error Handling**: New `BrokenInternalLink` error variant
  - Clear error messages: `Broken internal link in 'blog/my-post.md': target '@/missing.md' not found`
  - Helps catch broken links before they reach production

- **Route Registry Helper**: New `find_by_content_file` method
  - Looks up routes by their source content file path
  - Used internally for internal link resolution

### Changed

- **Build Pipeline**: Internal link resolution happens in `process_content` before markdown-to-HTML conversion
- **Documentation**: Added internal links section to content documentation

## [0.1.27] - 2026-03-22

### Added

#### Sitemap Generation

- **Sitemap.xml Generation**: Automatic sitemap generation for SEO optimization
  - All routes (pages and sections) are included
  - Draft pages are excluded from the sitemap
  - URLs constructed from `base_url` + route path
  - `<lastmod>` dates included when pages have a `date` field
  - Priority: home=1.0, sections=0.8, pages=0.7
  - Changefreq: weekly for home, monthly for others

- **Build Pipeline Integration**: New Stage 7 for sitemap generation
  - Runs after robots.txt generation
  - Returns `GeneratedSitemap` with content and URL count
  - Respects `dry_run` mode

#### Robots.txt Generation

- **Robots.txt Generation**: Automatic robots.txt for search engine crawlers
  - Default content allows all crawlers
  - Includes sitemap URL reference
  - Skipped if `static/robots.txt` exists (custom robots.txt preserved)

- **Build Pipeline Integration**: New Stage 6 for robots.txt generation
  - Runs after page rendering
  - Respects `dry_run` mode

### Changed

- **Build Pipeline**: Added Stages 6 and 7 (now 11 stages total)
  - Stage 6: Generate robots.txt
  - Stage 7: Generate sitemap.xml
  - Feeds and assets shifted to stages 9 and 10
- **BuildReport**: Added `sitemap_urls` field to track sitemap entry count

## [0.1.26] - 2026-03-22

### Added

#### Co-located Assets

- **Co-located Asset Copying**: Non-Markdown files in the content directory are automatically copied to the output directory
  - Images, PDFs, and other assets can be placed alongside content
  - Relative paths are preserved: `content/blog/photo.jpg` → `dist/blog/photo.jpg`
  - Markdown files (`.md`) are skipped (they become HTML pages)
  - Directory structure is preserved
  - Parent directories are created automatically

- **Build Pipeline Integration**: New Stage 4 for co-located asset copying
  - Runs between content processing and page rendering
  - Returns `AssetReport` for build report aggregation
  - Respects `dry_run` mode

### Changed

- **Build Pipeline**: Added Stage 4 for co-located asset copying (now 9 stages total)
- **Build Report**: Co-located assets are included in the asset report

## [0.1.24] - 2026-03-22

### Added

#### Pagination

- **Section Pagination**: Split large content collections across multiple pages
  - `sort_by` field in frontmatter: Sort by `date`, `weight`, or `title` (default: `date`)
  - `paginate_by` field: Number of items per page (e.g., `paginate_by = 10`)
  - `paginate_template` field: Custom template for pagination pages
  - `weight` field: Weight value for weight-based sorting
  - `updated` field: Update date for feed entries

- **Pagination Types**:
  - `PaginationConfig`: Configuration for pagination behavior
  - `SortBy` enum: Date, Weight, Title variants with serde support
  - `PaginationInfo`: Metadata for a paginated page (current, total, prev/next URLs)
  - `PaginatedSlice`: A single page's worth of content with metadata
  - `Paginator`: Core type that splits pages into slices and generates URLs

- **Pagination URLs**: Clean URL structure for paginated pages
  - First page: `/blog/`
  - Subsequent pages: `/blog/page/2/`, `/blog/page/3/`, etc.
  - Previous/next navigation URLs included in context

- **Template Context**: `PaginationContext` available in section templates
  - `current`: Current page number
  - `total`: Total number of pages
  - `per_page`: Items per page
  - `total_items`: Total items across all pages
  - `prev`: URL to previous page (if any)
  - `next`: URL to next page (if any)
  - `first`: URL to first page
  - `last`: URL to last page

#### RSS/Atom Feeds

- **Feed Generation**: Automatic RSS 2.0 and Atom feed generation
  - RSS 2.0 feed at `/rss.xml`
  - Atom feed at `/atom.xml`
  - Configurable feed paths and titles
  - Optional full content or summary-only feeds

- **Feed Configuration** (`site.toml`):
  - `feed.rss_enabled`: Enable/disable RSS feed (default: true)
  - `feed.atom_enabled`: Enable/disable Atom feed (default: true)
  - `feed.limit`: Maximum number of entries (default: 20)
  - `feed.full_content`: Include full content vs summary (default: false)
  - `feed.title`: Custom feed title (defaults to site name)
  - `feed.rss_path`: RSS feed path (default: `rss.xml`)
  - `feed.atom_path`: Atom feed path (default: `atom.xml`)

- **Feed Types**:
  - `FeedConfig`: Configuration for feed generation
  - `FeedEntry`: Single feed entry with title, url, summary, content, dates, author, tags
  - `FeedGenerator`: Main type with `generate_rss()` and `generate_atom()` methods

- **Feed Format Support**:
  - RSS 2.0 with RFC 2822 date formatting
  - Atom with RFC 3339 date formatting
  - XML escaping for special characters
  - Category/tag support in feed entries

### Changed

- **Build Pipeline**: Added Stage 6 for feed generation (now 8 stages total)
- **SectionContext**: Added `pagination: Option<PaginationContext>` field
- **Frontmatter**: Added `sort_by`, `paginate_by`, `paginate_template`, `weight`, `updated` fields

## [0.1.23] - 2026-03-22

### Added

#### Blog Post Enhancements

- **Summary/Excerpt Support**: Automatic and manual summary extraction for blog posts
  - `summary` field in frontmatter for explicit summaries
  - Automatic extraction from first paragraph
  - `<!-- more -->` marker support for manual summary breaks
  - Frontmatter summary takes precedence over automatic extraction
  - Markdown formatting stripped from summaries

- **Reading Time and Word Count**: Content metrics for blog posts
  - `word_count` field: Counts words in content (200 words per minute baseline)
  - `reading_time` field: Estimated reading time in minutes
  - Available in templates via `page.word_count` and `page.reading_time`

- **Slug Customization**: Custom URL paths for content
  - `slug` field in frontmatter for custom URL paths
  - Example: `slug = "my-custom-url"` produces `/my-custom-url/` instead of `/original-filename/`
  - Works with aliases for URL migration

#### Taxonomies

- **Tags, Categories, and Series**: Content organization system
  - `tags`: Multiple tags per page (e.g., `tags = ["rust", "web", "tutorial"]`)
  - `categories`: Multiple categories per page (e.g., `categories = ["programming", "tutorials"]`)
  - `series`: Single series assignment (e.g., `series = "Learning Rust"`)
  - All taxonomy fields are optional and default to empty/none

- **Taxonomy Data Structures**:
  - `TaxonomyKind` enum: Tag, Category, Series variants
  - `TaxonomyTerm` struct: Term name, slug, page count, associated pages
  - `TaxonomyMap` struct: Aggregates all taxonomies from processed pages
  - Automatic slugification of taxonomy terms for URL-friendly paths

- **Taxonomy Page Rendering**:
  - `build_taxonomy_map()`: Aggregates taxonomies from all pages
  - `render_taxonomy_pages()`: Renders taxonomy list and term pages
  - `write_taxonomy_pages()`: Writes taxonomy pages to output
  - Template contexts: `TaxonomyTermContext` and `TaxonomyListContext`
  - Taxonomy pages only rendered if corresponding templates exist

### Changed

- **PageContext**: Added `tags`, `categories`, `series`, `summary`, `word_count`, `reading_time` fields
- **Frontmatter**: Added `summary`, `slug`, `tags`, `categories`, `series` fields
- **Build Pipeline**: Added Stage 5 for taxonomy page rendering (now 7 stages total)

## [0.1.22] - 2026-03-22

### Added

#### Development Server with Hot Reloading

- **`serve` command**: New command for local development with live reload support
  - `yew-ssg serve`: Starts a development server on port 3000
  - `--port` / `-p`: Custom port selection
  - `--site-dir`: Serve from a different directory
  - `--open`: Automatically open browser on start
  - WebSocket-based live reload with instant browser refresh on file changes

- **File watching**: Automatic rebuild on content, template, style, config, and static file changes
  - Uses `notify` 6.x for cross-platform file system watching
  - 100ms debounce to batch rapid changes
  - Change categorization (Content, Template, Style, Static, Config, Unknown)

- **Browser integration**:
  - Live reload script injected into HTML pages
  - Error overlay displays build failures in the browser
  - Graceful error recovery when issues are fixed

- **Server features**:
  - Built on `axum` 0.8 (hyper-based) with WebSocket support
  - Custom `HtmlInjectService` for proper Content-Type handling
  - Dedicated `/favicon.ico` route with caching headers
  - Graceful shutdown via Ctrl+C

- **Dependencies added**:
  - `axum = "0.8"` with WebSocket support
  - `axum-extra = "0.10"` with typed-header feature
  - `tower = "0.5"` for service utilities
  - `tower-http = "0.6"` for filesystem serving
  - `tokio-stream = "0.1"` for async streams
  - `notify = "6.1"` for file watching
  - `futures-util = "0.3"` for async utilities
  - `webbrowser = "1.0"` for cross-platform browser opening

### Changed

- **`init` command output**: Now recommends `yew-ssg serve --open` after scaffolding

## [0.1.21] - 2026-03-21

### Added

#### Structured Logging with Tracing

- **`tracing` crate integration**: Replaced `println!`/`eprintln!` with structured logging
  - Added `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` dependencies
  - Created `generator/src/tracing.rs` module with `init()` and `init_with_level()` functions
  - Supports `RUST_LOG` environment variable for runtime log level configuration

- **CLI log level flags**:
  - `--verbose` / `-v`: Enables debug level logging for detailed output
  - `--quiet` / `-q`: Suppresses all but error messages
  - Default: Info level logging

- **Instrumented modules** with `#[instrument]` attributes and structured spans:
  - `generator/src/build/builder.rs`: Build stage logging (discover_routes, load_templates, process_content, render_pages, process_assets, write_output)
  - `generator/src/build/pipeline.rs`: Page rendering with `debug_span!`
  - `generator/src/build/report.rs`: Build summary and warnings
  - `generator/src/init/mod.rs`: Site initialization logging
  - `generator/src/routes/discovery.rs`: Route discovery with content directory context
  - `generator/src/assets/static_files.rs`: Static file copying operations
  - `generator/src/assets/styles.rs`: SCSS compilation operations

- **Structured error logging**:
  - `render_error()` now emits `tracing::error!` with structured fields
  - Includes error type, error message, and contextual hints

## [0.1.20] - 2026-03-21

### Added

#### Init Command Islands Flag

- **`--islands` flag for `init` command**: Scaffold sites with or without Yew/WASM hydration support
  - `yew-ssg init my-site`: Creates a plain SSG site (no WASM hydration script in templates)
  - `yew-ssg init my-site --islands`: Creates a site with islands support (includes WASM hydration script)
  - `InitOptions` struct now has `islands: bool` field with `with_islands()` builder method
  - `InitScaffolder` conditionally includes WASM hydration script in `base.html` based on flag

### Fixed

#### Clippy Warnings Resolved

- **`should_implement_trait`** in `generator/src/content/frontmatter.rs`:
  - Changed custom `from_str` method to implement `std::str::FromStr` trait

- **`result_large_err`** in `generator/src/bin/main.rs`:
  - Added `#[allow(clippy::result_large_err)]` to `run_build`, `run_clean`, `run_init`, `run_routes`

- **`ptr_arg`** in `generator/src/bin/main.rs`:
  - Changed `&PathBuf` parameters to `&Path` for idiomatic Rust

- **`field_reassign_with_default`** in `generator/src/build/report.rs`:
  - Fixed struct initialization to use `..Default::default()` syntax

- **`items_after_test_module`** in `generator/src/content/mod.rs`:
  - Moved `MockContentSource` definition before the test module

## [0.1.18] - 2026-03-20

### Added

#### Islands Architecture — Yew SSR + WASM Hydration

- **Counter Island Component** (`common/src/components/counter.rs`):
  - Yew `#[function_component(Counter)]` with `CounterProps { initial: i32 }`
  - Props derive `Serialize + Deserialize` for JSON round-trip (SSR → static HTML → hydration)
  - Renders `<div class="counter"><span>N</span><button>+</button></div>`
  - Increments count on click via `use_state`

- **`render_island_counter()` in `generator/src/build/pipeline.rs`**:
  - Renders the `Counter` component server-side at build time using `yew::ServerRenderer`
  - Creates a self-contained `tokio::runtime::Builder::new_current_thread()` runtime per call
    (avoids requiring an ambient tokio context from the synchronous `fn main()`)
  - Serializes `CounterProps` to JSON and wraps the SSR output in the hydration mount point:
    `<div data-island="Counter" data-props='{"initial":N}'>...SSR HTML...</div>`

- **`island()` Tera Custom Function** in `generator/src/templates/renderer.rs`:
  - Registered on the `TeraRenderer` during `from_dir()` construction
  - Callable in any Tera template: `{{ island(component="Counter", initial=3) | safe }}`
  - Dispatches to the appropriate `render_island_*` function by component name string
  - Unknown component names emit `<!-- unknown island: NAME -->` safely

- **`page.html` Default Template** (via `generator/src/init/scaffold.rs`):
  - Now includes `{{ island(component="Counter", initial=3) | safe }}` to demonstrate
    the islands pattern in every newly scaffolded site

- **`base.html` Default Template** (via `generator/src/init/scaffold.rs`):
  - Loads the WASM hydration bundle using the correct ES module pattern:
    ```html
    <script type="module">
      import init, * as bindings from "/wasm/client.js";
      const wasm = await init({ module_or_path: "/wasm/client_bg.wasm" });
      window.wasmBindings = bindings;
    </script>
    ```
  - Note: `wasm-bindgen` output is an ES module; a plain `<script src>` tag causes
    `Unexpected token 'export'` — `type="module"` is required

- **WASM Hydration Client** (`client/src/main.rs`):
  - Implemented `#[wasm_bindgen(start)]` entry point `hydrate_islands()`
  - Uses `#![no_main]` to suppress the implicit Rust binary entry symbol (prevents
    `entry symbol 'main' declared multiple times` error at compile time)
  - On startup: `document.querySelectorAll("[data-island]")` → deserialize `data-props`
    JSON → `yew::Renderer::<C>::with_root_and_props(el, props).hydrate()`
  - Added `hydrate_island()` dispatcher matching component name strings to Yew types

- **`client/index.html`**: New Trunk entry point (required by Trunk 0.21)
  - Contains `<link data-trunk rel="rust" data-wasm-opt="z" data-bin="client" />`

- **`client/Trunk.toml`**: Trunk build configuration
  - `dist = "../dist/wasm"` — writes WASM assets into a dedicated subdirectory,
    preventing Trunk's shell `index.html` from overwriting SSG-generated pages
  - `public_url = "/wasm/"` — ES module paths resolve to `/wasm/client.js`
  - `filehash = false` — disables content hashing for predictable filenames
    (`client.js`, `client_bg.wasm`) that match the hardcoded template references

### Changed

- **`common/src/components/mod.rs`**: Now only exports `counter` module
  (old placeholder components `home`, `about`, `layout`, `page` removed as they
  were unused and not hooked into the generator pipeline)

- **`common/Cargo.toml`**: Added `serde = { workspace = true }` dependency
  (required for `CounterProps` serialization)

- **`client/Cargo.toml`**: Added dependencies for WASM client:
  - `yew = { version = "0.23", features = ["hydration"] }`
  - `serde_json = "1.0"`
  - `wasm-bindgen = "0.2"`
  - `web-sys` with features: `Document`, `Window`, `HtmlElement`, `DomStringMap`,
    `NodeList`, `Element`

### Removed

- **`generator/src/init/templates.rs`** (`DefaultTemplates` struct):
  - Deleted as dead code — `InitScaffolder` in `scaffold.rs` never called it;
    it maintained its own duplicate template strings
  - All functional test coverage now lives in `scaffold.rs` unit tests and
    `generator/tests/init_scaffolding.rs` integration tests (no coverage lost)
  - `pub use init::DefaultTemplates` removed from `generator/src/lib.rs`

## [0.1.17] - 2026-03-19

### Added

#### CLI Improvements

- **`clean` Subcommand**: New standalone subcommand to remove the output directory
  - `yew-ssg clean`: Delete generated files from the output directory
  - `yew-ssg clean --dir PATH`: Clean a site in a different directory
  - Previously, cleaning required `yew-ssg build --clean` which also triggered a build

- **`routes` Subcommand**: New diagnostic command to inspect discovered routes
  - `yew-ssg routes`: List all routes that would be generated from the content directory
  - Displays a sorted table of URL path → source file → output file
  - Prints counts of total routes, pages, and sections
  - Useful for debugging route discovery without running a full build

- **`--quiet` / `-q` Flag on `build`**: Suppress all output except errors
  - Mutually exclusive with `--verbose`
  - Build summary is not shown; only errors are printed to stderr

- **`--output` / `-o PATH` Flag on `build`**: Override the output directory at runtime
  - Avoids editing `site.toml` just to preview output in a temporary location
  - Works with all other flags: `yew-ssg build --output /tmp/preview --dry-run`

- **Richer Help Text**: All subcommands and arguments now have expanded descriptions
  - Root command includes a "Quick start" block
  - Each subcommand has a `long_about` with a description and usage examples
  - Every argument explains _what_ it is and _when_ to use it

- **Actionable Error Messages**: Errors now include a contextual `Hint:` line
  - `Config::NotFound` → suggests `yew-ssg init` or `--dir`
  - `Build::NoContent` → suggests adding `.md` files to `content/`
  - `Template::NotFound` / `Template::DirNotFound` → suggests checking `templates/`
  - `Init::Cancelled` → silent (intentional user action)

#### InitReport Enhancements

- **`created_dirs: Vec<PathBuf>`**: Tracks every directory created during `init`
- **`created_files: Vec<PathBuf>`**: Tracks every file created during `init`
- **`print_summary()`**: Now enumerates each created directory and file by path
  - Opens with `✓ Site initialized at <path>/`
  - Shows `cd <path>` as the first next step

### Changed

- **`BuildReport::print_summary()`** ([`generator/src/build/report.rs`](generator/src/build/report.rs)):
  - Header now includes build status (`✓ Build complete` / `⚠ Build completed with warnings`) and duration
  - All stat labels use a fixed-width column for consistent alignment
  - Warnings shown in a separate section below the separator

- **`generator/src/bin/main.rs`**: Refactored `run_build` to load config first so `--output` can be applied before cleaning

- **`generator/src/init/scaffold.rs`**: All file/directory creation helpers now push to `InitReport.created_dirs` and `InitReport.created_files`

## [0.1.16] - 2026-03-19

### Changed

- **Documentation and project cleanup**
  - Removed placeholder `content/pages/home.md` and `content/pages/about.md` (superseded by the `init` scaffolding workflow)
  - Removed `styles/styles.scss` (workspace-root stylesheet replaced by per-site `styles/main.scss`)
  - Updated `docs/src/getting-started.md` to reflect current build workflow
  - Removed `plans/cli-improvements.md` after the plan was implemented in 0.1.17

## [0.1.15] - 2026-03-19

### Added

- **GitHub Actions CI/CD** — restored and corrected workflow files
  - `.github/workflows/doc.yml`: Deploy mdBook documentation
  - `.github/workflows/security.yml`: `cargo audit` dependency security scan
  - Fixed typo in directory name from earlier attempt (`.github/worflows/` → `.github/workflows/`)
  - Added `target/` to `.gitignore`

## [0.1.14] - 2026-03-18

### Fixed

- **Static file scaffolding** (`generator/src/init/scaffold.rs`):
  - `create_static_files()` now generates both `static/scripts.js` and an embedded minimal `static/favicon.png` (1×1 transparent PNG bytes, no external dependencies)
  - Previously these files were not created during `yew-ssg init`, leaving the base template with a broken favicon and missing scripts reference
  - Integration test `test_scaffold_creates_static_files` added to cover both files

- **Removed workspace-level static assets**:
  - `static/favicon.png` and `static/scripts.js` deleted from the workspace root
  - These files were leftover from the pre-scaffolding era; they are now generated per-site by `yew-ssg init`

## [0.1.13] - 2026-03-18

### Fixed

- **Template loading precedence** (`generator/src/templates/renderer.rs`):
  - Base templates (those without `{% extends %}`) are now sorted and registered before child templates
  - Previously, Tera could fail to find a parent template if a child was registered first
  - Fix: collect all `.html` files, sort by whether they contain `{% extends %}`, then `add_raw_template` in order

- **Static asset path in build pipeline** (`generator/src/assets/static_files.rs`):
  - Static files are now correctly copied with proper source/destination path handling
  - Previously, files could end up with incorrect relative paths in `dist/static/`

- **Year in `NowContext`** (`generator/src/templates/context.rs`):
  - `NowContext` struct added with `year: i32` field populated via `chrono::Utc::now().year()`
  - `{{ now.year }}` is now available in all Tera templates (used in `base.html` copyright footer)

- **Documentation test fixes** — all `///` doc examples updated to compile against the current library API

### Changed

- **`TeraRenderer::from_dir()`**: Switched from `Tera::new("glob")` to manual `WalkDir` + `add_raw_template` approach for more reliable cross-platform template discovery

## [0.1.12] - 2026-03-18

### Fixed

- **`base.html` template year field** (`generator/src/init/templates.rs`):
  - `{{ year }}` corrected to `{{ now.year }}` to match the `NowContext` variable name

- **SCSS processor include paths** (`generator/src/assets/styles.rs`):
  - `ScssProcessor::with_include_paths()` now correctly propagates paths to `grass::Options`
  - Processing now walks the styles directory and compiles all top-level `.scss` files (those not starting with `_`)

## [0.1.11] - 2026-03-18

### Added

- **GitHub Actions CI/CD** (initial attempt)
  - `.github/worflows/general.yml`: Build + test on push
  - `.github/worflows/security.yml`: `cargo audit`
  - `.github/worflows/docs.yml`: mdBook documentation deployment
  - Note: directory misspelling (`worflows`) caused workflows not to trigger; corrected in 0.1.15

- **`.gitignore` updates**: Added `test-site/` to prevent scaffolded test sites from being committed

## [0.1.8] through ## [0.1.10]

> These versions covered the integration of the init command with the full build pipeline — verifying the end-to-end flow from `yew-ssg init` through `yew-ssg build` to a working static site. Bug fixes were iterative; see individual commits `355fad7`, `666e650`, `5e93e5b` in the git history.

## [0.1.7] - 2026-03-17

### Added

#### Init Command for Site Scaffolding

- **Init Module Structure**: Created initialization module for scaffolding new sites
  - `generator/src/init/mod.rs`: Init module with public API re-exports
  - `generator/src/init/scaffold.rs`: `InitScaffolder` implementation
  - `generator/src/init/templates.rs`: Default template content

- **InitOptions Struct**: Configuration for site initialization
  - `name`: Site name (used in configuration and templates)
  - `base_url`: Base URL for the site
  - `force`: Force initialization in non-empty directories
  - `validate()`: Validate options before scaffolding

- **InitScaffolder Struct**: Main entry point for creating new sites
  - `new()`: Create scaffolder with options
  - `scaffold()`: Create directory structure and files
  - Creates: content/, templates/, static/, styles/ directories
  - Generates: site.toml, \_index.md, base.html, page.html, section.html, main.scss

- **InitReport Struct**: Statistics and results from initialization
  - `directories_created`: Count of directories created
  - `files_created`: Count of files created
  - `print_summary()`: Print initialization summary to stdout

- **Error Types**: Init-specific error handling
  - `InitError::DirectoryNotEmpty`: Directory is not empty
  - `InitError::DirectoryCreation`: Failed to create directory
  - `InitError::FileWrite`: Failed to write file
  - `InitError::Cancelled`: User cancelled the operation
  - `InitError::InvalidName`: Invalid site name
  - `InitError::InvalidBaseUrl`: Invalid base URL

- **CLI Subcommands**: Refactored binary with subcommand structure
  - `yew-ssg init [path]`: Initialize a new site
  - `yew-ssg init --name "My Site"`: Set site name
  - `yew-ssg init --base-url "https://example.com"`: Set base URL
  - `yew-ssg init --force`: Force initialization in non-empty directory
  - `yew-ssg build`: Build the static site (moved to subcommand)

- **User Prompt**: Interactive confirmation for non-empty directories
  - Prompts user before initializing in non-empty directory
  - Simple stdin reading for y/N confirmation

- **Testing Infrastructure**: Comprehensive test coverage
  - 14 unit tests for InitOptions validation
  - 12 unit tests for InitScaffolder methods
  - 7 unit tests for InitError types
  - 14 integration tests for full scaffolding workflow

### Changed

- **generator/src/lib.rs**: Added init module and re-exports
  - Re-exports: `InitOptions`, `InitScaffolder`, `InitReport`, `InitError`, `DefaultTemplates`

- **generator/src/error.rs**: Added `InitError` type
  - Added `Init` variant to `GeneratorError`
  - Added 7 unit tests for init error types

- **generator/src/bin/main.rs**: Refactored to use subcommands
  - `Build` subcommand with existing options
  - `Init` subcommand with new options

- **README.md**: Updated documentation
  - Added init command usage examples
  - Added site initialization section
  - Updated build command to use subcommand syntax

## [0.1.6] - 2026-03-17

### Added

#### Generator Library Refactor (Phase 6: Build System)

- **Build Module Structure**: Created build orchestration module for the generator
  - `generator/src/build/mod.rs`: Build module with public API re-exports
  - `generator/src/build/builder.rs`: `SiteBuilder` implementation
  - `generator/src/build/pipeline.rs`: Build pipeline stage functions
  - `generator/src/build/report.rs`: `BuildReport` for statistics and output

- **SiteBuilder Struct**: Main entry point for building static sites
  - `from_dir()`: Create builder from directory containing site.toml
  - `new()`: Create builder from existing SiteConfig
  - `dry_run()`: Enable dry-run mode (no files written)
  - `verbose()`: Enable verbose output
  - `include_drafts()`: Include draft pages in build
  - `build()`: Execute the full build pipeline
  - `clean()`: Clean the output directory

- **BuildReport Struct**: Statistics and results from a build
  - `pages_rendered`: Count of rendered pages
  - `sections_rendered`: Count of rendered sections
  - `drafts_skipped`: Count of skipped drafts
  - `assets`: Asset processing report
  - `duration`: Build duration
  - `warnings`: List of warnings generated
  - `total_files()`: Total files generated
  - `has_warnings()`: Check for warnings
  - `print_summary()`: Print build summary to stdout

- **Build Pipeline Stages**: Individual functions for each build stage
  - `load_config()`: Load configuration from directory
  - `discover_routes()`: Discover routes from content directory
  - `load_templates()`: Load templates from directory
  - `process_content()`: Process content files into HTML
  - `render_pages()`: Render pages with templates
  - `process_assets()`: Process SCSS and static files
  - `write_output()`: Write rendered pages to output

- **Error Types**: Build-specific error handling
  - `BuildError::NoContent`: No content found to build
  - `BuildError::OutputDirCreation`: Failed to create output directory
  - `BuildError::PageRenderFailed`: Page rendering failed
  - `BuildError::ContentProcessing`: Content processing failed
  - `BuildError::AssetProcessing`: Asset processing failed
  - `BuildError::RouteDiscovery`: Route discovery failed

- **CLI with clap**: Refactored binary with argument parsing
  - `--dir`: Directory containing site.toml (default: ".")
  - `--verbose`: Enable verbose output
  - `--include-drafts`: Include draft pages in build
  - `--dry-run`: Dry run mode (no files written)
  - `--clean`: Clean output directory before build

- **Testing Infrastructure**: Comprehensive test coverage
  - 10 unit tests for BuildReport methods
  - 6 unit tests for SiteBuilder configuration
  - 4 unit tests for pipeline markdown processing
  - 10 unit tests for BuildError types

### Changed

- **generator/src/lib.rs**: Added build module and re-exports
  - Re-exports: `SiteBuilder`, `BuildReport`, `BuildError`

- **generator/src/error.rs**: Added `BuildError` type
  - Added `Build` variant to `GeneratorError`
  - Added 10 unit tests for build error types

- **generator/Cargo.toml**: Added clap dependency
  - `clap = { version = "4.5", features = ["derive"] }`

- **generator/src/bin/main.rs**: Complete refactor
  - Replaced monolithic implementation with thin CLI wrapper
  - Uses `SiteBuilder` for build orchestration
  - Added clap for argument parsing

- **generator/src/assets/mod.rs**: Added Clone derive to AssetReport

### Test Coverage

- Unit tests: 179 tests (26 new for build module)
- Integration tests: 59 tests (unchanged)
- Total: 238 tests (179 unit + 59 integration)

## [0.1.5] - 2026-03-17

### Added

#### Generator Library Refactor (Phase 3: Route System)

- **Route Module Structure**: Created route discovery and management module for the generator
  - `generator/src/routes/mod.rs`: Route module with public API re-exports
  - `generator/src/routes/registry.rs`: `RouteKind`, `RouteInfo`, and `RouteRegistry` types
  - `generator/src/routes/discovery.rs`: `RouteDiscovery` implementation

- **RouteKind Enum**: Type distinguishing between page and section routes
  - `Page`: Regular content pages (e.g., `/about/`)
  - `Section`: Section index pages (e.g., `/blog/`)
  - Helper methods: `is_page()`, `is_section()`

- **RouteInfo Struct**: Information about a single route
  - `path`: URL path (e.g., `/about/`)
  - `content_file`: Source file path relative to content directory
  - `output_file`: Output file path relative to output directory
  - `kind`: RouteKind (Page or Section)
  - Path validation ensuring proper `/` prefix and trailing slash

- **RouteRegistry Struct**: HashMap-based storage for all routes
  - `register()`: Add route with duplicate detection
  - `get()`: Retrieve route by path
  - `contains()`: Check if route exists
  - `len()`, `is_empty()`: Count methods
  - `iter()`: Iterate over all routes
  - `pages()`: Iterator over page routes only
  - `sections()`: Iterator over section routes only
  - `generate_rust_manifest()`: Generate client router code (stub for future use)

- **RouteDiscovery Struct**: Discovers routes from content directory
  - `discover()`: Walk content directory and create routes from `.md` files
  - `discover_from_source()`: Discover routes using `ContentSource` trait
  - Automatic `_index.md` detection for section routes
  - Path conversion: `about.md` → `/about/` → `about/index.html`

- **Error Types**: Route-specific error handling
  - `RouteError::NotFound`: Route not found
  - `RouteError::Duplicate`: Duplicate route detected
  - `RouteError::InvalidPath`: Invalid route path format
  - `RouteError::DiscoveryFailed`: Content discovery failed

- **Testing Infrastructure**: Comprehensive test coverage
  - 28 unit tests for route types (registry and discovery)
  - 14 integration tests for route discovery scenarios
  - Tests using existing content fixtures

### Changed

- **generator/src/lib.rs**: Added routes module and re-exports
  - Re-exports: `RouteDiscovery`, `RouteInfo`, `RouteKind`, `RouteRegistry`, `RouteError`

- **generator/src/error.rs**: Added `RouteError` type
  - Added `Route` variant to `GeneratorError`
  - Added 5 unit tests for route error types

- **docs/src/api-reference.md**: Updated with route module documentation
  - Added `RouteKind`, `RouteInfo`, `RouteRegistry` documentation
  - Added `RouteDiscovery` documentation
  - Added route discovery examples

- **README.md**: Updated with route system features
  - Added route discovery to features list
  - Added route registry usage examples

### Test Coverage

- Unit tests: 152 tests (28 new for routes)
- Integration tests: 14 new tests for route discovery
- Total: 211 tests (152 unit + 59 integration)

## [0.1.4] - 2026-03-17

### Added

#### Generator Library Refactor (Phase 5: Asset Processing)

- **Asset Module Structure**: Created asset processing module for the generator
  - `generator/src/assets/mod.rs`: Asset module with `AssetProcessor` trait and `AssetReport`
  - `generator/src/assets/styles.rs`: `ScssProcessor` for compiling SCSS/SASS to CSS
  - `generator/src/assets/static_files.rs`: `StaticCopier` for copying static files

- **AssetProcessor Trait**: Abstraction for asset processing backends
  - `process`: Process assets from source to destination
  - `handles`: Check if processor handles a given file type
  - `name`: Get processor name for logging

- **ScssProcessor Implementation**: SCSS/SASS compilation using grass crate
  - Compile SCSS to CSS with variable resolution
  - Support for `@import` with include paths
  - Optional minification (compressed output style)
  - Automatic `.scss` → `.css` extension change

- **StaticCopier Implementation**: Static file copying
  - Copy files preserving directory structure
  - Exclusion patterns (glob-style: `*.scss`, `**/*.scss`)
  - Binary file support
  - Nested directory handling

- **AssetReport**: Processing statistics and error collection
  - `files_processed`: Count of successfully processed files
  - `files_skipped`: Count of skipped files (excluded)
  - `errors`: List of error messages
  - `merge`: Combine reports from multiple processors

- **Error Types**: Asset-specific error handling
  - `AssetError::NotFound`: Asset file not found
  - `AssetError::Scss`: SCSS compilation error
  - `AssetError::Io`: I/O errors with path context
  - `AssetError::CopyFailed`: File copy failure

- **Testing Infrastructure**: Comprehensive test coverage
  - 21 unit tests for asset processing
  - 14 integration tests for asset processing scenarios
  - Test fixtures for asset site with SCSS and static files

### Changed

- **generator/src/lib.rs**: Added assets module and re-exports
  - Re-exports: `AssetProcessor`, `AssetReport`, `ScssProcessor`, `StaticCopier`, `AssetError`

- **generator/src/error.rs**: Added `AssetError` type
  - Added `Asset` variant to `GeneratorError`
  - Added 5 unit tests for asset error types

- **docs/src/api-reference.md**: Updated with asset module documentation
  - Added `AssetProcessor`, `ScssProcessor`, `StaticCopier` documentation
  - Added `AssetReport` documentation
  - Added asset processing examples

- **README.md**: Updated with asset processing features
  - Added asset processing to features list
  - Added SCSS compilation examples

### Test Coverage

- Unit tests: 108 tests (21 new for assets)
- Integration tests: 14 new tests for asset processing
- Total: 153 tests (108 unit + 45 integration)

## [0.1.3] - 2026-03-17

### Added

#### Generator Library Refactor (Phase 4: Template System)

- **Template Module Structure**: Created template rendering module for the generator
  - `generator/src/templates/mod.rs`: Template module with public API re-exports
  - `generator/src/templates/context.rs`: Context types for template rendering
  - `generator/src/templates/renderer.rs`: `TemplateRenderer` trait and `TeraRenderer` implementation

- **Template Context Types**: Strongly-typed context for template rendering
  - `TemplateContext`: Main context container with builder pattern methods
  - `PageContext`: Page-specific variables (title, description, path, content, date, draft)
  - `SectionContext`: Section-specific variables (title, path, pages list)
  - `SiteContext`: Site-wide configuration (name, base_url, description, author)
  - `extra`: HashMap for custom variables from frontmatter

- **TemplateRenderer Trait**: Abstraction for template backends
  - `render`: Render template with context
  - `register_template`: Add template from string
  - `has_template`: Check template existence
  - `load_templates`: Load templates from directory

- **TeraRenderer Implementation**: Tera-based template rendering
  - Template loading from directories (`**/*.html` glob)
  - Template registration from strings
  - Template inheritance (`{% extends "base.html" %}`)
  - Blocks, loops, and conditionals
  - Filters including `safe` for unescaped HTML
  - Context serialization for Tera

- **Error Types**: Template-specific error handling
  - `TemplateError::NotFound`: Template not found
  - `TemplateError::Render`: Rendering failed
  - `TemplateError::Syntax`: Invalid template syntax
  - `TemplateError::Io`: I/O errors with path context
  - `TemplateError::DirNotFound`: Template directory missing

- **Testing Infrastructure**: Comprehensive test coverage
  - 74 unit tests for error types, context serialization, and template rendering
  - 16 integration tests for template rendering scenarios
  - Test fixtures for template site with base, page, and section templates

- **Dependencies**: Added new dependencies
  - `tera = "1.20"` for template rendering
  - `serde_json = "1.0"` for JSON serialization

### Changed

- **generator/src/lib.rs**: Added templates module and re-exports
  - Re-exports: `TemplateContext`, `PageContext`, `SectionContext`, `SiteContext`, `TemplateRenderer`, `TeraRenderer`, `TemplateError`

- **generator/src/error.rs**: Added `TemplateError` type
  - Added `Template` variant to `GeneratorError`
  - Added 5 unit tests for template error types

- **docs/src/api-reference.md**: Updated with template module documentation
  - Added `TemplateRenderer`, `TeraRenderer` documentation
  - Added context types documentation
  - Added template usage examples

- **README.md**: Updated with template system features
  - Added template system to features list
  - Added template rendering examples

### Test Coverage

- Unit tests: 74 tests covering error types, context serialization, and template rendering
- Integration tests: 16 tests for template rendering scenarios
- Total: 105 tests (74 unit + 31 integration)

## [0.1.2] - 2026-03-16

### Added

#### Generator Library Refactor (Phase 2: Content System)

- **Content Module Structure**: Created content parsing module for the generator
  - `generator/src/content/mod.rs`: Content module with `ContentSource` trait and `FilesystemContentSource`
  - `generator/src/content/frontmatter.rs`: `Frontmatter` type for TOML metadata parsing
  - `generator/src/content/page.rs`: `Page` type for individual content files
  - `generator/src/content/section.rs`: `Section` type for content collections (e.g., blogs)

- **Frontmatter Parsing**: TOML metadata support between `+++` markers
  - `title`: Page title (optional, defaults to empty)
  - `description`: Page description for SEO
  - `date`: Publication date (chrono::NaiveDate) for sorting
  - `template`: Template override (defaults to "page.html")
  - `draft`: Draft status (defaults to false)
  - `extra`: Custom metadata table

- **Page Type**: Individual content file parsing
  - `from_file`: Load page from Markdown file
  - `from_str`: Parse page from string
  - `template()`: Get template name
  - `is_draft()`: Check draft status
  - URL path generation from filename

- **Section Type**: Content collections with multiple pages
  - `from_dir`: Load section from directory
  - `add_page`: Add pages to section
  - `sort_by_date`: Sort pages by date (newest first)
  - Support for `_index.md` section index files

- **ContentSource Trait**: Abstraction for loading content
  - `load`: Load content from path
  - `exists`: Check if content exists
  - `list`: List all content files
  - `FilesystemContentSource`: Default filesystem implementation
  - `MockContentSource`: Test-only mock implementation

- **Error Types**: Content-specific error handling
  - `ContentError::NotFound`: Content file not found
  - `ContentError::InvalidFrontmatter`: TOML parsing errors
  - `ContentError::UnclosedFrontmatter`: Missing closing `+++` marker
  - `ContentError::Io`: I/O errors with path context
  - `ContentError::MissingField`: Required field missing
  - `ContentError::InvalidPath`: Invalid content path

- **Testing Infrastructure**: Comprehensive test coverage
  - 43 unit tests for content types and error handling
  - 9 integration tests for content loading
  - 8 doc tests for code examples
  - Test fixtures for content site with pages and blog section

- **Dependencies**: Added new dependencies
  - `chrono = "0.4"` with serde feature for date handling
  - `walkdir = "2.5"` for recursive content discovery

### Changed

- **generator/src/lib.rs**: Added content module and re-exports
  - Re-exports: `ContentSource`, `FilesystemContentSource`, `Frontmatter`, `Page`, `Section`, `ContentError`

- **generator/src/error.rs**: Added `ContentError` type
  - Added `Content` variant to `GeneratorError`
  - Added `UnclosedFrontmatter` variant for better error messages

- **docs/src/api-reference.md**: Updated with content module documentation
  - Added `Frontmatter`, `Page`, `Section`, `ContentSource` documentation
  - Added usage examples for content loading
  - Updated error handling examples

- **docs/src/content.md**: Updated with frontmatter and content API documentation
  - Documented all frontmatter fields
  - Added examples for pages, sections, and blog posts
  - Added content API usage examples

- **README.md**: Updated with content system features
  - Added content system to features list
  - Added content loading examples
  - Added frontmatter format documentation

### Test Coverage

- Unit tests: 43 tests covering content types, frontmatter parsing, and error handling
- Integration tests: 9 tests for content loading from filesystem
- Doc tests: 8 tests for code examples in documentation

## [0.1.1] - 2026-03-16

### Added

#### Generator Library Refactor (Phase 1)

- **Library Module Structure**: Created proper library module structure for the generator
  - `generator/src/lib.rs`: Library entry point with public API re-exports
  - `generator/src/error.rs`: Error types using `thiserror` for idiomatic error handling
  - `generator/src/config.rs`: Configuration types and loading functionality

- **Error Handling**: Comprehensive error type hierarchy
  - `GeneratorError`: Main error type for the library
  - `ConfigError`: Configuration-specific errors (NotFound, Invalid, Parse, MissingField)
  - `Result<T>`: Convenient result alias for generator operations

- **Configuration Types**: Strongly-typed configuration for site settings
  - `SiteConfig`: Main configuration type with `from_file`, `from_dir`, `new`, and `validate` methods
  - `SiteMeta`: Site metadata (name, base_url, description, author)
  - `BuildConfig`: Build settings with sensible defaults (content_dir, output_dir, etc.)

- **Testing Infrastructure**: Comprehensive test coverage
  - 13 unit tests for error types and configuration
  - 6 integration tests for configuration loading
  - 3 doc tests for code examples
  - Test fixtures for minimal and full site configurations

- **Documentation**: mdbook-based documentation in `docs/` directory
  - Introduction and getting started guides
  - Project structure and configuration documentation
  - Generator library API reference
  - Content, templates, and styling guides
  - Development workflow documentation

### Changed

- **generator/Cargo.toml**: Added `[lib]` section and new dependencies
  - Added `thiserror = "2.0"` for error handling
  - Added `tempfile = "3.10"` for test fixtures
  - Added `features = ["derive"]` to serde dependency

- **README.md**: Updated to reflect library usage
  - Added generator library section with usage example
  - Added configuration documentation
  - Updated project structure to show new library files
  - Updated workspace crates description

### Test Coverage

- Unit tests: 13 tests covering error types and configuration parsing
- Integration tests: 6 tests for configuration loading from files
- Doc tests: 3 tests for code examples in documentation

## [0.1.0] - 2026-03-15

### Added

- Initial project structure with Yew SSG (Static Site Generator) setup
- Workspace configuration with three crates: `client`, `common`, and `generator`
- Basic components for page rendering:
  - Home page component
  - About page component
  - Layout component
  - Page component
- Static site generator binary for building the site
- Content management with markdown pages for home and about
- Static assets including favicon and scripts
- SCSS styling support
- HTML template for site generation
- Project documentation (README.md)
- License file
- Git ignore configuration

[0.1.0]: https://github.com/crustyrustacean/yew-ssg/releases/tag/v0.1.0
