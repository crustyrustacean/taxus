# Introduction

Welcome to the **Yew Static Site Generator (SSG)** documentation. This is a Rust-based static site generator built with [Yew](https://yew.rs/), designed for building fast, SEO-friendly websites with the power of WebAssembly.

## What is Yew SSG?

Yew SSG is a static site generator that combines the best of both worlds:

- **Server-Side Rendering (SSR)**: Pre-rendered HTML pages for optimal performance and SEO
- **Client-Side Hydration**: Interactive WebAssembly components for rich user experiences
- **Markdown Content**: Write content in Markdown files for easy content management
- **SCSS Styling**: Modern styling with SCSS support

## Key Features

### Static Site Generation

Generate static HTML files at build time, ensuring:

- Fast page loads
- SEO optimization
- No JavaScript required for initial render
- Easy deployment to any static host

### Yew Components

Build reusable UI components using Yew's functional component API:

```rust
#[component]
fn MyComponent(props: &MyProps) -> Html {
    html! {
        <div class="my-component">
            <h1>{ &props.title }</h1>
            <p>{ &props.content }</p>
        </div>
    }
}
```

### Multi-Crate Workspace

The project is organized as a multi-crate workspace:

- **client**: WebAssembly client application
- **common**: Shared components and utilities
- **generator**: Static site generator library and binary

## Who is this for?

Yew SSG is ideal for:

- **Rust developers** who want to build websites without leaving their favorite language
- **Performance enthusiasts** who want fast, optimized static sites
- **SEO-conscious developers** who need pre-rendered content
- **Component lovers** who prefer component-based architecture

## License

This project is licensed under the MIT License - see the [License.txt](../License.txt) file for details.
