// generator/src/init/manifest.rs
//
//! The scaffold manifest: the single list of what `taxus init` creates.
//!
//! # Overview
//!
//! Every file the scaffolder writes is named here exactly once. The
//! scaffolder walks this list to create files; `taxus init --help` walks the
//! same list to describe them. Nothing else in the tree may name a scaffolded
//! file — that duplication is the drift class this module exists to close
//! (#111). Adding a template is one row in [`TEMPLATES`]; the help text
//! follows automatically.
//!
//! File *contents* live beside this module as real files, under
//! `init/templates/` for Tera and `init/styles/` for SCSS, and are read with
//! `include_str!`. They are ordinary text files: lintable, syntax-highlighted,
//! and editable without touching Rust. The stylesheets already worked this way;
//! the templates now do too.
//!
//! # Placeholders
//!
//! A template that varies with the [`islands`](super::InitOptions::islands)
//! flag carries a `{{ placeholder }}` token in the file on disk, and its
//! manifest row names the [`Variant`] that supplies the text. The token is
//! removed after substitution, so a `--no-islands` site receives no empty
//! island markup at all.
//!
//! Substitution touches only the tokens a row declares. A template's own Tera
//! — `{{ site.name }}`, `{% for page in section.pages %}` — is left exactly as
//! written. That is what lets the files on disk be real Tera rather than
//! `format!` strings with every literal brace doubled.
//!
//! ```
//! use taxus_lib::init::manifest::{strip, substitute, Variants};
//!
//! // A claimed token is replaced with its variant's text...
//! let out = substitute("{{ island_demo }}", &Variants::ISLAND_DEMO);
//! assert!(out.contains("Counter"));
//!
//! // ...or removed outright. Either way the scaffold placeholder goes, while
//! // the template's own Tera is left exactly as written.
//! let out = strip(
//!     "<h1>{{ site.name }}</h1>{{ island_demo }}",
//!     &Variants::ISLAND_DEMO,
//! );
//! assert_eq!(out, "<h1>{{ site.name }}</h1>");
//!
//! ```

use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// The text that replaces one named placeholder.
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    /// The placeholder this supplies, without braces: `wasm_script`.
    pub placeholder: &'static str,
    /// The text written in its place, newlines included.
    pub text: &'static str,
}

