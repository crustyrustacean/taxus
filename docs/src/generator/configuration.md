# Configuration Types

The generator library provides strongly-typed configuration types for site settings.

## SiteConfig

The main configuration type representing the complete site configuration.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `site` | `SiteMeta` | Site metadata |
| `build` | `BuildConfig` | Build configuration |

### Constructors

```rust
// Create with defaults
let config = SiteConfig::new("My Site", "https://example.com");

// Load from file
let config = SiteConfig::from_file("site.toml")?;

// Load from directory (looks for site.toml)
let config = SiteConfig::from_dir("./mysite")?;
```

### Methods

#### `from_file`

Load configuration from a specific file:

```rust
let config = SiteConfig::from_file("custom-config.toml")?;
```

Returns `ConfigError::NotFound` if the file doesn't exist.

#### `from_dir`

Load configuration from a directory containing `site.toml`:

```rust
let config = SiteConfig::from_dir("./myproject")?;
```

#### `new`

Create a new configuration programmatically:

```rust
let config = SiteConfig::new("Site Name", "https://example.com");
```

Uses default `BuildConfig` values.

#### `validate`

Validate the configuration:

```rust
let config = SiteConfig::new("", "https://example.com");
match config.validate() {
    Ok(()) => println!("Valid"),
    Err(e) => eprintln!("Invalid: {}", e),
}
```

Checks:
- `site.name` is not empty
- `site.base_url` is not empty

## SiteMeta

Site metadata from the `[site]` section.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `String` | Yes | Site name/title |
| `base_url` | `String` | Yes | Base URL for the site |
| `description` | `Option<String>` | No | Site description |
| `author` | `Option<String>` | No | Author name |

### Example

```rust
let config = SiteConfig::from_dir(".")?;

println!("Name: {}", config.site.name);
println!("URL: {}", config.site.base_url);

if let Some(desc) = &config.site.description {
    println!("Description: {}", desc);
}

if let Some(author) = &config.site.author {
    println!("Author: {}", author);
}
```

## BuildConfig

Build configuration from the `[build]` section.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content_dir` | `PathBuf` | `"content"` | Content directory |
| `output_dir` | `PathBuf` | `"dist"` | Output directory |
| `static_dir` | `PathBuf` | `"static"` | Static files directory |
| `styles_dir` | `PathBuf` | `"styles"` | Styles directory |
| `templates_dir` | `PathBuf` | `"templates"` | Templates directory |

### Default Values

```rust
let build = BuildConfig::default();

assert_eq!(build.content_dir, PathBuf::from("content"));
assert_eq!(build.output_dir, PathBuf::from("dist"));
assert_eq!(build.static_dir, PathBuf::from("static"));
assert_eq!(build.styles_dir, PathBuf::from("styles"));
assert_eq!(build.templates_dir, PathBuf::from("templates"));
```

### Serde Deserialization

All fields use `#[serde(default)]` for optional parsing:

```toml
[build]
output_dir = "public"
# Other fields use defaults
```

```rust
let config: SiteConfig = toml::from_str(r#"
[site]
name = "Test"
base_url = "https://test.com"

[build]
output_dir = "public"
"#)?;

assert_eq!(config.build.output_dir, PathBuf::from("public"));
assert_eq!(config.build.content_dir, PathBuf::from("content")); // default
```

## TOML Format

### Full Example

```toml
[site]
name = "My Site"
base_url = "https://example.com"
description = "A site built with Yew SSG"
author = "Your Name"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"
```

### Minimal Example

```toml
[site]
name = "My Site"
base_url = "https://example.com"
```

## Type Conversions

### From TOML String

```rust
let toml = r#"
[site]
name = "Test"
base_url = "https://test.com"
"#;

let config: SiteConfig = toml::from_str(toml)?;
```

### To TOML String

```rust
let config = SiteConfig::new("Test", "https://test.com");
let toml = toml::to_string_pretty(&config)?;
```

## Testing

The configuration types include comprehensive unit tests:

```rust
#[test]
fn test_site_config_new() {
    let config = SiteConfig::new("My Site", "https://example.com");
    assert_eq!(config.site.name, "My Site");
    assert_eq!(config.site.base_url, "https://example.com");
}

#[test]
fn test_build_config_defaults() {
    let config = BuildConfig::default();
    assert_eq!(config.content_dir, PathBuf::from("content"));
    assert_eq!(config.output_dir, PathBuf::from("dist"));
}
```
