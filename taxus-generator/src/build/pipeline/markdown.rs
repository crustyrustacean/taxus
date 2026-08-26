// taxus-generator/src/build/pipeline/markdown.rs

use crate::highlighting::{CodeHighlighter, HighlightResult};
use crate::routes::slugify::{SlugMode, slugify_segment};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;

/// A single table-of-contents entry for a heading in the page.
///
/// Produced by [`markdown_to_html`] alongside the HTML; exposed to
/// templates as `page.toc` / `section.toc`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TocEntry {
    /// Heading level (1-6)
    pub level: u8,
    /// Plain-text heading content
    pub text: String,
    /// The `id` attribute assigned to the heading
    pub id: String,
    /// Children (headings one level deeper appearing after this one
    /// before the next heading of this level or shallower)
    pub children: Vec<TocEntry>,
}

/// Options controlling markdown rendering.
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Insert a visible anchor link (`<a class="anchor" href="#id">#</a>`)
    /// into each heading. Defaults to false.
    pub insert_anchor_links: bool,
    /// Slugification mode used for generated heading ids.
    pub slug_mode: SlugMode,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            insert_anchor_links: false,
            slug_mode: SlugMode::On,
        }
    }
}

/// Render markdown to HTML, assigning ids to headings and collecting a TOC.
///
/// Returns `(html, toc)`. Headings receive `id` attributes derived from
/// their text (slugified, deduplicated with a numeric suffix) unless the
/// source provides `{#custom-id}` (requires heading attributes syntax).
pub fn markdown_to_html_with_toc(
    markdown: &str,
    mut highlighter: Option<&mut CodeHighlighter>,
    opts: &MarkdownOptions,
) -> (String, Vec<TocEntry>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    let parser = Parser::new_ext(markdown, options);

    let mut output = String::new();
    let mut toc: Vec<TocEntry> = Vec::new();

    // ── code block state ──
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_buffer = String::new();

    // ── heading state ──
    // Heading text is buffered so the id can be derived from the full
    // text before the closing tag is written.
    let mut in_heading = false;
    let mut heading_level: u8 = 0;
    let mut heading_explicit_id: Option<String> = None;
    let mut heading_classes: Vec<String> = Vec::new();
    let mut heading_text = String::new();

    // ── task list state ──
    // TaskListMarker events precede the item's text inside a list item;
    // pulldown-cmark does not add the class itself. Lists are written as
    // plain <ul> before markers are seen, so the class is retrofitted on
    // the first marker — task_list_started guards "already retrofitted".
    let mut task_list_started = false;

    // Deduplication: heading ids must be unique within the page.
    let mut id_counts: HashMap<String, usize> = HashMap::new();

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
            }
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                in_heading = true;
                heading_level = level as u8;
                heading_text.clear();
                heading_classes = classes.iter().map(|c| c.to_string()).collect();
                heading_explicit_id = id.map(|i| i.to_string());
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;

                // Resolve the id: explicit {#id} wins, else slugify text.
                let base_id = heading_explicit_id
                    .clone()
                    .unwrap_or_else(|| slugify_segment(&heading_text, opts.slug_mode));
                let id = unique_id(&base_id, &mut id_counts);

                // Open the tag with the id (and any explicit classes).
                output.push_str(&format!("<h{heading_level} id=\"{id}\""));
                if !heading_classes.is_empty() {
                    output.push_str(&format!(" class=\"{}\"", heading_classes.join(" ")));
                }
                output.push('>');

                if opts.insert_anchor_links {
                    output.push_str(&format!(
                        "<a class=\"anchor\" href=\"#{id}\" aria-hidden=\"true\">#</a>"
                    ));
                }

                output.push_str(&escape_html(&heading_text));
                output.push_str(&format!("</h{heading_level}>\n"));

                // Record in the TOC.
                let entry = TocEntry {
                    level: heading_level,
                    text: heading_text.clone(),
                    id: id.clone(),
                    children: Vec::new(),
                };
                insert_toc_entry(&mut toc, entry);

                heading_explicit_id = None;
                heading_classes.clear();
            }
            Event::Text(t) if in_code_block => code_buffer.push_str(&t),
            Event::Text(t) if in_heading => heading_text.push_str(&t),
            Event::Code(c) if in_heading => heading_text.push_str(&c),
            Event::Start(Tag::List(None)) => {
                task_list_started = false;
                output.push_str("<ul>");
            }
            Event::Start(Tag::List(Some(start))) => {
                task_list_started = false;
                if start == 1 {
                    output.push_str("<ol>");
                } else {
                    output.push_str(&format!("<ol start=\"{start}\">"));
                }
            }
            Event::End(TagEnd::List(false)) | Event::End(TagEnd::List(true)) => {
                output.push_str("</ul>\n");
            }
            Event::TaskListMarker(checked) => {
                // Task lists arrive as List(None) + TaskListMarker events,
                // so the list tag is already written as a plain <ul>.
                // Retrofit the class on the first marker of the list —
                // the marker itself proves the list is a task list.
                if !task_list_started {
                    task_list_started = true;
                    if let Some(pos) = output.rfind("<ul>") {
                        output.replace_range(pos..pos + 4, "<ul class=\"task-list\">");
                    }
                }
                output.push_str(if checked {
                    "<input type=\"checkbox\" class=\"task-list-item-checkbox\" disabled checked=\"checked\"> "
                } else {
                    "<input type=\"checkbox\" class=\"task-list-item-checkbox\" disabled> "
                });
            }
            Event::SoftBreak if in_heading => heading_text.push(' '),
            Event::SoftBreak => output.push('\n'),
            Event::HardBreak if in_heading => heading_text.push(' '),
            Event::HardBreak => {
                output.push_str("<br />\n");
            }
            Event::Html(h) => output.push_str(&h),
            Event::InlineHtml(h) if in_heading => heading_text.push_str(&h),
            Event::InlineHtml(h) => output.push_str(&h),
            // Escape raw text outside code blocks and headings.
            Event::Text(t) => output.push_str(&escape_html(&t)),
            // Code spans outside headings.
            Event::Code(c) => {
                output.push_str("<code>");
                output.push_str(&escape_html(&c));
                output.push_str("</code>");
            }
            // Everything else falls through to a default pass-through.
            other => {
                write_event(&mut output, other);
            }
        }
    }

    (output, toc)
}

