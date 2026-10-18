//! Storage-only ("flattened") content directories: `[build] flatten`.
//!
//! Uses the `flatten_site` fixture (`flatten = ["blog/*"]`):
//!
//! - `blog/_index.md` (`sort_by = "date"`) owns three posts stored one
//!   level down: `blog/2025/2025-12-01-old.md`, `blog/2026/2026-04-07-new.md`
//!   and the draft `blog/drafts/2026-11-01-wip.md`. All are served at
//!   `/blog/<slug>/`; `blog/2025`, `blog/2026` and `blog/drafts` are not
//!   sections.
//! - `new` links `@/blog/2025/2025-12-01-old.md` (storage path) and embeds
//!   `{{ image(src="@/blog/2026/diagram.png") }}`; the co-located
//!   `diagram.png` keeps its storage path in the output (rule 6, option B).
//! - `notes/2026/` (with its own `_index.md`) is not matched by the
//!   pattern, so it stays a section and `kept.md` is served at
//!   `/notes/2026/kept/`.
//!
//! Error cases mutate a temp-dir copy of the fixture. The last test builds
//! a scratch copy of `get-taxus-org` with its posts moved into `blog/2026/`
//! and compares it against the committed golden manifest.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use taxus_common::search::SearchIndex;
use taxus_lib::build::SiteBuilder;
use taxus_lib::build::pipeline::{discover_routes, discover_tree};
use taxus_lib::config::SiteConfig;
use taxus_lib::routes::{FlattenPattern, RouteDiscovery, RouteRegistry};
use tempfile::TempDir;

const FIXTURE: &str = "tests/fixtures/flatten_site";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_dir() -> PathBuf {
    crate_dir().join(FIXTURE)
}

/// Build `site_dir` into a fresh temp dir and return it.
fn build_site(site_dir: &Path, include_drafts: bool) -> TempDir {
    let out = TempDir::new().unwrap();
    SiteBuilder::from_dir(site_dir)
        .unwrap_or_else(|e| panic!("load {}: {e}", site_dir.display()))
        .output_dir(out.path())
        .include_drafts(include_drafts)
        .build()
        .unwrap_or_else(|e| panic!("build {}: {e}", site_dir.display()));
    out
}

fn build_fixture() -> TempDir {
    build_site(&fixture_dir(), false)
}

/// Build `site_dir` and return the error message; panics if it builds.
fn build_error(site_dir: &Path) -> String {
    let out = TempDir::new().unwrap();
    let result = SiteBuilder::from_dir(site_dir).and_then(|b| b.output_dir(out.path()).build());
    match result {
        Ok(_) => panic!("{} built, expected an error", site_dir.display()),
        // Fold Windows separators so assertions can name `blog/2026/x.md`.
        Err(e) => e.to_string().replace('\\', "/"),
    }
}

/// Recursively copy `from` into `to`, skipping a top-level `dist/`.
fn copy_site(from: &Path, to: &Path) {
    for entry in walkdir::WalkDir::new(from)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(from).unwrap();
        if relative.starts_with("dist") {
            continue;
        }
        let target = to.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(entry.path(), &target).unwrap();
    }
}

