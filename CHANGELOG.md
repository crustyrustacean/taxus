# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
  - Generates: site.toml, _index.md, base.html, page.html, section.html, main.scss

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
