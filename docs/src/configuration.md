# Configuration

Taxus uses a `site.toml` configuration file to define site settings and build options.

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

[feed]
rss_enabled = true
atom_enabled = false
limit = 20
full_content = false
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

### `[feed]` Section

RSS/Atom feed configuration for content syndication.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rss_enabled` | bool | `true` | Enable RSS 2.0 feed generation |
| `atom_enabled` | bool | `false` | Enable Atom feed generation |
| `limit` | number | `0` | Maximum entries in feed (0 = all) |
| `full_content` | bool | `false` | Include full content vs summary |
| `title` | string | `None` | Custom feed title (defaults to site name) |
| `rss_path` | string | `None` | RSS feed output path (default: `rss.xml`) |
| `atom_path` | string | `None` | Atom feed output path (default: `atom.xml`) |

## Minimal Configuration

The minimal required configuration:

```toml
[site]
name = "My Site"
base_url = "https://example.com"
```

All `[build]` and `[feed]` settings use their default values.

## Full Configuration Example

```toml
[site]
name = "My Blog"
base_url = "https://example.com"
description = "A blog about Rust and web development"
author = "Jane Doe"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"

[feed]
rss_enabled = true
atom_enabled = true
limit = 20
full_content = false
title = "My Blog Feed"
rss_path = "rss.xml"
atom_path = "atom.xml"
```

## Validation

Configuration is validated when loaded:

- `site.name` must not be empty
- `site.base_url` must not be empty

## Feed URLs

After generation, feeds are available at:

- RSS: `https://example.com/rss.xml` (or custom `rss_path`)
- Atom: `https://example.com/atom.xml` (or custom `atom_path`)
