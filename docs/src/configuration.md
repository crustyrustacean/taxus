# Configuration

Yew SSG uses a `site.toml` configuration file to define site settings and build options.

## Configuration File

Create a `site.toml` file in your project root:

```toml
[site]
name = "My Site"
base_url = "https://example.com"
description = "A description of my site"
author = "Your Name"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"
```

## Configuration Sections

### `[site]` Section

Site metadata and information.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Site name/title |
| `base_url` | string | Yes | Base URL for the site (used for absolute URLs) |
| `description` | string | No | Site description for SEO |
| `author` | string | No | Site author name |

### `[build]` Section

Build configuration options. All fields have defaults.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `content_dir` | string | `"content"` | Directory containing Markdown content |
| `output_dir` | string | `"dist"` | Output directory for generated files |
| `static_dir` | string | `"static"` | Directory containing static assets |
| `styles_dir` | string | `"styles"` | Directory containing SCSS stylesheets |
| `templates_dir` | string | `"templates"` | Directory containing HTML templates |

## Minimal Configuration

The minimal required configuration:

```toml
[site]
name = "My Site"
base_url = "https://example.com"
```

All `[build]` settings will use their default values.

## Programmatic Configuration

You can also create configuration programmatically using the library API:

```rust
use generator::{SiteConfig, Result};

fn main() -> Result<()> {
    // Create configuration with defaults
    let config = SiteConfig::new("My Site", "https://example.com");
    
    // Validate the configuration
    config.validate()?;
    
    // Access configuration values
    println!("Site: {}", config.site.name);
    println!("Output: {:?}", config.build.output_dir);
    
    Ok(())
}
```

## Loading Configuration

### From a File

```rust
use generator::SiteConfig;

let config = SiteConfig::from_file("site.toml")?;
```

### From a Directory

Looks for `site.toml` in the specified directory:

```rust
use generator::SiteConfig;

let config = SiteConfig::from_dir("./mysite")?;
```

### `[feed]` Section

RSS/Atom feed configuration for content syndication.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable feed generation |
| `format` | string | `"rss"` | Feed format: `"rss"`, `"atom"`, or `"both"` |
| `path` | string | `"feed.xml"` | RSS feed output path (relative to output directory) |
| `atom_path` | string | `"atom.xml"` | Atom feed output path (relative to output directory) |

Example:

```toml
[feed]
enabled = true
format = "both"
path = "rss.xml"
atom_path = "atom.xml"
```

### Full Configuration Example

```toml
[site]
name = "My Site"
base_url = "https://example.com"
description = "A description of my site"
author = "Your Name"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"

[feed]
enabled = true
format = "both"
path = "feed.xml"
atom_path = "atom.xml"
```

## Validation

Configuration is validated when loaded. The following checks are performed:

- `site.name` must not be empty
- `site.base_url` must not be empty

```rust
let config = SiteConfig::new("", "https://example.com");
let result = config.validate();
// Returns error: Missing required field 'site.name'
```

## Error Handling

Configuration errors are handled through the error system:

```rust
use generator::{SiteConfig, GeneratorError, ConfigError};

match SiteConfig::from_dir("./missing") {
    Ok(config) => println!("Loaded: {}", config.site.name),
    Err(GeneratorError::Config(ConfigError::NotFound(path))) => {
        eprintln!("Config not found: {}", path.display());
    }
    Err(GeneratorError::Config(ConfigError::Parse(e))) => {
        eprintln!("Parse error: {}", e);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```