/// The substitutions to apply to one scaffolded file.
///
/// A file's manifest row names the [`Variant`]s its placeholders resolve to.
/// [`Variants::NONE`] is the common case: a file with no placeholders, written
/// exactly as it is on disk.
#[derive(Debug, Clone, Copy)]
pub struct Variants(pub &'static [Variant]);

impl Variants {
    /// No placeholders: the file is written byte for byte as it is on disk.
    pub const NONE: Variants = Variants(&[]);

    /// `{{ wasm_script }}` in `base.html`: the WASM hydration bootstrap.
    ///
    /// The text carries its own leading and trailing newline, so the template
    /// can place the token mid-line and still produce well-formed markup.
    pub const WASM_SCRIPT: Variants = Variants(&[Variant {
        placeholder: "wasm_script",
        text: r#"
    <!-- WASM hydration client.
         client.js is a wasm-bindgen ES module; it must be loaded via
         `import init` inside a type="module" script, not via a plain src= tag. -->
    <script type="module">
        import init, * as bindings from '/wasm/client.js';
        const wasm = await init({ module_or_path: '/wasm/client_bg.wasm' });
        window.wasmBindings = bindings;
        bindings.hydrate_islands();
    </script>
"#,
    }]);

    /// `{{ island_demo }}` in `section.html`: a `Counter` island in a comment.
    pub const ISLAND_DEMO: Variants = Variants(&[Variant {
        placeholder: "island_demo",
        text: r#"
{# Place a Counter island on the page #}
{{ island(component="Counter", initial=3) | safe }}
"#,
    }]);
}

/// One scaffolded template: what it is called, what it is for, and how it
/// varies.
pub struct Template {
    /// File name, relative to `templates/` in the new site.
    pub name: &'static str,
    /// One-line description, shown in `taxus init --help`.
    pub description: &'static str,
    /// The file's text, as written in `init/templates/`.
    pub contents: &'static str,
    /// Placeholders to resolve before writing.
    pub variants: Variants,
}

/// Every template `taxus init` writes, in write order.
///
/// This is the list. The scaffolder iterates it and the help text renders it;
/// neither keeps a copy.
pub const TEMPLATES: &[Template] = &[
    Template {
        name: "base.html",
        description: "base HTML layout",
        contents: include_str!("templates/base.html"),
        variants: Variants::WASM_SCRIPT,
    },
    Template {
        name: "page.html",
        description: "single-page template",
        contents: include_str!("templates/page.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "section.html",
        description: "section/listing template",
        contents: include_str!("templates/section.html"),
        variants: Variants::ISLAND_DEMO,
    },
    Template {
        name: "404.html",
        description: "not-found page",
        contents: include_str!("templates/404.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "tags.html",
        description: "tag index",
        contents: include_str!("templates/tags.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "categories.html",
        description: "category index",
        contents: include_str!("templates/categories.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "series.html",
        description: "series index",
        contents: include_str!("templates/series.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "tags_term.html",
        description: "single tag page",
        contents: include_str!("templates/tags_term.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "categories_term.html",
        description: "single category page",
        contents: include_str!("templates/categories_term.html"),
        variants: Variants::NONE,
    },
    Template {
        name: "series_term.html",
        description: "single series page",
        contents: include_str!("templates/series_term.html"),
        variants: Variants::NONE,
    },
];

/// Every directory the scaffold creates, as `(name, description)`.
pub const DIRECTORIES: &[(&str, &str)] = &[
    ("content", "Markdown content and co-located assets"),
    ("templates", "Tera templates"),
    ("static", "files copied verbatim into the output"),
    ("styles", "SCSS entrypoint and highlighting themes"),
];

/// The scaffolded files that are not templates, as
/// `(path relative to the site root, description)`.
///
/// The templates come from [`TEMPLATES`] instead, since they need their
/// contents and variants. Together the two lists are every file `taxus init`
/// writes.
pub const OTHER_FILES: &[(&str, &str)] = &[
    ("site.toml", "site configuration"),
    ("content/_index.md", "home page content"),
    ("styles/main.scss", "starter stylesheet"),
    (
        "styles/_highlight-dark.scss",
        "dark syntax highlighting theme",
    ),
    (
        "styles/_highlight-light.scss",
        "light syntax highlighting theme",
    ),
    ("static/scripts.js", "placeholder scripts file"),
    ("static/favicon.png", "placeholder favicon"),
];

/// Every file the scaffold writes, as `(path, description)` pairs, in help-text
/// order.
///
/// Paths are relative to the new site's root, so this doubles as the help
/// table: the `OTHER_FILES` rows already carry their prefix (`site.toml`,
/// `content/_index.md`, `static/scripts.js`), and the templates get theirs here
/// so every row reads as a real output path. `taxus init --help` renders this,
/// so the help and the scaffolder cannot drift apart (#111).
///
/// Returns owned paths because the prefix has to be concatenated; this is
/// called once to build a help string, so the allocation does not matter.
///
/// ```
/// let rows = taxus_lib::init::manifest::layout();
/// assert_eq!(rows[0].0, "site.toml");
/// assert!(rows.iter().any(|(path, _)| path == "templates/base.html"));
/// assert!(rows.iter().all(|(_, description)| !description.is_empty()));
/// ```
pub fn layout() -> Vec<(String, &'static str)> {
    let mut rows: Vec<(String, &'static str)> = OTHER_FILES
        .iter()
        .map(|(path, description)| ((*path).to_string(), *description))
        .collect();
    for template in TEMPLATES {
        rows.push((format!("templates/{}", template.name), template.description));
    }
    rows
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// A template's contents with its placeholders resolved for one scaffold.
///
/// `islands` decides what each placeholder gets: its [`Variant`] text when on,
/// nothing at all when off. The tokens are removed in both cases, so a
/// `--no-islands` site has no leftover `{{ … }}` for Tera to print literally.
///
/// Borrows when there is nothing to do, which is the case for eight of the ten
/// templates and for every scaffold of a template without variants.
pub fn resolve(template: &Template, islands: bool) -> Cow<'_, str> {
    if template.variants.0.is_empty() {
        return Cow::Borrowed(template.contents);
    }
    if islands {
        Cow::Owned(substitute(template.contents, &template.variants))
    } else {
        Cow::Owned(strip(template.contents, &template.variants))
    }
}

/// Replace a file's placeholders with the text of the matching [`Variant`].
///
/// A token is the literal text `{{ name }}` where `name` is one of the
/// placeholder names in `variants`. Any other `{{ … }}` is left exactly as
/// written, so the scaffold's own Tera passes through untouched. Tera *tags* —
/// `{% … %}` — are never touched at all.
///
/// Substitution is single-pass: a [`Variant`] whose text itself contains
/// `{{ … }}` (the `island()` call in the demo, for instance) is inserted as
/// written and not rescanned.
pub fn substitute(contents: &str, variants: &Variants) -> String {
    rewrite(contents, variants, Mode::Substitute)
}

/// Remove a file's placeholders without substituting anything.
///
/// The `--no-islands` path: the token disappears, so the surrounding
/// conditional markup goes with it. Used by [`resolve`].
pub fn strip(contents: &str, variants: &Variants) -> String {
    rewrite(contents, variants, Mode::Strip)
}

/// What [`rewrite`] does with a token it recognises.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Replace the token with its [`Variant`] text.
    Substitute,
    /// Remove the token and leave nothing behind.
    Strip,
}

/// Walk a file's `{{ name }}` tokens, replacing or removing the ones the row
/// claims and copying the rest across byte for byte.
///
/// One pass, for both [`substitute`] and [`strip`]: the two differ only in
/// what happens to a claimed token, and everything else about the walk is
/// identical. Two copies of this loop would be two chances to get the token
/// arithmetic wrong in one of them.
fn rewrite(contents: &str, variants: &Variants, mode: Mode) -> String {
    if variants.0.is_empty() {
        return contents.to_string();
    }

    let mut out = String::with_capacity(contents.len());
    let mut rest = contents;

    // Find the next `{{`, then the `}}` that closes it.
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            // No closing braces anywhere ahead: what remains is not a token,
            // and copying it out is the whole job.
            break;
        };
        let name = after[..close].trim();
        let claimed = variants.0.iter().find(|v| v.placeholder == name);

        // Everything ahead of the token is copied across in every case; the
        // match only decides the token's own fate.
        out.push_str(&rest[..open]);
        match (mode, claimed) {
            (Mode::Substitute, Some(variant)) => out.push_str(variant.text),
            (Mode::Strip, Some(_)) => {}
            // Not ours. Copy the whole token — braces and body — so a Tera
            // expression like `{{ site.name }}` reaches the user intact.
            _ => out.push_str(&rest[open..open + 2 + close + 2]),
        }

        // Resume *after* the token in every case. That is what keeps the pass
        // single: a `{{ … }}` inside a variant's own text was already consumed
        // as part of the token we replaced, so it is never rescanned.
        rest = &after[close + 2..];
    }

    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `{{ name }}` token in a file, in order.
    fn tokens(source: &str) -> Vec<&str> {
        let mut found = Vec::new();
        let mut rest = source;
        while let Some(open) = rest.find("{{") {
            let after = &rest[open + 2..];
            let Some(close) = after.find("}}") else {
                break;
            };
            found.push(after[..close].trim());
            rest = &after[close + 2..];
        }
        found
    }

    #[test]
    fn substitute_resolves_a_known_placeholder() {
        let out = substitute(
            "<a>\n{{ island_demo }}{% endblock %}\n",
            &Variants::ISLAND_DEMO,
        );
        assert!(out.contains("Counter"), "{out}");
        assert!(out.ends_with("{% endblock %}\n"), "{out}");
    }

    #[test]
    fn substitute_leaves_tera_expressions_alone() {
        let source = "{% block title %}{{ site.name }}{% endblock %}";
        assert_eq!(substitute(source, &Variants::NONE), source);
        // ...and also when a variant is in play, since a row may declare one
        // placeholder while the file is full of ordinary Tera.
        assert_eq!(substitute(source, &Variants::WASM_SCRIPT), source);
    }

    #[test]
    fn substitute_is_a_no_op_without_variants() {
        for template in TEMPLATES {
            assert_eq!(
                substitute(template.contents, &Variants::NONE),
                template.contents,
                "{} changed with no variants declared",
                template.name
            );
        }
    }

    #[test]
    fn substitute_does_not_rescan_its_own_output() {
        // The demo variant's text contains `{{ island(…) }}`. A second pass
        // would see that token and mangle it; one pass must not.
        let once = substitute("{{ island_demo }}", &Variants::ISLAND_DEMO);
        let twice = substitute(&once, &Variants::ISLAND_DEMO);
        assert_eq!(once, twice);
        assert!(once.contains("{{ island(component=\"Counter\", initial=3) | safe }}"));
    }

    #[test]
    fn strip_removes_the_token_and_leaves_tera() {
        let out = strip(
            "a\n{{ wasm_script }}\n{{ site.name }}\n",
            &Variants::WASM_SCRIPT,
        );
        assert!(!out.contains("wasm_script"), "{out}");
        assert!(out.contains("{{ site.name }}"), "{out}");
    }

    #[test]
    fn strip_is_a_no_op_without_variants() {
        for template in TEMPLATES {
            assert_eq!(
                strip(template.contents, &Variants::NONE),
                template.contents,
                "{} changed with no variants declared",
                template.name
            );
        }
    }

    #[test]
    fn resolve_borrows_when_there_is_nothing_to_do() {
        // Eight of ten templates have no placeholders at all. Copying them
        // would be pure waste on every `taxus init`.
        for template in TEMPLATES {
            if template.variants.0.is_empty() {
                assert!(
                    matches!(resolve(template, true), Cow::Borrowed(_)),
                    "{} should not be copied",
                    template.name
                );
            }
        }
    }

    #[test]
    fn no_placeholder_survives_either_way() {
        // The failure this guards: a `{{ island_demo }}` left in a scaffold
        // would be printed literally by Tera, and a `{{ wasm_script }}` in a
        // --no-islands site would do the same.
        for template in TEMPLATES {
            for islands in [true, false] {
                let out = resolve(template, islands);
                for variant in template.variants.0 {
                    assert!(
                        !out.contains(&format!("{{{{ {} }}}}", variant.placeholder)),
                        "{} left {{{{ {} }}}} behind (islands={islands})",
                        template.name,
                        variant.placeholder
                    );
                }
            }
        }
    }

    #[test]
    fn no_islands_scaffold_has_no_island_markup() {
        let base = TEMPLATES
            .iter()
            .find(|t| t.name == "base.html")
            .expect("base.html is in the manifest");
        let section = TEMPLATES
            .iter()
            .find(|t| t.name == "section.html")
            .expect("section.html is in the manifest");

        let plain_base = resolve(base, false);
        assert!(!plain_base.contains("hydrate_islands"), "{plain_base}");
        assert!(!plain_base.contains("type=\"module\""), "{plain_base}");

        let plain_section = resolve(section, false);
        assert!(!plain_section.contains("island("), "{plain_section}");
    }

    #[test]
    fn every_declared_placeholder_occurs_in_its_file() {
        // Guards a manifest row that names a placeholder the file lacks: it
        // would compile, pass every other test, and do nothing at runtime.
        for template in TEMPLATES {
            let found = tokens(template.contents);
            for variant in template.variants.0 {
                assert!(
                    found.contains(&variant.placeholder),
                    "{} declares {{{{{}}}}} but the file does not contain it",
                    template.name,
                    variant.placeholder
                );
            }
        }
    }

    #[test]
    fn no_unclaimed_placeholder_is_left_in_a_template() {
        // A `{{ foo }}` in a template that no variant claims is almost
        // certainly a typo in the placeholder's name: it would reach the user
        // and be printed literally by Tera. The exception is a token that is
        // itself a Tera call or a string, which is real template content.
        for template in TEMPLATES {
            let claimed: Vec<&str> = template.variants.0.iter().map(|v| v.placeholder).collect();
            for token in tokens(template.contents) {
                if claimed.contains(&token) {
                    continue;
                }
                assert!(
                    !token.contains('(') && !token.contains('"') && !token.contains('\''),
                    "{} has an unclaimed placeholder {{{{ {token} }}}}",
                    template.name
                );
            }
        }
    }

    #[test]
    fn manifest_names_are_unique() {
        let mut names: Vec<&str> = TEMPLATES.iter().map(|t| t.name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate template name in TEMPLATES");
    }

    #[test]
    fn manifest_entries_have_descriptions() {
        for template in TEMPLATES {
            assert!(
                !template.description.trim().is_empty(),
                "{} has no description for the help text",
                template.name
            );
        }
        for (_, description) in OTHER_FILES.iter().chain(DIRECTORIES) {
            assert!(!description.trim().is_empty());
        }
    }

    #[test]
    fn manifest_templates_are_plain_names_under_templates() {
        for template in TEMPLATES {
            assert!(
                template.name.ends_with(".html"),
                "{} should be a .html template",
                template.name
            );
            assert!(!template.name.contains('/'), "{}", template.name);
        }
    }

    #[test]
    fn scaffolded_paths_do_not_collide() {
        // A row in TEMPLATES and a row in OTHER_FILES naming the same output
        // path would mean one silently overwrites the other.
        let mut paths: Vec<String> = TEMPLATES
            .iter()
            .map(|t| format!("templates/{}", t.name))
            .chain(OTHER_FILES.iter().map(|(path, _)| (*path).to_string()))
            .collect();
        let count = paths.len();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), count, "two manifest rows write the same path");
    }
}
