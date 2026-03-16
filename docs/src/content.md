# Content

Content in Yew SSG is written in Markdown files with TOML frontmatter.

## Content Files

Content files are stored in the `content/pages/` directory:

```
content/
└── pages/
    ├── home.md
    └── about.md
```

## Frontmatter

Each Markdown file should include TOML frontmatter enclosed in `+++`:

```markdown
+++
title = "Page Title"
+++

# Page Content

Your markdown content here.
```

### Frontmatter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | Yes | Page title |

## Example Pages

### Home Page

`content/pages/home.md`:

```markdown
+++
title = "Welcome"
+++

# Welcome to My Site

This is the home page content.

## Features

- Feature one
- Feature two
- Feature three
```

### About Page

`content/pages/about.md`:

```markdown
+++
title = "About"
+++

# About

This is the about page.

## Contact

- Email: example@example.com
- GitHub: @username
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
2. Parse the frontmatter
3. Convert Markdown to HTML
4. Render with Yew components
5. Apply the template
6. Write to output directory

## Output Structure

Content files are mapped to output paths:

| Content File | Output Path |
|--------------|-------------|
| `content/pages/home.md` | `dist/index.html` |
| `content/pages/about.md` | `dist/about/index.html` |

## Future Enhancements

Planned features for content handling:

- **Sections**: Group related pages (e.g., blog posts)
- **Taxonomies**: Tags and categories
- **Drafts**: Publish status control
- **Date-based routing**: For blog posts
- **Custom frontmatter**: User-defined fields
