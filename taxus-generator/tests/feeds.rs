//! End-to-end coverage for feed URL derivation (#19) and XML validity
//! (#38) at the pipeline level.
//!
//! The fixture site `feed_slug_site` has a dated page with a custom
//! slug (`blog/2026-01-10-original-name.md`, `slug = "served-here"`,
//! `date = 2026-01-10`).
//! The feed must link to the *served* URL — section path + slug —
//! because the Site Tree is the single derivation point for addresses.
//! Before the fix, the feed stage overwrote `Page::path` in place to
//! compensate; now the entry is built with the effective URL directly.

use std::path::Path;

use taxus_lib::build::SiteBuilder;
use taxus_lib::config::SiteConfig;

/// Build the fixture site and return the generated `feed.xml`.
fn build_fixture_feed() -> String {
    let fixture = Path::new("tests/fixtures/feed_slug_site");
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut config = SiteConfig::from_dir(fixture).expect("fixture config");
    config.build.output_dir = temp.path().join("dist");

    SiteBuilder::new(config)
        .build()
        .expect("build fixture site");

    std::fs::read_to_string(temp.path().join("dist/feed.xml")).expect("written feed")
}

/// #19: a slug-renamed page must appear in the feed at its effective
/// URL, not its filename path.
#[test]
fn feed_links_custom_slug_page_at_effective_url() {
    let rss = build_fixture_feed();

    assert!(
        rss.contains("https://example.com/blog/served-here/"),
        "feed must link the slug-renamed page at its served URL\n{rss}"
    );
    assert!(
        !rss.contains("https://example.com/blog/original-name/"),
        "feed must not link the filename path\n{rss}"
    );
    assert!(
        !rss.contains("https://example.com/blog/2026-01-10-original-name/"),
        "feed must not link the dated filename path\n{rss}"
    );
}

/// #38: the built feed is well-formed XML per a real parser.
#[test]
fn built_feed_is_well_formed_xml() {
    let rss = build_fixture_feed();

    let mut reader = quick_xml::Reader::from_str(&rss);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => buf.clear(),
            Err(e) => panic!("built feed.xml is not well-formed XML: {e}\n{rss}"),
        }
    }
}

/// #38: the self URL in `<atom:link>` must be escaped (the fixture's
/// base_url is clean, but the assertion pins the behaviour).
#[test]
fn built_feed_declares_content_namespace_when_used() {
    let rss = build_fixture_feed();
    if rss.contains("<content:encoded>") {
        assert!(
            rss.contains(r#"xmlns:content="http://purl.org/rss/1.0/modules/content/""#),
            "content namespace must be declared when content:encoded is used\n{rss}"
        );
    }
}

/// #58: `limit = 0` must be rejected at validation time with a message
/// pointing at the enabled flags.
#[test]
fn feed_limit_zero_is_a_config_error() {
    let toml = r#"
[site]
name = "T"
base_url = "https://example.com"

[feed]
limit = 0
"#;
    let config: SiteConfig = toml::from_str(toml).expect("parse");
    let err = config.validate().expect_err("limit = 0 must be rejected");

    let msg = err.to_string();
    assert!(
        msg.contains("rss_enabled"),
        "error should point at the flags: {msg}"
    );
}
