# CLI Improvement Plan for `yew-ssg`

## Current State Analysis

The CLI ([`generator/src/bin/main.rs`](../generator/src/bin/main.rs)) has two subcommands — `build` and `init` — implemented with `clap`. Below is a gap analysis against best-practice CLI UX.

### Current Issues

| # | Area | Problem |
|---|------|---------|
| 1 | **Help text** | All `clap` arg docs are one-liners with no examples; `long_about = None` on the root command |
| 2 | **Error output** | Errors print `"Error: {e}"` with no hint about how to fix them (e.g., missing `site.toml`) |
| 3 | **Clean** | `--clean` is a flag under `build`, not a standalone subcommand; `clean` alone is impossible |
| 4 | **Quiet mode** | No way to suppress summary output; `--verbose` is the only verbosity control |
| 5 | **Output override** | No way to redirect output to a different directory without editing `site.toml` |
| 6 | **Build summary** | `BuildReport::print_summary()` has misaligned columns and no visual success/failure indicator |
| 7 | **Init summary** | `InitReport::print_summary()` does not list the actual files/directories created |
| 8 | **Route inspection** | No command to inspect what routes the generator would discover from a site |

---

## Proposed Changes

### 1. Richer Help Text — `build` and `init`

**File:** [`generator/src/bin/main.rs`](../generator/src/bin/main.rs)

Add `long_about` strings with a usage example block for both subcommands.

```rust
/// Build the static site from Markdown content and templates.
///
/// Reads site.toml from DIR, discovers all .md files in content/,
/// renders them with Tera templates, compiles SCSS, copies static files,
/// and writes the result to the configured output directory (default: dist/).
///
/// Examples:
///   yew-ssg build
///   yew-ssg build --verbose
///   yew-ssg build --dir ./my-site --include-drafts
///   yew-ssg build --dry-run
#[command(long_about = "...")]
Build { ... }
```

Also expand each argument's doc comment to be more descriptive:

| Arg | Current | Proposed |
|-----|---------|----------|
| `--dir` | `Directory containing site.toml` | `Root directory of the site (must contain site.toml). Defaults to the current directory.` |
| `--verbose` | `Enable verbose output` | `Print detailed progress for each build stage (route discovery, template loading, rendering, assets)` |
| `--include-drafts` | `Include draft pages in build` | `Include pages marked draft = true in frontmatter. Drafts are excluded by default.` |
| `--dry-run` | `Dry run - don't write files` | `Simulate the build without writing any output files. Useful to check for errors.` |
| `--clean` | `Clean output directory before build` | `Remove all files from the output directory before building` |

### 2. Add a Standalone `clean` Subcommand

**File:** [`generator/src/bin/main.rs`](../generator/src/bin/main.rs)

Add `Commands::Clean` so users can clear the output directory independently:

```rust
/// Remove all generated files from the output directory
///
/// Example:
///   yew-ssg clean
///   yew-ssg clean --dir ./my-site
Clean {
    /// Root directory of the site (must contain site.toml)
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,
},
```

Implementation calls [`SiteBuilder::from_dir(dir)?.clean()`](../generator/src/build/builder.rs:211).

### 3. Add `--quiet` / `-q` Flag to `build`

**File:** [`generator/src/bin/main.rs`](../generator/src/bin/main.rs)

```rust
/// Suppress all output except errors
#[arg(short, long, conflicts_with = "verbose")]
quiet: bool,
```

When `quiet` is true, only `eprintln!` on errors; skip `report.print_summary()`. This is the inverse of `--verbose` and they should be mutually exclusive via `conflicts_with`.

### 4. Add `--output` / `-o` Override to `build`

**File:** [`generator/src/bin/main.rs`](../generator/src/bin/main.rs)

```rust
/// Override the output directory from site.toml
#[arg(short = 'o', long)]
output: Option<PathBuf>,
```

After loading `SiteConfig`, patch `config.build.output_dir` before constructing `SiteBuilder::new(config)`. This avoids forcing users to edit `site.toml` just to preview output in a different path.

### 5. Add `routes` Subcommand

**File:** [`generator/src/bin/main.rs`](../generator/src/bin/main.rs)

A diagnostic command that discovers and prints all routes without building anything:

```
yew-ssg routes
yew-ssg routes --dir ./my-site
yew-ssg routes --include-drafts
```

