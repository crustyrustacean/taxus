//! Template renderer trait and Tera implementation.
//!
//! This module provides the [`TemplateRenderer`] trait for template rendering
//! and [`TeraRenderer`] as the primary implementation using the Tera template
//! engine.

use crate::error::TemplateError;
use crate::templates::context::TemplateContext;
use std::path::Path;
use tera::{Context, Kwargs, State, Tera, TeraResult, Value};

/// Trait for template rendering backends.
///
/// This trait allows for different template engines to be used
/// interchangeably. The primary implementation is [`TeraRenderer`].
///
/// # Example
///
/// ```no_run
/// use taxus_lib::templates::{TemplateRenderer, TeraRenderer, TemplateContext, SiteContext};
///
/// // Create a renderer
/// let mut renderer = TeraRenderer::new()?;
///
/// // Register a template
/// renderer.register_template("page.html", "<h1>{{ page.title }}</h1>")?;
///
/// // Check if template exists
/// assert!(renderer.has_template("page.html"));
/// # Ok::<(), taxus_lib::error::TemplateError>(())
/// ```
pub trait TemplateRenderer: Send + Sync {
    /// Render a template with the given context.
    ///
    /// # Arguments
    ///
    /// * `template` - Name of the template to render
    /// * `context` - Variables available to the template
    ///
    /// # Returns
    ///
    /// Rendered HTML string or an error.
    fn render(&self, template: &str, context: &TemplateContext) -> Result<String, TemplateError>;

    /// Register a template from a string.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the template
    /// * `content` - Template content
    fn register_template(&mut self, name: &str, content: &str) -> Result<(), TemplateError>;

    /// Check if a template exists.
    ///
    /// # Arguments
    ///
    /// * `name` - Template name to check
    fn has_template(&self, name: &str) -> bool;

    /// Load templates from a directory.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory containing template files (supports `**/*.html` glob)
    fn load_templates(&mut self, dir: &Path) -> Result<(), TemplateError>;
}

/// Tera-based template renderer.
///
/// This is the primary template renderer implementation using the Tera
/// template engine, which provides Jinja2-like syntax.
///
/// # Features
///
/// - Template inheritance (`{% extends "base.html" %}`)
/// - Blocks (`{% block content %}{% endblock %}`)
/// - Loops and conditionals
/// - Filters (including `safe` for unescaped HTML)
/// - Variables from [`TemplateContext`]
///
/// # Example
///
/// ```no_run
/// use taxus_lib::templates::{TeraRenderer, TemplateRenderer, TemplateContext, SiteContext, PageContext};
///
/// // Load templates from directory
/// let renderer = TeraRenderer::from_dir("templates")?;
///
/// // Or create empty and register templates manually
/// let mut renderer = TeraRenderer::new()?;
/// renderer.register_template("page.html", "<h1>{{ page.title }}</h1>")?;
/// # Ok::<(), taxus_lib::error::TemplateError>(())
/// ```
#[derive(Debug)]
pub struct TeraRenderer {
    tera: Tera,
}

impl TeraRenderer {
    /// Create a new empty Tera renderer.
    ///
    /// # Example
    ///
    /// ```
    /// use taxus_lib::templates::TeraRenderer;
    ///
    /// let renderer = TeraRenderer::new();
    /// assert!(renderer.is_ok());
    /// ```
    pub fn new() -> Result<Self, TemplateError> {
        Ok(Self {
            tera: Tera::default(),
        })
    }

    /// Create a Tera renderer and load templates from a directory.
    ///
    /// The directory is searched recursively for `.html` files.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory containing template files
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::templates::TeraRenderer;
    ///
    /// let renderer = TeraRenderer::from_dir("templates");
    /// // Templates are loaded from templates/**/*.html
    /// ```
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, TemplateError> {
        let mut renderer = Self::new()?;
        renderer.load_templates(dir.as_ref())?;
        Ok(renderer)
    }

    /// Convert [`TemplateContext`] to Tera [`Context`].
    ///
    /// This method serializes the context types into the format
    /// expected by Tera's template engine.
    fn to_tera_context(&self, context: &TemplateContext) -> Context {
        let mut ctx = Context::new();

        if let Some(ref page) = context.page {
            ctx.insert("page", page);
        }

        if let Some(ref section) = context.section {
            ctx.insert("section", section);
        }

        ctx.insert("site", &context.site);
        ctx.insert("now", &context.now);
        ctx.insert("extra", &context.extra);

        ctx
    }
}

impl Default for TeraRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to create default TeraRenderer")
    }
}

