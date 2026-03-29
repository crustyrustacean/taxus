# Plan: Add 404.html Page Generation

## Overview

Add automatic generation of a `404.html` page for unknown routes. This is a standard static site feature that most hosting providers (Netlify, Vercel, GitHub Pages, etc.) automatically serve for non-existent routes.

## Implementation

### 1. Pipeline Changes (`generator/src/build/pipeline.rs`)

Add functions to generate and write the 404 page:

```rust
/// Generated 404.html content.
#[derive(Debug, Clone)]
pub struct Generated404 {
    /// HTML content
    pub content: String,
}

/// Generate 404.html page.
///
/// If 404.html template doesn't exist, returns None.
/// Otherwise, renders the template with site context.
pub fn generate_404(
    templates: &TeraRenderer,
    site_context: &SiteContext,
) -> Result<Option<Generated404>> {
    // Check if 404.html template exists
    if !templates.has_template("404.html") {
        return Ok(None);
    }
    
    // Render with just site context (no page/section context)
    let context = TemplateContext::new(site_context.clone());
    let content = templates.render("404.html", &context)?;
    
    Ok(Some(Generated404 { content }))
}

/// Write 404.html to output directory.
pub fn write_404(page: &Generated404, output_dir: &Path, dry_run: bool) -> Result<()> {
    // Write to dist/404.html (not in a subdirectory)
}
```

### 2. Builder Changes (`generator/src/build/builder.rs`)

Add stage after rendering pages (around stage 5-6):

```rust
// Stage X: Generate 404 page
let _404_span = info_span!("generate_404").entered();
info!("[X/11] Generating 404.html...");
if let Some(page_404) = pipeline::generate_404(&templates, &site_context)? {
    pipeline::write_404(&page_404, &output_dir, self.dry_run)?;
}
drop(_404_span);
```

### 3. Scaffolder Changes (`generator/src/init/scaffold.rs`)

Add `create_404_template()` method in `create_templates()`:

```rust
// Create 404.html
let notfound_path = path.join("templates/404.html");
if !notfound_path.exists() {
    let content = r#"{% extends "base.html" %}

{% block title %}404 - Page Not Found{% endblock %}

{% block content %}
<article class="error-page">
    <h1>404</h1>
    <h2>Page Not Found</h2>
    <p>Sorry, the page you're looking for doesn't exist.</p>
    <p><a href="/">Return to the home page</a></p>
</article>
{% endblock %}
"#;
    std::fs::write(&notfound_path, content)?;
    report.files_created += 1;
    report.created_files.push(notfound_path);
}
```

### 4. Styles (Optional)

Add basic 404 styles to `main.scss` in scaffolder:

```scss
.error-page {
    text-align: center;
    padding: 4rem 0;
    
    h1 {
        font-size: 6rem;
        margin: 0;
        color: #ccc;
    }
    
    h2 {
        margin: 0 0 1rem 0;
    }
    
    a {
        color: #0066cc;
    }
}
```

## File Changes Summary

| File | Change |
|------|--------|
| `generator/src/build/pipeline.rs` | Add `Generated404`, `generate_404()`, `write_404()` |
| `generator/src/build/builder.rs` | Add stage to call 404 generation |
| `generator/src/init/scaffold.rs` | Add `create_404_template()` call |

## Notes

- The 404 page is **optional** - if `templates/404.html` doesn't exist, no error is thrown
- Users can customize the 404 template by editing `templates/404.html`
- The page is placed at `dist/404.html` (root level) which is the standard location most hosts expect
- No changes needed to `BuildReport` - 404 generation is just one file

## Testing

1. Build a site with the new template - verify `dist/404.html` is generated
2. Build a site without `404.html` template - verify no error, no 404.html output
3. Run `init` - verify `templates/404.html` is created
