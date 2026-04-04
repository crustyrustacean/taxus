+++
title = "Structure"
+++

## Template Power

### Sensible but Flexible

Taxus sites are described by `tera` templates. Sensible defaults are built for you, including `base.html`, `section.html` and `page.html` templates. Howver you're not limited by those choices. Override them at your leisure with your own. Simply update the frontmatter in your content to describe the template that's needed.

### Variables

Frontmatter content becomes variables that are injectable into the `tera` templates. These allow templates to display different content for each page.

### Partials

Describe partial components for templates as you normally would. Crate a `partials` directory, under `templates` and build all your pieces there. Reference the partials with the standard `tera` extend syntax.

### Tera Syntax

The full `tera` syntax is available, including conditionals and looping, leaving you to build as you see fit.