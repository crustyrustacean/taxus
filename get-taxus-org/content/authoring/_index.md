+++
title = "Authoring"
+++

### Write with Ease

Author your words with leveraging the CommonMark markdown standard, thanks to the [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) crate.

Intuitively build out the `content` directory to define your site. Simply create individual directories which correspond to the routes that compose your final site. Inside each directory, write an `_index.md` file and write the page content in markdown format. The Taxus build system will walk the content directory and use it to build out each HTML page.

### Describe with Rich Metadata

Metadata is defined via a frontmatter section:

```toml
+++
title = "Home"
description = "The home page"
+++
```

The frontmatter is parsed and converted into injectable variables for the template system.

### References:
[pulldown-cmark guide](https://pulldown-cmark.github.io/pulldown-cmark/)