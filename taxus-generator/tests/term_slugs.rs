//! End-to-end coverage for taxonomy term slug agreement: the term page a
//! tag gets and the links templates build to it must derive from the same
//! rule (`slugify_term`), or non-ASCII terms link to pages that do not
//! exist (PR #88's audit item: `Café` → page at `/tags/café/`, link to
//! `/tags/cafe/`).
//!
//! The fixture site `term_slug_site` has one post tagged `Café`
//! (categorised `Crème`), and its `page.html` links tags with
//! `{{ tag | term_slug }}` exactly as `get-taxus-org` does.

use std::path::Path;

use taxus_lib::build::SiteBuilder;
use taxus_lib::config::SiteConfig;

/// Build the fixture site into a temp dir and return the output root.
fn build_fixture() -> tempfile::TempDir {
    let fixture = Path::new("tests/fixtures/term_slug_site");
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut config = SiteConfig::from_dir(fixture).expect("fixture config");
    config.build.output_dir = temp.path().join("dist");

    SiteBuilder::new(config)
        .build()
        .expect("build fixture site");
    temp
}

/// The tag link in the rendered post and the term page the build writes
/// must agree byte for byte: `<a href="/tags/café/">` in the post,
/// `tags/café/index.html` on disk.
#[test]
fn term_page_and_template_link_agree_for_non_ascii_terms() {
    let temp = build_fixture();
    let dist = temp.path().join("dist");

    let post = std::fs::read_to_string(dist.join("blog/cafe/index.html"))
        .expect("post HTML (non-ASCII slug kept by the term rule is percent-encoded by Tera; the file name uses the raw form)");
    let link = format!(r#"href="/tags/{}/""#, "café");
    assert!(
        post.contains(&link),
        "post must link the term with the term rule\nexpected `{link}` in:\n{post}"
    );

    assert!(
        dist.join("tags/café/index.html").is_file(),
        "term page must exist at /tags/café/ — term slugs keep non-ASCII letters"
    );

    // And the category: the term page for `Crème` must exist too, so the
    // fixture exercises both kinds without category links in the template.
    assert!(
        dist.join("categories/crème/index.html").is_file(),
        "category term page must exist at /categories/crème/"
    );
}
