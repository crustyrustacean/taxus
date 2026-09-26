+++
title = "Taxus Feature Focus: Shortcodes"
date = 2026-09-26
description = "Content-time macros in Markdown — {{ image(...) }}, {{ youtube(...) }}, and your own Tera shortcodes in shortcodes/*.html."
tags = ["rust", "ssg", "shortcodes", "content"]
categories = ["features"]
series = "Feature Focus"
+++

Every static site generator eventually grows the same feature: a way to write `{{ something(...) }}` in a Markdown file and have it become HTML at build time. Hugo calls them shortcodes, Jekyll reached for Liquid tags and then settled on a plugin ecosystem, and Zola had them too — until version 0.23 removed them outright.

Taxus now has shortcodes. Here is how they work, what they deliberately don't do, and the one design decision I'd defend hardest.

## The Syntax

Two forms, borrowed deliberately from Hugo because that syntax has earned its ten thousand users:

**Inline** — `{{ name(arg="value") }}`, replaced by whatever it renders to:

```markdown
{{ image(src="@/blog/photo.jpg", alt="A co-located photo", class="wide") }}
```

**Block** — `{{% name %}}…{{% /name %}}`, with a body:

```markdown
{{% box(class="callout") %}}
**Bold** works here — the body is Markdown.
{{% /box %}}
```

Arguments are named and comma-separated. Values are strings (quoted), integers, floats, or booleans. We do not support positional arguments in v1 — `name(arg="v")` is explicit about what every value is, and a typo'd positional argument should fail loudly rather than shuffle into the wrong slot.

## What You Get Out of the Box

Three built-ins:

| Shortcode | What it does |
|-----------|--------------|
| `{{ image(src=…, alt=…, class=…) }}` | A lazy-loaded `<figure>`/`<img>`; `@/` refs resolve to the co-located asset's URL |
| `{{ youtube(id=…, title=…) }}` | A lazy, `youtube-nocookie.com` iframe |
| `{{ island(component=…, …) }}` | Any Yew island, placed from content |

That last one deserves a moment. Islands used to be a template-only feature: to get a `Counter` into a post, you had to write a template. Now `{{ island(component="Counter", initial=5) }}` in Markdown SSR's the same component and emits the same hydration mount point — both paths funnel through one shared dispatch, and a test asserts the two produce byte-identical output. The line between "template feature" and "content feature" is, for islands, gone.

## Writing Your Own

Drop a `.html` file into `shortcodes/` at the site root. The filename is the name:

```
shortcodes/
└── box.html
```

```html
<div class="box {{ args.class }}">{{ body | safe }}</div>
```

That's the whole registration process. There is no config section, no macro, no manifest — the filesystem is the registry. Your shortcode renders with Tera and the same filters your templates use (`term_slug`, `slugify`, `date`, …).

The context your template gets is deliberately small:

- `args.*` — the invocation's arguments
- `body` — block form only, **already rendered as Markdown** (hence `| safe`)
- `page.title`, `page.description`, `page.draft`, `page.date` — the containing page
- `site_name`, `base_url`

That is the whole surface. No tree access, no `get_section`, no reaching into other pages — and that restraint is on purpose (more on it below).

## Where Shortcodes Run, and Why It Matters

Shortcodes expand during the content stage, *after* internal-link resolution and *before* Markdown rendering. That ordering has three consequences worth knowing:

1. **Your shortcode's HTML output is part of the Markdown stream.** A block shortcode on its own line becomes its own block; inline in a paragraph it stays inline. Markdown does the right thing around your markup without knowing it exists.
2. **Block bodies are Markdown, not text.** `{{% foo %}}**bold**{{% /foo %}}` gives your template a `<p><strong>bold</strong></p>`, not asterisks. (Hugo makes you choose between two notations to get this; we have one, and it behaves the way you'd guess.)
3. **A bad shortcode fails the build, with the file and the name:**

   ```
   Unknown shortcode '{{nope}}' in 'blog/post.md': no built-in or shortcodes/nope.html
   ```

## Code Is Immune

Documenting an example is a normal thing to want to do. Shortcode uses inside fenced blocks, indented blocks, or inline code spans are never expanded:

````markdown
```text
{{ image(src="never-expanded.png") }}
```
````

This isn't a regex guard. Taxus asks the Markdown parser for the byte ranges of every code construct and simply skips those ranges — the same machinery that keeps `@/` internal links from being rewritten inside code samples. Any shortcode that tried to expand inside a fence would be a real bug, so the code makes it structurally impossible instead of heuristically unlikely.

## The Text Paths Never See Them

Summaries, word counts, reading times, and the search index all consume your Markdown — and none of them should see shortcode markup. `{{ image(alt="a mountain sunset") }}` is not prose; a visitor should not find your page by searching for the words inside a YouTube ID, and your feed summary should not open with a wall of `{{`.

So those derivations strip shortcode spans before they extract anything. This has a consequence you should know about: **shortcode arguments don't count toward word counts or reading times.** We treat them as embed attributes, not content. If you write a post that is 90% shortcode output, the reading time will be short — which is arguably correct, since the visitor's reading time is spent looking at a video.

## The Deliberate Limits

Two things v1 doesn't do, stated plainly rather than discovered later:

- **No nesting.** A shortcode inside another shortcode's body is passed through as text. Hugo supports nesting; the escaping rules are subtle, and until there's a real use case pulling against it, the simpler rule wins. It is a small, well-contained feature to add later.
- **A minimal context.** Your shortcode cannot call `get_section` or read other pages. This keeps shortcodes free of ordering dependencies (the tree isn't fully resolved when content renders) and makes them trivially testable. If a shortcode genuinely needs site-wide data, a template is the right place for it.

## One Last Thing

Shortcodes work even inside code — and the documentation site you're reading is built with them. The callout box on this very site, explaining where the installer puts your binary, is a custom `{{% box %}}` shortcode:

```html
<div class="install-hint">{{ body | safe }}</div>
```

Dogfooding the feature in the site that documents it is the only honest test I can think of.

Try it in a [taxus](https://github.com/crustyrustacean/taxus) site of your own — `taxus init my-site`, drop a file in `shortcodes/`, and start writing.
