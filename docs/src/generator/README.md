# Generator Library

The generator is available as a reusable Rust library for programmatic static site generation.

## Overview

The generator library provides:

- **Configuration types**: Load and validate site configuration
- **Error handling**: Comprehensive error types with `thiserror`
- **Extensibility**: Build custom SSG solutions on top of the library

## Usage

### Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
generator = { path = "path/to/generator" }
```

### Basic Example

```rust
use generator::{SiteConfig, Result};

fn main() -> Result<()> {
    // Load configuration from current directory
    let config = SiteConfig::from_dir(".")?;
    
    // Validate the configuration
    config.validate()?;
    
    // Use configuration values
    println!("Building site: {}", config.site.name);
    println!("Output directory: {:?}", config.build.output_dir);
    
    Ok(())
}
```

## Public API

The library exports the following types:

```rust
// Re-exports from the library
pub use config::{BuildConfig, SiteConfig, SiteMeta};
pub use error::{GeneratorError, Result};
```

### SiteConfig

Main configuration type representing site settings.

```rust
let config = SiteConfig::new("My Site", "https://example.com");
// or
let config = SiteConfig::from_file("site.toml")?;
// or
let config = SiteConfig::from_dir("./mysite")?;
```

### BuildConfig

Build settings with sensible defaults.

```rust
let build = BuildConfig::default();
assert_eq!(build.content_dir, PathBuf::from("content"));
assert_eq!(build.output_dir, PathBuf::from("dist"));
```

### SiteMeta

Site metadata from the `[site]` section.

```rust
let config = SiteConfig::from_dir(".")?;
println!("Name: {}", config.site.name);
println!("URL: {}", config.site.base_url);
```

## Error Handling

The library uses `thiserror` for idiomatic error handling:

```rust
use generator::{GeneratorError, ConfigError};

fn handle_errors() {
    match SiteConfig::from_dir("./missing") {
        Ok(config) => println!("Loaded: {}", config.site.name),
        Err(GeneratorError::Config(ConfigError::NotFound(path))) => {
            eprintln!("Config not found: {}", path.display());
        }
        Err(GeneratorError::Config(ConfigError::Parse(e))) => {
            eprintln!("Parse error: {}", e);
        }
        Err(GeneratorError::Io { path, source }) => {
            eprintln!("IO error on {}: {}", path.display(), source);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

See [Error Handling](./error-handling.md) for more details.

## Architecture

The library is organized into modules:

| Module | Purpose |
|--------|---------|
| `config` | Configuration types and loading |
| `error` | Error types and Result alias |

Future phases will add:

| Module | Purpose |
|--------|---------|
| `content` | Page and section types |
| `routes` | Route discovery and registry |
| `templates` | Template rendering |
| `assets` | Asset processing (SCSS, static files) |
| `build` | Build orchestration |

## Testing

The library includes comprehensive tests:

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test config_loading
```

See the [API Reference](../api-reference.md) for detailed documentation.
