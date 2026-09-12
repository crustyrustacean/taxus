# Content Model

This chapter describes the content directory as a database: what the
directory means, how it becomes an in-memory structure, and how every output
is derived from it. The practical reference for frontmatter fields and file
conventions lives in [Content](./content.md). The [Theory](./theory/overview.md)
chapters cover the same ground from the compiler's point of view and define
the vocabulary in the [Glossary](./theory/glossary.md).

The short version: **the content directory is a database.** Markdown files are
rows, frontmatter is the schema, directories are parent–child relations, and
the build projects that data into every output the site has.

## The Three Layers

A Taxus build has three layers, and each concept in the system belongs to
exactly one of them:

```
┌─────────────────────────────────────────────────────────┐
│ 1. STORAGE          content/  (*.md + frontmatter)      │
│                     Dumb by design. No logic here.      │
├─────────────────────────────────────────────────────────┤
│ 2. MODEL            Sections + Pages (a tree)           │
│                     Built once per build. All meaning.  │
├─────────────────────────────────────────────────────────┤
│ 3. PROJECTIONS      HTML, feeds, sitemap, search index, │
│                     taxonomies, aliases                 │
│                     Queries over the model. No state.   │
└─────────────────────────────────────────────────────────┘
```

The build walks storage once, builds the model, then derives every projection
from the model. Nothing in a projection is ever written back or stored: URLs,
tag indexes, summaries — all of it is computed, discarded, and recomputed on
the next build.

## Pages Are Rows

Every Markdown file is one page — one row. `Page::from_str`
(`taxus-generator/src/content/page.rs`) parses it into:

- **Frontmatter** — the columns. Typed fields: `title`, `date`, `draft`,
  `slug`, `tags`, and so on. All meaning lives here.
- **Body** — the Markdown payload, rendered once and reused by every
  projection that needs it.

Derived values (summary, reading time, word count) are computed from these two
on demand by methods on `Page`. They are never stored.

## Sections Are Parent Pointers

In a database, a row points at its parent with a `parent_id` column. In Taxus,
*the directory is the foreign key*: a file belongs to whatever directory it
sits in.

Every directory becomes a `SectionNode` in the Site Tree
(`taxus-domain/src/tree.rs`):

```rust
pub struct SectionNode {
    pub path: NodePath,                 // membership path, e.g. ["blog"]
    pub content_file: Option<PathBuf>,  // "blog/_index.md", or None
    pub meta: Frontmatter,              // from _index.md (or defaults)
    pub body: Option<String>,
    pub pages: Vec<PageNode>,           // direct children = directory contents
    pub subsections: Vec<SectionNode>,  // direct child directories
}
```

A section with an `_index.md` is itself a document: the index file has
frontmatter and a body, and its frontmatter carries section *behaviour* —
`sort_by`, `paginate_by`, `pages_from`. The
result is a tree: the root section (the content directory itself) contains
pages and subsections, recursively.

This is why content organization requires no configuration. You never declare
"this post belongs to the blog"; the tree shape *is* the declaration, and URL
paths, membership, and section indexes all follow from it.

## Identity: Filename, Slug, Path, and URL Are Different Things

Four distinct concepts are easy to conflate, and the model deliberately keeps
them apart:

| Concept      | Lives in            | Example                              |
|--------------|---------------------|--------------------------------------|
| Storage path | the filesystem      | `content/blog/my-post.md`            |
| Slug         | the tree (`NodePath::last`) | `my-post`                    |
| Section path | the tree            | `/blog/`                             |
| URL          | *derived, never stored* | `/blog/my-post/`                 |

The slug is a computed value: the `slug` frontmatter field if set, otherwise
derived from the filename (`file_stem()`, date prefix stripped, slugified).
It is computed once, in `RouteDiscovery::discover_tree`, before the page
enters the tree; see [Identity](./theory/identity.md).
The URL is then composed as *section path + slug*: the Site Tree records the
membership path when discovery builds it
(`RouteDiscovery::discover_tree`), and `UrlPath::from_node_path` derives the
address from it. `ProcessedPage::effective_url_path()` is that derived
address, and the single accessor every downstream consumer uses. A `slug`
replaces the last segment only: `content/blog/e.md` with
`slug = "renamed-entry"` is `/blog/renamed-entry/`.

