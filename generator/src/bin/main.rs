// generator/src/main/rs

// dependencies
use common::Route;
use pulldown_cmark::{Parser, html::push_html};
use std::fs;
use std::path::Path;
use yew::{ServerRenderer, prelude::*};

#[derive(Properties, PartialEq)]
struct AppProps {
    pub route: Route,
    pub title: String,
    pub content: AttrValue,
}

#[derive(serde::Deserialize)]
struct Frontmatter {
    title: String,
}

fn parse_content(raw: &str) -> (Frontmatter, String) {
    let parts: Vec<&str> = raw.splitn(3, "+++").collect();
    let frontmatter: Frontmatter = toml::from_str(parts[1].trim()).expect("invalid frontmatter");
    let markdown = parts[2].trim();
    (frontmatter, md_to_html(markdown))
}

#[component]
fn App(props: &AppProps) -> Html {
    common::switch(&props.route, props.title.clone(), props.content.clone())
}

fn md_to_html(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();
    push_html(&mut output, parser);
    output
}

fn compile_styles(styles_dir: &Path, dist_dir: &Path) -> std::io::Result<()> {
    let css_dir = dist_dir.join("css");
    fs::create_dir_all(&css_dir)?;

    for entry in fs::read_dir(styles_dir)?.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "scss") {
            let css = grass::from_path(&path, &grass::Options::default())
                .expect("failed to compile SCSS");

            let filename = path.file_stem().unwrap();
            let dest = css_dir.join(format!("{}.css", filename.to_string_lossy()));
            fs::write(&dest, css)?;
            println!("Compiled {}", dest.display());
        }
    }

    Ok(())
}

fn copy_static(static_dir: &Path, dist_dir: &Path) -> std::io::Result<()> {
    if !static_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(static_dir)?.flatten() {
        let path = entry.path();
        let dest = dist_dir.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
            println!("Copied {}", dest.display());
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
            println!("Copied {}", target.display());
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let routes = vec![
        (Route::Home, "dist/index.html", "content/pages/home.md"),
        (
            Route::About,
            "dist/about/index.html",
            "content/pages/about.md",
        ),
    ];

    for (route, path, md_file) in &routes {
        let route = route.clone();
        let raw = fs::read_to_string(md_file)?;
        let (frontmatter, content_html) = parse_content(&raw);

        let html = ServerRenderer::<App>::with_props(move || AppProps {
            route,
            title: frontmatter.title,
            content: AttrValue::from(content_html),
        })
        .render()
        .await;

        let template = fs::read_to_string("templates/index.txt")?;
        let body_content = template.replace("{body}", &html);

        let dest = std::path::Path::new(path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, body_content.as_bytes())?;
        println!("Output written to {path}");
    }

    compile_styles(Path::new("styles"), Path::new("dist"))?;
    copy_static(Path::new("static"), Path::new("dist"))?;

    Ok(())
}
