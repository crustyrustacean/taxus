# Glossary

This page is the vocabulary of Taxus. Every other page uses these words in
these senses. Each entry gives a one-sentence meaning, an example from a
real site, and the Rust type or function that embodies the term.

Examples come from two sites. The **scaffold** is what `taxus init my-site`
creates: one content file, `content/_index.md`. The **product site** is
`get-taxus-org/` in the repository, which has a blog and is built in CI.
The scaffold has no blog post, so blog examples use the product site.

Where Taxus uses a word differently from [Zola](https://www.getzola.org/),
the entry says so under **Zola**.

## Storage

**Site directory.** The folder that holds `site.toml`, `content/`,
`templates/`, `styles/` and `static/`. Example: `my-site/`. Type:
`taxus_lib::config::SiteConfig` (loaded from `site.toml`; `base_dir` is
this folder).

**Config.** The `site.toml` file: site name, base URL, directory names,
feed, image and highlighting options. Example: `name = "My Site"`. Type:
`SiteConfig`.

**Content directory.** The folder whose Markdown files become the site,
`content/` by default. Example: `my-site/content/`. Field:
`BuildConfig::content_dir`.

**Content file.** One Markdown file inside the content directory, always
named by its path relative to that directory. Example:
`blog/2026-04-03-project-launch.md`. Fields: `PageNode::content_file`,
`SectionNode::content_file`, `Page::source`, `RouteInfo::content_file`.
These four fields hold the same value; it is the storage identity of a
document.

**Index file.** A content file named `_index.md`; it gives a section its
frontmatter and body. Example: `content/blog/_index.md`. Field:
`SectionNode::content_file` (`Some` when the index file exists).

**Frontmatter.** The TOML block between `+++` lines at the top of a content
file. Example: `title = "Project Launch"`. Type:
`taxus_domain::Frontmatter`; the node field is `meta`.

**Body.** Everything in a content file after the frontmatter, as raw
Markdown. Example: the paragraphs of `project-launch.md`. Fields:
`PageNode::body`, `SectionNode::body`, `Page::raw_content`.

**Date prefix.** A `YYYY-MM-DD-` at the start of a file name; it is removed
from the slug and supplies a default `date`. Example: `2026-04-03-` in
`2026-04-03-project-launch.md`. Function: `taxus_lib::content::split_date_prefix`.
**Zola:** the same convention, with the same effect.

**Co-located asset.** A non-Markdown file inside the content directory,
copied to the same relative path in the output. Example:
`content/blog/mountain_sunset.jpg` becomes `dist/blog/mountain_sunset.jpg`.
Function: `build::pipeline::copy_colocated_assets`. **Zola:** a page's
assets live in a folder with its `index.md`; Taxus has no per-page folders.

**Static asset.** A file under `static/`, copied to `dist/static/`.
Example: `static/favicon.png`. Type: `assets::StaticCopier`.

**Output directory.** Where the built site is written, `dist/` by default.
Field: `BuildConfig::output_dir`.

## The Site Tree

**Site Tree.** The in-memory model of one site: a tree of sections and
pages, built once per build from the content directory. Example: for the
scaffold, a root section with no pages. Types: `taxus_domain::SiteTree`,
`SiteTreeBuilder`. **Zola:** the equivalent structure is called the
library.

**Node.** One item in the Site Tree, either a section or a page. Type:
`taxus_domain::derivation::Node` (a borrowed view of either kind).

**Section.** A directory inside the content directory, including the
content directory itself; it groups pages and other sections. Example:
`content/blog/` is the section `blog`. Type: `taxus_domain::SectionNode`.
**Zola:** a directory is a section only if it contains `_index.md`; in
Taxus every directory is a section, and one without an index file has
default frontmatter and nothing to render.

**Root section.** The section that is the content directory itself; its
node path is empty and its address is `/`. Example: `content/_index.md`
in the scaffold. Functions: `NodePath::root`, field `SiteTree::root`.

**Page.** A content file that is not an index file; a leaf of the tree.
Example: `content/blog/2026-04-03-project-launch.md`. Type:
`taxus_domain::PageNode`. **Zola:** the same.

**Document.** Anything the build renders to an HTML file: every page, and
every section that has an index file. Example: the product site has
eleven documents, six sections with index files and five posts. Type:
`taxus_domain::derivation::Node`, yielded by `derivation::documents`.

**Containment.** The relation between a section and its direct children.
`SectionNode::pages` and `SectionNode::subsections` hold direct children
only. Example: `blog` contains `blog/project-launch`; the root does not.

**Reachability.** Everything below a section at any depth. It is computed
on request, never stored. Function: `derivation::descendant_pages`.

**Tree order.** The canonical order of documents: a section's index file,
then its pages by slug, then each subsection by slug, recursively.
Function: `derivation::documents`. Every list the build produces starts
from this order.

**Draft.** A document whose frontmatter sets `draft = true`; it is
excluded unless the build passes `--include-drafts`. Field:
`Frontmatter::draft`; method `PageNode::is_draft`. **Zola:** the same.

## Identity

**Slug.** One segment of a URL path, already made safe for that use.
Example: `project-launch`. Type: `taxus_domain::Slug`. Slugs are produced
by `taxus_lib::routes::slugify::slugify_segment` (lowercase, ASCII,
dashes) or taken verbatim from the frontmatter `slug` field.
**Zola:** the same word and the same override field.

**Node path.** The list of slugs from the root section to a node; the
tree's name for the node. Example: `["blog", "project-launch"]`, written
`blog/project-launch`. Type: `taxus_domain::NodePath`. Older text calls
this the membership path or the tree path; they are the same thing.

**URL path.** The address a document is served at, derived from its node
path in one place. Example: `/blog/project-launch/`. Type:
`taxus_domain::UrlPath`; function `UrlPath::from_node_path`. In the
generator this is `RouteInfo::path`, `ProcessedPage::effective_url_path()`
and the template variable `page.path`.

**Permalink.** The URL path joined to the site's base URL. Example:
`https://get-taxus.org/blog/project-launch/`. Function:
`taxus_lib::templates::compute_permalink`.

**Output file.** The file under the output directory that a document is
written to, mirroring the URL path. Example:
`blog/project-launch/index.html`. Field: `RouteInfo::output_file`.

**Route.** One document's URL path, content file, output file and kind
(page or section), as a plain record. Example: the row `taxus routes`
prints for `/blog/project-launch/`. Types: `RouteInfo`, `RouteKind`,
`RouteRegistry` (built from the tree by `RouteRegistry::from_tree`).

**Alias.** An old URL path that should redirect to a document. Example:
`aliases = ["/launch/"]`. Type: `build::pipeline::alias::AliasPage`.
**Zola:** the same field.

**Internal link.** A Markdown link whose target starts with `@/` and names
a content file; the build replaces it with the document's URL path.
Example: `[launch](@/blog/2026-04-03-project-launch.md)`. Function:
`build::pipeline::internal_links::resolve_internal_links`. **Zola:** the
same syntax.

## Derivations

**Derivation.** A pure function of the Site Tree and the config that
returns a list or a grouping; it stores nothing and reads no files.
Module: `taxus_domain::derivation`. Older pages say projection or query.

**Sorting.** Putting a list of pages in the order a section asks for:
date newest first with undated last, title case-insensitive, weight lowest
first, or none. Function: `taxus_domain::tree::sort_pages`; type `SortBy`;
frontmatter key `sort_by`. **Zola:** the same keys; Zola sorts undated
pages differently.

**Listing.** The pages a section shows on its own index page: its direct
child pages, plus any aggregation, sorted. Example: `/blog/` lists the
five posts. Template variable: `section.pages`. Function:
`build::pipeline::pages::collect_child_pages` (private).

**Aggregation.** A section listing pages it does not contain, declared by
naming the sections to take them from. Example: `pages_from = ["blog"]`
on the root index file. Function: `derivation::aggregate`; frontmatter
key `pages_from`. The listing section is the **receiver**; each named
section is a **donor**. **Zola:** the nearest feature is `transparent`,
which pushes pages up to the parent; `pages_from` pulls, and the receiver
chooses.

**Recent pages.** Every non-draft page in the site, newest first. Function:
`derivation::recent`. Feeds start from it.

**Taxonomy.** One of the three ways a document says what it is about:
tags, categories or series. Example: `tags = ["rust", "ssg"]`. Type:
`taxus_lib::content::TaxonomyKind`. **Zola:** taxonomies are declared in
`config.toml`; in Taxus the three kinds are fixed.

**Term.** One value of a taxonomy, with the documents that carry it.
Example: the tag `rust`, listed at `/tags/rust/`. Types:
`derivation::group_by_terms` (the grouping), `content::TaxonomyTerm`,
`templates::TaxonomyTermContext`. **Zola:** the same word.

**Pagination.** Splitting a listing into fixed-size slices, each rendered
to its own URL. Example: `paginate_by = 10` gives `/blog/` and
`/blog/page/2/`. Type: `templates::PaginationContext`; frontmatter keys
`paginate_by`, `paginate_template`. **Zola:** the same keys and URLs.

**Feed.** The RSS or Atom file that lists dated, non-draft pages newest
first. Example: `dist/feed.xml`. Functions:
`build::pipeline::feeds::feed_pages`, `feed::FeedEntry::from_page`.

**Sitemap.** The `sitemap.xml` that lists every non-draft document's
permalink. Function: `build::pipeline::sitemap::generate_sitemap`.

**Search index.** A binary file of every rendered document's words, read
by the browser. Example: `dist/search_index.bin`. Types:
`taxus_common::search::SearchIndex`, `SearchDocument`.

## Rendering

**Template.** A Tera HTML file under `templates/`. Example:
`templates/page.html`. Type: `taxus_lib::templates::TeraRenderer`.

**Context.** The set of variables one template render can see: `site`,
`page`, `section`, `now` and `extra`. Type: `templates::TemplateContext`.
The pieces are `SiteContext`, `PageContext`, `SectionContext`,
`NowContext`, and `HeroContext` under `page.hero`.

**Page context.** A document as a template sees it: title, URL path,
permalink, rendered HTML, summary, taxonomies. Type: `PageContext`. The
`page` variable holds one during every render, including a section's own
render, where it holds the index file.

**Section context.** A section as a template sees it, including its
listing and its direct subsections. Type: `SectionContext`; the
subsection entries are `SubsectionContext`.

**Tree function.** A Tera function that fetches any node's context by node
path: `get_section(path="blog")` and `get_page(path="blog/project-launch")`.
Function: `TeraRenderer::set_site_lookup` fills what they read.
**Zola:** the same functions; Zola takes only content-file paths.

**Summary.** The short text used for listings, feeds and search: the
frontmatter `summary`, else the text before `<!-- more -->`, else the
first paragraph. Method: `Page::summary`.

**Hero image.** An image named in frontmatter and resized into several
widths for a `<picture>` element. Example: `hero_image = "mountain_sunset.jpg"`.
Types: `images::ProcessedImage`, `templates::HeroContext`.

**Island.** A Yew component rendered to HTML at build time and made
interactive in the browser by the WASM client. Example:
`{{ island(component="Counter", initial=3) | safe }}`. The build-time call
is the Tera function `island()`; the HTML wrapper is the **mount point**,
`<div data-island="Counter" data-props='…'>`; the browser step is
**hydration**, `taxus_client::hydrate_islands`.

## The build

**Phase.** One of the three parts of a build: **parse** (files to tree),
**analyse** (derivations over the tree), **emit** (files out). See
[Overview](./overview.md).

**Stage.** One of the fifteen numbered steps `SiteBuilder::build` logs,
such as `[1/15] Discovering routes...`. Each stage belongs to one phase.
See [Architecture](../architecture.md).

**Processed page.** A document after its Markdown has been rendered:
route, parsed file, HTML, table of contents and hero image. Type:
`build::pipeline::ProcessedPage`.

**Rendered page.** A processed page after its template has run: route
and final HTML. Type: `build::pipeline::RenderedPage`.
