# Migrated Issues — taxus

Snapshot of the 22 open issues from the original GitHub repo
(<https://github.com/crustyrustacean/taxus>), captured during the
Codeberg migration on **2026-07-04**. All issues were opened by
`crustyrustacean` on **2026-04-29**, none had comments, and all were open
at migration time.

Labels: `bug`, `enhancement`, `documentation`.

This file is a faithful record so nothing is lost across the move. The
originals remain on GitHub; if/when these are recreated as live Codeberg
tracker issues they will get **new numbers** there (Codeberg assigns its
own), so the `#N` numbers below always refer back to the GitHub issue
linked in each entry.

---

## #1 — escape_xml() in feed.rs produces literal Unicode chars instead of XML entity references

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/1

The `escape_xml()` function in `taxus-generator/src/feed.rs` (around line 198) uses `\u{26}` which evaluates to the literal `&` Unicode character, not the text string `"&"`. This means the output contains raw `&` characters followed by `amp;` rather than the XML entity `&amp;`.

```rust
'&' => result.push_str("\u{26}amp;"),   // produces literal "&amp;" as raw chars
'<' => result.push_str("\u{26}lt;"),    // produces literal "&lt;" as raw chars
'>' => result.push_str("\u{26}gt;"),    // produces literal "&gt;" as raw chars
'"' => result.push_str("\u{26}quot;"),  // produces literal "&quot;" as raw chars
'\'' => result.push_str("\u{26}apos;"),  // produces literal "&apos;" as raw chars
```

## Expected

```rust
'&' => result.push_str("&amp;"),
'<' => result.push_str("&lt;"),
'>' => result.push_str("&gt;"),
'"' => result.push_str("&quot;"),
'\'' => result.push_str("&apos;"),
```

## Impact

RSS and Atom feeds contain malformed XML that fails validation and may confuse or break feed readers.

## Files

- `taxus-generator/src/feed.rs`

---

## #2 — Build pipeline stage numbers are inconsistent with and without the islands feature

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/2

The `SiteBuilder::build()` method in `taxus-generator/src/build/builder.rs` claims a 15-stage pipeline, but the numbering is inconsistent:

- Stages `[13/15]` (search index) and `[14/15]` (WASM client) only appear when the `islands` feature is enabled
- Stage `[15/15]` (write output) always runs, but its label says `[15/15]` even when there are only 13 stages without `islands`
- Without the `islands` feature, the final write stage is labeled `[15/15]` when it should be `[13/13]`

## Expected

The stage numbering should dynamically reflect the actual number of stages:
- Without `islands`: stages 1–13 (13 stages)
- With `islands`: stages 1–15 (15 stages)

## Impact

Confusing build log output. Non-islands builds show `[15/15]` as the final stage when only 13 stages exist.

## Files

- `taxus-generator/src/build/builder.rs`

---

## #3 — Weight-based sorting in sections silently falls back to title sort

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/3

In `taxus-generator/src/build/pipeline/pages.rs` (around line 324), the `sort_page_contexts()` function silently falls back to alphabetical title sorting when `SortBy::Weight` is requested:

```rust
SortBy::Weight => {
    // Weight isn't in PageContext, so fall back to title sort
    pages.sort_by(|a, b| a.title.cmp(&b.title));
}
```

## Expected

Either:
- Include `weight` in `PageContext` so weight-based sorting actually works, or
- Emit a warning when `sort_by = "weight"` is used but cannot be honored, or
- Remove `SortBy::Weight` from `SortBy` if it's not supported in sections

## Impact

Users who set `sort_by = "weight"` in section frontmatter will get alphabetical title sorting with no warning — a silent semantic mismatch that is hard to debug.

## Files

- `taxus-generator/src/build/pipeline/pages.rs`
- `taxus-generator/src/templates/context.rs` (PageContext definition)

---

## #4 — Internal link resolver code block detection is fragile

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/4

The `is_inside_code_block()` function in `taxus-generator/src/build/pipeline/internal_links.rs` uses a simple `split("```")` heuristic to detect fenced code blocks. This approach has several weaknesses:

- Does not handle indented code blocks (4-space indent)
- Cannot distinguish between fenced code blocks and inline code containing triple backticks
- Does not handle closing ``` with trailing characters (e.g., ` ```rust `)
- Can be confused by triple backticks inside code blocks themselves

