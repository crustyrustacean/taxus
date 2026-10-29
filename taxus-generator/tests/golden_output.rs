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
//! `GOLDEN_UPDATE` takes exactly two values. `1` re-baselines; `0` and
//! unset verify; anything else is an error. In particular `GOLDEN_UPDATE=0`
//! **verifies** — it must never re-baseline, because a run that rewrites its
//! own expectation cannot fail, and a green result would then say nothing
//! about the committed baseline. [`mode_from_var`] owns that rule and
//! `golden_update_zero_verifies_instead_of_rewriting` pins it end to end.
//!
//! # What is and is not covered
//!
//! - `wasm/*` and `search_index.bin` are excluded: they depend on the
//!   embedded client build, not on the generator's content pipeline.
//! - In `*.xml` feed files the `<lastBuildDate>`, `<pubDate>`, `<updated>`
//!   and `<published>` element bodies are blanked before hashing: feeds
//!   stamp the build time, and undated pages get the build time as their
//!   publication date.
//! - Text outputs (`html`, `xml`, `txt`, `svg`, `css`, `js`, `json`) are
//!   hashed with `\r\n` folded to `\n`. A git checkout with
//!   `core.autocrlf=true` (the Windows default) gives every template,
//!   content file and static asset CRLF endings, which the build copies
//!   through verbatim; the manifests were recorded from LF sources. The
//!   `crlf_sources_build_to_the_same_manifest` test pins this.
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
    ("feed_slug_site", "tests/fixtures/feed_slug_site"),
    // flatten_site pins `[build] flatten = ["blog/*"]`: posts stored in
    // blog/<year>/ and blog/drafts/ are served at /blog/<slug>/, while
    // co-located assets keep their storage path (blog/2026/diagram.png).
    ("flatten_site", "tests/fixtures/flatten_site"),
    // no_islands_site pins #56: islands = false, search = false — the
    // build must skip dist/wasm/ and dist/search_index.bin entirely.
    ("no_islands_site", "tests/fixtures/no_islands_site"),
    ("highlight_site", "tests/fixtures/highlight_site"),
    // highlight_web_site pins the web grammars: javascript, css, html
    // with its script/style injections, a style= attribute injection,
    // the js alias, and the unknown-language fallback.
    ("highlight_web_site", "tests/fixtures/highlight_web_site"),
    ("internal_links_site", "tests/fixtures/internal_links_site"),
    ("search_slug_site", "tests/fixtures/search_slug_site"),
    (
        "section_listing_site",
        "tests/fixtures/section_listing_site",
    ),
    // shortcode_site pins the shortcode engine: built-ins (image with
    // an @/ ref, youtube), a custom block shortcode with args, and
    // code-construct immunity for both fenced and inline uses.
    ("shortcode_site", "tests/fixtures/shortcode_site"),
    ("term_slug_site", "tests/fixtures/term_slug_site"),
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

/// What the harness does with a manifest: compare against it, or rewrite it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Compare generated output against the committed manifest.
    Verify,
    /// Rewrite the committed manifest from generated output.
    Update,
}

/// Resolve [`Mode`] from a `GOLDEN_UPDATE` lookup.
///
/// Only `1` re-baselines and only `0` or an unset variable verifies.
/// Everything else panics: a typo such as `GOLDEN_UPDATE=true` must be a
/// loud error, never a silent re-baseline. This test's guarantee is that a
/// green run compared the build against a reviewed baseline, so a mode the
/// harness only half-understands is worse than a failure.
///
/// Takes the lookup result rather than reading the environment so the
/// decision is testable in-process; [`mode`] supplies the real environment.
fn mode_from_var(value: Result<String, std::env::VarError>) -> Mode {
    match value.as_deref() {
        Ok("1") => Mode::Update,
        Ok("0") | Err(std::env::VarError::NotPresent) => Mode::Verify,
        Ok(other) => panic!(
            "GOLDEN_UPDATE={other:?} is not a recognised value; \
             use `GOLDEN_UPDATE=1` to re-baseline or `GOLDEN_UPDATE=0` (or unset) to verify"
        ),
        Err(e) => panic!("GOLDEN_UPDATE is not valid unicode: {e}"),
    }
}

/// [`Mode`] for this process, from the real environment.
fn mode() -> Mode {
    mode_from_var(std::env::var("GOLDEN_UPDATE"))
}

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

/// Output files whose bytes are text: hashed with CRLF folded to LF.
const TEXT_EXTENSIONS: &[&str] = &["html", "xml", "txt", "svg", "css", "js", "json"];

