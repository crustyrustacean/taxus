# Templates

Templates define the HTML structure for rendered pages using the [Tera](https://tera.netlify.app/) template engine.

## Template Location

Templates are stored in the `templates/` directory:

```
templates/
├── base.html       # Base template with common structure
├── page.html       # Single page template
└── section.html    # Section/list template (e.g., blog)
```

## Template Engine

Taxus uses [Tera](https://tera.netlify.app/), a Jinja2-like template engine for Rust:

- **Variables**: `{{ variable }}` syntax
- **Filters**: `{{ content | safe }}` for unescaped HTML
- **Conditionals**: `{% if condition %}...{% endif %}`
- **Loops**: `{% for item in items %}...{% endfor %}`
- **Template Inheritance**: `{% extends "base.html" %}` and `{% block name %}`
- **Includes**: `{% include "partial.html" %}`

## Base Template

The base template defines the common HTML structure:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{% block title %}{{ site.name }}{% endblock %}</title>

    {% if page.permalink %}
    <link rel="canonical" href="{{ page.permalink }}" />
    <meta property="og:url" content="{{ page.permalink }}" />
    {% endif %}

    <link rel="stylesheet" href="/css/main.css" />
    <link rel="icon" href="/static/favicon.png" />
  </head>
  <body>
    <header>
      <nav>
        <a href="/">Home</a>
        <a href="/about/">About</a>
      </nav>
    </header>

    <main>{% block content %}{% endblock %}</main>

    <footer>
      <p>&copy; {{ now.year }} {{ site.author | default(value="") }}</p>
    </footer>

    <script src="/static/scripts.js"></script>
  </body>
</html>
```

## Page Template

Page templates extend the base template:

```html
{% extends "base.html" %}

{% block title %}{{ page.title }} - {{ site.name }}{% endblock %}

{% block content %}
<article>
  {% if page.hero %}
  <picture>
    <source srcset="{{ page.hero.srcset | safe }}" type="{{ page.hero.mime_type }}">
    <img src="{{ page.hero.src | safe }}" alt="{{ page.hero.alt }}"
         width="{{ page.hero.width }}" height="{{ page.hero.height }}"
         loading="eager" decoding="async">
  </picture>
  {% endif %}

  <h1>{{ page.title }}</h1>

  {% if page.description %}
  <p class="description">{{ page.description }}</p>
  {% endif %}

  {% if page.date %}
  <time datetime="{{ page.date }}">{{ page.date }}</time>
  {% endif %}

  {% if page.tags %}
  <div class="tags">
    {% for tag in page.tags %}
    <a href="/tags/{{ tag | slugify }}/">{{ tag }}</a>
    {% endfor %}
  </div>
  {% endif %}

  <div class="content">{{ page.content | safe }}</div>
</article>
{% endblock %}
```

## Section Template

Section templates render lists of pages:

```html
{% extends "base.html" %}

{% block title %}{{ section.title }} - {{ site.name }}{% endblock %}

{% block content %}
<section>
  <h1>{{ section.title }}</h1>

  {% if section.description %}
  <p class="description">{{ section.description }}</p>
  {% endif %}

  {% if section.content %}
  <div class="section-content">{{ section.content | safe }}</div>
  {% endif %}

  <ul class="page-list">
    {% for page in section.pages %}
    <li>
      <a href="{{ page.path }}">
        <span class="title">{{ page.title }}</span>
        {% if page.date %}
        <time datetime="{{ page.date }}">{{ page.date }}</time>
        {% endif %}
        <span class="reading-time">{{ page.reading_time }} min read</span>
      </a>
      {% if page.summary %}
      <p class="summary">{{ page.summary }}</p>
      {% endif %}
    </li>
    {% endfor %}
  </ul>

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
</section>
{% endblock %}
```

## Available Variables

### Site Context

| Variable | Type | Description |
|----------|------|-------------|
| `site.name` | String | Site name from configuration |
| `site.base_url` | String | Base URL from configuration |
| `site.description` | String? | Optional site description |
| `site.author` | String? | Optional site author |

### Page Context

| Variable | Type | Description |
|----------|------|-------------|
| `page.title` | String | Page title from frontmatter |
| `page.description` | String? | Optional page description |
| `page.path` | String | URL path (e.g., `/about/`) |
| `page.permalink` | String | Absolute URL (e.g., `https://example.com/about/`) |
| `page.content` | String | Rendered HTML content |
| `page.raw_content` | String | Raw markdown content |
| `page.date` | String? | Publication date (ISO 8601) |
| `page.draft` | Boolean | Whether page is a draft |
| `page.summary` | String | Summary/excerpt for the page |
| `page.word_count` | Number | Word count |
| `page.reading_time` | Number | Estimated reading time in minutes |
| `page.tags` | Array | Tags for the page |
| `page.categories` | Array | Categories for the page |
| `page.series` | String? | Series name |
| `page.hero` | Object? | Hero image context (see below) |

### Hero Image Context

When a page has `hero_image` in its frontmatter, `page.hero` contains:

| Variable | Type | Description |
|----------|------|-------------|
| `page.hero.src` | String | Fallback `<img>` src (middle variant) |
| `page.hero.srcset` | String | Full srcset string for `<source>` |
| `page.hero.width` | Number | Original image width |
| `page.hero.height` | Number | Original image height |
| `page.hero.alt` | String | Alt text (from `hero_alt`, or page title) |
| `page.hero.mime_type` | String | MIME type (e.g., `"image/webp"`) |

Example usage:

```html
{% if page.hero %}
<picture>
  <source srcset="{{ page.hero.srcset | safe }}" type="{{ page.hero.mime_type }}">
  <img src="{{ page.hero.src | safe }}" alt="{{ page.hero.alt }}"
       width="{{ page.hero.width }}" height="{{ page.hero.height }}"
       loading="eager" decoding="async">
</picture>
{% endif %}
```

See [Images](./images.md) for the complete guide.

### Section Context

| Variable | Type | Description |
|----------|------|-------------|
| `section.title` | String | Section title |
| `section.description` | String? | Optional section description |
| `section.path` | String | Section URL path |
| `section.content` | String? | Section HTML content |
| `section.pages` | Array | List of pages in section |
| `section.pagination` | Object? | Pagination information |

### Pagination Context

| Variable | Type | Description |
|----------|------|-------------|
| `section.pagination.current` | Number | Current page (1-indexed) |
| `section.pagination.total` | Number | Total pages |
| `section.pagination.per_page` | Number | Items per page |
| `section.pagination.total_items` | Number | Total items across all pages |
| `section.pagination.prev` | String? | URL to previous page |
| `section.pagination.next` | String? | URL to next page |
| `section.pagination.first` | String | URL to first page |
| `section.pagination.last` | String | URL to last page |

### Current Date

| Variable | Type | Description |
|----------|------|-------------|
| `now.year` | Number | Current year (e.g., 2024) |

Useful for copyright notices: `&copy; {{ now.year }}`

### Extra Variables

Custom variables from frontmatter `extra` field:

```markdown
+++
title = "My Page"
[extra]
author = "John Doe"
custom = "value"
+++
```

Access in templates:

```html
<p>Author: {{ extra.author }}</p>
<p>Custom: {{ extra.custom }}</p>
```

## Template Inheritance

Templates can extend other templates:

**base.html**:

```html
<html>
  <head>{% block head %}{% endblock %}</head>
  <body>{% block body %}{% endblock %}</body>
</html>
```

**page.html**:

```html
{% extends "base.html" %}

{% block head %}
<title>{{ page.title }}</title>
{% endblock %}

{% block body %}
<h1>{{ page.title }}</h1>
{{ page.content | safe }}
{% endblock %}
```

## Filters

Commonly used filters:

| Filter | Description |
|--------|-------------|
| `safe` | Output without HTML escaping |
| `default(value="...")` | Provide default value |
| `upper` | Convert to uppercase |
| `lower` | Convert to lowercase |
| `trim` | Remove leading/trailing whitespace |
| `first` | Get first element of array |
| `last` | Get last element of array |
| `length` | Get length of string/array |
| `join(sep=", ")` | Join array with separator |
| `slugify` | Convert to URL-safe slug |

## Custom Templates

Pages can specify custom templates in frontmatter:

```markdown
+++
title = "Special Page"
template = "custom.html"
+++

This page uses custom.html instead of page.html.
```

## Island Components

Use the `island()` function to embed interactive Yew components:

```html
{% block content %}
  {{ page.content | safe }}
  {{ island(component="Counter", initial=5) | safe }}
{% endblock %}
```

**Important**: Always use `| safe` after `island()` to prevent HTML escaping.

See [Islands Architecture](./islands.md) for the complete guide.
