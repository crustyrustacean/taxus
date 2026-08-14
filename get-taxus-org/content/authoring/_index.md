+++
title = "Authoring"
+++

### Write with Ease

Author content using the CommonMark markdown standard, thanks to the [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) crate.

Intuitively build out the `content` directory to define your site. Create directories that correspond to your routes. Write `_index.md` files for section pages, and individual markdown files for regular pages.

### Describe with Rich Metadata

Metadata is defined via TOML frontmatter:

```toml
+++
title = "My Page"
description = "A brief description"
date = 2024-01-15
tags = ["rust", "web"]
+++
```

Frontmatter is parsed and converted into injectable variables for templates.

### Co-located Assets

Place images alongside your markdown—they're copied to output automatically:

```
content/blog/
├── my-post.md
├── photo.jpg
└── diagrams/
    └── architecture.png
```

### Internal Links

Reference other pages by content path with build-time validation. The `@/` prefix signals an internal link relative to `content/`:

```markdown
See the [other page](@/path/to/page.md) for details.
```

### References

- [pulldown-cmark guide](https://pulldown-cmark.github.io/pulldown-cmark/)
- [Content documentation](https://crustyrustacean.github.io/taxus/content.html)
