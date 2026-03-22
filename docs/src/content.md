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

## Future Enhancements

Planned features for content handling:

- **Pagination**: Split large collections
- **Related content**: Suggest related pages
- **Content relationships**: Parent/child pages
- **Multilingual support**: Translated content
