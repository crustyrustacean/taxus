# Worked Example

This page follows one real file through the whole system. The file is the
first blog post of the product site, `get-taxus-org/` in the repository.
It was chosen because the scaffold that `taxus init` creates has no blog
post, and because its date-prefixed name exercises every identity rule.
Every value below was taken from an actual build of that site; nothing is
invented.

The file:

```text
get-taxus-org/content/blog/2026-04-03-project-launch.md
```

The site's config, as far as this page needs it:

```toml
[site]
name = "Taxus"
base_url = "https://get-taxus.org"
```

## 1. Bytes on disk

```markdown
+++
title = "Project Launch"
date = 2026-04-03
description = "The inaugural blog post for the Taxus SSG project."
tags = ["rust", "ssg"]
categories = ["announcements"]
draft = false
+++

### Ready, Set, Go!

Welcome to the official launch of Taxus! I'm excited to introduce this new
static site generator and share the journey that brought us here. ...
```

The file sits next to `blog/_index.md` (title "Blog", `template =
"blog.html"`) and four other posts.

## 2. Frontmatter and body

Stage 1, `[1/15] Discovering routes...`, lists every `.md` file and calls
`Page::from_str(content, "blog/2026-04-03-project-launch.md")`. That
splits the file at the `+++` lines and parses the TOML into a
`Frontmatter`:

```rust
Frontmatter {
    title: "Project Launch",
    date: Some(2026-04-03),          // from frontmatter; the file name agrees
    description: Some("The inaugural blog post for the Taxus SSG project."),
    tags: ["rust", "ssg"],
    categories: ["announcements"],
    draft: false,
    sort_by: Date, paginate_by: 0, weight: 0, // defaults
    ..                                        // every other field None or empty
}
```

The body is everything after the closing `+++`, with leading blank lines
removed, as a `String` starting `### Ready, Set, Go!`.

Had the frontmatter omitted `date`, `split_date_prefix` would have
supplied `2026-04-03` from the file name. Here both agree.

## 3. The node and its path

Still in stage 1, `RouteDiscovery::discover_tree` computes where the file
goes in the tree:

1. Parent directory `blog` becomes the node path `["blog"]` (slugified;
   unchanged here).
2. File stem `2026-04-03-project-launch`. No frontmatter `slug`, so
   `split_date_prefix` removes `2026-04-03-`, and `slugify_segment`
   turns `project-launch` into `project-launch`.
3. The node path is the parent joined to the slug.

`SiteTreeBuilder::add_page` receives the final path and stores it:

```rust
PageNode {
    path:         NodePath(["blog", "project-launch"]),
    content_file: "blog/2026-04-03-project-launch.md",
    meta:         Frontmatter { title: "Project Launch", .. },
    body:         "### Ready, Set, Go!\n\nWelcome to the official launch ...",
}
```

`SiteTreeBuilder::build` places it under the `blog` section, whose
`pages` end up in slug order:

```text
blog/project-launch
blog/taxus-feature-focus-hero-images
blog/taxus-feature-focus-search-island
blog/taxus-feature-focus-syntax-highlighting
blog/understanding-static-site-generators
```

`RouteRegistry::from_tree` then derives the route from the path and
nothing else:

```text
[page   ]  /blog/project-launch/   blog/2026-04-03-project-launch.md   blog/project-launch/index.html
```

That line is what `taxus routes --dir get-taxus-org` prints. The URL
path came from `UrlPath::from_node_path(&["blog", "project-launch"])`.
The date is not in it.

## 4. The processed page

Stage 3, `[3/15] Processing content...`, takes the tree node — the
same parse discovery already made; nothing is read from disk a second
time — resolves `@/` links (there are none), and renders the Markdown.
The result:

```rust
ProcessedPage {
    route:        RouteInfo { path: "/blog/project-launch/", .. },
    page:         Page { frontmatter, raw_content: body },
    html_content: "<h3 id=\"ready-set-go\">Ready, Set, Go!</h3>\n<p>Welcome to the official launch of Taxus! ...",
    toc:          [TocEntry { level: 3, text: "Ready, Set, Go!", id: "ready-set-go", .. }, ..],
    hero_image:   None,
}
```

Stage 4 does nothing for this page; it has no `hero_image`.

## 5. The derivations that include it

Every list that mentions this post is computed from the tree. None of
them is stored on the node.

**Tree order** (`derivation::documents`). The post is the fifth document:
root index, `appearance`, `authoring`, `blog` index, then `blog/project-launch`.
The route above was registered in that position.

**The blog listing** (stage 6, `collect_child_pages`). The `blog` section
has no `pages_from`, so `aggregate` returns its five direct pages. Its
`sort_by` is the default, `Date`, so `sort_pages` orders them newest
first. The post is the oldest and comes last. This is the order
`section.pages` has when `blog.html` renders `/blog/`:

```text
Taxus Feature Focus: Search Island         2026-04-14
Taxus Feature Focus: Hero Images           2026-04-11
Taxus Feature Focus: Syntax Highlighting   2026-04-10
Understanding Static Site Generators       2026-04-05
Project Launch                             2026-04-03
```

The root section lists nothing: its `_index.md` has no `pages_from`, and
the post is not its direct child.

**Taxonomies** (stage 10, `group_by_terms`). The post's frontmatter puts
it under three terms. `TaxonomyMap` records its content file under each:

