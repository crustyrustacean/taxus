# Introduction

Welcome to the **Yew Static Site Generator (SSG)** documentation. This is a Rust-based static site generator built with [Yew](https://yew.rs/), designed for building fast, SEO-friendly websites with the power of WebAssembly.

## What is Yew SSG?

Yew SSG combines a Tera-based static site generator with the Yew WebAssembly framework using the **Islands Architecture**:

- **Tera templates** render the static "sea" of HTML — page layout, content, navigation
- **Yew components** are the "islands" — pre-rendered server-side at build time, then hydrated by WASM in the browser for interactivity
- **Markdown content** with TOML frontmatter drives the content system
- **SCSS** compiles to CSS for modern styling

## Key Features

### Islands Architecture

Generate static HTML that includes pre-rendered Yew components. When the WASM bundle loads in the browser, those components are hydrated in-place without re-rendering the page:

```html
<!-- Generated at build time: -->
<div data-island="Counter" data-props='{"initial":3}'>
  <div class="counter"><span>3</span><button>+</button></div>
</div>
```

The page is immediately visible with no JavaScript required. Interactivity layer loads asynchronously.

### Two-Tier Interactivity

| Tier | Technology | Use for |
|---|---|---|
| General | `static/scripts.js` (vanilla JS) | Menus, toggles, analytics, lightweight DOM work |
| Performance | Yew WASM island | Heavy computation, complex reactive state, data-intensive UI |

### Tera Templates with Island Support

Embed Yew components directly in Tera templates using the `island()` function:

```html
{% block content %}
  {{ page.content | safe }}
  {{ island(component="Counter", initial=3) | safe }}
{% endblock %}
```

### Multi-Crate Workspace

The project is organized as a multi-crate workspace:

- **`common`**: Yew components shared between the generator (SSR) and the WASM client (hydration)
- **`generator`**: Static site generator library (`yew_ssg_lib`) and CLI binary (`yew-ssg`)
- **`client`**: WASM hydration bootstrap — finds island mount points on page load and attaches Yew renderers

### Static Content System

- Markdown files with TOML frontmatter (`+++...+++`)
- Automatic route discovery: `_index.md` → section, other `.md` → page
- Draft support, date-based sorting, custom template selection per page

## Who is this for?

Yew SSG is ideal for:

- **Rust developers** who want to build websites without leaving their favorite language
- **Performance enthusiasts** who want fast, optimized static sites with selective WASM interactivity
- **SEO-conscious developers** who need pre-rendered content with no JavaScript dependency for initial render
- **Component lovers** who prefer the Yew component model for interactive UI pieces

## License

This project is licensed under the MIT License - see the [License.txt](../License.txt) file for details.