impl TemplateRenderer for TeraRenderer {
    fn render(&self, template: &str, context: &TemplateContext) -> Result<String, TemplateError> {
        let tera_ctx = self.to_tera_context(context);

        self.tera.render(template, &tera_ctx).map_err(|e| {
            // Check if the error message indicates template not found
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                TemplateError::NotFound(template.to_string())
            } else {
                TemplateError::Render(err_msg)
            }
        })
    }

    fn register_template(&mut self, name: &str, content: &str) -> Result<(), TemplateError> {
        self.tera
            .add_raw_template(name, content)
            .map_err(|e| TemplateError::Syntax {
                template: name.to_string(),
                message: e.to_string(),
            })
    }

    fn has_template(&self, name: &str) -> bool {
        self.tera.get_template_names().any(|n| n == name)
    }

    fn load_templates(&mut self, dir: &Path) -> Result<(), TemplateError> {
        if !dir.exists() {
            return Err(TemplateError::DirNotFound(dir.to_path_buf()));
        }

        use std::fs;
        use walkdir::WalkDir;

        // Create a new empty Tera instance.
        //
        // Tera v2 checks at template-add time that every function/filter/test
        // referenced by a template exists, so the island() function must be
        // registered before any templates are added.
        let mut tera = Tera::default();
        register_island_function(&mut tera);
        register_contrib_filters(&mut tera);

        // Collect all templates first (name -> content)
        let mut templates: Vec<(String, String)> = Vec::new();

        // Walk the templates directory and collect each template
        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process .html files
            if path.extension().is_some_and(|ext| ext == "html") {
                // Get the relative path from the templates directory
                let relative = path
                    .strip_prefix(dir)
                    .map_err(|_| TemplateError::DirNotFound(dir.to_path_buf()))?;

                // Use forward slashes for template names (Tera convention)
                let name = relative.to_string_lossy().replace('\\', "/");

                // Read the template content
                let content = fs::read_to_string(path).map_err(|e| TemplateError::Syntax {
                    template: name.clone(),
                    message: e.to_string(),
                })?;

                templates.push((name, content));
            }
        }

        // Sort templates topologically so every template is registered before
        // the templates that reference it. Tera v2 resolves `{% extends %}`
        // and `{% include %}` targets at template-add time, so a template must
        // be added after everything it references. We parse each template's
        // references out of its content, then repeatedly emit templates whose
        // references are all already emitted (Kahn's algorithm). Any leftover
        // templates form a cycle; they fall back to the original order.
        let ordered = order_templates_by_dependency(templates);

        // Register templates in dependency order
        for (name, content) in ordered {
            tera.add_raw_template(&name, &content)
                .map_err(|e| TemplateError::Syntax {
                    template: name.clone(),
                    message: e.to_string(),
                })?;
        }

        self.tera = tera;
        Ok(())
    }
}
/// The `island()` Tera function (Tera v2 signature).
///
/// Calls Yew SSR to pre-render an island component and wraps the output in a
/// hydration mount point div carrying the serialized props. Templates should
/// apply `| safe` to the call so the HTML is emitted unescaped.
fn island(kwargs: Kwargs, _state: &State) -> TeraResult<Value> {
    let component = kwargs.get::<String>("component")?.unwrap_or_default();

    let html = match component.as_str() {
        "Counter" => {
            use crate::build::pipeline::render_island_counter;
            use taxus_common::components::counter::CounterProps;

            let initial = kwargs.get::<i64>("initial")?.unwrap_or(0) as i32;
            let class = kwargs.get::<String>("class")?.unwrap_or_default();

            render_island_counter(CounterProps { initial, class })
        }
        "SearchBox" => {
            use crate::build::pipeline::render_search_box;
            use taxus_common::components::search_box::SearchBoxProps;

            let placeholder = kwargs
                .get::<String>("placeholder")?
                .unwrap_or_else(|| "Search...".to_string());
            let class = kwargs.get::<String>("class")?.unwrap_or_default();

            render_search_box(SearchBoxProps {
                placeholder,
                max_results: 5,
                class,
            })
        }
        other => format!("<!-- unknown island: {other} -->"),
    };

    Ok(Value::from(html))
}

/// Register the `island()` Tera function on a Tera instance.
///
/// Must be called before any templates referencing `island()` are added —
/// Tera v2 validates function existence at template-add time.
fn register_island_function(tera: &mut Tera) {
    tera.register_function("island", island);
}

/// Register the tera-contrib filters taxus templates rely on.
///
/// Tera v1 shipped `slugify` as a built-in; in v2 it moved to the
/// `tera-contrib` crate as `slug`. We register it under both names so that
/// templates written for either convention keep working.
fn register_contrib_filters(tera: &mut Tera) {
    tera.register_filter("slug", tera_contrib::slug::slug);
    tera.register_filter("slugify", tera_contrib::slug::slug);
    tera.register_filter("date", tera_contrib::dates::date);
}


