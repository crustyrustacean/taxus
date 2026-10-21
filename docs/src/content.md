# Content

Content in Taxus is written in Markdown files with TOML frontmatter.

> For the conceptual overview — how pages, sections, taxonomies, and URLs
> relate — see [Content Model](./content-model.md). This chapter is the
> practical reference.

## Content Files

Content files are stored in the `content/` directory:

```
content/
├── _index.md      # Home page
├── about.md       # About page
└── blog/
    ├── _index.md  # Blog section index
    ├── first-post.md
    └── second-post.md
```

### Special Files

| File | Purpose |
|------|---------|
| `_index.md` | Section index page (home page at root, section index in subdirectories) |
| `*.md` | Regular pages |

### Dated Filenames

A `YYYY-MM-DD-` prefix on a content filename is treated as a storage
convention, not as part of the slug:

- The prefix is **stripped from the slug**: `content/blog/2026-04-06-my-post.md`
  is served at `/blog/my-post/`.
- If the page has no `date` in frontmatter, the prefix **supplies the default
  publication date**. A frontmatter `date` always wins.
- Stems that are *only* a date (`2026-04-06.md`) and filenames whose prefix is
  not a valid date (`2026-13-45-post.md`) are left untouched.

This keeps dates out of URLs while letting filenames sort chronologically on
disk. See [Content Model](./content-model.md) for the design rule behind it.

### Co-located Assets

Non-Markdown files in the content directory are automatically copied to the output directory, preserving their relative paths. This allows you to keep images and other assets alongside the content that uses them.

```
content/
├── blog/
│   ├── first-post.md
│   ├── photo.jpg          → dist/blog/photo.jpg
│   └── diagrams/
│       └── architecture.png  → dist/blog/diagrams/architecture.png
└── about/
    ├── about.md
    └── headshot.png       → dist/about/headshot.png
```

**Referencing co-located assets:** an asset's URL is its path under
`content/`. Reference it from the site root, or with the `image` shortcode's
`@/` form, which resolves to the same URL:

```markdown
![Photo](/blog/photo.jpg)
![Diagram](/blog/diagrams/architecture.png)
{{ image(src="@/blog/photo.jpg", alt="Photo") }}
```

Bare relative references such as `![Photo](photo.jpg)` do not resolve. The
browser resolves them against the page's URL, and a page is rendered one level
below its source directory: `content/blog/first-post.md` becomes
`/blog/first-post/`, so `photo.jpg` is requested as `/blog/first-post/photo.jpg`,
while the file was copied to `/blog/photo.jpg`. Use one of the forms above.

**When to use co-located assets:**

- Blog post images and diagrams
- Page-specific downloads (PDFs, etc.)
- Content-specific data files

For global assets (logos, favicons, shared images), use the `static/` directory instead.

## Organising files without changing URLs

By default every directory under `content/` is a section and a URL segment.
A long-running blog eventually wants its files split up on disk without
changing its addresses. The `[build] flatten` setting in `site.toml` marks
directories as **storage only**: a flattened directory is not a section and
adds nothing to the URL. A page inside it belongs to the nearest ancestor
section that is not flattened.

```toml
[build]
flatten = ["blog/*"]
```