## Example

A markdown file with inline code mentioning triple backticks could be misclassified:

```markdown
Use \`\`\` to start a code block. See [link](@/about.md) for help.
```

## Impact

Internal links inside code blocks with edge-case formatting may be incorrectly resolved (causing broken links in output HTML) or may cause false-positive broken link errors.

## Files

- `taxus-generator/src/build/pipeline/internal_links.rs`

---

## #5 — TaxonomyMap uses different page identifiers than the build pipeline

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/5

`TaxonomyMap::from_pages()` in `taxus-generator/src/content/taxonomy.rs` uses `page.path` (URL paths like `/blog/post-1/`) as keys for taxonomy terms. However, the build pipeline in `taxus-generator/src/build/pipeline/taxonomy.rs` has its own `build_taxonomy_map()` function that uses `route.content_file` (filesystem paths like `blog/post-1.md`) as keys.

This means:
- The public `TaxonomyMap::from_pages()` method is dead code — it is never called by the build pipeline
- The two APIs use incompatible key conventions, which could confuse anyone using the library programmatically

## Suggested Fix

Either remove `TaxonomyMap::from_pages()` or unify the key convention so the public API and internal pipeline agree.

## Files

- `taxus-generator/src/content/taxonomy.rs` (`TaxonomyMap::from_pages`)
- `taxus-generator/src/build/pipeline/taxonomy.rs` (`build_taxonomy_map`)

---

## #6 — Section::parse_frontmatter fails on empty frontmatter while Page handles it correctly

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/6

`Page::parse_frontmatter()` in `taxus-generator/src/content/page.rs` correctly handles empty frontmatter (`+++\n+++\n`) by detecting the `+++\n` prefix before searching for `\n+++\n`. But `Section::parse_frontmatter()` in `taxus-generator/src/content/section.rs` (around line 72) only looks for `\n+++\n`, so empty frontmatter causes an `UnclosedFrontmatter` error.

## Reproduction

```toml
# content/blog/_index.md
+++
+++
Blog section with empty frontmatter.
```

Running `taxus build` with this file will fail with `UnclosedFrontmatter` for the section but succeed for a page with the same frontmatter.

## Expected

Both `Page` and `Section` should handle empty frontmatter identically.

## Files

- `taxus-generator/src/content/section.rs` (`parse_frontmatter`)
- `taxus-generator/src/content/page.rs` (`parse_frontmatter`)

---

## #7 — Alias paths without trailing slashes produce non-standard URL paths

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/7

Users can define aliases in frontmatter without a trailing slash:

```toml
aliases = ["/old-url"]
```

But `AliasPage::new()` in `taxus-generator/src/build/pipeline/alias.rs` doesn't validate or normalize the path. The alias `/old-url` generates `old-url/index.html` and a redirect at `/old-url` — a URL without a trailing slash, which is inconsistent with how all other routes work in Taxus (they always end with `/`).

## Expected

Either normalize aliases to always have a trailing slash, or document that trailing slashes are required.

## Files

- `taxus-generator/src/build/pipeline/alias.rs`
- `taxus-generator/src/content/frontmatter.rs` (aliases field)

---

## #8 — strip_markdown() in page.rs doesn't handle many common markdown constructs

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/8

The `strip_markdown()` method in `taxus-generator/src/content/page.rs` is used to generate summaries and calculate word counts, but it doesn't handle several common markdown constructs:

