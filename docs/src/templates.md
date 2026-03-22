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

The generator uses [Tera](https://tera.netlify.app/), a Jinja2-like template engine for Rust. Tera provides:

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

    <!-- Canonical URL and Open Graph meta tags -->
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
      <p>&copy; {{ site.author | default(value="") }}</p>
    </footer>

    <script src="/static/scripts.js"></script>
  </body>
</html>
```

## Page Template

Page templates extend the base template:

```html
{% extends "base.html" %} {% block title %}{{ page.title }} - {{ site.name }}{%
endblock %} {% block content %}
<article>
  <h1>{{ page.title }}</h1>

  {% if page.description %}
  <p class="description">{{ page.description }}</p>
  {% endif %} {% if page.date %}
  <time datetime="{{ page.date }}">{{ page.date }}</time>
  {% endif %}

  <div class="content">{{ page.content | safe }}</div>
</article>
{% endblock %}
```

## Section Template

Section templates render lists of pages:

```html
{% extends "base.html" %} {% block title %}{{ section.title }} - {{ site.name
}}{% endblock %} {% block content %}
<section>
  <h1>{{ section.title }}</h1>

  <ul class="page-list">
    {% for page in section.pages %}
    <li>
      <a href="{{ page.path }}">
        <span class="title">{{ page.title }}</span>
        {% if page.date %}
        <time datetime="{{ page.date }}">{{ page.date }}</time>
        {% endif %}
      </a>
      {% if page.description %}
      <p class="description">{{ page.description }}</p>
      {% endif %}
    </li>
    {% endfor %}
  </ul>
</section>
{% endblock %}
```

## Available Variables

### Site Context

| Variable           | Type    | Description                  |
| ------------------ | ------- | ---------------------------- |
| `site.name`        | String  | Site name from configuration |
| `site.base_url`    | String  | Base URL from configuration  |
| `site.description` | String? | Optional site description    |
| `site.author`      | String? | Optional site author         |

### Page Context

| Variable           | Type    | Description                                       |
| ------------------ | ------- | ------------------------------------------------- |
| `page.title`       | String  | Page title from frontmatter                       |
| `page.description` | String? | Optional page description                         |
| `page.path`        | String  | URL path (e.g., "/about/")                        |
| `page.permalink`   | String  | Absolute URL (e.g., "https://example.com/about/") |
| `page.content`     | String  | Rendered HTML content                             |
| `page.raw_content` | String  | Raw markdown content                              |
| `page.date`        | String? | Publication date (ISO 8601)                       |
| `page.draft`       | Boolean | Whether page is a draft                           |

### Section Context

| Variable        | Type   | Description              |
| --------------- | ------ | ------------------------ |
| `section.title` | String | Section title            |
| `section.path`  | String | Section URL path         |
| `section.pages` | Array  | List of pages in section |

### Extra Variables

Custom variables from frontmatter are available in `extra`:

```markdown
+++
title = "My Page"
[extra]
author = "John Doe"
tags = ["rust", "web"]
+++
```

Access in templates:

```html
<p>Author: {{ extra.author }}</p>
{% for tag in extra.tags %}
<span class="tag">{{ tag }}</span>
{% endfor %}
```

## Template Inheritance

Templates can extend other templates:

**base.html**:

```html
<html>
  <head>
    {% block head %}{% endblock %}
  </head>
  <body>
    {% block body %}{% endblock %}
  </body>
</html>
```

**page.html**:

```html
{% extends "base.html" %} {% block head %}
<title>{{ page.title }}</title>
{% endblock %} {% block body %}
<h1>{{ page.title }}</h1>
{{ page.content | safe }} {% endblock %}
```

## Filters

Commonly used filters:

| Filter                 | Description                        |
| ---------------------- | ---------------------------------- |
| `safe`                 | Output without HTML escaping       |
| `default(value="...")` | Provide default value              |
| `upper`                | Convert to uppercase               |
| `lower`                | Convert to lowercase               |
| `trim`                 | Remove leading/trailing whitespace |
| `first`                | Get first element of array         |
| `last`                 | Get last element of array          |
| `length`               | Get length of string/array         |
| `join(sep=", ")`       | Join array with separator          |

## Custom Templates

Pages can specify custom templates in frontmatter:

```markdown
+++
title = "Special Page"
template = "custom.html"
+++

This page uses custom.html instead of page.html.
```

## Using the Template API

### Loading Templates

```rust
use generator::{TeraRenderer, TemplateRenderer, Result};

fn main() -> Result<()> {
    // Load all templates from directory
    let renderer = TeraRenderer::from_dir("templates")?;

    // Or create empty and register manually
    let mut renderer = TeraRenderer::new()?;
    renderer.register_template("page.html", "<html>...</html>")?;

    Ok(())
}
```

### Rendering Templates

```rust
use generator::{
    TeraRenderer, TemplateRenderer, TemplateContext,
    SiteContext, PageContext, Result
};

fn main() -> Result<()> {
    let mut renderer = TeraRenderer::new()?;
    renderer.register_template("page.html", "<h1>{{ page.title }}</h1>")?;

    let site = SiteContext {
        name: "My Site".to_string(),
        base_url: "https://example.com".to_string(),
        description: None,
        author: None,
    };

    let page = PageContext {
        title: "Hello".to_string(),
        description: None,
        path: "/hello/".to_string(),
        permalink: "https://example.com/hello/".to_string(),
        content: "<p>World</p>".to_string(),
        raw_content: "World".to_string(),
        date: None,
        draft: false,
        summary: String::new(),
        word_count: 1,
        reading_time: 1,
        tags: vec![],
        categories: vec![],
        series: None,
    };

    let ctx = TemplateContext::new(site).with_page(page);
    let html = renderer.render("page.html", &ctx)?;

    println!("{}", html);
    Ok(())
}
```

## Error Handling

Template errors are reported with context:

```rust
use generator::{TeraRenderer, TemplateRenderer, TemplateError};

match renderer.render("missing.html", &ctx) {
    Err(TemplateError::NotFound(name)) => {
        eprintln!("Template not found: {}", name);
    }
    Err(TemplateError::Syntax { template, message }) => {
        eprintln!("Syntax error in {}: {}", template, message);
    }
    Err(TemplateError::Render(message)) => {
        eprintln!("Render error: {}", message);
    }
    Ok(html) => println!("{}", html),
}
```
