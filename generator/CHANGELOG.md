# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.34] - 2026-03-29

### Miscellaneous

- Re-factoring to address clippy lints, dependencies update
- Refinements to template variable rendering
- Fix clippy lints

### Bug

- Fix rendering of content into templates
- Directories for robots,txt, sitemap.xml not created
- Static files not generated properly
- Fixed error in year field of `base.html` template
- Template rendering error

### Bugs

- Fix errors in doc tests, fix template precedence and loading, fix static asset serving

### Improvement

- 404.html template is created on init, dev server uses it
- Page permalinks
- Add internal linking capability
- Add sitemap.xml and robots.txt generation
- Handle co-located assets
- Pagination and RSS feeds
- Add foundation for blog posts,including slugs and taxonomies
- Development server with hot reloading
- Tracing implementation
- Init command can scaffold without islands architecture
- Proof of concept of Yew SSG with WASM Hydration
- Embelish CLI module to produce richer messages and commands
- Change project name to yew-ssg
- Ssg function complete end to end
- Generator re-factor, phase 6, build pipeline and `main.rs`
- Generator re-factor, phase 3 routes
- Generator re-factor, phase 5, asset serving
- Generator re-factor, phase 4, template system
- Generator refactor phase 2, complete with docs update

### Improvment

- Implement generator re-factor, phase 1