The design rule that falls out of this: **metadata belongs in frontmatter, not
in filenames.** A filename is a storage detail; the model should not depend on
encoding data (such as dates) into it. Filenames that carry data are a storage
convention, and the loader — not the model — is the right place to interpret
them.

## Taxonomies Are Indexes

The tree answers *where does this live* — one parent per page, hierarchical.
Taxonomies answer *what is this about* — many memberships per page, flat.

Tags, categories, and series are declared in frontmatter and **derived** into
listing and term pages after the tree walk
(`taxus_domain::derivation::group_by_terms`, rendered by
`taxus-generator/src/build/pipeline/taxonomy.rs`). They are indexes over the model,
never stored: add a tag to five posts and `/tags/your-tag/` exists; remove the
last one and it disappears. No configuration, no manifest.

## URLs Are a Computed Column

No URL is ever written down anywhere in a Taxus site. Every URL — page links,
feed entries, the sitemap, `@/` internal links — is
computed at build time from the same formula:

```
URL = section path + slug
```

This has a practical consequence that is easy to underestimate: because the
formula exists in exactly one place, changing it (or changing a slug, or moving
a file) updates *every* URL on the site consistently on the next build. You
edit the derivation, not fifty outputs.

When URLs must change without breaking the world, the `aliases` frontmatter
field bridges the gap: the build emits redirect pages from old URLs to the new
derived one (`build/pipeline/alias.rs`).

## Projections

Everything the build emits is a query over the model:

| Output            | Derivation                                   | Code                         |
|-------------------|----------------------------------------------|------------------------------|
| HTML pages        | each node rendered through its template      | `build/`, `templates.rs`     |
| Section indexes   | `section.pages`, sorted by `sort_by`         | `build/pipeline/pages.rs` (`derivation::aggregate`) |
| Pagination        | slices of a section's pages                  | `build/pipeline/pages.rs`    |
| Taxonomy pages    | group pages by `tags`/`categories`/`series`  | `build/pipeline/taxonomy.rs` (`derivation::group_by_terms`) |
| RSS/Atom feeds    | `recent`: dated pages, newest first, limited | `build/pipeline/feeds.rs`    |
| Sitemap           | `effective_url_path()` of every node         | `build/pipeline/sitemap.rs`  |
| Search index      | page bodies and titles                       | `build/pipeline/search.rs`   |
| Alias redirects   | `aliases` frontmatter → derived URL          | `build/pipeline/alias.rs`    |
| Internal links    | `@/path.md` resolved against the tree        | `build/pipeline/internal_links.rs` |

## Worked Example

```
content/
├── _index.md                ROOT section        → /
├── about/_index.md          section             → /about/
└── blog/
    ├── _index.md            section             → /blog/
    │                        (sort_by = "date", paginate_by = 10)
    └── my-post.md           page                → /blog/my-post/
```

The model after the walk:

```
root (SectionNode, path [])
├── about (PageNode, path ["about"])
└── blog (SectionNode, path ["blog"], sort_by = date)
    └── my-post (PageNode { title: "My Post", date: 2026-04-06, tags: ["rust"] })
```

Every output follows:

- `/` lists the root's own pages — and, with `pages_from = ["blog"]` in its
  `_index.md`, the blog's pages too (and can paginate)
- `/blog/` lists `blog.pages` — its direct children — sorted by date, sliced
  into pages of 10
- `/blog/my-post/` renders the page
- `/tags/rust/` exists because the taxonomy index says so
- `feed.xml` and `sitemap.xml` read the same tree through `effective_url_path()`
- a template reads the same tree: `section.subsections` for a section's
  children, `get_section(path="blog")` and `get_page(path="about")` for any
  other node

## Design Invariants

Rules the codebase tries to hold onto — useful when evaluating new features:

1. **Model before projections.** Walk storage once; derive everything from the
   tree. Projections never read the filesystem directly.
2. **One derivation point per concept.** Slugs, URLs, summaries, reading time —
   each is computed in one place and shared. Fixing a derivation fixes all
   consumers.
3. **Membership by location, meaning by frontmatter, aboutness by taxonomy.**
   Three orthogonal axes; don't blend them.
4. **URLs are derived, never stored.** Addressability is a projection of the
   model. Use `aliases` when a derived URL must change in the wild.
5. **Storage stays dumb.** Conventions that smuggle data into filenames or
   directory names belong in the loader, interpreted *into* frontmatter — the
   model itself should never depend on them.
