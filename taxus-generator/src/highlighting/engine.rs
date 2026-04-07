// taxus-generator/src/highlighting/engine.rs

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
    /// Successfully highlighted, contains HTML with <span> tags
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

        let events = match self
            .highlighter
            .highlight(config, code.as_bytes(), None, |_| None)
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

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
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
