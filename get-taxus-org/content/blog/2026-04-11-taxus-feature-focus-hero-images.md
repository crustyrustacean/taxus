+++
title = "Taxus Feature Focus: Hero Images"
date = 2026-04-11
description = "Responsive hero images with automatic WebP conversion and srcset generation."
hero_image = "mountain_sunset.jpg"
hero_alt = "A dramatic mountain landscape showcasing hero image capabilities"
+++

Static sites need great visuals. Today we're introducing hero image support in Taxus: drop an image next to your markdown, add one line of frontmatter, and get a fully responsive `<picture>` element with WebP variants.

## The Problem

Every blog wants hero images. The implementation is usually tedious:

1. Resize the image to 3-4 breakpoints manually
2. Convert to WebP for modern browsers
3. Write the `<picture>` element with all the srcset attributes
4. Remember to update everything when the image changes

This is exactly the kind of repetitive work a static site generator should handle.

## The Solution

Taxus now supports hero images through simple frontmatter:

```toml
+++
title = "My Post"
hero_image = "hero.jpg"
hero_alt = "A mountain sunset"
+++
```

That's it. Taxus handles the rest:

- **Automatic resizing**: Generates three variants (small, medium, large)
- **WebP conversion**: Modern format with fallbacks
- **Responsive markup**: Full `<picture>` element with srcset
- **Layout stability**: Width and height attributes prevent layout shift

## How It Works

### 1. Co-located Images

Place your hero image next to your markdown file:

```
content/
  blog/
    my-post.md
    my-post/
      hero.jpg
```

Or simply:

```
content/
  blog/
    my-post.md
    hero.jpg  # shared across posts
```

### 2. Build-Time Processing

During the build pipeline, Taxus:

1. Detects the `hero_image` frontmatter field
2. Resolves the image path relative to the content file
3. Generates responsive variants using the image configuration
4. Stores metadata in the image registry
5. Attaches `HeroContext` to the page template

### 3. Template Rendering

Your templates receive a `hero` object:

```html
{% if page.hero %}
<picture>
  <source srcset="{{ page.hero.srcset | safe }}" type="{{ page.hero.mime_type }}">
  <img src="{{ page.hero.src }}"
       alt="{{ page.hero.alt }}"
       width="{{ page.hero.width }}"
       height="{{ page.hero.height }}">
</picture>
{% endif %}
```

## Configuration

Image processing is configurable in `site.toml`:

```toml
[images]
default_width = 800
default_quality = 85

[[images.variants]]
width = 400
suffix = "small"

[[images.variants]]
width = 800
suffix = "medium"

[[images.variants]]
width = 1200
suffix = "large"
```

## The Technical Details

The hero image system integrates with Taxus's existing image pipeline:

- **ImageProcessor**: Handles resizing and WebP encoding
- **ImageRegistry**: Tracks all processed images for deduplication
- **ProcessedImage**: Carries metadata through the build pipeline
- **HeroContext**: Template-friendly structure with all needed attributes

Alt text falls back to the page title if not specified, ensuring accessibility even when you forget.

## Performance Impact

Hero images are processed at build time, not request time. The impact on your site's runtime performance is zero—only the optimized variants ship to users. The build-time cost is minimal thanks to efficient image processing.

## What's Next

This foundation enables future enhancements:

- **Automatic placeholder generation**: Low-quality image placeholders (LQIP) for perceived performance
- **Blur-up effects**: CSS transitions from placeholder to full image
- **Art direction**: Different crops for different breakpoints
- **Lazy loading integration**: Native `loading="lazy"` support

Hero images are available now in Taxus. Update your templates, drop in an image, and make your content shine.
