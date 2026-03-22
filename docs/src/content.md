# Content

Content in Yew SSG is written in Markdown files with TOML frontmatter.

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
| `title` | string | No* | `""` | Page title |
| `description` | string | No | `None` | Page description for SEO |
| `date` | date | No | `None` | Publication date (YYYY-MM-DD) |
| `template` | string | No | `"page.html"` | Template override |
| `draft` | bool | No | `false` | Draft status |
| `summary` | string | No | `None` | Custom summary/excerpt (overrides automatic extraction) |
| `slug` | string | No | `None` | Custom URL slug (overrides filename-based path) |
| `tags` | array | No | `[]` | Tags for the page (e.g., `["rust", "web"]`) |
| `categories` | array | No | `[]` | Categories for the page (e.g., `["tutorial"]`) |
| `series` | string | No | `None` | Series the page belongs to (e.g., `"Learning Rust"`) |
| `aliases` | array | No | `[]` | Old URLs that should redirect to this page |
| `extra` | table | No | `None` | Custom metadata |

*Title is recommended but not required by the library.

### Date Format

Dates use the TOML date format:

```toml
date = 2024-01-15
```

This is parsed as a `chrono::NaiveDate` and can be used for sorting blog posts.

### Extra Metadata

The `extra` field allows custom metadata:

```toml
[extra]
author = "Jane Doe"
tags = ["rust", "yew", "ssg"]
custom_field = "any value"
```

Access extra metadata in templates through the page context.

## Page Types

### Regular Pages

Regular pages are Markdown files in the content directory:

```
content/
├── about.md    # Creates /about/
└── contact.md  # Creates /contact/
```

### Index Pages

`_index.md` files create index pages:

- `content/_index.md` → Home page (`/`)
- `content/blog/_index.md` → Blog index (`/blog/`)

### Sections

Directories with an `_index.md` file become sections:

```
content/blog/
├── _index.md    # Section index
├── first-post.md
└── second-post.md
```

Sections can:
- Have their own frontmatter
- Contain multiple pages
- Be sorted by date
- Use custom templates

## Example Pages

### Home Page

`content/_index.md`:

```markdown
+++
title = "Welcome"
description = "Welcome to my site"
+++

# Welcome to My Site

This is the home page content.

## Features

- Feature one
- Feature two
- Feature three
```

### About Page

`content/about.md`:

```markdown
+++
title = "About"
description = "About this site"
+++

# About

This is the about page.

## Contact

- Email: example@example.com
- GitHub: @username
```

### Blog Section

`content/blog/_index.md`:

```markdown
+++
title = "Blog"
description = "My thoughts and tutorials"
template = "section.html"
+++

# Blog

Welcome to my blog!
```

### Blog Post

`content/blog/my-first-post.md`:

```markdown
+++
title = "My First Post"
description = "Getting started with Yew SSG"
date = 2024-01-15
+++

# My First Post

This is my first blog post using Yew SSG.

## Introduction

Content goes here...
```

### Draft Post

`content/blog/work-in-progress.md`:

```markdown
+++
title = "Work in Progress"
date = 2024-02-01
draft = true
+++

# Work in Progress

This post is not ready for publication.
```

## Markdown Support

Yew SSG supports standard Markdown syntax:

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

### Images

```markdown
![Alt text](/images/photo.png)
```

### Code

```markdown
Inline `code` in text.

```rust
fn main() {
    println!("Hello, world!");
}
```
```

### Blockquotes

```markdown
> This is a blockquote.
```

## Content Processing

The generator processes content files in the following order:

1. Read the Markdown file
2. Parse the frontmatter (between `+++` markers)
3. Extract the raw Markdown content
4. Convert Markdown to HTML
5. Render with Yew components
6. Apply the template
7. Write to output directory

## URL Path Generation

Content files are mapped to URL paths:

| Content File | URL Path |
|--------------|----------|
| `content/_index.md` | `/` |
| `content/about.md` | `/about/` |
| `content/blog/_index.md` | `/blog/` |
| `content/blog/my-post.md` | `/blog/my-post/` |

### Custom Slugs

You can override the default URL path generation using the `slug` frontmatter field:

```markdown
+++
title = "My First Post"
slug = "hello-world"
+++

# My First Post
```

This creates the URL `/blog/hello-world/` instead of `/blog/my-first-post/`.

## Blog Features

### Summary and Excerpt

Yew SSG automatically extracts a summary/excerpt for each page. You can control this in three ways:

1. **Automatic extraction**: The first paragraph of content is used as the summary
2. **Manual marker**: Use `<!-- more -->` to mark where the summary ends
3. **Frontmatter override**: Set a custom summary in frontmatter

```markdown
+++
title = "My Post"
summary = "A custom summary for SEO and previews"
+++

This is the first paragraph that would normally be used as the summary.

<!-- more -->

The rest of the content appears after the summary...
```

In templates, access the summary via:

```html
<div class="excerpt">{{ page.summary }}</div>
```

### Reading Time and Word Count

Each page automatically calculates reading time and word count:

- **Word count**: Total words in the content
- **Reading time**: Estimated minutes to read (assuming 200 words/minute)

Access these in templates:

```html
<span class="reading-time">{{ page.reading_time }} min read</span>
<span class="word-count">{{ page.word_count }} words</span>
```

## Taxonomies

Yew SSG supports three types of taxonomies for content organization:

### Tags

Tags are multiple keywords associated with a page:

```markdown
+++
title = "Introduction to Rust"
tags = ["rust", "programming", "tutorial"]
+++
```

### Categories

Categories are broader classifications (also multiple):

```markdown
+++
title = "My Tutorial"
categories = ["tutorial", "beginner"]
+++
```

### Series

Series groups related posts in a sequence (single value):

```markdown
+++
title = "Part 1: Getting Started"
series = "Learning Rust"
+++
```

### Taxonomy Pages

Yew SSG automatically generates taxonomy listing pages:

- `/tags/` - Lists all tags
- `/tags/rust/` - Lists all pages with the "rust" tag
- `/categories/` - Lists all categories
- `/categories/tutorial/` - Lists all pages in the "tutorial" category
- `/series/` - Lists all series
- `/series/learning-rust/` - Lists all pages in the "Learning Rust" series

### Taxonomy Templates

Create custom templates for taxonomy pages:

**`templates/tag.html`**:
```html
{% extends "base.html" %}

{% block content %}
<h1>Posts tagged "{{ term }}"</h1>
<ul>
{% for page in pages %}
  <li><a href="{{ page.path }}">{{ page.title }}</a></li>
{% endfor %}
</ul>
{% endblock %}
```

Similar templates: `category.html`, `series.html`, `tags.html`, `categories.html`, `series.html`.

## Using the Content API

### Loading a Page

```rust
use generator::{Page, Result};

fn main() -> Result<()> {
    let page = Page::from_file("content/about.md")?;
    
    println!("Title: {}", page.frontmatter.title);
    println!("Path: {}", page.path);
    println!("Is draft: {}", page.is_draft());
    
    Ok(())
}
```

### Loading a Section

```rust
use generator::{Section, Page, Result};

fn main() -> Result<()> {
    let mut section = Section::from_dir("content/blog")?;
    
    // Add pages to section
    let post = Page::from_file("content/blog/my-post.md")?;
    section.add_page(post);
    
    // Sort by date (newest first)
    section.sort_by_date();
    
    println!("Section: {}", section.frontmatter.title);
    for page in &section.pages {
        println!("  - {} ({:?})", page.frontmatter.title, page.frontmatter.date);
    }
    
    Ok(())
}
```

### Listing Content Files

```rust
use generator::{ContentSource, FilesystemContentSource, Result};

fn main() -> Result<()> {
    let source = FilesystemContentSource::new("content");
    
    for file in source.list()? {
        println!("Found: {}", file.display());
    }
    
    Ok(())
}
```

## Pagination

Yew SSG supports pagination for sections with large collections of pages. This is useful for blogs with many posts.

### Enabling Pagination

Add pagination settings to a section's `_index.md` frontmatter:

```markdown
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
paginate_template = "blog.html"
+++

# Blog

Welcome to my blog!
```

