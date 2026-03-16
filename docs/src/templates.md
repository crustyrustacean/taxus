# Templates

Templates define the HTML structure for rendered pages.

## Template Location

Templates are stored in the `templates/` directory:

```
templates/
└── index.txt
```

## Base Template

The base template (`templates/index.txt`) defines the HTML structure:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Yew SSG</title>
    <link rel="stylesheet" href="/css/styles.css">
    <link rel="icon" href="/favicon.png">
    <script type="module">
        import init from '/scripts.js';
        init();
    </script>
</head>
<body>
    {body}
</body>
</html>
```

## Template Variables

The template uses `{body}` as a placeholder for the rendered content:

| Variable | Description |
|----------|-------------|
| `{body}` | Rendered HTML from Yew components |

## Template Processing

The generator processes templates in the following order:

1. Read the template file
2. Render Yew components to HTML
3. Replace `{body}` with rendered HTML
4. Write to output file

## Current Implementation

The current implementation uses a simple string replacement:

```rust
let template = fs::read_to_string("templates/index.txt")?;
let body_content = template.replace("{body}", &html);
```

## Future Enhancements

Planned improvements for the template system:

### Tera Templates

Integration with [Tera](https://tera.netlify.app/) for advanced templating:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{{ page.title }} | {{ site.name }}</title>
    <meta name="description" content="{{ page.description }}">
    <link rel="stylesheet" href="/css/styles.css">
</head>
<body>
    {% block header %}
    <header>
        <nav>
            <a href="/">Home</a>
            <a href="/about/">About</a>
        </nav>
    </header>
    {% endblock %}
    
    <main>
        {% block content %}{% endblock %}
    </main>
    
    {% block footer %}
    <footer>
        <p>&copy; {{ site.author }}</p>
    </footer>
    {% endblock %}
</body>
</html>
```

### Template Inheritance

Support for template inheritance:

```
templates/
├── base.html       # Base template
├── page.html       # Page template (extends base)
└── section.html    # Section template (extends base)
```

### Available Variables

| Variable | Type | Description |
|----------|------|-------------|
| `site` | Object | Site configuration |
| `page` | Object | Current page data |
| `page.title` | String | Page title |
| `page.content` | String | Rendered HTML content |
| `page.url` | String | Page URL path |

### Custom Templates

Allow pages to specify custom templates:

```markdown
+++
title = "Special Page"
template = "custom.html"
+++

Content here.
```
