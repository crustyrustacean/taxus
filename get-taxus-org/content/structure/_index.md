+++
title = "Structure"
+++

## Template Power

### Sensible but Flexible

Taxus sites are built with [Tera](https://keats.github.io/tera) templates. Sensible defaults are provided: `base.html`, `section.html`, and `page.html`. Override them with your own, or specify custom templates per-page in frontmatter.

### Variables

Frontmatter content becomes variables injected into templates. Site configuration from `site.toml` is also available:

```html
<h1>{{ page.title }}</h1>
<p>{{ site.description }}</p>
```

### Partials

Create reusable template components in `templates/partials/`. Reference them with standard Tera syntax:

```html
{% include "partials/nav.html" %}
```

### Tera Syntax

The full Tera syntax is available—inheritance with `{% extends %}` and `{% block %}`, loops, conditionals, filters, and functions. Build templates as you see fit.

### Documentation

For complete template reference, see the [Templates documentation](https://crustyrustacean.github.io/taxus/templates.html).
