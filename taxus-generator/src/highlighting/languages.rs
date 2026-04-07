// taxus-generator/src/highlighting/language.rs

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