Missing handling for:
- Horizontal rules (`---`, `***`, `___`)
- Blockquotes (`> text`)
- Ordered/unordered lists (`- item`, `1. item`)
- Strikethrough (`~~text~~`)
- Footnotes (`[^1]`)
- HTML tags (`<br>`, `<hr>`, `<div>`)
- Task lists (`- [x] done`)
- Definition lists
- Tables

## Impact

Word counts and auto-generated summaries contain markdown artifacts or miss stripped content, leading to inaccurate reading times and potentially messy summary text in feeds.

## Files

- `taxus-generator/src/content/page.rs` (`strip_markdown`)

---

## #9 — File watcher ChangeType::from_path misclassifies files in directories named content/styles/static

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/9

`ChangeType::from_path()` in `taxus-generator/src/serve/watcher.rs` uses substring matching to classify files:

```rust
if path_str.contains("content/") || path_str.contains("content\\") {
    return ChangeType::Content;
}
```

This can misclassify files if the parent directory name happens to contain these keywords. For example:
- `my-content/pages/post.md` would be classified as content
- `styles-extra/main.scss` would be classified as styles
- `static-assets/img.png` would be classified as static

## Suggested Fix

Use path component-based matching instead of string containment — check the first or second path component against the configured directory names, or match against the actual site directory structure.

## Files

- `taxus-generator/src/serve/watcher.rs` (`ChangeType::from_path`)

---

## #10 — Misleading comment in telemetry.rs about init() being called for RUST_LOG

**Label:** documentation · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/10

In `taxus-generator/src/telemetry.rs`, the `init_tracing()` function has this code path:

```rust
pub fn init_tracing(verbose: bool, quiet: bool) {
    let level = if quiet {
        "error"
    } else if verbose {
        "debug"
    } else {
        // Respect RUST_LOG if set, otherwise use info
        if std::env::var("RUST_LOG").is_ok() {
            return; // init() will use RUST_LOG
        }
        "info"
    };

    init_with_level(level);
}
```

The comment `// init() will use RUST_LOG` is misleading — `init()` is never called in this path. The function returns early and the caller (`main.rs`) doesn't call `init()`. The behavior actually works correctly because `init_with_level()` also tries `RUST_LOG` first, but the comment is wrong.

## Suggested Fix

Change the comment to accurately describe the behavior:

```rust
if std::env::var("RUST_LOG").is_ok() {
    return; // RUST_LOG is set, init_with_level below will use it
}
```

## Files

- `taxus-generator/src/telemetry.rs`

---

## #11 — Stale cached images not regenerated when output format is changed in site.toml

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/11

## Description

In `taxus-generator/src/images/processor.rs`, the `build_from_cache()` method returns a `ProcessedImage` whose `format` field is derived from the cached filename extension (via `format_from_filename()`). 

If a user changes the output format in `site.toml` from `webp` to `jpeg` after images were already cached (with `.webp` extension), the cached files still have `.webp` extensions. The `all_variants_exist()` check passes because the files do exist, but the `format` field in the returned `ProcessedImage` will be `"webp"` instead of the configured `"jpeg"`.

## Expected

The cache should be invalidated when `site.toml` format changes, or the `format` field should be derived from the config rather than from the cached filenames.

## Files

- `taxus-generator/src/images/processor.rs` (`build_from_cache`, `format_from_filename`)

---

## #12 — Paginator::total_pages() returns 1 for per_page == 0 instead of signaling an error

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/12

In `taxus-generator/src/content/pagination.rs`, the `Paginator::total_pages()` method returns 1 when per_page == 0. The `is_paginated()` check in `Section` prevents `paginate_by = 0` from reaching the paginator, but this guard is only in one place. If `total_pages()` is called directly or the guard is bypassed in the future, it silently produces 1 page instead of 0. Suggested fix: return 0 for per_page == 0, or return Err since it represents a configuration error.

---

## #13 — Duplicate escape_html() implementations in markdown.rs and highlighting/engine.rs

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/13

