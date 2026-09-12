# Derivations

Anything you can compute, you don't store.

A **derivation** is a pure function of the Site Tree and the config that
returns a view: a list of pages, a grouping of documents, an order. It
reads the tree. It reads nothing else. It writes nothing. Call it twice
and you get the same answer. That is the whole definition, and it is what
lets every output of a build agree with every other output.

The pure derivations live in `taxus-domain/src/derivation.rs` and
`taxus-domain/src/tree.rs`. The generator has a few functions that fit the
definition but need the config or the generator's own types; this page
lists those too and says where they live.

## Tree order: `derivation::documents`

**Question.** In what order should the site be walked?

**Inputs.** The tree.

**Where.** `taxus_domain::derivation::documents(tree) -> Vec<Node>`.

**Answer.** Every document in tree order: a section's own index file if
it has one, then its pages by slug, then each subsection by slug,
recursively. Drafts are included; callers filter. This is the order
routes are registered in (`RouteRegistry::from_tree`), the order content
is processed in, and the order every later list starts from. It is what
makes builds deterministic.

**Consumed by.** `RouteRegistry::from_tree` (stage 1), the sitemap
(stage 8), taxonomy grouping (stage 10), and `iter_pages` on the tree.

## Reachability: `derivation::descendant_pages`

**Question.** Which pages are anywhere under this section?

**Inputs.** A section node.

**Where.** `taxus_domain::derivation::descendant_pages(section) -> Vec<&PageNode>`.

**Answer.** The section's own pages, then each subsection's, depth-first.
This is the query that replaces a stored "all pages below me" field. The
tree's `SiteTree::iter_pages` is this function applied to the root.

**Consumed by.** `recent` (below). No template field exposes it directly.

## Sorting: `tree::sort_pages`

**Question.** In what order should a listing show its pages?

**Inputs.** A list of pages and a `SortBy`.

**Where.** `taxus_domain::tree::sort_pages(&mut [&PageNode], SortBy)`.

**Answer.** `date`: newest first, undated last. `title`:
case-insensitive ascending. `weight`: lowest first. `none`: leave the
input order. The sort is stable, so ties keep tree order. `SortBy` comes
from the listing section's `sort_by` frontmatter.

**Consumed by.** Section listings (stage 6) and `recent`.

## Recent pages: `derivation::recent`

**Question.** What are the newest pages on the whole site?

**Inputs.** The tree and whether drafts count.

**Where.** `taxus_domain::derivation::recent(tree, include_drafts) -> Vec<&PageNode>`.

**Answer.** Every page in the site, filtered by draft status, sorted by
date newest first. Section index files are not pages and are not
included.

**Consumed by.** Feeds (stage 11), through `feed_pages` below.

## Aggregation: `derivation::aggregate` (`pages_from`)

**Question.** Which pages does this section list, including ones it does
not own?

**Inputs.** The receiving section, the tree, and the node paths named in
the receiver's `pages_from`.

**Where.** `taxus_domain::derivation::aggregate(section, tree, from) -> Vec<&PageNode>`.

**Answer.** The receiver's direct pages, then each donor's direct pages in
the order the donors are named, with duplicates removed by node path.
Donors that do not exist are skipped. The result is not sorted; the
caller sorts with the receiver's `sort_by`.

**Consumed by.** `build::pipeline::pages::collect_child_pages`, which
resolves the `pages_from` strings to node paths (warning on bad ones),
calls `aggregate`, sorts, and drops pages the build skipped. The result
is the template field `section.pages` (stage 6) and what `get_section`
returns.

## Taxonomies: `derivation::group_by_terms`

**Question.** Which documents carry each tag, category or series?

**Inputs.** The tree, whether drafts count, and a function that reads one
taxonomy's terms off a frontmatter.

**Where.** `taxus_domain::derivation::group_by_terms(tree, include_drafts, terms_of)
-> BTreeMap<String, Vec<Node>>`.

**Answer.** A map from term name to the documents that declare it, in
tree order, with term names sorted. Sections with an index file
participate like pages. A document that repeats a term appears twice
under it.

**Consumed by.** `build::pipeline::taxonomy::build_taxonomy_map` (stage
10) calls it three times, once per kind, and fills a `TaxonomyMap`. The
term pages `/tags/rust/` and the list pages `/tags/` are rendered from
that map, with `extra.taxonomy` as the template variable.

## Pagination

**Question.** How is a long listing split across several URLs?

**Inputs.** The section's sorted listing and its `paginate_by`.

**Where.** `build::pipeline::pages::render_paginated_section` (private,
in `taxus-generator/src/build/pipeline/pages.rs`).

**Answer.** Slices of `paginate_by` items. Slice 1 renders at the
section's own URL; slice *n* renders at `<section>/page/n/`. Each render
gets a `PaginationContext` with `current`, `total`, `prev`, `next`,
`first` and `last`. This is a derivation over a derivation: it takes the
listing above and cuts it. It is not in the domain crate because the
slices exist only to be rendered.

**Consumed by.** The template field `section.pagination` (stage 6).
`get_section` never returns pagination; slicing belongs to the section's
own render.

## Feed entries

**Question.** Which pages does the feed announce, and in what order?

**Inputs.** The tree and the `[feed] sections` config.

**Where.** `build::pipeline::feeds::feed_pages(tree, sections) -> Vec<&PageNode>`,
then `feed::FeedEntry::from_page` for each.

**Answer.** `recent(tree, false)`, narrowed to pages that have a `date`,
and to pages under one of the configured sections when any are named.
The feed generator then applies `[feed] limit`. Undated pages are left
out on purpose: a feed entry needs a publication date, and stamping it
with the build time would re-announce the page on every build.

**Consumed by.** `dist/feed.xml` and `dist/feed.atom` (stage 11).

## Sitemap entries

**Question.** Which addresses does the site have?

**Inputs.** The tree, the processed pages, and the site's base URL.

**Where.** `build::pipeline::sitemap::generate_sitemap`.

**Answer.** One entry per non-draft document from `documents`, with the
permalink, the `date` as `lastmod`, priority 1.0 for the root, 0.8 for
sections and 0.7 for pages, sorted by URL. Taxonomy and pagination pages
are not documents and are not listed.

**Consumed by.** `dist/sitemap.xml` (stage 8).

## Prev/next and ancestors

There is no prev/next derivation and no ancestors derivation in the code
today. `NodePath::parent` gives a node's parent path, and `get_section`
can fetch it, so a template can reach the parent. Nothing walks the
ancestor chain for it, and no context field names a previous or next
page.

## Tree method or derivation?

Two kinds of function take a tree. The rule for deciding which to write:

- It is a **tree method** if it answers "is this node here, and give it
  to me": a lookup by path, or an iteration that takes no parameters and
  applies no policy. `SiteTree::get_section`, `get_page`, `iter_pages`.
- It is a **derivation** if it selects, filters, orders or groups, or if
  the answer depends on frontmatter values, config, or a caller's choice
  such as whether drafts count. Everything in `derivation.rs`.

A method that took `include_drafts` would be a derivation in disguise. A
derivation that returned a single node by path would be a method in
disguise. Keeping them apart keeps the tree small and the policy visible.

Two further rules for a new derivation:

1. It goes in `taxus_domain::derivation` if it needs only the tree and
   plain values. It stays in the generator if it needs the config struct,
   rendered HTML, or generator types such as `ProcessedPage`.
2. It returns borrowed nodes (`&PageNode`, `Node<'_>`), never copies. The
   tree owns the data; a derivation is a view of it.
