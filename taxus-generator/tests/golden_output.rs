//! Golden output test.
//!
//! Builds every complete fixture site (and `get-taxus-org/`) into a temp
//! directory and compares a manifest of `(relative path, sha256)` for every
//! output file against the manifest committed under `tests/golden/`.
//!
//! The committed manifests were generated from the `trunk` state *before*
//! the Site Tree was wired into the generator, so this test proves that
//! wiring changed nothing in the generated output.
//!
//! # Re-baselining
//!
//! ```text
//! GOLDEN_UPDATE=1 cargo test -p taxus --test golden_output
//! ```
//!
//! rewrites the manifests. Only do this in a dedicated commit whose message
//! says which files changed and why; a manifest change is a behaviour change.
//!
//! # What is and is not covered
//!
//! - `wasm/*` and `search_index.bin` are excluded: they depend on the
//!   embedded client build, not on the generator's content pipeline.
//! - In `*.xml` feed files the `<lastBuildDate>`, `<pubDate>`, `<updated>`
//!   and `<published>` element bodies are blanked before hashing: feeds
//!   stamp the build time, and undated pages get the build time as their
//!   publication date.
//! - Hero image variants are named `<stem>-<hash>-<width>w.<ext>`, where
//!   the hash is a digest of the source's bytes and the encoding quality.
//!   It is the same on every checkout, so variant names are recorded as
//!   is; a manifest change in an `images/` path means the key derivation
//!   or the source image changed.
//! - When updating, each site is built several times. A file whose hash
//!   differs between runs is recorded as `<unstable>`: the test then only
//!   checks that the file exists. This happens where output order depends
//!   on `HashMap` iteration order (e.g. two undated pages listed in the
//!   same section).

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use taxus_lib::build::SiteBuilder;

/// Sites that are complete enough to build: content + templates.
///
/// Paths are relative to the crate root (`taxus-generator/`), which is the
/// working directory for integration tests.
const SITES: &[(&str, &str)] = &[
    ("highlight_site", "tests/fixtures/highlight_site"),
    ("internal_links_site", "tests/fixtures/internal_links_site"),
    ("search_slug_site", "tests/fixtures/search_slug_site"),
    (
        "section_listing_site",
        "tests/fixtures/section_listing_site",
    ),
    ("get-taxus-org", "../get-taxus-org"),
];

/// Fixtures that are deliberately partial (no content and/or no templates)
/// and therefore cannot be built end-to-end. Every directory under
/// `tests/fixtures/` must appear either here or in [`SITES`].
const NOT_BUILDABLE: &[&str] = &[
    "asset_site",     // assets only, no site.toml
    "colocated_site", // site.toml has no [site] table; tests build it by hand
    "content_site",   // content only, no templates
    "full_site",      // site.toml only
    "hero_site",      // content only, no templates
    "minimal_site",   // site.toml only
    "template_site",  // templates only, no content
];

/// Builds per site when updating; used to detect unstable files.
const UPDATE_RUNS: usize = 5;

const UNSTABLE: &str = "<unstable>";

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn manifest_path(site: &str) -> PathBuf {
    golden_dir().join(format!("{site}.manifest"))
}

fn is_excluded(relative: &Path) -> bool {
    relative.starts_with("wasm") || relative == Path::new("search_index.bin")
}

/// Blank the body of every `<tag>…</tag>` occurrence.
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

/// Normalise build-time-dependent bytes before hashing.
fn normalise(relative: &Path, bytes: Vec<u8>) -> Vec<u8> {
    let ext = relative.extension().and_then(|e| e.to_str());
    if matches!(ext, Some("xml" | "html"))
        && let Ok(text) = String::from_utf8(bytes.clone())
    {
        let mut text = text;
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
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Walk `output_dir` and hash every file, keyed by `/`-separated relative path.
fn manifest_of(output_dir: &Path) -> BTreeMap<String, String> {
    let mut manifest = BTreeMap::new();
    for entry in walkdir::WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry
            .path()
            .strip_prefix(output_dir)
            .expect("entry is under output_dir");
        if is_excluded(relative) {
            continue;
        }
        let bytes = fs::read(entry.path()).expect("read output file");
        let key = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        manifest.insert(key, sha256_hex(&normalise(relative, bytes)));
    }
    manifest
}

/// Build `site_dir` into a fresh temp directory and return its manifest.
fn build_manifest(site_dir: &Path) -> BTreeMap<String, String> {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let output_dir = tmp.path().join("dist");
    SiteBuilder::from_dir(site_dir)
        .unwrap_or_else(|e| panic!("load {}: {e}", site_dir.display()))
        .output_dir(&output_dir)
        .build()
        .unwrap_or_else(|e| panic!("build {}: {e}", site_dir.display()));
    manifest_of(&output_dir)
}

fn read_manifest(path: &Path) -> BTreeMap<String, String> {
    let text = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "missing golden manifest {} ({e}); run with GOLDEN_UPDATE=1 to create it",
            path.display()
        )
    });
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