/// Normalise build-time-dependent and platform-dependent bytes before
/// hashing: feed timestamps, and CRLF line endings that a Windows checkout
/// introduces into every text source.
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

    if mode() == Mode::Update {
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

/// A site whose sources have CRLF endings — what `core.autocrlf=true`
/// checks out on Windows — builds to the committed manifest.
#[test]
fn crlf_sources_build_to_the_same_manifest() {
    const SOURCE_TEXT: &[&str] = &[
        "md", "html", "toml", "txt", "scss", "css", "js", "svg", "xml",
    ];
    let site = "internal_links_site";
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/internal_links_site");
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let copy = tmp.path().join(site);
    let mut converted = 0;
    for entry in walkdir::WalkDir::new(&source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(&source).unwrap();
        // The fixture's own build output is not a source.
        if relative.starts_with("dist") {
            continue;
        }
        let target = copy.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let bytes = fs::read(entry.path()).unwrap();
        let ext = relative.extension().and_then(|e| e.to_str());
        if ext.is_some_and(|e| SOURCE_TEXT.contains(&e)) {
            let text = String::from_utf8(bytes).expect("text source is UTF-8");
            fs::write(&target, text.replace("\r\n", "\n").replace('\n', "\r\n")).unwrap();
            converted += 1;
        } else {
            fs::write(&target, bytes).unwrap();
        }
    }
    assert!(converted > 0, "fixture has text sources to convert");

    let expected = read_manifest(&manifest_path(site));
    let actual = build_manifest(&copy);
    let differing: Vec<&String> = expected
        .iter()
        .filter(|(path, hash)| *hash != UNSTABLE && actual.get(*path) != Some(hash))
        .map(|(path, _)| path)
        .collect();
    assert!(
        differing.is_empty(),
        "CRLF sources changed the manifest for: {differing:?}"
    );
    assert_eq!(actual.len(), expected.len(), "file set differs");
}

#[test]
fn golden_no_islands_site() {
    check_site("no_islands_site");
}

#[test]
fn golden_highlight_site() {
    check_site("highlight_site");
}

#[test]
fn golden_highlight_web_site() {
    check_site("highlight_web_site");
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
fn golden_feed_slug_site() {
    check_site("feed_slug_site");
}

#[test]
fn golden_search_slug_site() {
    check_site("search_slug_site");
}

#[test]
fn golden_term_slug_site() {
    check_site("term_slug_site");
}

#[test]
fn golden_flatten_site() {
    check_site("flatten_site");
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

#[test]
fn golden_shortcode_site() {
    check_site("shortcode_site");
}

/// `GOLDEN_UPDATE` is read per-process, so its off-state has to be pinned
/// here: a falsy value must verify, not re-baseline. `var_os(..).is_some()`
/// matched `"0"`, which silently rewrote the manifests and passed.
#[test]
fn golden_update_only_accepts_one() {
    use std::env::VarError;

    assert_eq!(mode_from_var(Ok("1".into())), Mode::Update);
    assert_eq!(mode_from_var(Ok("0".into())), Mode::Verify);
    assert_eq!(mode_from_var(Err(VarError::NotPresent)), Mode::Verify);
}

/// A value the harness does not recognise must fail loudly rather than
/// quietly rewriting the baseline. Both strings here are ones a person
/// reaches for intending "on".
#[test]
fn golden_update_rejects_unrecognised_values() {
    for value in ["true", "yes", ""] {
        let result = std::panic::catch_unwind(|| mode_from_var(Ok(value.to_owned())));
        assert!(
            result.is_err(),
            "GOLDEN_UPDATE={value:?} must panic, not pick a mode"
        );
    }
}

/// The end-to-end guard: with `GOLDEN_UPDATE=0` the test binary must take
/// the verify path. Run as a subprocess, because the harness reads the
/// variable from the process environment, which a test cannot safely
/// mutate for its own siblings.
///
/// Update mode prints `wrote <manifest>` for every site and never compares;
/// verify mode never prints it. So the presence of that line is a direct
/// signal of which branch ran, and it holds whether or not the manifest is
/// currently in sync — unlike a byte comparison, which would only notice a
/// rewrite if the content had actually changed.
#[test]
fn golden_update_zero_verifies_instead_of_rewriting() {
    let exe = std::env::current_exe().expect("test binary path");
    // A fixture whose manifest is in sync: the run must succeed, which
    // proves the verify branch compared rather than bailing out.
    let site = "no_islands_site";
    let before = fs::read(manifest_path(site)).expect("read manifest");

    let output = std::process::Command::new(&exe)
        .args([&format!("golden_{site}"), "--exact", "--nocapture"])
        .env("GOLDEN_UPDATE", "0")
        .output()
        .expect("run test binary");

    let after = fs::read(manifest_path(site)).expect("read manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("wrote "),
        "GOLDEN_UPDATE=0 took the update path (printed `wrote`):\n{stderr}"
    );
    assert_eq!(
        before, after,
        "GOLDEN_UPDATE=0 rewrote the manifest; it must only verify"
    );
    assert!(
        output.status.success(),
        "GOLDEN_UPDATE=0 must verify an in-sync manifest successfully:\n{stderr}"
    );
}
