+++
title = "Where Taxus Stands: A Feature Comparison"
date = 2026-10-05
description = "An honest look at how Taxus compares to Hugo, Zola, and Jekyll — what we have, what we don't, and what we think is the right trade."
tags = ["rust", "ssg", "hugo", "zola", "jekyll"]
categories = ["announcements"]
draft = true
+++

<!--
DRAFT until 2026-10-05. A future-dated post sorts FIRST in the blog
listing (newest-first) and is announced in the RSS feed and sitemap
immediately — so without this flag the October post would top the blog
and ship to subscribers two weeks early. On 2026-10-05: delete the
`draft = true` line above and this comment, commit, and push. The
golden re-baseline in the same PR picks up the rendered page.
-->

Taxus is young — the first commit landed in March 2026, and we're now at version 1.2.2. That context matters for any comparison: Hugo has been shipping for over a decade and has nearly 90,000 GitHub stars; Zola is a mature Rust project with a large theme ecosystem; Jekyll is the old guard of GitHub Pages. None of them are going anywhere, and I have no intention of "beating" any of them. Hugo in particular will outlive us all.

So this post is not a scoreboard. It's an honest map: where Taxus genuinely differs, where it's behind, and where I've made a choice that a reasonable person could argue with.

## The Short Version

If you want the table, here's the table. Everything below it is explanation, and every claim is one you can check.

| | **Taxus** | **Zola** | **Hugo** | **Jekyll** |
|---|---|---|---|---|
| Language | Rust | Rust | Go | Ruby |
| Template engine | [Tera](https://keats.github.io/tera/) | Tera | Go templates | Liquid |
| Config format | TOML | TOML | YAML/TOML/JSON | YAML |
| SASS/SCSS | ✅ (grass) | ✅ | ✅ | ✅ (sass-converter) |
| Syntax highlighting | ✅ (tree-sitter) | ✅ | ✅ (Chroma) | ✅ (Rouge) |
| Built-in full-text search | ✅ (TF-IDF, in-browser) | ❌ (search index is opt-in via plugins) | ❌ (FlexSearch integration, community) | ❌ (plugins) |
| Interactive islands (WASM) | ✅ (Yew, first-class) | ❌ | ❌ | ❌ |
| Shortcodes | ✅ | ❌ (removed in 0.23) | ✅ (three types) | Via plugins / includes |
| Multilingual | ❌ | ✅ (built-in) | ✅ (mature) | ❌ (plugins) |
| Image processing pipelines | ✅ (hero variants, WebP) | ✅ (`resize_image`) | ✅ (mature) | ❌ (plugins) |
| Multilingual taxonomies | ✅ (tags/categories/series) | ✅ (any taxonomy) | ✅ (any taxonomy) | ❌ (plugins) |
| Dependencies to install | **One binary** | One binary | One binary | Ruby + Bundler + gems |
| Typing | ✅ (Rust) | ✅ (Rust) | ❌ | ❌ |
| Plugin ecosystem | Small and young | Large (themes) | Massive | Massive |

Read that table with care: the ✅s are things Taxus does *today*; the ❌s marked "plugins" in Hugo/Jekyll columns are not missing — they're third-party, and in Hugo's ecosystem many are maintained by people whose day job is not Hugo.

## Where Taxus Is Genuinely Different

### Islands, properly

This is the axis Hugo and Zola have both declined to move on, and it's the reason Taxus exists at all. A Yew component is server-rendered to HTML at build time, then hydrated in the browser — so the page is complete and readable with JavaScript disabled, and interactive when JavaScript runs. The client bundle only ships if you use an island; a content site pays nothing.

The [SearchBox island](/interactivity/) is the clearest example: Taxus ships a TF-IDF index of your content, the browser searches it, and the whole thing works on static hosting with no API key, no Algolia account, and no service to pay for. Jekyll's answer is a plugin; Hugo's is a FlexSearch integration or Lunr.js wired up by hand; Zola's is opt-in. Not because those approaches are wrong — but because in each case the search feature is a *choice you assemble*, and assembly is exactly what static site generators have historically been bad at.

### One binary, no toolchain

`curl … | sh` and you're building sites. No Ruby, no `npm install`, no Go toolchain, no `bundle exec`. The WASM client is compiled and embedded during `cargo build --release`; there is no second build step. Taxus is also the only one of the four where a contributor's *entire* test suite runs on a bare stable Rust toolchain.

### The model is documented, not folklore

Taxus's book leads with a [Theory section](/theory/overview/) that explains the model before the commands: what a Site Tree is, why listings only ever show direct children, how addresses are derived, and a worked example tracing one real post through every build stage. Hugo has the best documentation of the four by a wide margin — but it's documentation *about* the tool. Taxus tries to document *why it works the way it does*, because a generated site you can reason about is the entire point.

## Where Taxus Is Behind — Honestly

- **Multilingual.** Zola and Hugo have first-class i18n with per-language content trees, fallback chains, and translation-aware URLs. Taxus has none of it. This is the largest gap and the most-requested.
- **Taxonomy flexibility.** Taxus ships tags, categories, and series. Hugo and Zola let you define *arbitrary* taxonomies. Taxus's model is a deliberate simplification — three kinds, well-understood semantics — but a real site with "authors" and "series-of-books" taxonomies will feel it.
- **Ecosystem.** This is the one that doesn't improve by writing code. Hugo's theme library and Zola's theme collection are years of other people's work. A new generator starts at zero, and that gap is measured in years, not sprints.
- **The long tail.** Redirect maps, output formats beyond HTML+feed, image processing across multiple sizes with cache invalidation, paginated taxonomy pages — these exist in Hugo. Taxus has *some* of these (aliases, paginated sections, hero image variants) but not all, and I won't pretend the gap isn't real.
- **Shortcodes are simpler.** Hugo's shortcodes come in three flavors (embedded, custom, inline), support nesting, and have a large ecosystem of reusable ones. Taxus has three built-ins and yours; no nesting; no positional arguments. Deliberate, and a fair thing to argue with.

## Two Places I Think We Chose Differently — And I'd Argue It

**Zola removed shortcodes in 0.23** and made all content Tera-templatable, telling users to reach for Tera components instead. That's a defensible call — one language instead of two — but it means a Zola user writing `{{< youtube id=… >}}` must now learn template compilation to get the same result, and a typo in content can become a template *compilation* error. Taxus keeps content and templates separate: your Markdown is Markdown, and the only place template syntax can appear is inside a shortcode span, where the engine knows what it's doing. A `{{ typo }}` in a page is just text. I think that's the right call for a generator whose selling point is that content authors aren't template authors.

**Hugo's shortcode notation split** (`{{% %}}` renders Markdown, `{{< >}}` doesn't) is a genuine sharp edge, documented carefully, but a sharp edge nonetheless. Taxus has one block form and it always renders Markdown.

## What I'd Actually Tell You

If you have an established Hugo or Zola site with a theme you like, there's no reason to move. Those projects are excellent and the switching cost is real.

If you write content more than you write code, and you want: search that works without a service, islands that are readable without JavaScript, a tool that installs with one command, and a data model you can actually read — taxus might be worth a try. Install it, scaffold a site, and see whether the model clicks. It might not. That's fine too; there are excellent alternatives.

Taxus is a young project with a real point of view and a small but real community of people who care about static site generation being understandable. More traction is always welcome. And more contributors, obviously.

[Try taxus](https://github.com/crustyrustacean/taxus) — or tell me [where I'm wrong](https://github.com/crustyrustacean/taxus/issues). I would genuinely rather have the argument than the silence.