Both `taxus-generator/src/build/pipeline/markdown.rs` and `taxus-generator/src/highlighting/engine.rs` contain identical `escape_html()` functions. This should be extracted into a shared utility module to follow DRY principles.

---

## #14 — RouteRegistry::generate_rust_manifest() is dead stub code

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/14

The `generate_rust_manifest()` method in `taxus-generator/src/routes/registry.rs` generates Yew router code but is never called anywhere in the codebase. The method has a comment saying it is a stub that "will be expanded in a future phase." This dead code should either be removed or have an open tracking issue for when it will be implemented.

---

## #15 — RenderedPage does not carry hero_image metadata from ProcessedPage

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/15

`ProcessedPage` has a `hero_image` field but `RenderedPage` only contains `route` and `content`. The hero image data is embedded in the rendered HTML but not available as structured data for any downstream consumers. If anything needs to inspect or transform hero images after rendering, the data is inaccessible.

---

## #16 — Page.content is Option<String> but is never set by the build pipeline

**Label:** documentation · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/16

The `content` field on `Page` in `taxus-generator/src/content/page.rs` is documented as "Rendered HTML content (set after rendering)" but the build pipeline never sets it — it uses `ProcessedPage.html_content` instead. The field is only set to `Some` when `full_content` feed mode is enabled in `feed.rs`. Consider removing the `Option` wrapper or documenting the field's actual usage.

---

## #17 — FeedEntry::from_page produces wrong URLs for pages with custom slugs

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/17

The public `FeedEntry::from_page()` method in `taxus-generator/src/feed.rs` uses `page.path` (which is always the filename-derived path) rather than respecting the slug. The pipeline's `generate_feeds()` correctly overrides `page.path` with the slug-derived path, but anyone calling the public API directly will get wrong URLs for pages with custom slugs.

---

## #18 — Error overlay IDs mismatch in live reload injector script

**Label:** bug · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/18

In `taxus-generator/src/serve/injector.rs`, the `showErrorOverlay()` function creates the overlay with `id='__yew_ssg_error__'` but the existing overlay check looks for `id='__taxus_error__'`. This mismatch means the close button won't properly replace a previous error overlay if one already exists — the old overlay remains hidden behind the new one.

---

## #19 — xtask cmd_test feature flag handling is fragile with --nextest

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/19

In `xtask/src/main.rs`, `cmd_test()` builds feature args then inserts `'nextest'` and `'run'` at positions 0 and 1. While this currently works with cargo's argument parsing, the command construction is fragile and the feature args could get lost if the structure changes. Consider building the args list more explicitly.

---

## #20 — SearchIndex::search() has no relevance threshold — returns all matching documents

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/20

In `taxus-common/src/search.rs`, the `search()` method returns all documents that match any query term, regardless of how low their TF-IDF score is. A document matching a single common word with a very low score will still appear in results. Consider adding a minimum score threshold or result count limit.

---

## #21 — Page::from_file() discards directory context in the source field

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/21

In `taxus-generator/src/content/page.rs`, `Page::from_file()` only captures the filename (e.g., 'post-1.md') in the `source` field, not the relative path from the content directory (e.g., 'blog/post-1.md'). This means the `source` field is less useful for debugging and error messages. Consider preserving the relative path.

---

## #22 — BuildReport.has_warnings() treats asset errors as warnings rather than errors

**Label:** enhancement · **Opened:** 2026-04-29 · **Original:** https://github.com/crustyrustacean/taxus/issues/22

In `taxus-generator/src/build/report.rs`, `has_warnings()` returns true when `self.assets.has_errors()` is true. SCSS compilation failures and file copy errors are real errors, not warnings. The build exits with code 1 due to warnings, which makes the terminology confusing. Consider renaming or splitting the concept.

---

*End of migrated issues. 22 issues total (10 bug, 10 enhancement, 2 documentation). All open, all opened 2026-04-29 by `crustyrustacean`, no comments.*
