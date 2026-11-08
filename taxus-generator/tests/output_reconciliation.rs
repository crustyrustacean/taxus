//! Regression coverage for stale files left in the generated output directory.
//!
//! These tests describe the output reconciliation contract from #159. They
//! intentionally fail until a build removes files that the current Site Tree
//! no longer produces.

use std::fs;
use std::path::PathBuf;
use taxus::SiteBuilder;
use taxus::config::SiteConfig;
use tempfile::TempDir;

struct TestSite {
    _temp_dir: TempDir,
    site_dir: PathBuf,
    output_dir: PathBuf,
}

impl TestSite {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let site_dir = temp_dir.path().join("site");
        let output_dir = temp_dir.path().join("dist");

        fs::create_dir_all(site_dir.join("content/blog")).unwrap();
        fs::create_dir_all(site_dir.join("templates")).unwrap();
        fs::write(
            site_dir.join("site.toml"),
            r#"[site]
name = "Output reconciliation test"
base_url = "https://example.com"

[build]
output_dir = "dist"
islands = false
"#,
        )
        .unwrap();
        fs::write(
            site_dir.join("templates/page.html"),
            "<html><body>{{ page.content | safe }}</body></html>",
        )
        .unwrap();
        fs::write(
            site_dir.join("templates/section.html"),
            "<html><body>{{ section.content | safe }}</body></html>",
        )
        .unwrap();
        fs::write(
            site_dir.join("content/_index.md"),
            "+++\ntitle = \"Home\"\n+++\nHome\n",
        )
        .unwrap();
        fs::write(
            site_dir.join("content/blog/_index.md"),
            "+++\ntitle = \"Blog\"\n+++\nBlog\n",
        )
        .unwrap();

        Self {
            _temp_dir: temp_dir,
            site_dir,
            output_dir,
        }
    }

    fn write_content(&self, relative: &str, content: &str) {
        let path = self.site_dir.join("content").join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn build(&self, include_drafts: bool) -> taxus::BuildReport {
        let mut config = SiteConfig::from_dir(&self.site_dir).unwrap();
        config.build.output_dir = self.output_dir.clone();
        SiteBuilder::new(config)
            .include_drafts(include_drafts)
            .build()
            .unwrap()
    }
}

fn old_post(alias: Option<&str>, draft: bool) -> String {
    let mut frontmatter = String::from("+++\ntitle = \"Old Post\"\ndate = 2026-01-01\n");
    if let Some(alias) = alias {
        frontmatter.push_str(&format!("aliases = [\"{alias}\"]\n"));
    }
    if draft {
        frontmatter.push_str("draft = true\n");
    }
    format!("{frontmatter}+++\noriginal body\n")
}

#[test]
fn removing_an_alias_removes_its_redirect_page() {
    let site = TestSite::new();
    site.write_content("blog/old-post.md", &old_post(Some("/old-url"), false));

    site.build(false);
    let alias_output = site.output_dir.join("old-url/index.html");
    assert!(alias_output.is_file(), "initial alias was not generated");

    site.write_content("blog/old-post.md", &old_post(None, false));
    site.build(false);

    assert!(
        !alias_output.exists(),
        "removed alias still exists at {}",
        alias_output.display()
    );
}

#[test]
fn deleting_a_page_removes_its_rendered_output() {
    let site = TestSite::new();
    site.write_content("blog/old-post.md", &old_post(None, false));

    site.build(false);
    let page_output = site.output_dir.join("blog/old-post/index.html");
    assert!(page_output.is_file(), "initial page was not generated");

    fs::remove_file(site.site_dir.join("content/blog/old-post.md")).unwrap();
    site.build(false);

    assert!(
        !page_output.exists(),
        "deleted page still exists at {}",
        page_output.display()
    );
}

#[test]
fn newly_drafted_page_is_removed_from_output_when_drafts_are_skipped() {
    let site = TestSite::new();
    site.write_content("blog/secret-post.md", &old_post(None, false));

    site.build(true);
    let page_output = site.output_dir.join("blog/secret-post/index.html");
    assert!(page_output.is_file(), "included draft was not generated");

    site.write_content("blog/secret-post.md", &old_post(None, true));
    let report = site.build(false);

    assert_eq!(report.drafts_skipped, 1);
    assert!(
        !page_output.exists(),
        "newly drafted page still exists at {}",
        page_output.display()
    );
}

#[test]
fn dry_run_does_not_delete_existing_output() {
    let site = TestSite::new();
    let sentinel = site.output_dir.join("stale/index.html");
    fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    fs::write(&sentinel, "keep me").unwrap();

    let mut config = SiteConfig::from_dir(&site.site_dir).unwrap();
    config.build.output_dir = site.output_dir.clone();
    SiteBuilder::new(config).dry_run(true).build().unwrap();

    assert!(sentinel.is_file(), "dry-run removed existing output");
}
