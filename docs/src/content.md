# Content

Content in Taxus is written in Markdown files with TOML frontmatter.

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

**Referencing co-located assets:**

```markdown
![Photo](photo.jpg)
![Diagram](diagrams/architecture.png)
```

Or use absolute paths from the site root:

```markdown
![Photo](/blog/photo.jpg)
```

**When to use co-located assets:**

- Blog post images and diagrams
- Page-specific downloads (PDFs, etc.)
- Content-specific data files

For global assets (logos, favicons, shared images), use the `static/` directory instead.

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

[extra]
author = "John Doe"
tags = ["rust", "web"]
+++

# Page Content

Your markdown content here.
```

### Frontmatter Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `title` | string | No\* | `""` | Page title |
| `description` | string | No | `None` | Page description for SEO |
| `date` | date | No | `None` | Publication date (YYYY-MM-DD) |
| `updated` | date | No | `None` | Last updated date |
| `template` | string | No | `"page.html"` | Template override |
| `draft` | bool | No | `false` | Draft status |
| `summary` | string | No | `None` | Custom summary/excerpt |
| `slug` | string | No | `None` | Custom URL slug |
| `aliases` | array | No | `[]` | Old URLs that redirect to this page |
| `tags` | array | No | `[]` | Tags (e.g., `["rust", "web"]`) |
| `categories` | array | No | `[]` | Categories (e.g., `["tutorial"]`) |
| `series` | string | No | `None` | Series name (e.g., `"Learning Rust"`) |
| `sort_by` | string | No | `"date"` | Sort order for sections: `"date"`, `"title"`, `"weight"`, `"none"` |
| `paginate_by` | number | No | `0` | Items per page (0 = no pagination) |
| `paginate_template` | string | No | `None` | Template for paginated pages |
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

This creates `/blog/hello-world/` instead of `/blog/my-first-post/`.

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

Taxus generates taxonomy listing pages automatically:

- `/tags/` — Lists all tags
- `/tags/rust/` — Lists pages with the "rust" tag
- `/categories/` — Lists all categories
- `/categories/tutorial/` — Lists pages in "tutorial" category
- `/series/` — Lists all series
- `/series/learning-rust/` — Lists pages in "Learning Rust" series

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
| `sort_by` | string | `"date"` | Sort order: `"date"`, `"weight"`, `"title"`, `"none"` |
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
```

### Feed Entry Fields

| Field | Source |
|-------|--------|
| Title | Page `title` |
| Description | Page `description` or auto-extracted summary |
| URL | Full page URL (`base_url` + path) |
| Published | Page `date` field |
| Updated | Page `updated` field (Atom only) |

### Feed URLs

- RSS: `https://example.com/rss.xml`
- Atom: `https://example.com/atom.xml`

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