/// Extract the names of templates that `content` references via
/// `{% extends "..." %}` and `{% include "..." %}`.
fn template_references(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for tag_open in ["{% extends", "{% include"] {
        let mut search_from = 0;
        while let Some(rel_pos) = content[search_from..].find(tag_open) {
            let tag_start = search_from + rel_pos + tag_open.len();
            // Find the first quote-delimited path in the tag.
            let after_tag = &content[tag_start..];
            let quote_start = match after_tag.find(|c| c == '"' || c == '\'') {
                Some(p) => tag_start + p,
                None => break,
            };
            let quote_char = content.as_bytes()[quote_start] as char;
            let path_start = quote_start + 1;
            if let Some(rel_end) = content[path_start..].find(quote_char) {
                refs.push(content[path_start..path_start + rel_end].to_string());
                search_from = path_start + rel_end;
            } else {
                break;
            }
        }
    }
    refs
}

/// Order templates so that each template appears after every template it
/// references via `{% extends %}` or `{% include %}`.
///
/// Tera v2 resolves `extends`/`include` targets at template-add time, so a
/// template must be added after everything it references. This performs a
/// topological sort (Kahn's algorithm); if the dependency graph has a cycle,
/// the remaining templates keep their original relative order.
fn order_templates_by_dependency(templates: Vec<(String, String)>) -> Vec<(String, String)> {
    // Map template name -> names of templates it references (deps within the set).
    let refs: std::collections::HashMap<String, Vec<String>> = templates
        .iter()
        .map(|(name, content)| (name.clone(), template_references(content)))
        .collect();

    let mut emitted: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result: Vec<(String, String)> = Vec::with_capacity(templates.len());
    let mut pending: Vec<(String, String)> = templates;

    // Repeatedly emit templates whose references are all satisfied.
    while !pending.is_empty() {
        let mut progressed = false;
        let mut still_pending = Vec::with_capacity(pending.len());

        for (name, content) in pending {
            let deps = refs.get(&name).cloned().unwrap_or_default();
            // A dependency is satisfied if it's already emitted OR it's not a
            // template we know about (e.g. a built-in or missing one — in that
            // case Tera will report the real error at add time).
            let unsatisfied = deps.iter().any(|d| refs.contains_key(d) && !emitted.contains(d));

            if unsatisfied {
                still_pending.push((name, content));
            } else {
                emitted.insert(name.clone());
                result.push((name, content));
                progressed = true;
            }
        }

        if !progressed {
            // Cycle (or self-reference): append the rest in original order so
            // we still register everything and let Tera surface the error.
            result.extend(still_pending);
            break;
        }

        pending = still_pending;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{PageContext, SectionContext, SiteContext};

    fn create_test_site_context() -> SiteContext {
        SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: Some("A test site".to_string()),
            author: Some("Test Author".to_string()),
        }
    }

    fn create_test_page_context() -> PageContext {
        PageContext {
            title: "Test Page".to_string(),
            description: Some("A test page".to_string()),
            tagline: Some("This is a tagline".to_string()),
            path: "/test/".to_string(),
            permalink: "https://example.com/test/".to_string(),
            content: "<p>Hello World</p>".to_string(),
            raw_content: "Hello World".to_string(),
            date: Some("2024-01-15".to_string()),
            draft: false,
            summary: "A test page summary".to_string(),
            word_count: 2,
            reading_time: 1,
            tags: vec![],
            categories: vec![],
            series: None,
            hero: None,
        }
    }

    fn create_test_section_context() -> SectionContext {
        SectionContext {
            title: "Blog".to_string(),
            description: Some("Blog section description".to_string()),
            path: "/blog/".to_string(),
            content: Some("<p>Welcome to the blog.</p>".to_string()),
            pages: vec![create_test_page_context()],
            pagination: None,
        }
    }

    fn create_test_context() -> TemplateContext {
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context())
    }

    #[test]
    fn test_tera_renderer_new() {
        let renderer = TeraRenderer::new();
        assert!(renderer.is_ok());
    }

    #[test]
    fn test_tera_renderer_default() {
        let renderer = TeraRenderer::default();
        assert!(!renderer.has_template("nonexistent.html"));
    }

    #[test]
    fn test_register_template() {
        let mut renderer = TeraRenderer::new().unwrap();
        let result = renderer.register_template("test.html", "<html>{{ page.title }}</html>");

        assert!(result.is_ok());
        assert!(renderer.has_template("test.html"));
    }

    #[test]
    fn test_register_invalid_template() {
        let mut renderer = TeraRenderer::new().unwrap();
        let result = renderer.register_template("bad.html", "<html>{{ unclosed");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::Syntax { .. }));
    }

    #[test]
    fn test_render_template() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template("test.html", "<h1>{{ page.title }}</h1>")
            .unwrap();

        let ctx = create_test_context();
        let result = renderer.render("test.html", &ctx);

        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("Test Page"));
    }

    #[test]
    fn test_render_with_safe_filter() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template("test.html", "{{ page.content | safe }}")
            .unwrap();

        let ctx = create_test_context();
        let result = renderer.render("test.html", &ctx);

        assert!(result.is_ok());
        let html = result.unwrap();
        // Content should not be escaped
        assert!(html.contains("<p>"));
    }

    #[test]
    fn test_render_missing_template() {
        let renderer = TeraRenderer::new().unwrap();
        let ctx = create_test_context();

        let result = renderer.render("nonexistent.html", &ctx);
        assert!(result.is_err());

        match result.unwrap_err() {
            TemplateError::NotFound(name) => assert_eq!(name, "nonexistent.html"),
            e => panic!("Expected NotFound error, got: {}", e),
        }
    }

    #[test]
    fn test_render_with_site_context() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template("site.html", "<title>{{ site.name }}</title>")
            .unwrap();

        let ctx = create_test_context();
        let result = renderer.render("site.html", &ctx);

        assert!(result.is_ok());
        assert!(result.unwrap().contains("Test Site"));
    }

    #[test]
    fn test_render_with_section_context() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template(
                "section.html",
                r#"<h1>{{ section.title }}</h1>{% for p in section.pages %}<a href="{{ p.path }}">{{ p.title }}</a>{% endfor %}"#,
            )
            .unwrap();

        let ctx = TemplateContext::new(create_test_site_context())
            .with_section(create_test_section_context());
        let result = renderer.render("section.html", &ctx);

        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("<h1>Blog</h1>"));
        assert!(html.contains("Test Page"));
        // Check that the path is rendered (it will be URL-encoded or similar)
        assert!(html.contains("test") || html.contains("/test/"));
    }

    #[test]
    fn test_has_template() {
        let mut renderer = TeraRenderer::new().unwrap();

        assert!(!renderer.has_template("missing.html"));

        renderer
            .register_template("exists.html", "content")
            .unwrap();
        assert!(renderer.has_template("exists.html"));
    }

    #[test]
    fn test_load_templates_missing_directory() {
        let mut renderer = TeraRenderer::new().unwrap();
        let result = renderer.load_templates(Path::new("nonexistent_dir"));

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::DirNotFound(_)));
    }

    #[test]
    fn test_from_dir_missing_directory() {
        let result = TeraRenderer::from_dir("nonexistent_templates");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateError::DirNotFound(_)));
    }

    #[test]
    fn test_template_inheritance() {
        let mut renderer = TeraRenderer::new().unwrap();

        // Register base template
        renderer
            .register_template(
                "base.html",
                r#"<html><head>{% block title %}{% endblock %}</head><body>{% block content %}{% endblock %}</body></html>"#,
            )
            .unwrap();

        // Register child template
        renderer
            .register_template(
                "page.html",
                r#"{% extends "base.html" %}{% block title %}{{ page.title }}{% endblock %}{% block content %}<p>{{ page.content | safe }}</p>{% endblock %}"#,
            )
            .unwrap();

        let ctx = create_test_context();
        let result = renderer.render("page.html", &ctx);

        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("<head>Test Page</head>"));
        assert!(html.contains("<p><p>Hello World</p></p>"));
    }

    #[test]
    fn test_render_with_extra_variables() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template("extra.html", "<div>{{ extra.custom }}</div>")
            .unwrap();

        let mut extra = std::collections::HashMap::new();
        extra.insert(
            "custom".to_string(),
            serde_json::Value::String("Custom Value".to_string()),
        );

        let ctx = TemplateContext::new(create_test_site_context()).with_extra(extra);

        let result = renderer.render("extra.html", &ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Custom Value"));
    }

    #[test]
    fn test_render_with_conditionals() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template(
                "conditional.html",
                r#"{% if page.draft %}<span>Draft</span>{% endif %}<h1>{{ page.title }}</h1>"#,
            )
            .unwrap();

        // Test with draft = false
        let ctx = create_test_context();
        let result = renderer.render("conditional.html", &ctx).unwrap();
        assert!(!result.contains("Draft"));
        assert!(result.contains("Test Page"));

        // Test with draft = true
        let mut page = create_test_page_context();
        page.draft = true;
        let ctx = TemplateContext::new(create_test_site_context()).with_page(page);
        let result = renderer.render("conditional.html", &ctx).unwrap();
        assert!(result.contains("Draft"));
    }

    #[test]
    fn test_render_with_date() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template("date.html", "<time>{{ page.date }}</time>")
            .unwrap();

        let ctx = create_test_context();
        let result = renderer.render("date.html", &ctx);

        assert!(result.is_ok());
        assert!(result.unwrap().contains("2024-01-15"));
    }
}