//! Integration tests for island server-side rendering (#37).
//!
//! `SiteBuilder::build()` is a synchronous API, so island SSR must work from
//! every calling context a library consumer or the CLI can reach it from:
//!
//! - plain synchronous code with no tokio runtime at all,
//! - inside a `current_thread` runtime (the `#[tokio::test]` default),
//! - inside a `multi_thread` runtime worker (the CLI build path),
//! - inside `spawn_blocking` on a `multi_thread` runtime (the dev-server
//!   rebuild path).
//!
//! Each test asserts the same thing: the render succeeds and the returned HTML
//! is the hydration mount point wrapping the SSR'd initial value.

use std::path::Path;
use taxus_common::components::counter::CounterProps;
use taxus_lib::build::SiteBuilder;
use taxus_lib::build::pipeline::render_island_counter;
use taxus_lib::config::SiteConfig;
use tempfile::TempDir;

/// Render a `Counter` island with `initial` and check the output shape.
fn assert_counter_html(initial: i32) {
    let html = render_island_counter(CounterProps {
        initial,
        class: String::new(),
    });

    assert!(
        html.contains(r#"data-island="Counter""#),
        "missing island mount point: {html}"
    );
    assert!(
        html.contains(&format!(r#"<span class="counter-value">{initial}</span>"#)),
        "missing SSR'd initial value {initial}: {html}"
    );
}

#[test]
fn island_ssr_works_without_any_runtime() {
    assert_counter_html(3);
}

#[tokio::test]
async fn island_ssr_works_inside_current_thread_runtime() {
    assert_counter_html(4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn island_ssr_works_inside_multi_thread_runtime_worker() {
    assert_counter_html(5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn island_ssr_works_inside_spawn_blocking() {
    tokio::task::spawn_blocking(|| assert_counter_html(6))
        .await
        .expect("spawn_blocking task panicked");
}

#[test]
fn island_ssr_works_from_concurrent_threads() {
    std::thread::scope(|s| {
        let a = s.spawn(|| assert_counter_html(10));
        let b = s.spawn(|| assert_counter_html(11));
        a.join().expect("first SSR thread panicked");
        b.join().expect("second SSR thread panicked");
    });
}

/// Scaffold a minimal site whose `page.html` and `section.html` place a
/// `Counter` island with `initial=7`, returning the site directory.
fn scaffold_island_site(root: &Path) -> std::path::PathBuf {
    let site_dir = root.join("site");
    std::fs::create_dir_all(site_dir.join("content")).unwrap();
    std::fs::create_dir_all(site_dir.join("templates")).unwrap();
    std::fs::create_dir_all(site_dir.join("styles")).unwrap();

    std::fs::write(
        site_dir.join("site.toml"),
        "[site]\nname = \"Island Site\"\nbase_url = \"https://example.com\"\n",
    )
    .unwrap();
    std::fs::write(
        site_dir.join("content/_index.md"),
        "+++\ntitle = \"Home\"\n+++\nHello",
    )
    .unwrap();
    std::fs::write(
        site_dir.join("content/about.md"),
        "+++\ntitle = \"About\"\n+++\nAbout us",
    )
    .unwrap();

    let island_template = "{% extends \"base.html\" %}\n\
         {% block content %}{{ page.content | safe }}\n\
         {{ island(component=\"Counter\", initial=7) | safe }}{% endblock %}\n";
    std::fs::write(site_dir.join("templates/page.html"), island_template).unwrap();
    std::fs::write(site_dir.join("templates/section.html"), island_template).unwrap();
    std::fs::write(
        site_dir.join("templates/base.html"),
        "<html><body>{% block content %}{% endblock %}</body></html>",
    )
    .unwrap();
    std::fs::write(site_dir.join("styles/main.scss"), "body { color: red; }").unwrap();

    site_dir
}

#[test]
fn dry_run_build_with_island_template_succeeds_without_runtime() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let site_dir = scaffold_island_site(temp_dir.path());

    let report = SiteBuilder::from_dir(&site_dir)
        .expect("Failed to load site")
        .dry_run(true)
        .build()
        .expect("dry-run build with island() template failed");

    assert!(report.pages_rendered + report.sections_rendered > 0);
}

#[test]
fn build_with_island_template_writes_ssr_html_without_runtime() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let site_dir = scaffold_island_site(temp_dir.path());
    let output_dir = temp_dir.path().join("dist");

    let mut config = SiteConfig::from_dir(&site_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    SiteBuilder::new(config)
        .build()
        .expect("build with island() template failed");

    let about = std::fs::read_to_string(output_dir.join("about/index.html"))
        .expect("about/index.html was not written");
    assert!(about.contains(r#"data-island="Counter""#), "{about}");
    assert!(
        about.contains(r#"<span class="counter-value">7</span>"#),
        "{about}"
    );
}

// ---------------------------------------------------------------------------
// #39: data-props JSON must survive a single quote in a prop value.
// ---------------------------------------------------------------------------

#[test]
fn island_props_with_quote_survive_attribute_embedding() {
    use taxus_common::components::counter::CounterProps;
    use taxus_common::components::search_box::SearchBoxProps;
    use taxus_lib::build::pipeline::{render_island_counter, render_search_box};

    // A placeholder containing a single quote — the #39 repro.
    let html = render_search_box(SearchBoxProps {
        placeholder: "What's new?".to_string(),
        max_results: 5,
        class: String::new(),
    });
    assert!(
        !html.contains("'What's"),
        "the raw quote must not appear inside data-props: {html}"
    );
    assert!(
        html.contains("data-props='{&quot;placeholder&quot;:&quot;What&#39;s new?&quot;"),
        "the JSON must be entity-escaped inside the attribute, got: {html}"
    );

    // A class with angle brackets exercises < / > escaping.
    let html = render_island_counter(CounterProps {
        initial: 1,
        class: "a<b&c".to_string(),
    });
    assert!(
        html.contains(
            "data-props='{&quot;initial&quot;:1,&quot;class&quot;:&quot;a&lt;b&amp;c&quot;}'"
        ),
        "the JSON must be entity-escaped inside the attribute, got: {html}"
    );

    // The attribute itself must stay well-formed: exactly one
    // data-props='...' span whose content is pure entities/safe chars.
    let start = html.find("data-props='").unwrap() + "data-props='".len();
    let end = html[start..].find('\'').unwrap() + start;
    let attr = &html[start..end];
    assert!(
        !attr.contains(['\'', '<', '>', '&']) || attr.rsplit("&amp;").count() > 1,
        "no unescaped delimiter characters in the attribute value: {attr}"
    );
}
