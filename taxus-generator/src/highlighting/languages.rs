// taxus-generator/src/highlighting/language.rs

//! The registry of tree-sitter grammars and their highlight queries (emit phase).

use std::collections::HashMap;

#[derive(Clone)]
pub struct LanguageSpec {
    pub name: &'static str,
    pub language: tree_sitter::Language,
    pub highlight_query: &'static str,
    pub injection_query: Option<&'static str>,
    pub locals_query: Option<&'static str>,
}

pub struct LanguageRegistry {
    languages: HashMap<&'static str, LanguageSpec>,
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            languages: HashMap::new(),
        };

        #[cfg(feature = "lang-rust")]
        registry.register_rust();

        #[cfg(feature = "lang-javascript")]
        registry.register_javascript();

        #[cfg(feature = "lang-css")]
        registry.register_css();

        #[cfg(feature = "lang-html")]
        registry.register_html();

        registry
    }

    pub fn canonical_name(&self, name: &str) -> Option<&'static str> {
        let lower = name.to_lowercase();
        self.languages.get(lower.as_str()).map(|spec| spec.name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&&'static str, &LanguageSpec)> {
        self.languages.iter()
    }

    fn register(&mut self, spec: LanguageSpec, aliases: &[&'static str]) {
        for alias in aliases {
            self.languages.insert(alias, spec.clone());
        }
        self.languages.insert(spec.name, spec);
    }

    #[cfg(feature = "lang-rust")]
    fn register_rust(&mut self) {
        let spec = LanguageSpec {
            name: "rust",
            language: tree_sitter_rust::LANGUAGE.into(),
            highlight_query: include_str!("queries/rust/highlights.scm"),
            injection_query: Some(include_str!("queries/rust/injections.scm")),
            locals_query: None,
        };

        self.register(spec, &["rs"]);
    }

    #[cfg(feature = "lang-javascript")]
    fn register_javascript(&mut self) {
        let spec = LanguageSpec {
            name: "javascript",
            language: tree_sitter_javascript::LANGUAGE.into(),
            highlight_query: include_str!("queries/javascript/highlights.scm"),
            injection_query: None,
            locals_query: None,
        };

        self.register(spec, &["js", "mjs", "cjs"]);
    }

    #[cfg(feature = "lang-css")]
    fn register_css(&mut self) {
        let spec = LanguageSpec {
            name: "css",
            language: tree_sitter_css::LANGUAGE.into(),
            highlight_query: include_str!("queries/css/highlights.scm"),
            injection_query: None,
            locals_query: None,
        };

        self.register(spec, &[]);
    }

    #[cfg(feature = "lang-html")]
    fn register_html(&mut self) {
        // The injection query is the point: an `html` block highlights
        // its markup and, through the javascript and css grammars, the
        // script and style bodies inside it.
        let spec = LanguageSpec {
            name: "html",
            language: tree_sitter_html::LANGUAGE.into(),
            highlight_query: include_str!("queries/html/highlights.scm"),
            injection_query: Some(include_str!("queries/html/injections.scm")),
            locals_query: None,
        };

        self.register(spec, &["htm"]);
    }

    pub fn get(&self, name: &str) -> Option<&LanguageSpec> {
        self.languages.get(name)
    }

    pub fn supports(&self, name: &str) -> bool {
        self.languages.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_rust_when_enabled() {
        let registry = LanguageRegistry::new();
        assert!(registry.supports("rust"));
    }

    #[test]
    fn test_registry_alias_rs() {
        let registry = LanguageRegistry::new();
        assert!(registry.supports("rs"));
    }

    #[test]
    fn test_registry_canonical_name() {
        let registry = LanguageRegistry::new();
        assert_eq!(registry.canonical_name("rs"), Some("rust"));
        assert_eq!(registry.canonical_name("rust"), Some("rust"));
    }

    #[test]
    fn test_registry_canonical_name_case_insensitive() {
        let registry = LanguageRegistry::new();
        assert_eq!(registry.canonical_name("Rust"), Some("rust"));
        assert_eq!(registry.canonical_name("RS"), Some("rust"));
    }

    #[test]
    fn test_registry_unknown_language() {
        let registry = LanguageRegistry::new();
        assert!(!registry.supports("brainfuck"));
        assert_eq!(registry.canonical_name("brainfuck"), None);
    }

    #[test]
    fn test_registry_iter() {
        let registry = LanguageRegistry::new();
        let count = registry.iter().count();
        // "rust" and "rs" entries
        assert!(count >= 2, "should have at least rust and rs entries");
    }
}

#[cfg(test)]
mod query_tests {
    use super::*;
    use crate::highlighting::engine::HIGHLIGHT_NAMES;

    /// Every registered query must compile against the shared
    /// highlight vocabulary. A node type that does not exist in the
    /// grammar is a *query* error, and `CodeHighlighter::new` turns that
    /// into a panic — so this test is the guard that catches a bad
    /// query at the nearest possible point rather than mid-build.
    #[test]
    fn every_registered_query_compiles() {
        for (name, spec) in LanguageRegistry::new().iter() {
            tree_sitter_highlight::HighlightConfiguration::new(
                spec.language.clone(),
                spec.name,
                spec.highlight_query,
                spec.injection_query.unwrap_or(""),
                spec.locals_query.unwrap_or(""),
            )
            .unwrap_or_else(|e| panic!("highlight query for `{name}` is invalid: {e}"));
        }
    }

    /// And it must also configure against `HIGHLIGHT_NAMES` — a capture
    /// name outside the shared vocabulary is rejected at `configure`
    /// time, just as fatally.
    #[test]
    fn every_registered_query_uses_only_known_capture_names() {
        for (name, spec) in LanguageRegistry::new().iter() {
            let mut config = tree_sitter_highlight::HighlightConfiguration::new(
                spec.language.clone(),
                spec.name,
                spec.highlight_query,
                spec.injection_query.unwrap_or(""),
                spec.locals_query.unwrap_or(""),
            )
            .unwrap_or_else(|e| panic!("highlight query for `{name}` is invalid: {e}"));
            // `configure` panics on an unknown capture name; the test
            // exists so the panic is attributable to this file.
            config.configure(HIGHLIGHT_NAMES);
        }
    }
}
