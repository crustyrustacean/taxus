// taxus-generator/src/highlighting/engine.rs

//! The highlighter: runs tree-sitter queries and emits `<span class="hl-…">` markup (emit phase).

use super::languages::LanguageRegistry;
use std::collections::HashMap;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "function",
    "function.builtin",
    "function.macro",
    "keyword",
    "label",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

pub enum HighlightResult {
    /// Successfully highlighted, contains HTML with `<span>` tags
    Highlighted(String),
    /// Language not in registry, contains HTML-escaped plain text
    Unsupported(String),
}

pub struct CodeHighlighter {
    registry: LanguageRegistry,
    highlighter: Highlighter,
    configs: HashMap<&'static str, HighlightConfiguration>,
    class_prefix: String,
}

impl CodeHighlighter {
    pub fn new(registry: LanguageRegistry, class_prefix: &str) -> Self {
        let highlighter = Highlighter::new();
        let mut configs = std::collections::HashMap::new();

        for (name, spec) in registry.iter() {
            let mut config = HighlightConfiguration::new(
                spec.language.clone(),
                spec.name,
                spec.highlight_query,
                spec.injection_query.unwrap_or(""),
                spec.locals_query.unwrap_or(""),
            )
            .unwrap_or_else(|_| panic!("Failed to create highlight config for {}", name));

            config.configure(HIGHLIGHT_NAMES);

            configs.insert(*name, config);
        }

        Self {
            registry,
            highlighter,
            configs,
            class_prefix: class_prefix.to_string(),
        }
    }

    pub fn highlight(&mut self, code: &str, language: &str) -> HighlightResult {
        // Look up the canonical name (handles aliases like "rs" -> "rust")
        let canonical = self.registry.canonical_name(language);

        let config = match canonical.and_then(|name| self.configs.get(name)) {
            Some(config) => config,
            None => return HighlightResult::Unsupported(escape_html(code)),
        };

        // The injection callback resolves an embedded language by name
        // against the same config map. Without it, injections are
        // silently skipped: html's injections.scm asks for "javascript"
        // and "css" and would be handed `None` every time. (Rust's
        // macro injections ask for "rust" inside "rust", so this path
        // was never exercised until a cross-language injection existed.)
        let events = match self
            .highlighter
            .highlight(config, code.as_bytes(), None, |name| self.configs.get(name))
        {
            Ok(events) => events,
            Err(_) => return HighlightResult::Unsupported(escape_html(code)),
        };

        let mut output = String::with_capacity(code.len() * 2);

        for event in events {
            match event {
                Ok(HighlightEvent::Source { start, end }) => {
                    output.push_str(&escape_html(&code[start..end]));
                }
                Ok(HighlightEvent::HighlightStart(highlight)) => {
                    let scope = HIGHLIGHT_NAMES[highlight.0];
                    let class = scope.replace('.', "-");
                    output.push_str(&format!("<span class=\"{}{}\">", self.class_prefix, class));
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    output.push_str("</span>");
                }
                Err(_) => {
                    return HighlightResult::Unsupported(escape_html(code));
                }
            }
        }

        HighlightResult::Highlighted(output)
    }
}

