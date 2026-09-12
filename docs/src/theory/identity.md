# Identity

A document has four names. This page says what each one is, where it
comes from, which one is the source of truth, and where the others are
derived from it.

| Name | Type | Example | Comes from |
|------|------|---------|------------|
| Content file | `PathBuf` | `blog/2026-04-03-project-launch.md` | the filesystem |
| Slug | `taxus_domain::Slug` | `project-launch` | the file name, or the frontmatter `slug` |
| Node path | `taxus_domain::NodePath` | `blog/project-launch` | the directory segments plus the slug |
| URL path | `taxus_domain::UrlPath` | `/blog/project-launch/` | the node path, in one function |

## Content file

The content file is where a document is stored, as a path relative to the
content directory. It is the storage identity. It appears on the tree
node (`content_file`), on the route (`RouteInfo::content_file`) and on
the parsed page (`Page::source`), always with the same value.

Stages that need to join two views of the same document join on the
content file. The render stage looks up a tree node's `ProcessedPage` by
content file; so do the feed, the sitemap and the taxonomy pages. It is
the one name that survives every transformation unchanged.

The content file is not an address. Its date prefix, its capital letters
and its spaces never reach a URL.

## Slug

A slug is one URL segment. The domain type `Slug` only checks that a
string can stand as a segment: not empty, no `/`, not `.` or `..`, no
control characters. It does not make strings safe; the generator does.

The generator has one slug algorithm, `routes::slugify::slugify_segment`:
lowercase, transliterate to ASCII, collapse runs of whitespace and
punctuation to a single dash, never start or end with a dash. A file
called `My Créative Post.md` gets the slug `my-creative-post`.

A page's slug is decided by two rules, in this order:

1. If the frontmatter sets `slug`, that string is the slug, verbatim.
   `Slug::new` validates it and the build fails if it cannot be a segment.
2. Otherwise, take the file stem, remove a `YYYY-MM-DD-` date prefix if
   there is one (`content::split_date_prefix`), and slugify what remains.

A section's slug is its directory name, slugified. A `slug` field in an
`_index.md` is ignored: a section is named by its directory.

## Node path

The node path is the list of slugs from the root section down to the
node. It is the node's name inside the tree and the key for
`get_section` and `get_page`. The root's node path is empty.

The node path is built in exactly one place, `RouteDiscovery::discover_tree`
in `taxus-generator/src/routes/discovery.rs`: the parent directory's
segments are slugified one by one, and the page's slug is appended.

## URL path

The URL path is the address. It is derived from the node path by one
function and nowhere else:

```rust
impl UrlPath {
    pub fn from_node_path(path: &NodePath) -> Self {
        if path.is_root() { "/" } else { format!("/{path}/") }
    }
}
```

The root is `/`. Every other document is `/` + segments joined by `/` +
`/`. There is no configuration that changes this shape and no frontmatter
field that sets a whole URL.

The output file mirrors the URL path: `blog/project-launch/index.html`,
and `index.html` for the root. `RouteRegistry::from_tree` computes both
from the node path when it builds the routes.

## Which one is the source of truth

The node path. It is stored on the node, it is what the builder checks
for duplicates, and everything downstream is a function of it:

```text
content file ──(slugify, strip date, apply slug override)──► node path
                                                                │
                                          UrlPath::from_node_path
                                                                ▼
                                                            URL path ──► output file
```

The arrow from content file to node path runs once, during parse. The
arrows from node path onward run whenever a URL is needed, and always
give the same answer.

## The boundary rule

Slug overrides and date prefixes are applied **before** a path enters the
tree. `SiteTreeBuilder::add_page` receives the final node path and stores
it as given. The builder does not look at `meta.slug`; it does not look at
the file name. Its rustdoc calls this "paths are final".

This puts all file-name interpretation in one function of the generator,
and lets the domain crate stay free of naming rules. A test can build a
tree with any paths it likes and never touch a file. It also means the
tree can never disagree with itself: there is no second field that a
later stage could reinterpret into a different address.

The old behaviour, where a frontmatter `slug` was reapplied after
discovery and moved the page to the site root, was the bug this rule
fixed ([PR #79](https://github.com/crustyrustacean/taxus/pull/79)).

## Three files, four names

| Content file | Frontmatter | Node path | URL path | Output file |
|--------------|-------------|-----------|----------|-------------|
| `about.md` | (none) | `about` | `/about/` | `about/index.html` |
| `blog/e.md` | `slug = "renamed-entry"` | `blog/renamed-entry` | `/blog/renamed-entry/` | `blog/renamed-entry/index.html` |
| `blog/2026-04-03-project-launch.md` | (none) | `blog/project-launch` | `/blog/project-launch/` | `blog/project-launch/index.html` |

The first row is plain. The second shows that a slug override replaces
the last segment only; the page stays in `blog`. The third shows the date
prefix removed from the path and kept in the file name. In that third
case the frontmatter has no `date` of its own, so the prefix also becomes
the page's `date`: `2026-04-03`.

Two files that reach the same node path are an error, reported as
`Duplicate route: /blog/project-launch/`. So is a page and a section at
the same path.
