# Decisions

Short answers to "why is it like this?", one paragraph each, with a link
to the pull request or issue where the choice was made when there is one.

## Why a tree and not a flat route list

Taxus 0.x ran on a flat list of routes: one record per file, keyed by
URL. That list could not answer "which pages belong to this section?"
without guessing from URL prefixes, and the guess was wrong: the root's
listing matched every URL that started with `/`, so the home page listed
the whole site ([#70](https://github.com/crustyrustacean/taxus/issues/70)).
A tree answers membership by construction: a section's children are the
nodes under it, nothing more. The route list still exists, but it is now
a projection of the tree (`RouteRegistry::from_tree`), so the two cannot
disagree. The tree was introduced in
[PR #71](https://github.com/crustyrustacean/taxus/pull/71) and wired
through the build in [PR #72](https://github.com/crustyrustacean/taxus/pull/72).

## Why derivations are free functions rather than methods

A method on `SiteTree` suggests the answer is a property of the tree. A
listing is not: it depends on a section's `sort_by`, on whether drafts
count, on which donors `pages_from` names. Free functions make those
inputs explicit parameters, so a reader sees at the call site what the
answer depends on. It also keeps the tree type small enough to read in
one sitting, and lets the generator add its own derivations (feed pages,
sitemap entries) in the same shape without touching the domain crate.
The rule for choosing is in [Derivations](./derivations.md#tree-method-or-derivation).

## Why the domain crate has no I/O

`taxus-domain` never reads a file, renders Markdown or runs a template.
Everything it does can be tested with a `SiteTreeBuilder` and a few
string literals, and its tests run in milliseconds with no fixtures. It
also fixes a boundary: file-name conventions such as the date prefix are
interpreted by the generator before a path enters the tree, so the model
does not depend on how files happen to be named. The decision is recorded
in [PR #71](https://github.com/crustyrustacean/taxus/pull/71) and the
boundary rules were settled in review
([commit feca3c7](https://github.com/crustyrustacean/taxus/commit/feca3c7)).

## Why there are no themes

A theme is a second source of templates, styles and static files that
the generator has to merge with the site's own, with rules for which one
wins. Taxus has one `templates/` directory, one `styles/` directory and
one `static/` directory, and `taxus init` copies a complete starting
set into them. Everything a site renders is in the site. There is no
lookup order to learn and no theme update that can change a page under
you. A site that wants to share a look with another copies the files.
There is no issue for this; it is the absence of a feature.

## Why islands instead of a JS framework

Most pages need no script. A framework that renders the page in the
browser makes every page wait for JavaScript. Islands keep the page as
HTML and add interactivity only where a template asks for it: the
`island()` function renders a Yew component to HTML at build time, and
the WASM client hydrates that one element in the browser. The HTML is
readable before the WASM loads, and a page without islands ships no
component code at all. Yew was chosen because the same component
compiles to both the build-time renderer and the browser, so there is
one source for each island. See [Islands](../islands.md). The runtime
plumbing that lets `build()` render islands from any calling context is
[PR #63](https://github.com/crustyrustacean/taxus/pull/63)
([#37](https://github.com/crustyrustacean/taxus/issues/37)).

## Why the image quality is part of the cache key

Hero image variants are named by a hash and skipped when the files
already exist. If the hash covered only the image bytes, changing
`images.quality` in `site.toml` would do nothing until someone deleted
`dist/images/`. Folding the effective quality into the hash means a
quality change re-encodes on the next build, and an unchanged image keeps
its file names, so cached URLs stay stable across deployments. The hash
is of the file's contents rather than its path and modification time so
that a fresh checkout produces the same names, which is what lets the
golden output test pin them.
[PR #65](https://github.com/crustyrustacean/taxus/pull/65) made quality
take effect ([#34](https://github.com/crustyrustacean/taxus/issues/34));
[PR #81](https://github.com/crustyrustacean/taxus/pull/81) moved the key
to content.

## Why section listings are direct children only

Before the tree, a section listed every page whose URL started with the
section's URL. That made the home page a copy of the whole site and made
nested sections list each other's pages. Listing direct children only
matches what the directory shows, and `pages_from` lets an author opt in
to more. The alternative, Zola's `transparent`, pushes pages upward from
the child; `pages_from` pulls from the parent, so the section that shows
the pages is the one that declares it.
[PR #76](https://github.com/crustyrustacean/taxus/pull/76)
([#70](https://github.com/crustyrustacean/taxus/issues/70)).

## Why feeds carry dated pages only

A feed entry must have a publication date. The old feed put every
document in, including the home page and section indexes, and stamped
undated ones with the build time, which re-announced them to every
subscriber on every build.
[PR #77](https://github.com/crustyrustacean/taxus/pull/77)
([#44](https://github.com/crustyrustacean/taxus/issues/44)).

## Why dates come out of file names

`2026-04-03-project-launch.md` sorts by date in a file listing, which is
useful on disk. The date is not part of the page's name, so it is removed
from the slug and, when the frontmatter sets no `date`, used as the
default. Metadata belongs in frontmatter; the file name is a storage
convention the parser interprets.
[PR #68](https://github.com/crustyrustacean/taxus/pull/68)
([#67](https://github.com/crustyrustacean/taxus/issues/67)).

## Why the slug override stays inside its section

`slug = "renamed-entry"` on `content/blog/e.md` used to move the page to
`/renamed-entry/`. A slug is one segment, so it replaces the last
segment of the node path and nothing else: `/blog/renamed-entry/`. A site
that relied on the old address keeps it with `aliases`.
[PR #79](https://github.com/crustyrustacean/taxus/pull/79).
