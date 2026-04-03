# CLI Reference

The `taxus` binary provides five subcommands for building and managing static sites.

## Global Usage

```bash
taxus <SUBCOMMAND> [OPTIONS]
```

## `taxus build`

Build the static site from Markdown content and templates.

```
taxus build [OPTIONS]

Options:
  -d, --dir <PATH>         Root directory (must contain site.toml) [default: .]
  -v, --verbose            Print detailed progress for each build stage
  -q, --quiet              Suppress all output except errors
      --include-drafts     Include pages marked draft = true
      --dry-run            Simulate without writing files
      --clean              Remove output directory before building
  -o, --output <PATH>      Override the output directory from site.toml
  -h, --help               Print help
```

### Examples

```bash
# Build from current directory
taxus build

# Build with verbose output
taxus build --verbose

# Build from a specific directory
taxus build --dir ./my-site

# Build including drafts
taxus build --include-drafts

# Dry run (validate without writing)
taxus build --dry-run

# Clean and rebuild
taxus build --clean

# Override output directory
taxus build --output /tmp/preview
```

### Build Pipeline Stages

1. Discover routes from `content/`
2. Load Tera templates from `templates/`
3. Parse Markdown + frontmatter
4. Copy co-located assets
5. Render pages with templates
6. Generate `robots.txt`
7. Generate `sitemap.xml`
8. Generate `404.html`
9. Build and render taxonomy pages
10. Generate feeds (RSS/Atom)
11. Process assets (SCSS, static files)
12. Write output files

## `taxus clean`

Remove all generated files from the output directory.

```
taxus clean [OPTIONS]

Options:
  -d, --dir <PATH>   Root directory (must contain site.toml) [default: .]
  -h, --help         Print help
```

### Examples

```bash
# Clean current site
taxus clean

# Clean a site in a different directory
taxus clean --dir ./my-site
```

## `taxus init`

Initialize a new site with a default directory structure.

```
taxus init [OPTIONS] [PATH]

Arguments:
  [PATH]               Directory to initialize [default: .]

Options:
  -n, --name <NAME>       Site name used in templates and site.toml
  -u, --base-url <URL>    Base URL (must start with http:// or https://)
  -f, --force             Initialize even if directory is not empty
  -i, --islands           Include WASM hydration script in templates
  -h, --help              Print help
```

### Files Created

| File | Description |
|------|-------------|
| `site.toml` | Site configuration |
| `content/_index.md` | Home page content |
| `templates/base.html` | Base HTML layout |
| `templates/page.html` | Single-page template |
| `templates/section.html` | Section/listing template |
| `styles/main.scss` | Starter stylesheet |
| `static/scripts.js` | Placeholder scripts file |
| `static/favicon.png` | Placeholder favicon |

### Examples

```bash
# Initialize in current directory
taxus init

# Initialize in a new directory
taxus init my-site

# Initialize with custom options
taxus init my-site --name "My Blog" --base-url "https://myblog.com"

# Initialize with islands support
taxus init my-site --islands

# Force initialization in non-empty directory
taxus init my-site --force
```

## `taxus routes`

List all routes that would be discovered from the content directory without building.

```
taxus routes [OPTIONS]

Options:
  -d, --dir <PATH>   Root directory (must contain site.toml) [default: .]
  -h, --help         Print help
```

### Example Output

```
Routes for "My Site"
────────────────────────────────────────────────────
  [section]  /          →  _index.md            →  index.html
  [page]     /about/    →  about.md             →  about/index.html
  [section]  /blog/     →  blog/_index.md       →  blog/index.html
  [page]     /blog/post/ → blog/post.md         →  blog/post/index.html
────────────────────────────────────────────────────
  Total: 4 routes (2 pages, 2 sections)
```

### Examples

```bash
# List routes for current site
taxus routes

# List routes for a specific site
taxus routes --dir ./my-site
```

## `taxus serve`

Start a development server with live reload.

```
taxus serve [OPTIONS] [DIR]

Arguments:
  [DIR]               Root directory (must contain site.toml) [default: .]

Options:
  -p, --port <PORT>   Port to listen on [default: 3000]
  -v, --verbose       Print detailed progress for each build stage
  -q, --quiet         Suppress all output except errors
  -o, --open          Open browser automatically
  -h, --help          Print help
```

The serve command performs an initial build automatically, then watches for file changes.

### Examples

```bash
# Start on default port
taxus serve

# Start with custom port
taxus serve --port 8080

# Start and open browser
taxus serve --open

# Serve from specific directory
taxus serve ./my-site

# Combined options
taxus serve ./my-site --port 8080 --open --verbose
```

## Error Hints

When a command fails, the CLI prints an actionable hint alongside the error:

| Error | Hint |
|-------|------|
| `site.toml` not found | Run `taxus init` or use `--dir` |
| No content found | Add `.md` files to `content/`, start with `content/_index.md` |
| Template not found | Check that `templates/` contains `base.html` and `page.html` |

## Logging

Control log output with CLI flags or the `RUST_LOG` environment variable:

```bash
# Default: info level (build progress)
taxus build

# Verbose: debug level (detailed stages)
taxus build --verbose

# Quiet: errors only
taxus build --quiet

# Custom via RUST_LOG
RUST_LOG=debug taxus build
RUST_LOG=taxus_lib=trace taxus build
```

Log levels:

| Level | Description |
|-------|-------------|
| `error` | Build failures only |
| `warn` | Warnings and errors |
| `info` | Build progress (default) |
| `debug` | Detailed stage information |
| `trace` | Verbose internal diagnostics |
