//! Default template content for site scaffolding.

/// Default templates for new sites.
pub struct DefaultTemplates;

impl DefaultTemplates {
    /// Get the base.html template content.
    pub fn base_html() -> &'static str {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ site.name }}{% endblock %}</title>
    <link rel="stylesheet" href="/css/styles.css">
</head>
<body>
    <header>
        <h1>{{ site.name }}</h1>
        <nav>
            <a href="/">Home</a>
        </nav>
    </header>
    <main>
        {% block content %}{% endblock %}
    </main>
    <footer>
        <p>&copy; {{ "now" | date(format="%Y") }} {{ site.name }}</p>
    </footer>
</body>
</html>
"#
    }

    /// Get the page.html template content.
    pub fn page_html() -> &'static str {
        r#"{% extends "base.html" %}

{% block title %}{{ page.title }} - {{ site.name }}{% endblock %}

{% block content %}
<article>
    <h1>{{ page.title }}</h1>
    {% if page.description %}
    <p class="description">{{ page.description }}</p>
    {% endif %}
    {{ page.content | safe }}
</article>
{% endblock %}
"#
    }

    /// Get the section.html template content.
    pub fn section_html() -> &'static str {
        r#"{% extends "base.html" %}

{% block title %}{{ section.title }} - {{ site.name }}{% endblock %}

{% block content %}
<section>
    <h1>{{ section.title }}</h1>
    {% if section.description %}
    <p class="description">{{ section.description }}</p>
    {% endif %}
    
    {% if section.pages %}
    <ul class="page-list">
        {% for page in section.pages %}
        <li>
            <a href="{{ page.path }}">
                <h2>{{ page.title }}</h2>
                {% if page.description %}
                <p>{{ page.description }}</p>
                {% endif %}
            </a>
        </li>
        {% endfor %}
    </ul>
    {% endif %}
    
    {{ section.content | safe }}
</section>
{% endblock %}
"#
    }

    /// Get the main.scss content.
    pub fn main_scss() -> &'static str {
        r#"// Basic site styles
* {
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    line-height: 1.6;
    margin: 0;
    padding: 0;
}

header {
    background: #f5f5f5;
    padding: 1rem;
    
    h1 {
        margin: 0;
    }
    
    nav {
        margin-top: 0.5rem;
        
        a {
            margin-right: 1rem;
            text-decoration: none;
            color: #0066cc;
            
            &:hover {
                text-decoration: underline;
            }
        }
    }
}

main {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
    
    h1 {
        margin-top: 0;
    }
}

article {
    .description {
        color: #666;
        font-style: italic;
    }
}

.page-list {
    list-style: none;
    padding: 0;
    
    li {
        margin-bottom: 1.5rem;
        padding-bottom: 1.5rem;
        border-bottom: 1px solid #eee;
        
        &:last-child {
            border-bottom: none;
        }
        
        a {
            text-decoration: none;
            color: inherit;
            
            &:hover h2 {
                color: #0066cc;
            }
        }
        
        h2 {
            margin: 0 0 0.5rem 0;
            color: #333;
        }
        
        p {
            margin: 0;
            color: #666;
        }
    }
}

footer {
    text-align: center;
    padding: 1rem;
    background: #f5f5f5;
    margin-top: 2rem;
}
"#
    }

    /// Generate site.toml content with the given name and base URL.
    pub fn site_toml(name: &str, base_url: &str) -> String {
        format!(
            r#"[site]
name = "{}"
base_url = "{}"

[build]
content_dir = "content"
output_dir = "dist"
static_dir = "static"
styles_dir = "styles"
templates_dir = "templates"
"#,
            name, base_url
        )
    }

    /// Generate _index.md content with the given site name.
    pub fn index_md(site_name: &str) -> String {
        format!(
            r#"+++
title = "Home"
description = "Welcome to {}"
+++

# Welcome to {}

This is your new static site. Start editing this file to add your content.

## Getting Started

1. Edit `site.toml` to configure your site settings
2. Add new markdown files in the `content/` directory
3. Customize templates in `templates/`
4. Run `yew-ssg build` to generate your site
"#,
            site_name, site_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_html_not_empty() {
        let content = DefaultTemplates::base_html();
        assert!(!content.is_empty());
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("{% block title %}"));
        assert!(content.contains("{% block content %}"));
    }

    #[test]
    fn test_page_html_not_empty() {
        let content = DefaultTemplates::page_html();
        assert!(!content.is_empty());
        assert!(content.contains("{% extends"));
        assert!(content.contains("{{ page.title }}"));
    }

    #[test]
    fn test_section_html_not_empty() {
        let content = DefaultTemplates::section_html();
        assert!(!content.is_empty());
        assert!(content.contains("{% extends"));
        assert!(content.contains("{{ section.title }}"));
        assert!(content.contains("{% for page in section.pages %}"));
    }

    #[test]
    fn test_main_scss_not_empty() {
        let content = DefaultTemplates::main_scss();
        assert!(!content.is_empty());
        assert!(content.contains("box-sizing"));
        assert!(content.contains("font-family"));
    }

    #[test]
    fn test_site_toml_generation() {
        let content = DefaultTemplates::site_toml("My Site", "https://example.com");

        assert!(content.contains("name = \"My Site\""));
        assert!(content.contains("base_url = \"https://example.com\""));
        assert!(content.contains("[site]"));
        assert!(content.contains("[build]"));
    }

    #[test]
    fn test_index_md_generation() {
        let content = DefaultTemplates::index_md("Test Site");

        assert!(content.contains("+++"));
        assert!(content.contains("title = \"Home\""));
        assert!(content.contains("Test Site"));
        assert!(content.contains("# Welcome to"));
    }
}
