# Content Model

This chapter describes the *conceptual model* behind Taxus — what the content
directory means, how it becomes an in-memory structure, and how every output is
derived from it. The practical reference for frontmatter fields and file
conventions lives in [Content](./content.md); this page explains the ideas
underneath.

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

Every Markdown file is one page — one row. `Page::from_file`
(`taxus-generator/src/content/page.rs`) parses it into:

- **Frontmatter** — the columns. Typed fields: `title`, `date`, `draft`,
  `slug`, `tags`, and so on. All meaning lives here.
- **Body** — the Markdown payload, rendered once and reused by every
  projection that needs it.

Derived values (summary, reading time, word count) are computed from these two
and cached on the page. They are never stored on disk.

## Sections Are Parent Pointers

In a database, a row points at its parent with a `parent_id` column. In Taxus,
*the directory is the foreign key*: a file belongs to whatever directory it
sits in.

Every directory becomes a `Section` (`taxus-generator/src/content/section.rs`):

```rust
pub struct Section {
    pub frontmatter: Frontmatter, // from _index.md (or defaults)
    pub path: String,             // e.g. "/blog/"
    pub source: PathBuf,          // e.g. "content/blog"
    pub pages: Vec<Page>,         // membership = directory contents
}
```

A section is itself a page: its `_index.md` has frontmatter and a body, and
its frontmatter carries section *behaviour* — `sort_by`, `paginate_by`. The
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
| Slug         | `Page::slug()`      | `my-post`                            |
| Section path | the tree            | `/blog/`                             |
| URL          | *derived, never stored* | `/blog/my-post/`                 |

The slug is a computed value: the `slug` frontmatter field if set, otherwise
derived from the filename (`file_stem()`). The URL is then composed as
*section path + slug* during route discovery
(`taxus-generator/src/build/pipeline.rs`, `discover_routes`), and
`ProcessedPage::effective_url_path()` is the single accessor every downstream
consumer uses.

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
(`taxus-generator/src/content/taxonomy.rs`). They are indexes over the model,
never stored: add a tag to five posts and `/tags/your-tag/` exists; remove the
last one and it disappears. No configuration, no manifest.

## URLs Are a Computed Column

No URL is ever written down anywhere in a Taxus site. Every URL — page links,
feed entries, the sitemap, next/prev navigation, `@/` internal links — is
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
| Section indexes   | `section.pages`, sorted by `sort_by`         | `content/section.rs`         |
| Pagination        | slices of a section's pages                  | `content/pagination.rs`      |
| Taxonomy pages    | group pages by `tags`/`categories`/`series`  | `content/taxonomy.rs`        |
| RSS/Atom feeds    | pages ordered by date, limited               | `build/pipeline/feeds.rs`    |
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
root (Section "/")
├── about (Section "/about/")
└── blog (Section "/blog/", pages sorted by date)
    └── Page { title: "My Post", date: 2026-04-06, tags: ["rust"] }
```

Every output follows:

- `/` lists the root's pages (and can paginate)
- `/blog/` lists `blog.pages`, sorted by date, sliced into pages of 10
- `/blog/my-post/` renders the page
- `/tags/rust/` exists because the taxonomy index says so
- `feed.xml` and `sitemap.xml` read the same tree through `effective_url_path()`

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