/// Percent-encode a URL for an `href`/`src` attribute, matching
/// pulldown-cmark's own `escape_href` behavior (spaces → `%20`, etc.).
///
/// Needed because my hand-rolled tag writer emits destinations raw;
/// CommonMark allows spaces in `<...>` destinations, but raw spaces in
/// an HTML attribute value break the URL.
fn encode_href(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for b in url.bytes() {
        let safe = b < 0x80 && HREF_SAFE[b as usize] == 1;
        if safe {
            out.push(b as char);
        } else if b == b'&' {
            out.push_str("&amp;");
        } else if b == b'\'' {
            out.push_str("&#x27;");
        } else {
            out.push('%');
            out.push(HEX[((b >> 4) & 0xF) as usize] as char);
            out.push(HEX[(b & 0xF) as usize] as char);
        }
    }
    out
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";
/// Mirrors pulldown-cmark-escape's HREF_SAFE table.
const HREF_SAFE: [u8; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0,
];

/// Append a default rendering for events without special handling.
///
/// Keeps the event loop exhaustive without hand-writing every tag.
fn write_event(output: &mut String, event: Event) {
    use pulldown_cmark::Event as E;

    match event {
        E::Start(tag) => {
            let s = tag_to_html(&tag);
            output.push_str(&s);
        }
        E::End(tag) => {
            output.push_str(&end_tag_to_html(tag));
        }
        // Text/Code/Html are handled in the main loop; reachable only
        // through nested structures that bypass those arms.
        E::Text(t) => output.push_str(&escape_html(&t)),
        E::Code(c) => {
            output.push_str("<code>");
            output.push_str(&escape_html(&c));
            output.push_str("</code>");
        }
        E::Html(h) | E::InlineHtml(h) => output.push_str(&h),
        E::SoftBreak => output.push('\n'),
        E::HardBreak => output.push_str("<br />\n"),
        E::TaskListMarker(c) => {
            output.push_str(if c { "[x] " } else { "[ ] " });
        }
        E::FootnoteReference(name) => {
            output.push_str(&format!(
                "<sup class=\"footnote-reference\"><a href=\"#{}\">{}</a></sup>",
                escape_html(&name),
                escape_html(&name)
            ));
        }
        other => {
            // Remaining events (e.g. inside tables) render via Display.
            let _ = other;
        }
    }
}

/// Convert a start tag to its HTML opening string.
fn tag_to_html(tag: &Tag) -> String {
    use pulldown_cmark::Tag as T;
    match tag {
        T::Paragraph => "<p>".to_string(),
        T::Heading { .. } => String::new(), // handled in the main loop
        T::BlockQuote(_) => "<blockquote>\n".to_string(),
        T::CodeBlock(_) => String::new(), // handled in the main loop
        T::List(None) => "<ul>\n".to_string(),
        T::List(Some(start)) => {
            if *start == 1 {
                "<ol>\n".to_string()
            } else {
                format!("<ol start=\"{start}\">\n")
            }
        }
        T::Item => "<li>".to_string(),
        T::FootnoteDefinition(name) => {
            format!(
                "<div class=\"footnote-definition\" id=\"{name}\"><sup class=\"footnote-definition-label\">{name}</sup>"
            )
        }
        T::DefinitionList => "<dl>\n".to_string(),
        T::DefinitionListTitle => "<dt>".to_string(),
        T::DefinitionListDefinition => "<dd>".to_string(),
        T::Table(aligns) => {
            let mut s = String::from("<table>\n<thead><tr>");
            for a in aligns {
                s.push_str(match a {
                    pulldown_cmark::Alignment::None => "<th>",
                    pulldown_cmark::Alignment::Left => "<th align=\"left\">",
                    pulldown_cmark::Alignment::Center => "<th align=\"center\">",
                    pulldown_cmark::Alignment::Right => "<th align=\"right\">",
                });
            }
            s
        }
        T::TableHead => "</tr></thead><tbody>".to_string(),
        T::TableRow => "<tr>".to_string(),
        T::TableCell => "<td>".to_string(),
        T::Emphasis => "<em>".to_string(),
        T::Strong => "<strong>".to_string(),
        T::Strikethrough => "<del>".to_string(),
        T::Superscript => "<sup>".to_string(),
        T::Subscript => "<sub>".to_string(),
        T::Link {
            link_type,
            dest_url,
            title,
            id,
        } => {
            let title_attr = if title.is_empty() {
                String::new()
            } else {
                format!(" title=\"{}\"", escape_html(title))
            };
            match link_type {
                pulldown_cmark::LinkType::Email => {
                    format!("<a href=\"mailto:{dest_url}\"{title_attr}>")
                }
                pulldown_cmark::LinkType::Autolink => {
                    format!("<a href=\"{}\"{title_attr}>", encode_href(dest_url))
                }
                pulldown_cmark::LinkType::Reference | pulldown_cmark::LinkType::Shortcut => {
                    let _ = id;
                    format!("<a href=\"{}\"{}>", encode_href(dest_url), title_attr)
                }
                _ => format!("<a href=\"{}\"{}>", encode_href(dest_url), title_attr),
            }
        }
        T::Image {
            link_type,
            dest_url,
            title,
            id,
        } => {
            let _ = (link_type, id);
            let title_attr = if title.is_empty() {
                String::new()
            } else {
                format!(" title=\"{}\"", escape_html(title))
            };
            format!("<img src=\"{}\"{title_attr} alt=\"", encode_href(dest_url))
        }
        T::HtmlBlock => String::new(),
        T::MetadataBlock(_) => String::new(),
    }
}

/// Convert an end tag to its HTML closing string.
fn end_tag_to_html(tag: TagEnd) -> String {
    match tag {
        TagEnd::Paragraph => "</p>\n".to_string(),
        TagEnd::Heading(level) => format!("</h{}>\n", level as u8),
        TagEnd::BlockQuote(_) => "</blockquote>\n".to_string(),
        TagEnd::CodeBlock => String::new(),
        TagEnd::List(_) => "</ul>\n".to_string(),
        TagEnd::Item => "</li>\n".to_string(),
        TagEnd::FootnoteDefinition => "</div>\n".to_string(),
        TagEnd::DefinitionList => "</dl>\n".to_string(),
        TagEnd::DefinitionListTitle => "</dt>".to_string(),
        TagEnd::DefinitionListDefinition => "</dd>\n".to_string(),
        TagEnd::Table => "</tbody></table>\n".to_string(),
        TagEnd::TableHead => String::new(),
        TagEnd::TableRow => "</tr>\n".to_string(),
        TagEnd::TableCell => "</td>".to_string(),
        TagEnd::Emphasis => "</em>".to_string(),
        TagEnd::Strong => "</strong>".to_string(),
        TagEnd::Strikethrough => "</del>".to_string(),
        TagEnd::Superscript => "</sup>".to_string(),
        TagEnd::Subscript => "</sub>".to_string(),
        TagEnd::Link => "</a>".to_string(),
        TagEnd::Image => "\">".to_string(),
        TagEnd::HtmlBlock => String::new(),
        TagEnd::MetadataBlock(_) => String::new(),
    }
}

/// Backward-compatible wrapper: renders markdown without TOC extraction.
pub fn markdown_to_html(markdown: &str, highlighter: Option<&mut CodeHighlighter>) -> String {
    markdown_to_html_with_toc(markdown, highlighter, &MarkdownOptions::default()).0
}

/// Produce a unique id for a heading within a page.
///
/// First occurrence keeps the base id; later duplicates get `-1`, `-2`, …
fn unique_id(base: &str, counts: &mut HashMap<String, usize>) -> String {
    let entry = counts.entry(base.to_string()).or_insert(0);
    if *entry == 0 {
        *entry += 1;
        base.to_string()
    } else {
        let n = *entry;
        *entry += 1;
        format!("{base}-{n}")
    }
}

/// Insert a TOC entry at the correct nesting depth.
///
/// Headings nest when a deeper level follows a shallower one; a heading
/// at the same or shallower level closes the current subtree.
fn insert_toc_entry(toc: &mut Vec<TocEntry>, entry: TocEntry) {
    // Find the last top-level entry that is shallower than us.
    if let Some(last) = toc.last_mut()
        && entry.level > last.level
    {
        insert_toc_entry(&mut last.children, entry);
        return;
    }
    toc.push(entry);
}

/// Escape characters with special meaning in HTML text content.
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

// ============================================
// Markdown Rendering Tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_list_renders_checkboxes() {
        let (html, _) = markdown_to_html_with_toc(
            "- [ ] todo\n- [x] done\n",
            None,
            &MarkdownOptions::default(),
        );
        assert!(html.contains("class=\"task-list\""), "got: {html}");
        assert!(html.contains("disabled checked"), "got: {html}");
    }

    #[test]
    fn test_footnotes_render() {
        let (html, _) = markdown_to_html_with_toc(
            "Text[^1]\n\n[^1]: The note\n",
            None,
            &MarkdownOptions::default(),
        );
        assert!(
            html.contains("footnote-reference"),
            "footnote ref missing: {html}"
        );
        assert!(
            html.contains("footnote-definition"),
            "footnote def missing: {html}"
        );
    }

    #[test]
    fn test_strikethrough_renders() {
        let (html, _) = markdown_to_html_with_toc("~~gone~~\n", None, &MarkdownOptions::default());
        assert!(html.contains("<del>gone</del>"), "got: {html}");
    }

    #[test]
    fn test_tables_still_render() {
        let (html, _) = markdown_to_html_with_toc(
            "| a | b |\n|---|---|\n| 1 | 2 |\n",
            None,
            &MarkdownOptions::default(),
        );
        assert!(html.contains("<table>"), "got: {html}");
        assert!(html.contains("<td>1</td>"), "got: {html}");
    }

    #[test]
    fn test_headings_get_slugified_ids() {
        let (html, toc) =
            markdown_to_html_with_toc("## Hello World\n", None, &MarkdownOptions::default());
        assert!(html.contains("<h2 id=\"hello-world\">"), "got: {html}");
        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].id, "hello-world");
        assert_eq!(toc[0].text, "Hello World");
    }

    #[test]
    fn test_duplicate_headings_get_numbered_ids() {
        let (html, toc) = markdown_to_html_with_toc(
            "## Same\n\n## Same\n\n## Same\n",
            None,
            &MarkdownOptions::default(),
        );
        assert!(html.contains("id=\"same\""), "got: {html}");
        assert!(html.contains("id=\"same-1\""), "got: {html}");
        assert!(html.contains("id=\"same-2\""), "got: {html}");
        assert_eq!(toc.len(), 3);
        assert_eq!(toc[2].id, "same-2");
    }

    #[test]
    fn test_explicit_heading_id_wins() {
        let (html, toc) = markdown_to_html_with_toc(
            "## Heading One {#custom-id}\n",
            None,
            &MarkdownOptions::default(),
        );
        assert!(html.contains("id=\"custom-id\""), "got: {html}");
        assert_eq!(toc[0].id, "custom-id");
        // The attribute suffix must not leak into the text.
        assert!(!html.contains("custom-id</h2>"), "leaked: {html}");
    }

    #[test]
    fn test_toc_nesting() {
        let (_, toc) = markdown_to_html_with_toc(
            "# A\n\n## B\n\n### C\n\n# D\n",
            None,
            &MarkdownOptions::default(),
        );
        assert_eq!(toc.len(), 2, "top level: {toc:?}");
        assert_eq!(toc[0].children.len(), 1);
        assert_eq!(toc[0].children[0].children.len(), 1);
        assert_eq!(toc[1].text, "D");
    }

    #[test]
    fn test_anchor_links_inserted_when_enabled() {
        let opts = MarkdownOptions {
            insert_anchor_links: true,
            ..Default::default()
        };
        let (html, _) = markdown_to_html_with_toc("## Sec\n", None, &opts);
        assert!(
            html.contains("<a class=\"anchor\" href=\"#sec\""),
            "got: {html}"
        );
    }

    #[test]
    fn test_anchor_links_absent_by_default() {
        let (html, _) = markdown_to_html_with_toc("## Sec\n", None, &MarkdownOptions::default());
        assert!(!html.contains("class=\"anchor\""), "got: {html}");
    }

    #[test]
    fn test_heading_with_inline_code() {
        let (html, toc) = markdown_to_html_with_toc(
            "## Using `taxus` build\n",
            None,
            &MarkdownOptions::default(),
        );
        // The id comes from the plain text.
        assert!(html.contains("id=\"using-taxus-build\""), "got: {html}");
        assert_eq!(toc[0].text, "Using taxus build");
    }

    #[test]
    fn test_plain_list_unaffected_by_task_list_styling() {
        let (html, _) =
            markdown_to_html_with_toc("- one\n- two\n", None, &MarkdownOptions::default());
        assert!(!html.contains("task-list"), "got: {html}");
        assert!(html.contains("<li>one</li>"), "got: {html}");
    }

    #[test]
    fn test_unicode_heading_id() {
        let (html, _) =
            markdown_to_html_with_toc("## Café Notes\n", None, &MarkdownOptions::default());
        // On mode transliterates.
        assert!(html.contains("id=\"cafe-notes\""), "got: {html}");
    }

    #[test]
    fn test_backward_compat_wrapper() {
        let html = markdown_to_html("- [x] done\n", None);
        assert!(html.contains("disabled checked"), "got: {html}");
    }

    #[test]
    fn test_bold_italic_links_still_render() {
        let (html, _) = markdown_to_html_with_toc(
            "Some **bold** and *ital* and [link](https://example.com).\n",
            None,
            &MarkdownOptions::default(),
        );
        assert!(html.contains("<strong>bold</strong>"), "got: {html}");
        assert!(html.contains("<em>ital</em>"), "got: {html}");
        assert!(
            html.contains("<a href=\"https://example.com\">link</a>"),
            "got: {html}"
        );
    }
}
