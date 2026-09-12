# Theory

Taxus is a compiler for websites. This chapter and the ones under it
explain how a build works; the reference pages that follow describe the
code.

A compiler reads source files, builds a model of what they mean, computes
facts about that model, and writes output files. Taxus does the same with
a site. The input is a folder: content files, assets, templates and a
config file. The output is a folder of HTML plus a few generated files
(feeds, a sitemap, a search index, `robots.txt`, `404.html`, the WASM
client).

The build has three phases.

**Parse.** Read the content directory once and build the
[Site Tree](./site-tree.md). Every content file is parsed into frontmatter
and body and placed in the tree at its [node path](./identity.md). After
this phase the tree does not change.

**Analyse.** Compute [derivations](./derivations.md) over the tree: the
order of documents, each section's listing, the taxonomy terms, the recent
pages a feed should carry, the entries a sitemap needs. A derivation is a
pure function of the tree and the config. Nothing is written back.

**Emit.** Turn the tree and the derivations into files: render Markdown to
HTML, run templates, resize hero images, compile SCSS, copy assets, and
write everything under the output directory.

```text
   content/  templates/  styles/  static/  site.toml
        │
        ▼
  ┌──────────────────────────────────────────────────────┐
  │ PARSE     filesystem ──► Site Tree                    │
  │           taxus-generator (routes::discovery) builds  │
  │           taxus-domain   (tree, identity, schema) owns│
  └──────────────────────────┬───────────────────────────┘
                             │  SiteTree (immutable)
                             ▼
  ┌──────────────────────────────────────────────────────┐
  │ ANALYSE   derivations over (tree, config)             │
  │           taxus-domain   (derivation) owns the pure   │
  │           functions; taxus-generator calls them       │
  └──────────────────────────┬───────────────────────────┘
                             │  lists, groupings, orders
                             ▼
  ┌──────────────────────────────────────────────────────┐
  │ EMIT      tree + derivations ──► files                │
  │           taxus-generator (build, templates, images,  │
  │           assets, feed); taxus-common supplies island │
  │           components; taxus-client is written to      │
  │           dist/wasm/                                  │
  └──────────────────────────┬───────────────────────────┘
                             │
                             ▼
                          dist/
```

Which crate owns which phase:

| Phase | Owner | What it holds |
|-------|-------|---------------|
| Parse | `taxus-domain` defines the tree; `taxus-generator` fills it | `SiteTree`, `SiteTreeBuilder`, `RouteDiscovery::discover_tree` |
| Analyse | `taxus-domain` | `derivation::documents`, `descendant_pages`, `recent`, `aggregate`, `group_by_terms`, `tree::sort_pages` |
| Emit | `taxus-generator`, with `taxus-common` and `taxus-client` | `SiteBuilder::build`, `templates`, `images`, `assets`, `feed`, the pipeline stages |

The domain crate does no I/O. It never reads a file, never renders
Markdown, and never runs a template. That is what makes its functions
easy to test and easy to reason about: give them a tree and they give
back a list. The generator does all the reading and writing around it.

The fifteen numbered stages that `taxus build` logs are a finer cut of
the same three phases. [Architecture](../architecture.md) maps each stage
to its phase and says what it reads and produces.