| Kind | Term | Page appears at |
|------|------|-----------------|
| tags | `rust` | `/tags/rust/` (with all five posts) |
| tags | `ssg` | `/tags/ssg/` |
| categories | `announcements` | `/categories/announcements/` |

On `/tags/rust/` the post is listed first: the term page keeps tree
order, and the post is the first blog page by slug.

**Feed** (stage 11, `feed_pages`). `recent(tree, false)` yields all five
posts newest first; the post has a `date`, so it stays; no `[feed]
sections` is configured, so nothing is scoped out. It is the last of five
items.

**Sitemap** (stage 8). The post is one of eleven documents, none of them
drafts, so it gets an entry.

**Search** (stage 13). Every processed page gets a `SearchDocument`.

## 6. The page context

Stage 6 builds a `PageContext` for the post from the processed page and
the site's base URL (`page_context_from` in `build/pipeline/pages.rs`):

```rust
PageContext {
    title:        "Project Launch",
    description:  Some("The inaugural blog post for the Taxus SSG project."),
    tagline:      None,
    path:         "/blog/project-launch/",
    permalink:    "https://get-taxus.org/blog/project-launch/",
    content:      "<h3 id=\"ready-set-go\">Ready, Set, Go!</h3>\n<p>Welcome ...",
    raw_content:  "### Ready, Set, Go!\n\nWelcome ...",
    date:         Some("2026-04-03"),
    draft:        false,
    summary:      "Ready, Set, Go!",
    word_count:   278,
    reading_time: 2,
    toc:          [ .. ],
    tags:         ["rust", "ssg"],
    categories:   ["announcements"],
    series:       None,
    weight:       0,
    hero:         None,
}
```

Two values deserve a note. `summary` is `"Ready, Set, Go!"` because the
frontmatter sets no `summary` and the body has no `<!-- more -->`, so
`Page::summary` takes the first paragraph, which is the heading, and
strips its `### `. `reading_time` is 278 words at 200 words a minute,
rounded up.

The same `PageContext` is what the `blog` listing, the tag pages and
`get_page(path="blog/project-launch")` hand to templates.

## 7. The template

The post has no `template` field, so it renders with `page.html`. The
context is:

```text
site    = SiteContext { name: "Taxus", base_url: "https://get-taxus.org", .. }
page    = the PageContext above
section = (absent; this is a page, not a section)
now     = NowContext { year: 2026 }
extra   = {}                     // the post has no [extra] table
```

The product site's `page.html` extends `base.html` and, in its content
block, prints the date, the title, the description, the rendered body,
and the tag links:

```html
<time datetime="{{ page.date }}">{{ page.date }}</time>
<h2>{{ page.title }}</h2>
<p class="description">{{ page.description }}</p>
{{ page.content | safe }}
{% for tag in page.tags %}
<a href="/tags/{{ tag | term_slug }}/">{{ tag }}</a>
{% endfor %}
```

The rendered result, `RenderedPage { route, content }`, contains:

```html
<title>Project Launch - Taxus</title>
<meta name="description" content="The inaugural blog post for the Taxus SSG project.">
<link rel="canonical" href="https://get-taxus.org/blog/project-launch/">
...
<time datetime="2026-04-03">2026-04-03</time>
<h2>Project Launch</h2>
<p class="description">The inaugural blog post for the Taxus SSG project.</p>
<h3 id="ready-set-go">Ready, Set, Go!</h3>
<p>Welcome to the official launch of Taxus! ...</p>
...
<a href="/tags/rust/">rust</a>, <a href="/tags/ssg/">ssg</a>
```

## 8. The files

Stage 15 writes the rendered page to `route.output_file` under the
output directory:

```text
dist/blog/project-launch/index.html
```

The other outputs that mention the post, each written by its own stage:

`dist/sitemap.xml` (stage 8):

```xml
<url>
  <loc>https://get-taxus.org/blog/project-launch/</loc>
  <lastmod>2026-04-03</lastmod>
  <changefreq>monthly</changefreq>
  <priority>0.7</priority>
</url>
```

`dist/feed.xml` (stage 11), the last of five items; the description is
the frontmatter `description` because `FeedEntry::from_page` prefers it
over the computed summary:

```xml
<item>
  <title>Project Launch</title>
  <link>https://get-taxus.org/blog/project-launch/</link>
  <description>The inaugural blog post for the Taxus SSG project.</description>
  <pubDate>Fri, 03 Apr 2026 00:00:00 +0000</pubDate>
  <category>rust</category>
  <category>ssg</category>
  <guid isPermaLink="true">https://get-taxus.org/blog/project-launch/</guid>
</item>
```

`dist/search_index.bin` (stage 13), one record:

```rust
SearchDocument {
    id: 4,                          // its position in registry order
    title: "Project Launch",
    path: "/blog/project-launch/",
    summary: "Ready, Set, Go!",
    tags: ["rust", "ssg"],
    categories: ["announcements"],
}
```

And the listing pages that link to it: `dist/blog/index.html`,
`dist/tags/rust/index.html`, `dist/tags/ssg/index.html`,
`dist/categories/announcements/index.html`.

## What to take from this

Six outputs name the post. All six got its address from one node path,
computed once in stage 1. All six got its membership from one tree.
Rename the file to `2026-04-03-launch.md` and every one of them changes
together on the next build, and `aliases = ["/blog/project-launch/"]` in
the frontmatter would keep the old address working.