/// A temp-dir copy of the fixture, for mutation.
fn fixture_copy() -> TempDir {
    let tmp = TempDir::new().unwrap();
    copy_site(&fixture_dir(), tmp.path());
    tmp
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Replace the `flatten = [...]` line of a copied fixture's site.toml.
fn set_flatten_line(site: &Path, line: &str) {
    let toml_path = site.join("site.toml");
    let toml = fs::read_to_string(&toml_path)
        .unwrap()
        .replace("\r\n", "\n");
    let replaced: Vec<&str> = toml
        .lines()
        .map(|l| {
            if l.trim_start().starts_with("flatten") {
                line
            } else {
                l
            }
        })
        .collect();
    assert!(
        replaced.contains(&line),
        "fixture site.toml has a flatten line"
    );
    fs::write(&toml_path, replaced.join("\n") + "\n").unwrap();
}

fn read(out: &Path, file: &str) -> String {
    fs::read_to_string(out.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"))
}

/// The `href` targets of the `<ul class="{class}">` list, in document order.
fn links_in(out: &Path, page: &str, class: &str) -> Vec<String> {
    let html = read(out, page);
    let list = html
        .split(&format!("<ul class=\"{class}\">"))
        .nth(1)
        .unwrap_or_else(|| panic!("no <ul class=\"{class}\"> in {page}:\n{html}"))
        .split("</ul>")
        .next()
        .unwrap();
    list.split("href=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .collect()
}

/// Bodies of every `<tag>…</tag>` element.
fn element_bodies(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    xml.split(&open)
        .skip(1)
        .filter_map(|rest| rest.split(&close).next().map(str::to_owned))
        .collect()
}

/// Values of every `<link href="…"` (Atom).
fn atom_link_hrefs(xml: &str) -> Vec<String> {
    xml.split("<link href=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .collect()
}

/// No page URL in `urls` sits under a flattened storage directory.
fn assert_no_storage_urls(what: &str, urls: &[String]) {
    for url in urls {
        for storage in ["/blog/2025/", "/blog/2026/", "/blog/drafts/"] {
            assert!(
                !url.contains(storage),
                "{what}: page URL {url} contains flattened segment {storage}\nall: {urls:#?}"
            );
        }
    }
}

fn fixture_routes() -> RouteRegistry {
    let config = SiteConfig::from_dir(fixture_dir()).expect("fixture config");
    discover_routes(&config).expect("routes")
}

// ---------------------------------------------------------------------------
// Config & pattern API
// ---------------------------------------------------------------------------

#[test]
fn fixture_config_carries_flatten_patterns() {
    let config = SiteConfig::from_dir(fixture_dir()).expect("fixture config");
    assert_eq!(config.build.flatten, vec!["blog/*".to_string()]);
}

#[test]
fn flatten_pattern_star_matches_exactly_one_segment() {
    let p = FlattenPattern::parse("blog/*").unwrap();
    assert_eq!(p.as_str(), "blog/*");
    assert!(p.matches(Path::new("blog/2026")));
    assert!(p.matches(Path::new("blog/drafts")));
    assert!(
        !p.matches(Path::new("blog")),
        "the parent itself is not flattened"
    );
    assert!(!p.matches(Path::new("blog/2026/04")), "* is one segment");
    assert!(!p.matches(Path::new("notes/2026")));
    assert!(!p.matches(Path::new("")));

    let literal = FlattenPattern::parse("blog/drafts").unwrap();
    assert!(literal.matches(Path::new("blog/drafts")));
    assert!(!literal.matches(Path::new("blog/2026")));
}

#[test]
fn flatten_pattern_rejects_invalid_patterns() {
    for bad in ["/blog", "blog/**", "../x", "blog//x", ""] {
        assert!(
            FlattenPattern::parse(bad).is_err(),
            "pattern {bad:?} must be rejected"
        );
    }
}

#[test]
fn route_discovery_with_flatten_matches_pipeline() {
    let config = SiteConfig::from_dir(fixture_dir()).unwrap();
    let patterns = vec![FlattenPattern::parse("blog/*").unwrap()];
    let tree = RouteDiscovery::new(&config.build.content_dir)
        .with_flatten(patterns)
        .discover_tree()
        .unwrap();
    let direct: BTreeSet<String> = RouteRegistry::from_tree(&tree)
        .iter()
        .map(|r| r.path.clone())
        .collect();
    let piped: BTreeSet<String> = RouteRegistry::from_tree(&discover_tree(&config).unwrap())
        .iter()
        .map(|r| r.path.clone())
        .collect();
    assert_eq!(direct, piped);
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

#[test]
fn flattened_pages_are_served_under_the_parent_section() {
    let routes = fixture_routes();
    let all: Vec<&str> = routes.iter().map(|r| r.path.as_str()).collect();

    for present in ["/", "/blog/", "/blog/old/", "/blog/new/", "/blog/wip/"] {
        assert!(routes.contains(present), "missing {present}: {all:?}");
    }
    // The draft has a route (discovery doesn't filter drafts); rendering
    // skips it — see the listing tests.
    for absent in [
        "/blog/2025/",
        "/blog/2026/",
        "/blog/drafts/",
        "/blog/2025/old/",
        "/blog/2026/new/",
        "/blog/drafts/wip/",
    ] {
        assert!(!routes.contains(absent), "unexpected {absent}: {all:?}");
    }
}

#[test]
fn unflattened_directories_keep_their_sections() {
    let routes = fixture_routes();
    let all: Vec<&str> = routes.iter().map(|r| r.path.as_str()).collect();
    assert!(routes.contains("/notes/"), "{all:?}");
    assert!(routes.contains("/notes/2026/"), "{all:?}");
    assert!(routes.contains("/notes/2026/kept/"), "{all:?}");
    assert!(routes.get("/notes/2026/").unwrap().is_section());
}

#[test]
fn flattened_page_keeps_its_storage_content_file() {
    let routes = fixture_routes();
    let new = routes.get("/blog/new/").expect("/blog/new/");
    assert!(
        new.content_file
            .components()
            .eq(Path::new("blog/2026/2026-04-07-new.md").components()),
        "content_file is the real storage path, got {}",
        new.content_file.display()
    );
    assert!(
        new.output_file
            .components()
            .eq(Path::new("blog/new/index.html").components()),
        "output_file follows the URL, got {}",
        new.output_file.display()
    );
    let old = routes.get("/blog/old/").expect("/blog/old/");
    assert!(
        old.content_file
            .components()
            .eq(Path::new("blog/2025/2025-12-01-old.md").components())
    );
}

// ---------------------------------------------------------------------------
// Rendered output
// ---------------------------------------------------------------------------

#[test]
fn no_pages_are_rendered_at_storage_paths() {
    let out = build_fixture();
    let o = out.path();
    assert!(o.join("blog/old/index.html").is_file());
    assert!(o.join("blog/new/index.html").is_file());
    assert!(o.join("notes/2026/kept/index.html").is_file());
    assert!(o.join("notes/2026/index.html").is_file());
    assert!(
        !o.join("blog/2025").exists(),
        "blog/2025 has no assets or pages"
    );
    assert!(!o.join("blog/2026/index.html").exists());
    assert!(!o.join("blog/2026/new").exists());
    assert!(!o.join("blog/drafts").exists());
    assert!(
        !o.join("blog/wip").exists(),
        "draft not rendered by default"
    );
}

#[test]
fn blog_section_lists_flattened_pages_newest_first() {
    let out = build_fixture();
    assert_eq!(
        links_in(out.path(), "blog/index.html", "page-list"),
        ["/blog/new/", "/blog/old/"]
    );
    // No year sections hang off the blog.
    assert_eq!(
        links_in(out.path(), "blog/index.html", "subsections"),
        Vec::<String>::new()
    );
}

#[test]
fn get_section_sees_flattened_pages() {
    let out = build_fixture();
    assert_eq!(
        links_in(out.path(), "index.html", "recent"),
        ["/blog/new/", "/blog/old/"]
    );
    assert_eq!(
        links_in(out.path(), "index.html", "subsections"),
        ["/blog/", "/notes/"]
    );
}

#[test]
fn flattened_draft_is_listed_only_with_include_drafts() {
    let out = build_site(&fixture_dir(), true);
    assert_eq!(
        links_in(out.path(), "blog/index.html", "page-list"),
        ["/blog/wip/", "/blog/new/", "/blog/old/"]
    );
    assert!(out.path().join("blog/wip/index.html").is_file());
    assert!(!out.path().join("blog/drafts").exists());
}

#[test]
fn internal_link_to_storage_path_resolves_to_flattened_url() {
    let out = build_fixture();
    let html = read(out.path(), "blog/new/index.html");
    assert!(html.contains(r#"href="/blog/old/""#), "{html}");
    assert!(!html.contains("@/"), "no unresolved @/ refs: {html}");
}

#[test]
fn colocated_asset_keeps_its_storage_path() {
    let out = build_fixture();
    let copied = out.path().join("blog/2026/diagram.png");
    assert!(copied.is_file(), "diagram.png copied to blog/2026/");
    assert!(!out.path().join("blog/diagram.png").exists());
    assert!(!out.path().join("blog/new/diagram.png").exists());
    assert_eq!(
        fs::read(&copied).unwrap(),
        fs::read(fixture_dir().join("content/blog/2026/diagram.png")).unwrap()
    );

    let html = read(out.path(), "blog/new/index.html");
    assert!(html.contains(r#"src="/blog/2026/diagram.png""#), "{html}");
}

#[test]
fn taxonomy_term_pages_link_flattened_urls() {
    let out = build_fixture();
    let rust: BTreeSet<String> = links_in(out.path(), "tags/rust/index.html", "term-pages")
        .into_iter()
        .collect();
    assert_eq!(
        rust,
        BTreeSet::from(["/blog/new/".to_string(), "/blog/old/".to_string()]),
        "draft excluded, flattened URLs used"
    );
    assert_eq!(
        links_in(out.path(), "categories/news/index.html", "term-pages"),
        ["/blog/new/"]
    );
}

#[test]
fn feeds_link_flattened_urls_only() {
    let out = build_fixture();

    let rss = read(out.path(), "feed.xml");
    let rss_links = element_bodies(&rss, "link");
    assert!(
        rss_links
            .iter()
            .any(|l| l == "https://example.com/blog/new/"),
        "{rss}"
    );
    assert!(
        rss_links
            .iter()
            .any(|l| l == "https://example.com/blog/old/"),
        "{rss}"
    );
    assert!(
        !rss_links.iter().any(|l| l.ends_with("/blog/wip/")),
        "{rss}"
    );
    assert_no_storage_urls("feed.xml <link>", &rss_links);

    let atom = read(out.path(), "feed.atom");
    let atom_links = atom_link_hrefs(&atom);
    assert!(
        atom_links
            .iter()
            .any(|l| l == "https://example.com/blog/new/"),
        "{atom}"
    );
    assert!(
        atom_links
            .iter()
            .any(|l| l == "https://example.com/blog/old/"),
        "{atom}"
    );
    assert_no_storage_urls("feed.atom <link href>", &atom_links);
    assert_no_storage_urls("feed.atom <id>", &element_bodies(&atom, "id"));
}

#[test]
fn sitemap_lists_flattened_urls_only() {
    let out = build_fixture();
    let xml = read(out.path(), "sitemap.xml");
    let locs = element_bodies(&xml, "loc");
    for want in [
        "https://example.com/blog/",
        "https://example.com/blog/new/",
        "https://example.com/blog/old/",
        "https://example.com/notes/2026/",
        "https://example.com/notes/2026/kept/",
    ] {
        assert!(locs.iter().any(|l| l == want), "missing {want}: {locs:#?}");
    }
    assert!(!locs.iter().any(|l| l.ends_with("/blog/wip/")), "{locs:#?}");
    assert_no_storage_urls("sitemap <loc>", &locs);
}

#[test]
fn search_index_uses_flattened_urls() {
    let out = build_fixture();
    let bytes = fs::read(out.path().join("search_index.bin")).expect("search index written");
    let index = SearchIndex::from_bytes(&bytes).expect("index roundtrip");
    let paths: Vec<String> = index.documents.values().map(|d| d.path.clone()).collect();

    assert!(paths.iter().any(|p| p == "/blog/new/"), "{paths:?}");
    assert!(paths.iter().any(|p| p == "/blog/old/"), "{paths:?}");
    assert!(paths.iter().any(|p| p == "/notes/2026/kept/"), "{paths:?}");
    assert_no_storage_urls("search index", &paths);
    for p in &paths {
        let page = out
            .path()
            .join(p.trim_start_matches('/'))
            .join("index.html");
        assert!(page.is_file(), "search hit {p} has no built page");
    }
}

// ---------------------------------------------------------------------------
// Errors and edge cases
// ---------------------------------------------------------------------------

#[test]
fn index_file_in_flattened_dir_is_an_error() {
    let site = fixture_copy();
    write(
        &site.path().join("content/blog/2026/_index.md"),
        "+++\ntitle = \"2026\"\n+++\n",
    );
    let msg = build_error(site.path());
    assert!(msg.contains("_index.md"), "names the file: {msg}");
    assert!(msg.contains("blog/2026"), "names the directory: {msg}");
    assert!(msg.contains("blog/*"), "names the pattern: {msg}");
}

#[test]
fn slug_collision_across_flattened_dirs_names_both_files() {
    let site = fixture_copy();
    write(
        &site.path().join("content/blog/2025/a.md"),
        "+++\ntitle = \"A 2025\"\ndate = 2025-01-01\n+++\nA.\n",
    );
    write(
        &site.path().join("content/blog/2026/a.md"),
        "+++\ntitle = \"A 2026\"\ndate = 2026-01-01\n+++\nA.\n",
    );
    let msg = build_error(site.path());
    assert!(msg.contains("Duplicate route"), "{msg}");
    assert!(
        msg.contains("blog/2025/a.md"),
        "names the first file: {msg}"
    );
    assert!(
        msg.contains("blog/2026/a.md"),
        "names the second file: {msg}"
    );
}

#[test]
fn date_prefix_does_not_avoid_slug_collision() {
    // `2025-06-01-old.md` slugs to `old`, colliding with the fixture's
    // `blog/2025/2025-12-01-old.md` once both are flattened into `blog`.
    let site = fixture_copy();
    write(
        &site.path().join("content/blog/2026/2026-06-01-old.md"),
        "+++\ntitle = \"Another old\"\ndate = 2026-06-01\n+++\nB.\n",
    );
    let msg = build_error(site.path());
    assert!(msg.contains("Duplicate route"), "{msg}");
    assert!(msg.contains("2025-12-01-old.md"), "{msg}");
    assert!(msg.contains("2026-06-01-old.md"), "{msg}");
}

#[test]
fn invalid_flatten_patterns_fail_config_load() {
    for bad in ["/blog", "blog/**", "../x", "blog//x", ""] {
        let site = fixture_copy();
        set_flatten_line(site.path(), &format!("flatten = [\"{bad}\"]"));
        let err = match SiteConfig::from_dir(site.path()) {
            Ok(_) => panic!("pattern {bad:?} was accepted"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("build.flatten"), "{bad:?}: {err}");
        if !bad.is_empty() {
            assert!(err.contains(bad), "{bad:?} named in: {err}");
        }
    }
}

#[test]
fn pages_from_naming_a_flattened_dir_only_warns() {
    let site = fixture_copy();
    write(
        &site.path().join("content/_index.md"),
        "+++\ntitle = \"Home\"\ntemplate = \"home.html\"\npages_from = [\"blog/2026\"]\n+++\nHome.\n",
    );
    let out = build_site(site.path(), false);
    assert!(out.path().join("index.html").is_file());
    assert!(out.path().join("blog/new/index.html").is_file());
}

#[test]
fn empty_flatten_keeps_storage_urls() {
    // The `flatten = []` regression guard on this fixture: year dirs are
    // ordinary sections again.
    let site = fixture_copy();
    set_flatten_line(site.path(), "flatten = []");
    let config = SiteConfig::from_dir(site.path()).unwrap();
    let routes = discover_routes(&config).unwrap();
    let all: Vec<&str> = routes.iter().map(|r| r.path.as_str()).collect();
    assert!(routes.contains("/blog/2025/old/"), "{all:?}");
    assert!(routes.contains("/blog/2026/new/"), "{all:?}");
    assert!(!routes.contains("/blog/new/"), "{all:?}");
}

// ---------------------------------------------------------------------------
// get-taxus-org scratch copy vs the committed golden manifest
//
// The manifest helpers below duplicate the minimal subset of
// `tests/golden_output.rs` (hashing, normalisation, manifest reading).
// ---------------------------------------------------------------------------

const UNSTABLE: &str = "<unstable>";
const TEXT_EXTENSIONS: &[&str] = &["html", "xml", "txt", "svg", "css", "js", "json"];

fn is_excluded(relative: &Path) -> bool {
    relative.starts_with("wasm") || relative == Path::new("search_index.bin")
}

fn blank_element(xml: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let body_start = start + open.len();
        match rest[body_start..].find(&close) {
            Some(len) => {
                out.push_str(&rest[..body_start]);
                out.push_str("TIMESTAMP");
                rest = &rest[body_start + len..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

fn normalise(relative: &Path, bytes: Vec<u8>) -> Vec<u8> {
    let ext = relative.extension().and_then(|e| e.to_str());
    if ext.is_some_and(|e| TEXT_EXTENSIONS.contains(&e))
        && let Ok(text) = String::from_utf8(bytes.clone())
    {
        let mut text = text.replace("\r\n", "\n");
        if ext == Some("xml") {
            for tag in ["lastBuildDate", "pubDate", "updated", "published"] {
                text = blank_element(&text, tag);
            }
        }
        return text.into_bytes();
    }
    bytes
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn manifest_of(output_dir: &Path) -> BTreeMap<String, String> {
    let mut manifest = BTreeMap::new();
    for entry in walkdir::WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(output_dir).unwrap();
        if is_excluded(relative) {
            continue;
        }
        let bytes = fs::read(entry.path()).unwrap();
        let key = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        manifest.insert(key, sha256_hex(&normalise(relative, bytes)));
    }
    manifest
}

fn read_manifest(path: &Path) -> BTreeMap<String, String> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("missing golden manifest {} ({e})", path.display()));
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let (hash, path) = line
                .split_once("  ")
                .unwrap_or_else(|| panic!("malformed manifest line: {line:?}"));
            (path.to_owned(), hash.to_owned())
        })
        .collect()
}

/// Insert `line` directly after the `[build]` header of a TOML document.
fn insert_after_build_header(toml: &str, line: &str) -> String {
    let toml = toml.replace("\r\n", "\n");
    let mut out = Vec::new();
    let mut inserted = false;
    for l in toml.lines() {
        out.push(l.to_owned());
        if !inserted && l.trim() == "[build]" {
            out.push(line.to_owned());
            inserted = true;
        }
    }
    assert!(inserted, "site.toml has a [build] table");
    out.join("\n") + "\n"
}

#[test]
fn get_taxus_org_with_year_folders_matches_golden_manifest() {
    let source = crate_dir().join("../get-taxus-org");
    let tmp = TempDir::new().unwrap();
    let site = tmp.path().join("get-taxus-org");
    copy_site(&source, &site);

    // [build] flatten = ["blog/*"]
    let toml_path = site.join("site.toml");
    let toml = fs::read_to_string(&toml_path).unwrap();
    fs::write(
        &toml_path,
        insert_after_build_header(&toml, "flatten = [\"blog/*\"]"),
    )
    .unwrap();

    // Move every post and every co-located image into blog/2026/.
    let blog = site.join("content/blog");
    let year = blog.join("2026");
    fs::create_dir_all(&year).unwrap();
    let mut moved_md = 0;
    let mut moved_jpg = 0;
    for entry in fs::read_dir(&blog).unwrap().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let ext = path.extension().and_then(|e| e.to_str());
        match ext {
            Some("md") if name != "_index.md" => moved_md += 1,
            Some("jpg") => moved_jpg += 1,
            _ => continue,
        }
        fs::rename(&path, year.join(&name)).unwrap();
    }
    assert!(moved_md >= 5, "moved {moved_md} posts");
    assert_eq!(moved_jpg, 2, "the two hero images");

    let output_dir = tmp.path().join("dist");
    SiteBuilder::from_dir(&site)
        .unwrap_or_else(|e| panic!("load scratch get-taxus-org: {e}"))
        .output_dir(&output_dir)
        .build()
        .unwrap_or_else(|e| panic!("build scratch get-taxus-org: {e}"));
    let actual = manifest_of(&output_dir);

    // The committed manifest, with the two raw image copies moved to their
    // storage path (rule 6, option B). Everything else is identical.
    let mut expected = read_manifest(&crate_dir().join("tests/golden/get-taxus-org.manifest"));
    for img in ["mountain_sunset.jpg", "syntax_highlighting.jpg"] {
        let hash = expected
            .remove(&format!("blog/{img}"))
            .unwrap_or_else(|| panic!("golden manifest lists blog/{img}"));
        expected.insert(format!("blog/2026/{img}"), hash);
    }

    let mut problems = Vec::new();
    for (path, hash) in &expected {
        match actual.get(path) {
            None => problems.push(format!("missing {path}")),
            Some(_) if hash == UNSTABLE => {}
            Some(a) if a != hash => problems.push(format!("changed {path}")),
            Some(_) => {}
        }
    }
    for path in actual.keys() {
        if !expected.contains_key(path) {
            problems.push(format!("added {path}"));
        }
    }
    assert!(
        problems.is_empty(),
        "flattened get-taxus-org differs from the golden manifest beyond the \
         two moved images:\n  {}",
        problems.join("\n  ")
    );
}
