# Styling

Yew SSG supports SCSS for modern CSS authoring.

## Styles Directory

SCSS files are stored in the `styles/` directory:

```
styles/
└── main.scss
```

## SCSS Compilation

The generator compiles SCSS to CSS during the build process:

1. Read SCSS files from `styles/`
2. Compile to CSS using `grass`
3. Write to `dist/css/`

## Example Stylesheet

`styles/main.scss`:

```scss
// Main stylesheet

// Variables
$primary-color: #0066cc;
$text-color: #333;
$background: #fff;

// Base styles
body {
  font-family:
    -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  line-height: 1.6;
  color: $text-color;
  background: $background;
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
}

// Headings
h1,
h2,
h3 {
  margin-top: 1.5em;
  color: darken($text-color, 10%);
}

// Links
a {
  color: $primary-color;
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}

// Code blocks
pre {
  background: #f4f4f4;
  padding: 1rem;
  border-radius: 4px;
  overflow-x: auto;
}

code {
  font-family: "Consolas", "Monaco", monospace;
  font-size: 0.9em;
}
```

## SCSS Features

### Variables

```scss
$primary: #0066cc;
$spacing: 1rem;

.button {
  background: $primary;
  padding: $spacing;
}
```

### Nesting

```scss
nav {
  ul {
    list-style: none;
  }

  li {
    display: inline-block;
  }

  a {
    color: $primary;
  }
}
```

### Partials

Split styles into multiple files:

```
styles/
├── main.scss        # Main file
├── _variables.scss  # Variables
├── _base.scss       # Base styles
├── _nav.scss        # Navigation
└── _footer.scss     # Footer
```

Import in main file:

```scss
@use "variables";
@use "base";
@use "nav";
@use "footer";
```

### Mixins

```scss
@mixin flex-center {
  display: flex;
  justify-content: center;
  align-items: center;
}

.container {
  @include flex-center;
}
```

## Output

Compiled CSS is written to:

```
dist/
└── css/
    └── main.css
```

## Linking Styles

Include the stylesheet in your template:

```html
<link rel="stylesheet" href="/css/main.css" />
```

## Development

For development, you can use the `sass` CLI for live compilation:

```bash
# Install sass
npm install -g sass

# Watch for changes
sass --watch styles/main.scss dist/css/main.css
```

## Future Enhancements

Planned improvements for styling:

- **PostCSS**: Autoprefixer and other transformations
- **CSS minification**: Production-ready output
- **Source maps**: Debug support
- **CSS modules**: Scoped styles for components
