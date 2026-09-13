# Shortcodes

Shortcodes are content-time macros: small invocations in Markdown that
render to HTML during the build, between internal-link resolution and
Markdown rendering. They are how content reaches everything from simple
embeds up to interactive islands — without editing templates.

```markdown
Watch this:

{{ youtube(id="dQw4w9WgXcQ", title="A demonstration") }}
```

## Syntax

Two forms, Hugo-shaped:

**Inline** — `{{ name(arg="value") }}`, replaced by its rendering:

```markdown
{{ image(src="@/blog/photo.jpg", alt="A co-located photo", class="wide") }}
```

**Block** — `{{% name %}}…{{% /name %}}`, with a body:

```markdown
{{% box(class="callout") %}}
**Bold** works here — the body is Markdown.
{{% /box %}}
```

Arguments are named (`k=v`, comma-separated). Values are strings
(`"quoted"` or `'quoted'`), integers, floats, or `true`/`false`.
Shortcode names are letters, digits, `-` and `_`, starting with a
letter.

## Where shortcodes come from

**Built-ins** ship with taxus:

| Name | Kind | Args |
|------|------|------|
| `image` | inline | `src` (required; `@/path` refs resolve to the content-relative URL where co-located assets live), `alt`, `class` |
| `youtube` | inline | `id` (required), `title` |
| `island` | inline | `component` (required), plus the component's props — see below |

**Your own** live in `shortcodes/` at the site root — one `.html`
file per shortcode, named by its file stem. Tera renders them with
the same filters templates use (`term_slug`, `slugify`, `date`, …).
Nothing to register; drop the file in. `taxus init` does not scaffold
the directory — it appears when you need it.

A file whose name collides with a built-in is a build error: built-in
names are load-bearing.

### Template context

Your shortcode templates render with:

| Variable | Meaning |
|----------|---------|
| `args.*` | The invocation's arguments (strings are HTML-escaped on output) |
| `body` | Block form only: the body, **already rendered as Markdown** — emit with `{{ body \| safe }}` |
| `page.title`, `page.description`, `page.draft`, `page.date` | The containing page's frontmatter |
| `site_name`, `base_url` | Site identity |

Arguments are autoescaped; `body` is pre-rendered HTML and needs
`| safe` to pass through. Keep it that way — args come from content,
bodies are your own rendered Markdown.

## The `island` shortcode

The same islands templates place with `{{ island(...) | safe }}` can
be placed **from content** — one system, one shared dispatch:

```markdown
{{ island(component="Counter", initial=5) }}
```

`component` must be in the island registry (`Counter`, `SearchBox`);
the remaining arguments are that component's props (same names and
defaults the template function documents). Prefer the block placement —
an island on its own line renders as its own HTML block; an island
inline in a paragraph nests a `<div>` inside `<p>`, which browsers
tolerate but is best avoided.

An unknown `component` is a hard build error naming the file and the
component.

## Code constructs are immune

Shortcode uses inside fenced code blocks, indented code blocks, or
inline code spans are never expanded — documenting an example is safe:

````markdown
```text
{{ image(src="never-expanded.png") }}
```
````

The immunity is structural (the Markdown parser reports code ranges;
the expander skips them), the same machinery that protects `@/` links.

## Errors

- **Unknown shortcode** — no built-in and no `shortcodes/{name}.html` —
  fails the build naming the content file and the name.
- **Malformed arguments** fail the build with the byte position.
- **Name collision** with a built-in fails the build.

## What shortcodes are not

- **Not nested** (v1): a shortcode inside another shortcode's body is
  passed through as text to the outer template.
- **Not in summaries, word counts, or search** — shortcode spans are
  removed before those derivations, so `{{ image(alt="sunset") }}`
  never leaks into a feed summary or the search index.
- **Not runtime** — they render at build time. Interactivity is the
  island tier, reached through the `island` shortcode.
