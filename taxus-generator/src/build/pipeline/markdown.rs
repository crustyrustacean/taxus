// taxus-generator/src/build/pipeline/markdown.rs

use crate::highlighting::{CodeHighlighter, HighlightResult};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn markdown_to_html(markdown: &str, mut highlighter: Option<&mut CodeHighlighter>) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(markdown, options);
    let mut output = String::new();

    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_buffer.clear();
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang = lang.trim().to_string();
                        if lang.is_empty() { None } else { Some(lang) }
                    }
                    CodeBlockKind::Indented => None,
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                match &code_lang {
                    Some(lang) => {
                        let result = match highlighter {
                            Some(ref mut h1) => h1.highlight(&code_buffer, lang),
                            None => HighlightResult::Unsupported(escape_html(&code_buffer)),
                        };
                        match result {
                            HighlightResult::Highlighted(html) => {
                                output.push_str(&format!(
                                    "<pre class=\"highlight\"><code class=\"language-{}\">",
                                    lang
                                ));
                                output.push_str(&html);
                                output.push_str("</code></pre>\n");
                            }
                            HighlightResult::Unsupported(escaped) => {
                                output
                                    .push_str(&format!("<pre><code class=\"language-{}\">", lang));
                                output.push_str(&escaped);
                                output.push_str("</code></pre>\n");
                            }
                        }
                    }
                    None => {
                        output.push_str("<pre><code>");
                        output.push_str(&escape_html(&code_buffer));
                        output.push_str("</code></pre>\n");
                    }
                }

                code_lang = None;
                code_buffer.clear();
            }
            Event::Text(text) if in_code_block => {
                code_buffer.push_str(&text);
            }
            _ => {
                // For all non-code-block events, use pulldown-cmark's HTML output
                let single = std::iter::once(event);
                pulldown_cmark::html::push_html(&mut output, single);
            }
        }
    }

    output
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
    use crate::highlighting::LanguageRegistry;

    fn test_highlighter() -> CodeHighlighter {
        CodeHighlighter::new(LanguageRegistry::new(), "hl-")
    }

    #[test]
    fn test_markdown_to_html() {
        let mut hl = test_highlighter();
        let markdown = "# Hello\n\nThis is **bold** text.";
        let html = markdown_to_html(markdown, Some(&mut hl));
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_markdown_to_html_empty() {
        let mut hl = test_highlighter();
        let html = markdown_to_html("", Some(&mut hl));
        assert!(html.is_empty());
    }

    #[test]
    fn test_markdown_to_html_links() {
        let mut hl = test_highlighter();
        let markdown = "[link](https://example.com)";
        let html = markdown_to_html(markdown, Some(&mut hl));
        assert!(html.contains("<a href=\"https://example.com\">link</a>"));
    }

    #[test]
    fn test_markdown_to_html_code() {
        let mut hl = test_highlighter();
        let markdown = "```\ncode\n```";
        let html = markdown_to_html(markdown, Some(&mut hl));
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code>"));
    }

    #[test]
    fn test_markdown_to_html_tables() {
        let mut hl = test_highlighter();
        let markdown = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let html = markdown_to_html(markdown, Some(&mut hl));
        println!("TABLE OUTPUT: {}", html);
        assert!(html.contains("<table>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<th>Header 1</th>"));
        assert!(html.contains("<th>Cell 1</th>"));
    }

    #[test]
    fn test_markdown_to_html_rust_code_block() {
        let mut hl = test_highlighter();
        let markdown = r#"# Example
```rust
fn main() {
    println!("hello");
}
```

Some text after.
"#;
        let html = markdown_to_html(markdown, Some(&mut hl));

        // Should have the highlight wrapper
        assert!(html.contains("<pre class=\"highlight\">"));
        assert!(html.contains("language-rust"));

        // Should have highlighted spans
        assert!(html.contains("hl-keyword"));

        // Non-code content should still render normally
        assert!(html.contains("<h1>Example</h1>"));
        assert!(html.contains("Some text after."));
    }

    #[test]
    fn test_markdown_to_html_unknown_language_code_block() {
        let mut hl = test_highlighter();
        let markdown = "```brainfuck\n+++++\n```\n";
        let html = markdown_to_html(markdown, Some(&mut hl));

        // Should fall back to plain code block without highlight class
        assert!(html.contains("<pre><code"));
        assert!(html.contains("language-brainfuck"));
        assert!(!html.contains("highlight"));
    }

    #[test]
    fn test_markdown_to_html_no_language_code_block() {
        let mut hl = test_highlighter();
        let markdown = "```\nplain text\n```\n";
        let html = markdown_to_html(markdown, Some(&mut hl));

        // No language specified, plain code block
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("plain text"));
    }

    #[test]
    fn test_markdown_to_html_highlighting_disabled() {
        let markdown = "```rust\nfn main() {}\n```\n";
        let html = markdown_to_html(markdown, None);

        // Should render as plain code block without highlight class
        assert!(html.contains("<pre><code"));
        assert!(!html.contains("highlight"));
        assert!(!html.contains("hl-keyword"));
    }
}
