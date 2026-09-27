# AGENTS.md

## What this is
A static site built with Taxus. Content is in markdown, templates are Tera, output is plain HTML.

## Layout
- _index.md marks a section; everything else is treated as a page
- content/  Markdown pages and posts
- templates/  Tera templates
- static/  Files copied verbatim to the output
- styles/  SCSS, compiled to CSS
- site.toml  Site configuration (the only config file)

## The loop
1. Edit content or templates
2. `taxus serve` for live preview (it watches files and rebuilds automatically), or `taxus build` to write dist/
3. Check the output, iterate

## Rules
- Shortcodes: `image` and `youtube` render in the content; `island` mounts a Yew component. An unknown name fails the build
- dist/ is generated -- never edit or commit it (the path is set by `output_dir` in site.toml; every build clears it first)
- Posts use TOML frontmatter: title, date, description, tags, categories, series, draft, slug, weight, template

## Gotchas
- `{{ name(args) }}` spans in markdown content are shortcodes. An unknown name *with* parentheses fails the build (`TemplateError ; UnknownShortcode`); braces *without* parentheses stay as literal text
- Template syntax inside code fences is left alone -- put examples of `{{ ... }}` in fenced blocks and they render as written
- Pages with `draft = true` are skipped by `taxus build` unless `--include-drafts` is passed. If a new page "doesn't exist" check the draft flag
