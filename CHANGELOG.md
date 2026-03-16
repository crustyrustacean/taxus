# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
