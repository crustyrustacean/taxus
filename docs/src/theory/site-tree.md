# The Site Tree

A site is a tree. This page says what the tree is made of, what each node
carries, what it deliberately leaves out, and why it never changes once
built. The types live in `taxus-domain/src/tree.rs`.

## What a site is

The content directory is a folder of folders. Each folder is a
[section](./glossary.md#the-site-tree). Each Markdown file that is not
`_index.md` is a [page](./glossary.md#the-site-tree). A file named
`_index.md` gives its folder a title, a body and settings.

The Site Tree is that folder structure in memory, with every file already
parsed. There is one tree per build. The root of the tree is the content
directory itself.

```rust
pub struct SiteTree {
    pub root: SectionNode,
}
```

## The two node types

**A section node** is a directory.

```rust
pub struct SectionNode {
    pub path: NodePath,                 // where it is: ["blog"]; the root is []
    pub content_file: Option<PathBuf>,  // "blog/_index.md", or None
    pub meta: Frontmatter,              // from _index.md, or defaults
    pub body: Option<String>,           // the Markdown of _index.md
    pub pages: Vec<PageNode>,           // direct child pages, by slug
    pub subsections: Vec<SectionNode>,  // direct child sections, by slug
}
```

A directory without an `_index.md` is still a section. Its `content_file`
is `None`, its `meta` is the default frontmatter, and it has nothing to
render, so no HTML file is written for it. Its pages are still rendered,
and templates can still fetch it with `get_section`.

**A page node** is a document.

```rust
pub struct PageNode {
    pub path: NodePath,        // ["blog", "project-launch"]
    pub content_file: PathBuf, // "blog/2026-04-03-project-launch.md"
    pub meta: Frontmatter,
    pub body: String,
}
```

Notice that `content_file` and `path` differ. The file name carries a date
prefix; the node path does not. The file is storage. The path is
identity. [Identity](./identity.md) explains the rule.

## What a node does not carry

A node holds what was read from disk and nothing that was computed from
it. In particular a node never holds:

- **Rendered HTML.** Markdown is rendered in the emit phase and kept on a
  `ProcessedPage`, outside the tree.
- **Its URL.** The address is derived from `path` by `UrlPath::from_node_path`
  whenever it is needed.
- **Computed lists.** A section has no "all posts sorted by date" field.
  A listing is a [derivation](./derivations.md), computed on request.
- **Taxonomy indexes.** Which pages carry the tag `rust` is computed by
  `derivation::group_by_terms`, not stored on the tag or the page.
- **Its summary, word count or reading time.** These are methods on the
  parsed page, computed when asked.

The reason is the one-source rule. If a section stored a sorted list and
a page was later found to be a draft, the stored list would be wrong. If
nothing derived is stored, nothing derived can go stale.

## Containment versus reachability

`pages` and `subsections` hold **direct children only**. This is
containment: `blog` contains `blog/project-launch`; the root does not.

Reachability is everything below a node at any depth. It is not a field.
It is a question you ask: `derivation::descendant_pages(section)` walks
the subtree and returns every page in it.

Keeping the two apart is what makes `section.pages` mean one thing. A
section lists what it owns. When a section should list pages it does not
own, the author says so with `pages_from`, and
[aggregation](./derivations.md#aggregation-derivationaggregate-pages_from) merges the named
sections' direct pages in. Nothing is listed by accident of depth.

## Why the tree is immutable

`SiteTreeBuilder::build` returns the tree, and from then on every stage
only reads it. The generator holds it as `&SiteTree`. There is no method
that adds, removes or edits a node after construction.

Three things depend on this:

1. **Every derivation agrees.** The feed, the sitemap, the section
   listings and the taxonomy pages are all computed from the same tree.
   If one stage could edit the tree, a later stage would see a different
   site than an earlier one.
2. **Order is deterministic.** Children are sorted by slug when the tree
   is built. Every list starts from that order, so the same content
   produces the same output on every machine. The golden output test
   relies on this.
3. **Errors are found once.** Two files that resolve to the same path, or
   a page and a section at the same path, are rejected by the builder
   before any output is written.

If the tree could change during a build, a template that calls
`get_section` early and a sitemap generated late could disagree about
which pages exist. The golden test would flap. Aliases could point at
URLs that no longer exist by the time they are written.

## The scaffolded site

`taxus init my-site --name "My Site"` creates one content file. The tree
is small enough to show whole. File names on the right are the `content_file` values,
relative to `content/`.

```text
SiteTree
└── root: SectionNode                         content/_index.md
        path:         []            (address: /)
        content_file: Some("_index.md")
        meta.title:   "Home"
        meta.description: "Welcome to My Site"
        body:         Some("# Welcome to My Site\n\n...")
        pages:        []
        subsections:  []
```

One node, one document, one output file: `dist/index.html`.

## A site with a blog

The product site in the repository, `get-taxus-org/`, is a real site with
a blog. Its tree, with file names next to the nodes:

```text
SiteTree
└── root: SectionNode  path []                      _index.md
    ├── subsections (by slug)
    │   ├── SectionNode  path ["appearance"]          appearance/_index.md
    │   ├── SectionNode  path ["authoring"]           authoring/_index.md
    │   ├── SectionNode  path ["blog"]                blog/_index.md
    │   │   └── pages (by slug)
    │   │       ├── PageNode  ["blog","project-launch"]
    │   │       │        blog/2026-04-03-project-launch.md
    │   │       ├── PageNode  ["blog","taxus-feature-focus-hero-images"]
    │   │       │        blog/2026-04-11-taxus-feature-focus-hero-images.md
    │   │       ├── PageNode  ["blog","taxus-feature-focus-search-island"]
    │   │       │        blog/2026-04-14-taxus-feature-focus-search-island.md
    │   │       ├── PageNode  ["blog","taxus-feature-focus-syntax-highlighting"]
    │   │       │        blog/2026-04-10-taxus-feature-focus-syntax-highlighting.md
    │   │       └── PageNode  ["blog","understanding-static-site-generators"]
    │   │                blog/2026-04-05-understanding-static-site-generators.md
    │   ├── SectionNode  path ["interactivity"]       interactivity/_index.md
    │   └── SectionNode  path ["structure"]           structure/_index.md
    └── pages: []
```

Two things to notice. The blog's pages are in slug order, not date order;
date order is a derivation the blog's `sort_by` asks for at render time.
And the two `.jpg` files in `content/blog/` are not in the tree. They are
co-located assets, copied by the emit phase; the tree holds documents
only.

The [Worked Example](./worked-example.md) follows the first post through
the whole build.
