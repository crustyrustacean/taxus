# Error Handling

The generator library uses `thiserror` for idiomatic error handling with comprehensive error types.

## Error Types

### GeneratorError

The main error type for the library:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
```

### ConfigError

Configuration-specific errors:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Failed to parse configuration: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Missing required field '{field}' in configuration")]
    MissingField { field: &'static str },
}
```

## Result Type

The library provides a convenient `Result` alias:

```rust
pub type Result<T> = std::result::Result<T, GeneratorError>;
```

## Handling Errors

### Basic Error Handling

```rust
use generator::{SiteConfig, Result};

fn load_config() -> Result<()> {
    let config = SiteConfig::from_dir(".")?;
    println!("Loaded: {}", config.site.name);
    Ok(())
}
```

### Matching Specific Errors

```rust
use generator::{SiteConfig, GeneratorError, ConfigError};

fn handle_errors() {
    match SiteConfig::from_dir("./missing") {
        Ok(config) => {
            println!("Loaded: {}", config.site.name);
        }
        Err(GeneratorError::Config(ConfigError::NotFound(path))) => {
            eprintln!("Configuration file not found: {}", path.display());
        }
        Err(GeneratorError::Config(ConfigError::Parse(e))) => {
            eprintln!("Failed to parse configuration: {}", e);
        }
        Err(GeneratorError::Config(ConfigError::MissingField { field })) => {
            eprintln!("Missing required field: {}", field);
        }
        Err(GeneratorError::Io { path, source }) => {
            eprintln!("I/O error on {}: {}", path.display(), source);
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        }
    }
}
```

### Converting Errors

Errors automatically convert using `?`:

```rust
use generator::{SiteConfig, Result, ConfigError};

fn load_and_validate() -> Result<()> {
    let config = SiteConfig::from_dir(".")?;
    
    // This returns ConfigError, which converts to GeneratorError
    config.validate()?;
    
    Ok(())
}
```

## Error Messages

All error types implement `Display` with user-friendly messages:

```rust
let err = ConfigError::NotFound(PathBuf::from("site.toml"));
assert_eq!(
    err.to_string(),
    "Configuration file not found: site.toml"
);

let err = ConfigError::MissingField { field: "site.name" };
assert_eq!(
    err.to_string(),
    "Missing required field 'site.name' in configuration"
);
```

## Best Practices

### Use the Result Alias

```rust
// Good
fn my_function() -> Result<()> {
    // ...
}

// Avoid
fn my_function() -> std::result::Result<(), GeneratorError> {
    // ...
}
```

### Propagate Errors

```rust
fn load_config() -> Result<SiteConfig> {
    // Let errors propagate with ?
    let config = SiteConfig::from_dir(".")?;
    config.validate()?;
    Ok(config)
}
```

### Handle Specific Cases

```rust
fn safe_load() -> SiteConfig {
    match SiteConfig::from_dir(".") {
        Ok(config) => config,
        Err(GeneratorError::Config(ConfigError::NotFound(_))) => {
            // Return default config if not found
            SiteConfig::new("Default", "https://example.com")
        }
        Err(e) => {
            eprintln!("Warning: {}", e);
            SiteConfig::new("Default", "https://example.com")
        }
    }
}
```

## Testing Errors

The library includes tests for error scenarios:

```rust
#[test]
fn test_config_error_not_found_display() {
    let err = ConfigError::NotFound(PathBuf::from("site.toml"));
    let msg = err.to_string();
    assert!(msg.contains("site.toml"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_missing_field_error() {
    let err = ConfigError::MissingField { field: "site.name" };
    let msg = err.to_string();
    assert!(msg.contains("site.name"));
}
```