`*` matches one path segment, so `blog/*` flattens every direct subdirectory
of `blog/`. See [Configuration](./configuration.md#flatten) for the pattern
rules.

### Year folders

```
content/
└── blog/
    ├── _index.md                                  → /blog/
    ├── 2025/
    │   └── 2025-12-01-year-in-review.md           → /blog/year-in-review/
    └── 2026/
        ├── 2026-04-07-rust-fundamentals-comments.md → /blog/rust-fundamentals-comments/
        └── diagram.png                            → /blog/2026/diagram.png
```

Both posts are pages of the `blog` section. There is no `/blog/2025/` or
`/blog/2026/` section. The date prefix is still stripped from the slug.
`blog`'s listing, `get_section(path="blog")`, pagination, feeds, the
sitemap, the search index, taxonomy pages and alias redirects all see the
two posts as direct children of `blog`, because they all read the same
Site Tree. Moving an existing flat blog into year folders this way leaves
every page URL unchanged.

For year/month folders, use `"blog/*/*"`. Patterns do not support `**`.

### A drafts folder

The same pattern covers a folder for unpublished work:

```
content/blog/drafts/2026-11-01-work-in-progress.md
```

```markdown
+++
title = "Work in progress"
draft = true
+++
```

`blog/drafts` matches `blog/*`, so the post is a draft page of `blog` at
`/blog/work-in-progress/`. The folder is for your own organisation only.
Whether the page is published depends on `draft = true`, not on the folder.
A literal pattern such as `"blog/drafts"` works too, if only that folder
should be flattened.

### Internal links name the real file

`@/` links identify a page by its storage path, not its URL, so they include
the flattened directory:

```markdown
See [the comments post](@/blog/2026/2026-04-07-rust-fundamentals-comments.md).
```

Moving a file into a year folder therefore changes its `@/` path, even though
its URL stays the same. Links that use the old path fail the build with a
broken internal link error, so nothing breaks unnoticed. Update them when
you move files.

Hero images also resolve from where the file is on disk: `hero_image =
"photo.jpg"` on `content/blog/2026/post.md` looks for
`content/blog/2026/photo.jpg`.

### Co-located assets keep their storage path

Flattening applies to pages only. Non-Markdown files are copied to their
path under `content/` as usual, so `content/blog/2026/diagram.png` is served
at `/blog/2026/diagram.png`. Reference it by that path:

```markdown
![Diagram](/blog/2026/diagram.png)
{{ image(src="@/blog/2026/diagram.png", alt="Diagram") }}
```

So if you move posts into year folders and leave their images where they
were, no image references need editing. As with any co-located asset, a bare
relative reference (`![Diagram](diagram.png)`) does not resolve. See
[Co-located Assets](#co-located-assets).

### Rules and errors

- **No `_index.md` in a flattened directory.** It would declare a section
  at the parent's path. The build fails, naming the file and the pattern
  that flattened its directory.
- **No two files with the same URL.** `blog/2025/notes.md` and
  `blog/2026/notes.md` both map to `/blog/notes/`. The build fails, naming
  both files. Give one of them a `slug`, or rename it.
- **`pages_from` and `[feed] sections` name sections, not storage.**
  `pages_from = ["blog/2026"]` names a section that no longer exists, so it
  is ignored with a warning in the build log. Use `"blog"`.
- **`taxus serve` picks up changes** to `flatten` on the next rebuild,
  because it reloads `site.toml`.

## Hero Images

Pages can have a hero image — a prominent image displayed at the top of the page. Place the image file next to your markdown and reference it in frontmatter:

```markdown
+++
title = "My Post"
hero_image = "sunset.jpg"
hero_alt = "A dramatic mountain sunset"
date = 2024-03-15
+++

# My Post
```

Taxus automatically:

- Generates responsive variants at multiple widths (default: 400, 800, 1200)
- Converts to WebP (or JPEG/PNG if configured)
- Produces a `<picture>` element with srcset for optimal browser delivery
- Falls back to the page title if `hero_alt` is not provided

See [Images](./images.md) for full configuration and template usage.

## Frontmatter

Each Markdown file can include TOML frontmatter enclosed in `+++`:

```markdown
+++
title = "Page Title"
description = "A brief description"
date = 2024-01-15
template = "custom.html"
draft = false
tags = ["rust", "web"]

[extra]
author = "John Doe"
+++

# Page Content

Your markdown content here.
```

### Frontmatter Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `title` | string | No\* | `""` | Page title |
| `description` | string | No | `None` | Page description for SEO |
| `date` | date | No | `None` | Publication date (YYYY-MM-DD). Falls back to a `YYYY-MM-DD-` filename prefix when omitted. A full TOML datetime (`2026-04-03T14:30:00Z`) is accepted but truncated to its date part, with a build-log warning — taxus dates are day-granular by design |
| `updated` | date | No | `None` | Last updated date (same truncation rule as `date`) |
| `template` | string | No | `"page.html"` | Template override |
| `draft` | bool | No | `false` | Draft status |
| `summary` | string | No | `None` | Custom summary/excerpt |
| `slug` | string | No | `None` | Custom last URL segment; the page stays in its section |
| `aliases` | array | No | `[]` | Old URLs that redirect to this page |
| `tags` | array | No | `[]` | Tags (e.g., `["rust", "web"]`) |
| `categories` | array | No | `[]` | Categories (e.g., `["tutorial"]`) |
| `series` | string | No | `None` | Series name (e.g., `"Learning Rust"`) |
| `sort_by` | string | No | `"date"` | Sort order for sections: `"date"` (newest first, undated last), `"title"` (case-insensitive), `"weight"` (lowest first), `"none"` (tree order) |
| `paginate_by` | number | No | `0` | Items per page (0 = no pagination) |
| `paginate_template` | string | No | `None` | Template for paginated pages |
| `pages_from` | array | No | `[]` | Sections whose direct pages this section also lists (e.g. `["blog"]`); see [Section listings](#section-listings) |
| `weight` | number | No | `0` | Weight for manual ordering |
| `hero_image` | string | No | `None` | Relative path to a co-located hero image |
| `hero_alt` | string | No | `None` | Alt text for hero image (falls back to page title) |
| `extra` | table | No | `None` | Custom metadata |

\*Title is recommended but not required by the library.

### Date Format

Dates use the TOML date format:

```toml
date = 2024-01-15
updated = 2024-02-20
```

### Extra Metadata

The `extra` field allows custom metadata:

```toml
[extra]
author = "Jane Doe"
custom_field = "any value"
```

Access extra metadata in templates through the `extra` variable.

## URL Path Generation

Content files are mapped to URL paths:

| Content File | URL Path |
|--------------|----------|
| `content/_index.md` | `/` |
| `content/about.md` | `/about/` |
| `content/blog/_index.md` | `/blog/` |
| `content/blog/my-post.md` | `/blog/my-post/` |

### Custom Slugs

Override the default URL path using the `slug` frontmatter field:

```markdown
+++
title = "My First Post"
slug = "hello-world"
+++
```

This creates `/blog/hello-world/` instead of `/blog/my-first-post/`. The slug
replaces only the last segment: the page stays in its section, and every
consumer — page links, section listings, feeds, the sitemap, the search index,
alias redirects — agrees on the URL because all of them derive it from the
Site Tree. To keep an old address working, add it to `aliases`.

## Markdown Support

Taxus supports standard Markdown syntax:

### Headings

```markdown
# Heading 1
## Heading 2
### Heading 3
```

### Lists

```markdown
- Unordered item
- Another item

1. Ordered item
2. Another item
```

### Links

```markdown
[Link text](https://example.com)
```

### Internal Links

Reference other pages by their content file path with build-time validation:

```markdown
See my [about page](@/about.md) for more details.
Check out this [blog post](@/blog/first-post.md).
```

The `@/` prefix signals an internal link. Paths are relative to `content/`.

| Markdown Link | Resolved HTML |
|---------------|--------------|
| `[about page](@/about.md)` | `<a href="/about/">about page</a>` |
| `[blog post](@/blog/post.md)` | `<a href="/blog/post/">blog post</a>` |

If an internal link references a non-existent file, the build fails with a clear error.

### Shortcodes

Embeds and reusable markup live in [Shortcodes](./shortcodes.md):

```markdown
{{ youtube(id="dQw4w9WgXcQ") }}
```

### Images

```markdown
![Alt text](/images/photo.png)
```

### Code

````markdown
Inline `code` in text.

```rust
fn main() {
    println!("Hello, world!");
}
```
````

Code blocks with a language identifier are highlighted using tree-sitter. See [Syntax Highlighting](./syntax-highlighting.md) for configuration and supported languages.

### Blockquotes

```markdown
> This is a blockquote.
```

## Blog Features

### Summary and Excerpt

Taxus automatically extracts a summary for each page:

1. **Automatic extraction**: First paragraph of content
2. **Manual marker**: Use `<!-- more -->` to mark where summary ends
3. **Frontmatter override**: Set a custom summary in frontmatter

```markdown
+++
title = "My Post"
summary = "A custom summary for SEO"
+++

This is the first paragraph.

<!-- more -->

The rest appears after the summary...
```

Access in templates: `{{ page.summary }}`

### Reading Time and Word Count

Each page calculates reading time (200 words/minute) and word count:

```html
<span class="reading-time">{{ page.reading_time }} min read</span>
<span class="word-count">{{ page.word_count }} words</span>
```

## Taxonomies

Taxus supports three taxonomy types:

### Tags

Multiple keywords associated with a page:

```markdown
+++
title = "Introduction to Rust"
tags = ["rust", "programming", "tutorial"]
+++
```

### Categories

Broader classifications (also multiple):

```markdown
+++
title = "My Tutorial"
categories = ["tutorial", "beginner"]
+++
```

### Series

Groups related posts in a sequence (single value):

```markdown
+++
title = "Part 1: Getting Started"
series = "Learning Rust"
+++
```

### Taxonomy Pages

Taxus generates taxonomy listing and term pages automatically when the corresponding templates exist. The scaffold (`taxus init`) creates all six templates:

| Template | URL | Purpose |
|----------|-----|---------|
| `tags.html` | `/tags/` | Lists all tags |
| `tags_term.html` | `/tags/rust/` | Lists pages with the "rust" tag |
| `categories.html` | `/categories/` | Lists all categories |
| `categories_term.html` | `/categories/tutorial/` | Lists pages in "tutorial" category |
| `series.html` | `/series/` | Lists all series |
| `series_term.html` | `/series/learning-rust/` | Lists pages in "Learning Rust" series |

If a template is missing, that particular page is skipped silently. See [Templates](./templates.md) for the full taxonomy template context and examples.

## Section listings

A section's `section.pages` are its **direct children**: the pages in its
own directory, and nothing deeper. `blog/` lists `blog/my-post.md` but not
`blog/2026/older-post.md` (unless `blog/2026` is
[flattened](#organising-files-without-changing-urls)), and the root `_index.md` lists only pages at the
top of `content/`.

To list pages a section does not own — the classic "recent posts on the
homepage" — declare where they come from:

```markdown
+++
title = "Home"
pages_from = ["blog"]
+++
```

`pages_from` names sections by their content-relative path (`"blog"`,
`"blog/2026"`). A directory flattened by `[build] flatten` is not a section,
so it cannot be named here; see
[Organising files without changing URLs](#organising-files-without-changing-urls). Each donor contributes its own direct pages; the merged list
is deduplicated and sorted by this section's `sort_by`, and `paginate_by`
slices it like any other listing. A `pages_from` entry that names a section
that does not exist is ignored with a warning in the build log.

## Pagination

Enable pagination in a section's `_index.md`:

```markdown
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
+++

# Blog

Welcome to my blog!
```

### Pagination Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sort_by` | string | `"date"` | Sort order: `"date"` (newest first, undated last), `"title"` (case-insensitive), `"weight"` (lowest first), `"none"` (tree order) |
| `paginate_by` | number | `0` | Pages per slice (0 = no pagination) |
| `paginate_template` | string | `None` | Template for paginated pages |

### Pagination URLs

- `/blog/` — First page
- `/blog/page/2/` — Second page
- `/blog/page/3/` — Third page

### Pagination in Templates

```html
{% if section.pagination %}
<nav class="pagination">
  {% if section.pagination.prev %}
  <a href="{{ section.pagination.prev }}">← Previous</a>
  {% endif %}
  
  <span>Page {{ section.pagination.current }} of {{ section.pagination.total }}</span>
  
  {% if section.pagination.next %}
  <a href="{{ section.pagination.next }}">Next →</a>
  {% endif %}
</nav>
{% endif %}
```

## RSS/Atom Feeds

Configure feeds in `site.toml`:

```toml
[feed]
rss_enabled = true
atom_enabled = true
limit = 20
full_content = false
sections = ["blog"]
```

### What a feed contains

Feeds syndicate **dated pages**: every non-draft page with a `date`, newest
first, up to `limit` entries. Section index pages (`_index.md`) and undated
pages such as `/about/` are never feed entries — an undated page has no
publication date to announce, and stamping it with the build time would
re-announce it to subscribers on every build.

`sections` narrows the feed to pages under the named sections
(content-relative paths such as `"blog"` or `"blog/2026"`; a page anywhere
beneath a listed section counts). A flattened directory is not a section,
so name its parent instead. Leave it out to syndicate dated pages from
the whole site. An entry that names no section is ignored with a warning in
the build log.

### Feed Entry Fields

| Field | Source |
|-------|--------|
| Title | Page `title` |
| Description | Page `description` or auto-extracted summary |
| URL | Full page URL (`base_url` + path) |
| Published | Page `date` field |
| Updated | Page `updated` field (Atom only) |

### Feed URLs

- RSS: `https://example.com/feed.xml` (or custom `rss_path`)
- Atom: `https://example.com/feed.atom` (or custom `atom_path`)

## Sitemap Generation

Taxus generates `sitemap.xml` automatically:

- All routes included (pages and sections)
- Draft pages excluded
- Last modification date from page `date` field
- Priorities: home `1.0`, sections `0.8`, pages `0.7`

### Sitemap URL

`https://example.com/sitemap.xml`

## Robots.txt Generation

Taxus generates `robots.txt` automatically if no `static/robots.txt` exists:

```text
User-agent: *
Allow: /

Sitemap: https://example.com/sitemap.xml
```

To provide a custom `robots.txt`, create it in `static/`.
