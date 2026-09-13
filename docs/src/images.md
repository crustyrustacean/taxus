# Images

Taxus provides built-in responsive image processing for hero images — automatically generating multiple size variants, converting to modern formats, and producing the `<picture>` markup needed for optimal delivery.

## Hero Images

Hero images are large, prominent images displayed at the top of a page. Taxus handles resizing, format conversion, and responsive markup automatically.

### Adding a Hero Image

Place the image file alongside your markdown content (co-located), then reference it in frontmatter:

```markdown
+++
title = "My Post"
hero_image = "sunset.jpg"
hero_alt = "A dramatic mountain sunset"
date = 2024-03-15
+++

# My Post

Content goes here...
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `hero_image` | string | No | `None` | Relative path to a co-located image file |
| `hero_alt` | string | No | `None` | Alt text; falls back to page title |

The `hero_image` path is resolved relative to the content file's directory. If your markdown is at `content/blog/my-post.md`, then `hero_image = "photo.jpg"` looks for `content/blog/photo.jpg`.

### What Taxus Does

When a page has a `hero_image`, the build pipeline:

1. **Reads** the source image and records its dimensions
2. **Generates** responsive variants at each configured width (default: 400, 800, 1200), clamped to the original — a width at or beyond the original ships the original pixels, and clamped widths deduplicate, so a small source yields fewer variants, never duplicates
3. **Converts** to the configured format (default: WebP)
4. **Writes** variant files to the output directory (default: `dist/images/`)
5. **Attaches** image metadata to the page's template context as `page.hero`, and carries it on the rendered page (`RenderedPage::hero_image`) for downstream consumers

`images.widths` must list at least one breakpoint; an empty list is a configuration error.

Variant filenames include a content hash for cache-busting:

```
dist/images/sunset-a3b2c1-400w.webp
dist/images/sunset-a3b2c1-800w.webp
dist/images/sunset-a3b2c1-1200w.webp
```

The hash is derived from the image's bytes (and the encoding quality), so the same image gets the same filenames on every machine. If all variants already exist on disk, processing is skipped — no redundant re-encoding.

### Rendering in Templates

The `page.hero` object is available in page templates when a hero image is set:

```html
{% if page.hero %}
<picture>
  <source srcset="{{ page.hero.srcset | safe }}" type="{{ page.hero.mime_type }}">
  <img src="{{ page.hero.src | safe }}"
       alt="{{ page.hero.alt }}"
       width="{{ page.hero.width }}"
       height="{{ page.hero.height }}"
       loading="eager"
       decoding="async">
</picture>
{% endif %}
```

**Important**: Use `| safe` on `srcset` and `src` to prevent Tera from HTML-escaping the URLs.

### Hero Context Variables

| Variable | Type | Description |
|----------|------|-------------|
| `page.hero.src` | String | Fallback `<img>` src (middle variant) |
| `page.hero.srcset` | String | Full srcset string for `<source>` element |
| `page.hero.width` | Number | Original image width (for layout shift prevention) |
| `page.hero.height` | Number | Original image height (for layout shift prevention) |
| `page.hero.alt` | String | Alt text (from `hero_alt`, or page title as fallback) |
| `page.hero.mime_type` | String | MIME type (e.g., `"image/webp"`) |

### Alt Text Fallback

If `hero_alt` is not set in frontmatter, Taxus falls back to the page `title`:

```markdown
+++
title = "Announcing Taxus 1.0"
hero_image = "banner.jpg"
+++
```

In this case, `page.hero.alt` will be `"Announcing Taxus 1.0"`.

## Image Configuration

Configure image processing in `site.toml` under the `[images]` section:

```toml
[images]
widths = [400, 800, 1200]
quality = 80
format = "webp"
output_dir = "images"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `widths` | array | `[400, 800, 1200]` | Responsive breakpoint widths in pixels |
| `quality` | number | `80` | Output quality (1–100). Applies to `"jpeg"` and `"webp"` only; `"png"` ignores it |
| `format` | string | `"webp"` | Output format: `"webp"`, `"jpeg"` (alias `"jpg"`), or `"png"` |
| `output_dir` | string | `"images"` | Subdirectory within `dist/` for processed images |

`quality` outside 1–100 or an unknown `format` is a configuration error.

### Omitting the Section

If `[images]` is not present in `site.toml`, all defaults are used.

### Lossy WebP and the `webp-lossy` Feature

WebP variants are encoded with libwebp (the `webp` crate) at the configured `quality`. This is behind the `webp-lossy` cargo feature, which is enabled by default. If you build Taxus with `--no-default-features`, the C dependency is dropped and WebP output falls back to the `image` crate's lossless encoder: `quality` is then ignored for WebP and a warning is logged once per build. JPEG always honours `quality`; PNG is always lossless.

## How It Works

### Build Pipeline

Image processing runs as **Stage 4** of the build pipeline, between content processing and co-located asset copying:

1. **Discover routes** → 2. **Load templates** → 3. **Process content** → 4. **Process images** → 5. **Copy co-located assets** → ...

This means hero image variants are generated before assets are copied and pages are rendered, ensuring the image metadata is available in template context.

### Caching

The image processor uses content-hash-based filenames. The hash is a digest of the source file's bytes and (for lossy formats) the `quality` setting — not its path or modification time, so a fresh clone or a `touch` produces the same variant names. If all expected variant files already exist on disk with the correct hash, the processor skips re-encoding and rebuilds the metadata from the cache. This makes subsequent builds fast, editing the image or changing `quality` in `site.toml` re-encodes on the next build, and unchanged images keep stable URLs across deployments.

### Small Source Images

If the source image is smaller than a configured breakpoint width, Taxus does not upscale it. Instead, the original dimensions are used for that variant, preventing quality loss from upscaling.

### Dry Run

When running `taxus build --dry-run`, the image processor calculates metadata and variant paths without reading pixel data or writing files. This allows you to inspect what would be generated without the I/O cost.
