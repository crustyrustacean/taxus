// taxus-generator/tests/sitemap.rs

//! Integration tests for `sitemap.xml` generation (#47).
//!
//! The sitemap's URL set composes from final outputs: every rendered
//! page (pagination included) plus every taxonomy list and term page.
//! Alias redirects are excluded deliberately. `<loc>` is XML-escaped.

use std::path::{Path, PathBuf};
use taxus_lib::config::SiteConfig;
use tempfile::TempDir;

/// Build a fixture site into a temp dir and return the generated
/// `sitemap.xml` content.
fn sitemap_for(site_dir: &Path) -> String {
    let output = TempDir::new().unwrap();
    let config = SiteConfig::from_dir(site_dir).unwrap();
    let output_dir = output.path().to_path_buf();

    // Run the full pipeline via SiteBuilder so the sitemap reflects
    // the real stage composition (render → taxonomy → sitemap).
    let builder = taxus_lib::build::SiteBuilder::new(config.clone()).output_dir(output_dir.clone());
    let _report = builder.build().unwrap();

    std::fs::read_to_string(output_dir.join("sitemap.xml")).unwrap()
}

#[test]
fn sitemap_includes_taxonomy_and_pagination() {
    let site = PathBuf::from("tests/fixtures/term_slug_site");
    let xml = sitemap_for(&site);

    // The two posts.
    assert!(xml.contains("https://example.com/blog/cafe/"));
    assert!(xml.contains("https://example.com/blog/latte/"));

    // Pagination page emitted by stage 6 (paginate_by = 1, two posts).
    assert!(xml.contains("https://example.com/blog/page/2/"));

    // Taxonomy list and term pages emitted by stage 9.
    assert!(xml.contains("https://example.com/tags/"));
    assert!(xml.contains("https://example.com/tags/café/"));
    assert!(xml.contains("https://example.com/categories/"));
    assert!(xml.contains("https://example.com/categories/crème/"));

    // The home page keeps its weekly changefreq.
    let home = xml.find("https://example.com/</loc>").unwrap();
    let entry = &xml[home..home + 200];
    assert!(entry.contains("<changefreq>weekly</changefreq>"));
    assert!(entry.contains("<priority>1.0</priority>"));
}

#[test]
fn sitemap_escapes_ampersand_in_loc() {
    // A base URL with an ampersand: legal in a URL, invalid as raw
    // XML text — <loc> must escape it (#47).
    let site = PathBuf::from("tests/fixtures/term_slug_site");
    let output = TempDir::new().unwrap();

    // Copy the fixture and override base_url.
    let staged = TempDir::new().unwrap();
    copy_dir(&site, staged.path());
    let toml_path = staged.path().join("site.toml");
    let toml = std::fs::read_to_string(&toml_path).unwrap();
    let toml = toml.replace(
        "https://example.com",
        "https://example.com/?ref=taxus&src=test",
    );
    std::fs::write(&toml_path, toml).unwrap();

    let config = SiteConfig::from_dir(staged.path()).unwrap();
    let builder =
        taxus_lib::build::SiteBuilder::new(config).output_dir(output.path().to_path_buf());
    let _report = builder.build().unwrap();

    let xml = std::fs::read_to_string(output.path().join("sitemap.xml")).unwrap();

    // Well-formed under a real XML parser, with the & escaped.
    let mut reader = quick_xml::Reader::from_str(&xml);
    reader.config_mut().check_end_names = true;
    let mut locs = Vec::new();
    let mut buf = Vec::new();
    // quick-xml splits text runs at entity boundaries: `&amp;` arrives
    // as Event::GeneralRef. Accumulate the decoded characters so the
    // assembled <loc> can be compared to the expected URL.
    let mut current: Option<String> = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                if e.name().as_ref() == "loc" {
                    current = Some(String::new());
                }
            }
            Ok(quick_xml::events::Event::Text(ref t)) => {
                if let Some(slot) = current.as_mut() {
                    slot.push_str(t.xml10_content().as_ref());
                }
            }
            Ok(quick_xml::events::Event::GeneralRef(ref r)) => {
                if let Some(slot) = current.as_mut() {
                    match r.as_ref() {
                        "amp" => slot.push('&'),
                        "lt" => slot.push('<'),
                        "gt" => slot.push('>'),
                        "quot" => slot.push('"'),
                        "apos" => slot.push('\''),
                        other => panic!("unexpected entity in <loc>: {other:?}"),
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                if e.name().as_ref() == "loc" {
                    locs.push(current.take().expect("text before </loc> end"));
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => panic!("sitemap is not well-formed XML: {e}"),
            _ => {}
        }
        buf.clear();
    }
    // With entities resolved by a real parser, the & is back: the
    // decoded loc carries the base URL verbatim.
    assert!(!locs.is_empty());
    assert!(
        locs.iter()
            .all(|l| l.starts_with("https://example.com/?ref=taxus&src=test")),
        "decoded locs should carry the base URL: {locs:?}"
    );
    assert!(
        !xml.contains("&src=test<"),
        "raw & must be escaped in <loc>: {}",
        xml.lines().find(|l| l.contains("src=test")).unwrap_or("?")
    );
}

/// Minimal recursive copy for staging fixtures.
fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}