### Pagination Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sort_by` | string | `"none"` | Sort order: `"date"`, `"weight"`, or `"none"` |
| `paginate_by` | integer | `None` | Number of pages per paginated slice |
| `paginate_template` | string | `None` | Template for paginated pages (defaults to section template) |

### Sorting Options

- **`"date"`**: Sort by publication date (newest first)
- **`"weight"`**: Sort by weight field in page frontmatter (lowest first)
- **`"none"`**: No sorting (filesystem order)

### Weight-Based Sorting

Add a `weight` field to page frontmatter for custom ordering:

```markdown
+++
title = "Getting Started"
weight = 1
+++
```

```markdown
+++
title = "Advanced Topics"
weight = 2
+++
```

### Pagination URLs

When pagination is enabled, the section generates multiple pages:

- `/blog/` - First page (page 1)
- `/blog/page/2/` - Second page
- `/blog/page/3/` - Third page
- And so on...

### Pagination in Templates

Access pagination information in your section template:

```html
{% extends "base.html" %}

{% block content %}
<h1>{{ section.title }}</h1>

{% if section.pagination %}
<div class="pagination-info">
  Page {{ section.pagination.current_page }} of {{ section.pagination.total_pages }}
  ({{ section.pagination.total_items }} items)
</div>
{% endif %}

<ul class="post-list">
{% for page in section.pages %}
  <li>
    <a href="{{ page.path }}">{{ page.title }}</a>
    <span class="date">{{ page.date }}</span>
  </li>
{% endfor %}
</ul>

{% if section.pagination %}
<nav class="pagination">
  {% if section.pagination.prev_url %}
  <a href="{{ section.pagination.prev_url }}" class="prev">← Previous</a>
  {% endif %}
  
  <span class="page-numbers">
    Page {{ section.pagination.current_page }} of {{ section.pagination.total_pages }}
  </span>
  
  {% if section.pagination.next_url %}
  <a href="{{ section.pagination.next_url }}" class="next">Next →</a>
  {% endif %}
</nav>
{% endif %}
{% endblock %}
```

### Pagination Context Fields

| Field | Type | Description |
|-------|------|-------------|
| `current_page` | integer | Current page number (1-indexed) |
| `total_pages` | integer | Total number of pages |
| `total_items` | integer | Total number of items across all pages |
| `items_per_page` | integer | Items per page |
| `prev_url` | string | URL to previous page (None on first page) |
| `next_url` | string | URL to next page (None on last page) |
| `first_url` | string | URL to first page |
| `last_url` | string | URL to last page |

## RSS/Atom Feeds

Yew SSG can automatically generate RSS and Atom feeds for your content.

### Enabling Feeds

Add feed configuration to your `site.toml`:

```toml
[site]
name = "My Blog"
base_url = "https://example.com"

[feed]
enabled = true
format = "rss"  # or "atom" or "both"
path = "feed.xml"  # RSS feed path
atom_path = "atom.xml"  # Atom feed path
```

### Feed Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable feed generation |
| `format` | string | `"rss"` | Feed format: `"rss"`, `"atom"`, or `"both"` |
| `path` | string | `"feed.xml"` | RSS feed output path |
| `atom_path` | string | `"atom.xml"` | Atom feed output path |

### Feed Entries

Feeds are generated from section pages. To include pages in feeds:

1. Add a `date` field to page frontmatter
2. Optionally add an `updated` field for Atom feeds
3. The feed will include the most recent pages

```markdown
+++
title = "My Blog Post"
date = 2024-01-15
description = "A brief description for the feed"
+++

# My Blog Post

Content here...
```

### Feed URLs

After generation, feeds are available at:

- RSS: `https://example.com/feed.xml`
- Atom: `https://example.com/atom.xml`

### Feed Entry Fields

Each feed entry includes:

| Field | Source |
|-------|--------|
| Title | Page `title` |
| Description | Page `description` or auto-extracted summary |
| URL | Full page URL (base_url + path) |
| Published | Page `date` field |
| Updated | Page `updated` field (Atom only) |

## Future Enhancements

Planned features for content handling:

- **Related content**: Suggest related pages
- **Content relationships**: Parent/child pages
- **Multilingual support**: Translated content
