// taxus-generator/tests/shortcodes.rs

//! Integration tests for the shortcode engine (Phase D): the negative
//! path through a full build, and the positive paths already pinned
//! byte-for-byte by the golden `shortcode_site` fixture.

use taxus_lib::SiteBuilder;

use std::path::Path;

fn fixture(name: &str) -> SiteBuilder {
    SiteBuilder::from_dir(Path::new(&format!("tests/fixtures/{name}"))).unwrap()
}

#[test]
fn unknown_shortcode_fails_the_build_naming_file_and_name() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("content")).unwrap();
    std::fs::create_dir_all(dir.path().join("templates")).unwrap();
    std::fs::write(
        dir.path().join("content/_index.md"),
        "+++\ntitle = \"T\"\n+++\n\n{{ definitely_not_registered() }}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("templates/base.html"),
        "<html><body>{% block content %}{% endblock %}</body></html>",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("templates/page.html"),
        "{% extends \"base.html\" %}{% block content %}{{ page.content | safe }}{% endblock %}",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("site.toml"),
        "[site]\nname = \"S\"\nbase_url = \"https://e.com\"\n",
    )
    .unwrap();

    let err = SiteBuilder::from_dir(dir.path())
        .unwrap()
        .build()
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("definitely_not_registered"),
        "error names the shortcode, got: {msg}"
    );
    assert!(
        msg.contains("_index.md"),
        "error names the content file, got: {msg}"
    );
}

#[test]
fn shortcode_site_builds_and_golden_pins_it() {
    // The full positive surface is pinned byte-for-byte by the golden
    // manifest (inline + block + built-ins + code immunity). This test
    // asserts the build succeeds and spot-checks the three headline
    // behaviors beyond what the hashes prove.
    let report = fixture("shortcode_site").build().unwrap();
    assert_eq!(report.pages_rendered, 1);

    let html = std::fs::read_to_string("tests/fixtures/shortcode_site/dist/blog/embeds/index.html")
        .unwrap();

    // Built-in image: @/ ref rewritten to the content-relative path.
    assert!(html.contains(r#"src="/blog/photo.jpg""#), "got: {html}");
    // Built-in youtube: nocookie lazy embed.
    assert!(html.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"));
    assert!(html.contains(r#"loading="lazy""#));
    // Custom block shortcode with args; body rendered as Markdown.
    assert!(
        html.contains(
            r#"<div class="box callout"><p><strong>Bold</strong> inside a block body.</p>"#
        ),
        "got: {html}"
    );
    // Code immunity: the fenced and inline examples stay literal.
    assert!(html.contains("never.png"), "fenced use expanded: {html}");
    assert!(html.contains("nope"), "inline use expanded: {html}");
}

#[test]
fn shortcodes_dir_collision_with_builtin_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("shortcodes")).unwrap();
    // "image" is a built-in; a file with that name must be rejected.
    std::fs::write(
        dir.path().join("shortcodes/image.html"),
        "<img src='hijacked'>",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("site.toml"),
        "[site]\nname = \"S\"\nbase_url = \"https://e.com\"\n",
    )
    .unwrap();

    let mut r = taxus_lib::build::pipeline::shortcodes::ShortcodeRenderer::new().unwrap();
    let err = r.load_dir(&dir.path().join("shortcodes")).unwrap_err();
    assert!(err.to_string().contains("image"), "got: {err}");
}
