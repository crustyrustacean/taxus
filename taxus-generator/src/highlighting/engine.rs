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
            .expect(&format!("Failed to create highlight config for {}", name));

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
