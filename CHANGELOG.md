# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