Output:

```
Routes for "My Site"
─────────────────────────────────
  [section]  /          →  _index.md             →  index.html
  [page]     /about/    →  about.md              →  about/index.html
  [section]  /blog/     →  blog/_index.md        →  blog/index.html
  [page]     /blog/hi/  →  blog/hello-world.md   →  blog/hi/index.html
─────────────────────────────────
Total: 4 routes (2 pages, 2 sections)
```

Implementation uses [`RouteDiscovery`](../generator/src/routes/discovery.rs) directly (already part of the public API).

### 6. Improve `BuildReport::print_summary()`

**File:** [`generator/src/build/report.rs`](../generator/src/build/report.rs)

**Current output:**
```
 Build Summary
─────────────────────────────────
  Pages rendered:  5
  Sections rendered: 2
...
```

**Proposed output (richer, aligned, with success/warning prefix):**
```
✓ Build complete  (1.23s)
─────────────────────────────────
  Pages          5
  Sections       2
  Drafts skipped 0
  Assets         8
  Total files    15
  Output         dist/
─────────────────────────────────
```

Or on warning/error:
```
⚠ Build completed with warnings  (1.23s)
```

Changes:
- Align values with fixed-width column (use `{:<16}` format)
- Move duration to the header line 
- Replace the current missing success indicator
- No duplicate `─` separator lines; one separator after all stats

### 7. Improve `InitReport::print_summary()`

**File:** [`generator/src/init/mod.rs`](../generator/src/init/mod.rs)

**Current output:**
```
✓ Site initialized successfully at my-site
  Directories created: 4
  Files created: 6

Next steps:
  1. Edit site.toml to configure your site
  ...
```

**Proposed output (listing actual paths):**
```
✓ Site initialized at my-site/

  Directories
    my-site/content/
    my-site/templates/
    my-site/static/
    my-site/styles/

  Files
    my-site/site.toml
    my-site/content/_index.md
    my-site/templates/base.html
    my-site/templates/page.html
    my-site/templates/section.html
    my-site/styles/main.scss

Next steps:
  cd my-site
  Edit site.toml to set your site name and base URL
  Run: yew-ssg build --verbose
```

This requires `InitReport` to track created path lists, not just counts. Add:

```rust
pub struct InitReport {
    pub path: PathBuf,
    pub directories_created: usize,
    pub files_created: usize,
    pub created_dirs: Vec<PathBuf>,   // NEW
    pub created_files: Vec<PathBuf>,  // NEW
}
```

Update `InitScaffolder` to push to these vecs when creating each entry.

### 8. Actionable Error Messages

**File:** [`generator/src/bin/main.rs`](../generator/src/bin/main.rs)

The current error handler:
```rust
Err(e) => {
    eprintln!("Error: {e}");
    std::process::exit(1);
}
```

Replace with a `render_error()` function that matches on `GeneratorError` variants and appends a hint:

| Error | Hint |
|-------|------|
| `Config(ConfigError::NotFound(_))` | `Hint: Run 'yew-ssg init' to create a new site, or use --dir to point to your site directory.` |
| `Build(BuildError::NoContent)` | `Hint: Add .md files to your content/ directory. Start with content/_index.md.` |
| `Template(TemplateError::NotFound(_))` | `Hint: Check that your templates/ directory exists and contains base.html and page.html.` |
| `Init(InitError::Cancelled)` | `(print nothing — cancellation is intentional)` |
| All others | Print as-is |

---

## File Change Summary

| File | Changes |
|------|---------|
| [`generator/src/bin/main.rs`](../generator/src/bin/main.rs) | Richer arg docs, `Clean` subcommand, `Routes` subcommand, `--quiet`, `--output`, `render_error()` |
| [`generator/src/build/report.rs`](../generator/src/build/report.rs) | Reformatted `print_summary()` output |
| [`generator/src/init/mod.rs`](../generator/src/init/mod.rs) | `InitReport` now tracks path lists; `print_summary()` lists created files |
| [`generator/src/init/scaffold.rs`](../generator/src/init/scaffold.rs) | Push to `created_dirs` / `created_files` after each creation |

---

## No New Dependencies Required

All proposed changes use existing crates (`clap`, `yew_ssg_lib`). Colour output (e.g. with `colored` or `termcolor`) is considered optional future work and is not included in this plan.
