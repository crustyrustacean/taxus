# API Reference

This page documents the public API of the generator library.

## Modules

### `config`

Configuration types for loading and representing site configuration.

#### `SiteConfig`

Main configuration type.

```rust
pub struct SiteConfig {
    pub site: SiteMeta,
    pub build: BuildConfig,
}
```

##### Methods

| Method | Description |
|--------|-------------|
| `from_file<P: AsRef<Path>>(path: P) -> Result<Self>` | Load configuration from a file |
| `from_dir<P: AsRef<Path>>(dir: P) -> Result<Self>` | Load configuration from a directory |
| `new(name: impl Into<String>, base_url: impl Into<String>) -> Self` | Create a new configuration |
| `validate(&self) -> Result<()>` | Validate the configuration |

#### `SiteMeta`

Site metadata.

```rust
pub struct SiteMeta {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub author: Option<String>,
}
```

#### `BuildConfig`

Build configuration.

```rust
pub struct BuildConfig {
    pub content_dir: PathBuf,
    pub output_dir: PathBuf,
    pub static_dir: PathBuf,
    pub styles_dir: PathBuf,
    pub templates_dir: PathBuf,
}
```

Implements `Default` with sensible defaults.

### `error`

Error types for the library.

#### `GeneratorError`

Main error type.

```rust
pub enum GeneratorError {
    Config(ConfigError),
    Io { path: PathBuf, source: std::io::Error },
}
```

Implements `std::error::Error` and `Display`.

#### `ConfigError`

Configuration errors.

```rust
pub enum ConfigError {
    NotFound(PathBuf),
    Invalid(String),
    Parse(toml::de::Error),
    MissingField { field: &'static str },
}
```

#### `Result`

Result alias for generator operations.

```rust
pub type Result<T> = std::result::Result<T, GeneratorError>;
```

## Re-exports

The library re-exports commonly used types:

```rust
pub use config::{BuildConfig, SiteConfig, SiteMeta};
pub use error::{GeneratorError, Result};
```

## Usage Examples

### Loading Configuration

```rust
use generator::{SiteConfig, Result};

fn main() -> Result<()> {
    // From a specific file
    let config = SiteConfig::from_file("site.toml")?;
    
    // From a directory
    let config = SiteConfig::from_dir(".")?;
    
    // Programmatic creation
    let config = SiteConfig::new("My Site", "https://example.com");
    
    Ok(())
}
```

### Validating Configuration

```rust
use generator::{SiteConfig, ConfigError, GeneratorError};

fn validate_site() -> Result<(), String> {
    let config = SiteConfig::from_dir(".")
        .map_err(|e| format!("Failed to load: {}", e))?;
    
    config.validate()
        .map_err(|e| format!("Invalid config: {}", e))?;
    
    Ok(())
}
```

### Error Handling

```rust
use generator::{SiteConfig, GeneratorError, ConfigError};

fn load_config() {
    match SiteConfig::from_dir(".") {
        Ok(config) => {
            println!("Site: {}", config.site.name);
        }
        Err(GeneratorError::Config(ConfigError::NotFound(path))) => {
            eprintln!("Config not found: {}", path.display());
        }
        Err(GeneratorError::Config(ConfigError::Parse(e))) => {
            eprintln!("Parse error: {}", e);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

### Accessing Configuration Values

```rust
use generator::SiteConfig;

let config = SiteConfig::from_dir(".")?;

// Site metadata
println!("Name: {}", config.site.name);
println!("URL: {}", config.site.base_url);

if let Some(desc) = &config.site.description {
    println!("Description: {}", desc);
}

// Build configuration
println!("Content: {:?}", config.build.content_dir);
println!("Output: {:?}", config.build.output_dir);
```

## Feature Flags

The library currently has no feature flags. Future versions may add:

- `async`: Async API support
- `cli`: CLI utilities
- `serve`: Development server

## Versioning

The library follows [Semantic Versioning](https://semver.org/):

- **Major**: Breaking API changes
- **Minor**: New features, backward compatible
- **Patch**: Bug fixes, backward compatible

## Stability

The current API is considered **unstable** and may change between versions.

The following are considered stable and will not change in minor versions:

- Error type hierarchy
- `SiteConfig::from_file` and `from_dir` signatures
- `BuildConfig::default()` behavior