/// Escape characters with special meaning in HTML text content.
///
/// The single shared implementation for the crate (markdown.rs's copy was
/// consolidated here, #15). The two former copies differed only in the
/// apostrophe entity (`&#39;` vs `&#x27;`) — numerically identical
/// codepoints, no behavioral difference.
pub(crate) fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_let_binding() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let result = highlighter.highlight("let x: u32 = 42;", "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'let' as keyword"
                );
                assert!(
                    html.contains("hl-type-builtin") || html.contains("hl-type"),
                    "should highlight 'u32' as type"
                );
                assert!(
                    html.contains("hl-constant-builtin") || html.contains("hl-number"),
                    "should highlight '42' as constant or number"
                );
                assert!(
                    !html.contains("<script>"),
                    "should not contain unescaped HTML"
                );
            }
            HighlightResult::Unsupported(_) => {
                panic!("Rust should be supported when lang-rust feature is enabled");
            }
        }
    }

    #[test]
    fn test_highlight_rust_function_definition() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r#"fn greet(name: &str) -> String {
    format!("Hello, {}", name)
}"#;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(html.contains("hl-keyword"));
                assert!(html.contains("hl-function"));
                assert!(html.contains("hl-variable-parameter"));
                assert!(html.contains("hl-type-builtin"));
                assert!(html.contains("hl-type"));
                assert!(html.contains("hl-function-macro"));
                assert!(html.contains("hl-string"));
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_rust_lifetimes() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r#"fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}"#;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(
                    html.contains("hl-label"),
                    "should highlight lifetime as label"
                );
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'fn' and 'if' as keywords"
                );
                assert!(
                    html.contains("hl-function"),
                    "should highlight 'longest' and 'len' as functions"
                );
                assert!(
                    html.contains("hl-type-builtin"),
                    "should highlight 'str' as builtin type"
                );
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_rust_attributes() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r#"#[derive(Debug, Clone)]
#[cfg(feature = "islands")]
pub struct Config {
    pub name: String,
}"#;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(html.contains("hl-attribute"), "should highlight attributes");
                assert!(
                    html.contains("hl-constructor"),
                    "should highlight derive traits as constructors"
                );
                assert!(
                    html.contains("hl-string"),
                    "should highlight feature string"
                );
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'pub' and 'struct' as keywords"
                );
                assert!(
                    html.contains("hl-type"),
                    "should highlight 'Config' and 'String' as types"
                );
                assert!(
                    html.contains("hl-property"),
                    "should highlight 'name' as property"
                );
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_rust_turbofish() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r#"let x = "42".parse::<u32>().unwrap();
let v = Vec::<i32>::new();"#;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'let' as keyword"
                );
                assert!(
                    html.contains("hl-function"),
                    "should highlight 'parse', 'unwrap', 'new' as functions"
                );
                assert!(
                    html.contains("hl-type-builtin"),
                    "should highlight 'u32' and 'i32' as builtin types"
                );
                assert!(html.contains("hl-type"), "should highlight 'Vec' as type");
                assert!(html.contains("hl-string"), "should highlight '42' string");
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_rust_closures_and_async() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r#"let add = |a, b| a + b;
let result = add(2, 3);

async fn fetch_data(url: &str) -> Result<String, Error> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}"#;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'let', 'async', 'fn', 'await' as keywords"
                );
                assert!(
                    html.contains("hl-function"),
                    "should highlight function names"
                );
                assert!(
                    html.contains("hl-type"),
                    "should highlight 'Result', 'String', 'Error' as types"
                );
                assert!(
                    html.contains("hl-type-builtin"),
                    "should highlight 'str' as builtin type"
                );
                assert!(
                    html.contains("hl-variable-parameter"),
                    "should highlight 'url' as parameter"
                );
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_rust_raw_strings_and_doc_comments() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r####"/// This is a doc comment
/// with multiple lines
fn example() {
    let raw = r#"raw string with "quotes""#;
    let multi = r##"another "raw" string"##;
}"####;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(html.contains("hl-comment"), "should highlight doc comments");
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'fn' and 'let' as keywords"
                );
                assert!(
                    html.contains("hl-function"),
                    "should highlight 'example' as function"
                );
                assert!(html.contains("hl-string"), "should highlight raw strings");
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_rust_impl_with_traits() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let code = r#"impl<T: Clone + Send> Display for MyType<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}"#;

        let result = highlighter.highlight(code, "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(
                    html.contains("hl-keyword"),
                    "should highlight 'impl', 'for', 'where', 'fn', 'mut' as keywords"
                );
                assert!(html.contains("hl-type"), "should highlight type names");
                assert!(
                    html.contains("hl-variable-builtin"),
                    "should highlight 'self' as builtin variable"
                );
                assert!(
                    html.contains("hl-variable-parameter"),
                    "should highlight 'f' as parameter"
                );
                assert!(
                    html.contains("hl-function-macro"),
                    "should highlight 'write!' as macro"
                );
                assert!(
                    html.contains("hl-function"),
                    "should highlight 'fmt' as function"
                );
                assert!(
                    html.contains("hl-label"),
                    "should highlight anonymous lifetime '_"
                );
                assert!(html.contains("hl-string"), "should highlight format string");
            }
            HighlightResult::Unsupported(_) => panic!("Rust should be supported"),
        }
    }

    #[test]
    fn test_highlight_unknown_language() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let result = highlighter.highlight("some code", "brainfuck");

        assert!(matches!(result, HighlightResult::Unsupported(_)));
    }

    #[test]
    fn test_highlight_rust_alias() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let result = highlighter.highlight("fn main() {}", "rs");

        assert!(matches!(result, HighlightResult::Highlighted(_)));
    }

    #[test]
    fn test_html_escaping_in_code() {
        let registry = LanguageRegistry::new();
        let mut highlighter = CodeHighlighter::new(registry, "hl-");

        let result = highlighter.highlight("let v: Vec<String> = vec![];", "rust");

        match result {
            HighlightResult::Highlighted(html) => {
                assert!(html.contains("&lt;"), "angle brackets should be escaped");
                assert!(html.contains("&gt;"), "angle brackets should be escaped");
                assert!(
                    !html.contains("<String>"),
                    "should not contain raw angle brackets around types"
                );
            }
            _ => panic!("should highlight successfully"),
        }
    }
}