fn write_manifest(path: &Path, manifest: &BTreeMap<String, String>) {
    let mut text =
        String::from("# Golden manifest: <sha256 or <unstable>>  <path relative to output dir>\n");
    for (p, h) in manifest {
        text.push_str(h);
        text.push_str("  ");
        text.push_str(p);
        text.push('\n');
    }
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Build several times; files whose hash varies are marked unstable.
fn update_manifest(site_dir: &Path) -> BTreeMap<String, String> {
    let mut merged: BTreeMap<String, String> = BTreeMap::new();
    for run in 0..UPDATE_RUNS {
        let current = build_manifest(site_dir);
        if run == 0 {
            merged = current;
            continue;
        }
        assert_eq!(
            merged.keys().collect::<Vec<_>>(),
            current.keys().collect::<Vec<_>>(),
            "the set of output files changed between runs of {}",
            site_dir.display()
        );
        for (path, hash) in current {
            let existing = merged.get_mut(&path).unwrap();
            if *existing != hash {
                *existing = UNSTABLE.to_owned();
            }
        }
    }
    merged
}

fn check_site(site: &str) {
    let (_, dir) = SITES
        .iter()
        .find(|(name, _)| *name == site)
        .expect("site is listed in SITES");
    let site_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let manifest_file = manifest_path(site);

    if std::env::var_os("GOLDEN_UPDATE").is_some() {
        let manifest = update_manifest(&site_dir);
        write_manifest(&manifest_file, &manifest);
        eprintln!("wrote {}", manifest_file.display());
        return;
    }

    let expected = read_manifest(&manifest_file);
    let actual = build_manifest(&site_dir);

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for (path, hash) in &actual {
        match expected.get(path) {
            None => added.push(path.clone()),
            Some(e) if e == UNSTABLE => {}
            Some(e) if e != hash => {
                changed.push(format!("{path}\n      expected {e}\n      actual   {hash}"))
            }
            Some(_) => {}
        }
    }
    for path in expected.keys() {
        if !actual.contains_key(path) {
            removed.push(path.clone());
        }
    }

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        return;
    }

    let mut report = format!(
        "generated output for `{site}` differs from {}\n",
        manifest_file.display()
    );
    let section = |title: &str, items: &[String]| {
        if items.is_empty() {
            return String::new();
        }
        let mut s = format!("  {title} ({}):\n", items.len());
        for item in items {
            s.push_str("    ");
            s.push_str(item);
            s.push('\n');
        }
        s
    };
    report.push_str(&section("added", &added));
    report.push_str(&section("removed", &removed));
    report.push_str(&section("changed", &changed));
    report.push_str(
        "If this change is intended, re-baseline with GOLDEN_UPDATE=1 in a dedicated commit.\n",
    );
    panic!("{report}");
}

#[test]
fn golden_highlight_site() {
    check_site("highlight_site");
}

#[test]
fn golden_internal_links_site() {
    check_site("internal_links_site");
}

#[test]
fn golden_section_listing_site() {
    check_site("section_listing_site");
}

#[test]
fn golden_search_slug_site() {
    check_site("search_slug_site");
}

#[test]
fn golden_get_taxus_org() {
    check_site("get-taxus-org");
}

/// Every fixture directory must be classified, so a new fixture site gets a
/// golden manifest instead of silently going untested.
#[test]
fn every_fixture_is_classified() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut unclassified = Vec::new();
    for entry in fs::read_dir(&fixtures).unwrap().filter_map(|e| e.ok()) {
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let listed =
            SITES.iter().any(|(s, _)| *s == name) || NOT_BUILDABLE.contains(&name.as_str());
        if !listed {
            unclassified.push(name);
        }
    }
    assert!(
        unclassified.is_empty(),
        "fixtures not listed in SITES or NOT_BUILDABLE in golden_output.rs: {unclassified:?}"
    );
    for (site, _) in SITES {
        assert!(
            manifest_path(site).exists(),
            "no golden manifest for `{site}`; run with GOLDEN_UPDATE=1"
        );
    }
}
