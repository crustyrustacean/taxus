# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