#[cfg(test)]
mod web_lang_tests {
    use super::*;
    use crate::highlighting::languages::LanguageRegistry;

    fn hl(code: &str, lang: &str) -> String {
        let mut h = CodeHighlighter::new(LanguageRegistry::new(), "hl-");
        match h.highlight(code, lang) {
            HighlightResult::Highlighted(s) => s,
            HighlightResult::Unsupported(s) => format!("UNSUPPORTED: {s}"),
        }
    }

    #[test]
    fn javascript_highlights_keywords_strings_and_calls() {
        let out = hl("const msg = \"hi\";", "js");
        assert!(out.contains("hl-keyword"), "{out}");
        assert!(out.contains("hl-string"), "{out}");
        assert!(out.contains("hl-variable"), "{out}");
    }

    #[test]
    fn css_highlights_selectors_properties_and_values() {
        let out = hl(".card { color: red; }", "css");
        assert!(out.contains("hl-type"), "selector: {out}");
        assert!(out.contains("hl-property"), "{out}");
        assert!(out.contains("hl-string"), "{out}");
    }

    #[test]
    fn html_highlights_tags_and_attributes() {
        let out = hl("<a href=\"/x\">y</a>", "html");
        assert!(out.contains("hl-tag"), "{out}");
        assert!(out.contains("hl-attribute"), "{out}");
    }

    /// The cross-language injection: an html block must highlight the
    /// markup *and* the script body inside it. This is what the
    /// injection resolver in `highlight` exists for — the first
    /// cross-language injection in the project (Rust's inject rust
    /// into rust, so the callback was never consulted for anything
    /// other than the top language).
    #[test]
    fn html_injects_javascript_highlighting_into_script_bodies() {
        let out = hl("<script>const x = 1;</script>", "html");
        assert!(out.contains("hl-keyword"), "script keyword: {out}");
        assert!(out.contains("hl-number"), "script number: {out}");
    }

    #[test]
    fn html_injects_css_highlighting_into_style_bodies() {
        let out = hl("<style>.a { color: red; }</style>", "html");
        assert!(out.contains("hl-property"), "style property: {out}");
        assert!(out.contains("hl-type"), "style selector: {out}");
    }

    #[test]
    fn html_injects_css_highlighting_into_style_attributes() {
        let out = hl("<p style=\"color: red;\">x</p>", "html");
        assert!(out.contains("hl-property"), "style attr property: {out}");
        assert!(out.contains("hl-string"), "style attr value: {out}");
    }

    /// A quoted attribute must produce exactly one span around the
    /// value, not a span inside a span (invalid HTML, and a styling
    /// bug — the outer span is what the theme targets).
    #[test]
    fn quoted_attribute_values_are_not_nested() {
        let out = hl("<a href=\"/x\">y</a>", "html");
        assert!(!out.contains("<span class=\"hl-string\"><span"), "{out}");
    }

    #[test]
    fn aliases_resolve_to_the_same_highlighting() {
        assert_eq!(hl("const a=1;", "js"), hl("const a=1;", "javascript"));
        assert_eq!(hl("<p>x</p>", "htm"), hl("<p>x</p>", "html"));
    }
}
