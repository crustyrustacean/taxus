// taxus-generator/src/build/pipeline/markdown.rs

//! Stage 3 (emit): Markdown to HTML, with heading ids and a table of contents.
//!
//! This is where a node's body becomes the `page.content` a template sees.
//! The tree never holds rendered HTML; it lives on the `ProcessedPage`
//! this stage produces. See the book's
//! [Worked Example](https://crustyrustacean.github.io/taxus/theory/worked-example.html).

use crate::highlighting::engine::escape_html;
use crate::highlighting::{CodeHighlighter, HighlightResult};
use crate::routes::slugify::slugify_segment;
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
#[derive(Debug, Clone, Default)]
pub struct MarkdownOptions {
    /// Insert a visible anchor link (`<a class="anchor" href="#id">#</a>`)
    /// into each heading. Defaults to false.
    pub insert_anchor_links: bool,
}

/// Render markdown to HTML, assigning ids to headings and collecting a TOC.
///
/// Returns `(html, toc)`. Headings receive `id` attributes derived from
/// their text (slugified, deduplicated with a numeric suffix) unless the
/// source provides `{#custom-id}` (requires heading attributes syntax).
pub fn markdown_to_html_with_toc(
    markdown: &str,
    highlighter: Option<&mut CodeHighlighter>,
    opts: &MarkdownOptions,
) -> (String, Vec<TocEntry>) {
    let parser = Parser::new_ext(markdown, markdown_parser_options());
    let mut renderer = MarkdownRenderer::new(highlighter, opts);
    for event in parser {
        renderer.handle_event(event);
    }
    (renderer.output, renderer.toc)
}

fn markdown_parser_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options
}

#[derive(Default)]
enum CaptureState {
    #[default]
    Normal,
    CodeBlock {
        lang: Option<String>,
        buffer: String,
    },
    Heading {
        level: u8,
        explicit_id: Option<String>,
        classes: Vec<String>,
        text: String,
    },
}

struct TableState {
    alignments: Vec<pulldown_cmark::Alignment>,
    column: usize,
    in_head: bool,
}

struct MarkdownRenderer<'a, 'h> {
    output: String,
    toc: Vec<TocEntry>,
    highlighter: Option<&'h mut CodeHighlighter>,
    opts: &'a MarkdownOptions,
    capture: CaptureState,
    table: Option<TableState>,
    /// TaskListMarker events precede the item's text inside a list item;
    /// pulldown-cmark does not add the class itself. Lists are written as
    /// plain `<ul>` before markers are seen, so the class is retrofitted on
    /// the first marker — `task_list_started` guards "already retrofitted".
    task_list_started: bool,
    /// Deduplication: heading ids must be unique within the page.
    id_counts: HashMap<String, usize>,
}

impl<'a, 'h> MarkdownRenderer<'a, 'h> {
    fn new(highlighter: Option<&'h mut CodeHighlighter>, opts: &'a MarkdownOptions) -> Self {
        Self {
            output: String::new(),
            toc: Vec::new(),
            highlighter,
            opts,
            capture: CaptureState::Normal,
            table: None,
            task_list_started: false,
            id_counts: HashMap::new(),
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start(tag),
            Event::End(tag) => self.handle_end(tag),
            Event::Text(t) => self.handle_text(&t),
            Event::Code(c) => self.handle_code(&c),
            Event::TaskListMarker(checked) => self.handle_task_list_marker(checked),
            Event::SoftBreak => self.handle_break(false),
            Event::HardBreak => self.handle_break(true),
            Event::Html(h) => self.output.push_str(&h),
            Event::InlineHtml(h) => self.handle_inline_html(&h),
            Event::FootnoteReference(name) => write_footnote_reference(&mut self.output, &name),
            _ => {}
        }
    }

    fn handle_start(&mut self, tag: Tag) {
        match tag {
            Tag::CodeBlock(kind) => self.start_code_block(kind),
            Tag::Table(alignments) => self.start_table(alignments),
            Tag::TableHead => self.start_table_head(),
            Tag::TableRow => self.start_table_row(),
            Tag::TableCell => self.start_table_cell(),
            Tag::Heading {
                level, id, classes, ..
            } => self.start_heading(
                level as u8,
                id.map(|i| i.to_string()),
                classes.iter().map(|c| c.to_string()).collect(),
            ),
            Tag::List(start) => self.start_list(start),
            other => write_start_tag(&mut self.output, &other),
        }
    }

    fn handle_end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::CodeBlock => self.finish_code_block(),
            TagEnd::Heading(_) => self.finish_heading(),
            TagEnd::TableCell => self.end_table_cell(),
            TagEnd::TableHead => self.end_table_head(),
            TagEnd::TableRow => self.output.push_str("</tr>\n"),
            TagEnd::Table => self.end_table(),
            // `TagEnd::List(is_ordered)` carries the list kind: close
            // with the tag that matches the opener. A blanket `</ul>`
            // here produced `<ol>…</ul>` — invalid HTML that browsers
            // recover from but breaks validation and nested lists.
            TagEnd::List(is_ordered) => {
                self.output
                    .push_str(if is_ordered { "</ol>\n" } else { "</ul>\n" });
            }
            other => self.output.push_str(simple_end_tag(other)),
        }
    }

    fn start_code_block(&mut self, kind: CodeBlockKind) {
        let lang = match kind {
            CodeBlockKind::Fenced(lang) => {
                let lang = lang.trim().to_string();
                if lang.is_empty() { None } else { Some(lang) }
            }
            CodeBlockKind::Indented => None,
        };
        self.capture = CaptureState::CodeBlock {
            lang,
            buffer: String::new(),
        };
    }

    fn finish_code_block(&mut self) {
        if let CaptureState::CodeBlock { lang, buffer } = std::mem::take(&mut self.capture) {
            emit_code_block(
                &mut self.output,
                &buffer,
                lang.as_deref(),
                self.highlighter.as_deref_mut(),
            );
        }
    }

    fn start_table(&mut self, alignments: Vec<pulldown_cmark::Alignment>) {
        self.table = Some(TableState {
            alignments,
            column: 0,
            in_head: false,
        });
        self.output.push_str("<table>\n");
    }

    fn start_table_head(&mut self) {
        if let Some(table) = &mut self.table {
            table.in_head = true;
            table.column = 0;
        }
        self.output.push_str("<thead><tr>");
    }

    fn start_table_row(&mut self) {
        if let Some(table) = &mut self.table {
            table.column = 0;
        }
        self.output.push_str("<tr>");
    }

    fn start_table_cell(&mut self) {
        let tag = match &mut self.table {
            Some(table) => {
                let tag = if table.in_head {
                    alignment_header_tag(table.alignments.get(table.column))
                } else {
                    "<td>"
                };
                table.column += 1;
                tag
            }
            None => "<td>",
        };
        self.output.push_str(tag);
    }

    fn end_table_cell(&mut self) {
        let in_head = self.table.as_ref().is_some_and(|t| t.in_head);
        self.output
            .push_str(if in_head { "</th>" } else { "</td>" });
    }

    fn end_table_head(&mut self) {
        if let Some(table) = &mut self.table {
            table.in_head = false;
        }
        self.output.push_str("</tr></thead><tbody>");
    }

    fn end_table(&mut self) {
        self.table = None;
        self.output.push_str("</tbody></table>\n");
    }

    fn start_heading(&mut self, level: u8, explicit_id: Option<String>, classes: Vec<String>) {
        self.capture = CaptureState::Heading {
            level,
            explicit_id,
            classes,
            text: String::new(),
        };
    }

    fn finish_heading(&mut self) {
        let CaptureState::Heading {
            level,
            explicit_id,
            classes,
            text,
        } = std::mem::take(&mut self.capture)
        else {
            return;
        };

        // Resolve the id: explicit {#id} wins, else slugify text.
        let base_id = explicit_id.unwrap_or_else(|| slugify_segment(&text));
        let id = unique_id(&base_id, &mut self.id_counts);

        // Open the tag with the id (and any explicit classes).
        self.output.push_str(&format!("<h{level} id=\"{id}\""));
        if !classes.is_empty() {
            self.output
                .push_str(&format!(" class=\"{}\"", classes.join(" ")));
        }
        self.output.push('>');

        if self.opts.insert_anchor_links {
            self.output.push_str(&format!(
                "<a class=\"anchor\" href=\"#{id}\" aria-hidden=\"true\">#</a>"
            ));
        }

        self.output.push_str(&escape_html(&text));
        self.output.push_str(&format!("</h{level}>\n"));

        // Record in the TOC.
        let entry = TocEntry {
            level,
            text,
            id,
            children: Vec::new(),
        };
        insert_toc_entry(&mut self.toc, entry);
    }

    fn start_list(&mut self, start: Option<u64>) {
        self.task_list_started = false;
        match start {
            None => self.output.push_str("<ul>"),
            Some(1) => self.output.push_str("<ol>"),
            Some(n) => self.output.push_str(&format!("<ol start=\"{n}\">")),
        }
    }

    fn handle_task_list_marker(&mut self, checked: bool) {
        // Task lists arrive as List(None) + TaskListMarker events,
        // so the list tag is already written as a plain <ul>.
        // Retrofit the class on the first marker of the list —
        // the marker itself proves the list is a task list.
        if !self.task_list_started {
            self.task_list_started = true;
            if let Some(pos) = self.output.rfind("<ul>") {
                self.output
                    .replace_range(pos..pos + 4, "<ul class=\"task-list\">");
            }
        }
        self.output.push_str(if checked {
            "<input type=\"checkbox\" class=\"task-list-item-checkbox\" disabled checked=\"checked\"> "
        } else {
            "<input type=\"checkbox\" class=\"task-list-item-checkbox\" disabled> "
        });
    }

    fn handle_text(&mut self, text: &str) {
        match &mut self.capture {
            CaptureState::CodeBlock { buffer, .. } => buffer.push_str(text),
            CaptureState::Heading { text: heading, .. } => heading.push_str(text),
            CaptureState::Normal => self.output.push_str(&escape_html(text)),
        }
    }

    fn handle_code(&mut self, code: &str) {
        if let CaptureState::Heading { text, .. } = &mut self.capture {
            text.push_str(code);
        } else {
            self.output.push_str("<code>");
            self.output.push_str(&escape_html(code));
            self.output.push_str("</code>");
        }
    }

    fn handle_break(&mut self, hard: bool) {
        if let CaptureState::Heading { text, .. } = &mut self.capture {
            text.push(' ');
        } else if hard {
            self.output.push_str("<br />\n");
        } else {
            self.output.push('\n');
        }
    }

    fn handle_inline_html(&mut self, html: &str) {
        if let CaptureState::Heading { text, .. } = &mut self.capture {
            text.push_str(html);
        } else {
            self.output.push_str(html);
        }
    }
}

fn emit_code_block(
    output: &mut String,
    code: &str,
    lang: Option<&str>,
    highlighter: Option<&mut CodeHighlighter>,
) {
    let Some(lang) = lang else {
        output.push_str("<pre><code>");
        output.push_str(&escape_html(code));
        output.push_str("</code></pre>\n");
        return;
    };

    let result = match highlighter {
        Some(hl) => hl.highlight(code, lang),
        None => HighlightResult::Unsupported(escape_html(code)),
    };
    let (pre_open, body) = match result {
        HighlightResult::Highlighted(html) => ("<pre class=\"highlight\">", html),
        HighlightResult::Unsupported(escaped) => ("<pre>", escaped),
    };
    output.push_str(pre_open);
    output.push_str(&format!("<code class=\"language-{lang}\">"));
    output.push_str(&body);
    output.push_str("</code></pre>\n");
}

fn alignment_header_tag(alignment: Option<&pulldown_cmark::Alignment>) -> &'static str {
    match alignment {
        Some(pulldown_cmark::Alignment::Left) => "<th align=\"left\">",
        Some(pulldown_cmark::Alignment::Center) => "<th align=\"center\">",
        Some(pulldown_cmark::Alignment::Right) => "<th align=\"right\">",
        Some(pulldown_cmark::Alignment::None) | None => "<th>",
    }
}

fn write_footnote_reference(output: &mut String, name: &str) {
    let escaped = escape_html(name);
    output.push_str(&format!(
        "<sup class=\"footnote-reference\"><a href=\"#{escaped}\">{escaped}</a></sup>"
    ));
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

/// Write a non-stateful opening tag to `output`.
fn write_start_tag(output: &mut String, tag: &Tag) {
    if let Some(static_tag) = simple_start_tag(tag) {
        output.push_str(static_tag);
        return;
    }

    match tag {
        Tag::FootnoteDefinition(name) => {
            output.push_str(&format!(
                "<div class=\"footnote-definition\" id=\"{name}\"><sup class=\"footnote-definition-label\">{name}</sup>"
            ));
        }
        Tag::Link {
            link_type,
            dest_url,
            title,
            ..
        } => {
            output.push_str(&format_link_start(*link_type, dest_url, title));
        }
        Tag::Image {
            dest_url, title, ..
        } => {
            output.push_str(&format_image_start(dest_url, title));
        }
        _ => {}
    }
}

fn simple_start_tag(tag: &Tag) -> Option<&'static str> {
    use pulldown_cmark::Tag as T;
    Some(match tag {
        T::Paragraph => "<p>",
        T::BlockQuote(_) => "<blockquote>\n",
        T::Item => "<li>",
        T::DefinitionList => "<dl>\n",
        T::DefinitionListTitle => "<dt>",
        T::DefinitionListDefinition => "<dd>",
        T::Emphasis => "<em>",
        T::Strong => "<strong>",
        T::Strikethrough => "<del>",
        T::Superscript => "<sup>",
        T::Subscript => "<sub>",
        T::HtmlBlock | T::MetadataBlock(_) => "",
        _ => return None,
    })
}

fn format_title_attr(title: &str) -> String {
    if title.is_empty() {
        String::new()
    } else {
        format!(" title=\"{}\"", escape_html(title))
    }
}

fn format_link_start(link_type: pulldown_cmark::LinkType, dest_url: &str, title: &str) -> String {
    let title_attr = format_title_attr(title);
    if matches!(link_type, pulldown_cmark::LinkType::Email) {
        format!("<a href=\"mailto:{dest_url}\"{title_attr}>")
    } else {
        format!("<a href=\"{}\"{title_attr}>", encode_href(dest_url))
    }
}

fn format_image_start(dest_url: &str, title: &str) -> String {
    let title_attr = format_title_attr(title);
    format!("<img src=\"{}\"{title_attr} alt=\"", encode_href(dest_url))
}

/// Convert a non-stateful end tag to its HTML closing string.
fn simple_end_tag(tag: TagEnd) -> &'static str {
    match tag {
        TagEnd::Paragraph => "</p>\n",
        TagEnd::BlockQuote(_) => "</blockquote>\n",
        TagEnd::Item => "</li>\n",
        TagEnd::FootnoteDefinition => "</div>\n",
        TagEnd::DefinitionList => "</dl>\n",
        TagEnd::DefinitionListTitle => "</dt>",
        TagEnd::DefinitionListDefinition => "</dd>\n",
        TagEnd::Emphasis => "</em>",
        TagEnd::Strong => "</strong>",
        TagEnd::Strikethrough => "</del>",
        TagEnd::Superscript => "</sup>",
        TagEnd::Subscript => "</sub>",
        TagEnd::Link => "</a>",
        TagEnd::Image => "\">",
        _ => "",
    }
}

/// Backward-compatible wrapper: renders markdown without TOC extraction.
pub fn markdown_to_html(markdown: &str, highlighter: Option<&mut CodeHighlighter>) -> String {
    markdown_to_html_with_toc(markdown, highlighter, &MarkdownOptions::default()).0
}

/// The plain text of a Markdown document: every `Text` and inline-`Code`
/// event, space-joined.
///
/// What search indexes (#43) and summaries/word counts (#10) consume:
/// not rendered HTML, whose tags, attributes and highlighter classes
/// would inflate the term space (`div`, `href`, `hl-keyword` →
/// `keyword`), but the words a reader actually reads. Code blocks
/// contribute their source text as `Text` events; inline code arrives
/// as `Code` events and is included the same way — a visitor searching
/// `serde_json` should find the page whose snippet uses it.
pub fn markdown_text(markdown: &str) -> String {
    let mut text = String::with_capacity(markdown.len());
    // Full extension set so tables and strikethrough parse as structure
    // (cells and struck text contribute their words; markers do not).
    for event in Parser::new_ext(markdown, Options::all()) {
        match event {
            Event::Text(chunk) | Event::Code(chunk) => {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&chunk);
            }
            _ => {}
        }
    }
    text
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

// escape_html was consolidated into highlighting::engine (#15) — the
// duplicate lived here. Use the shared one below.

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
    fn test_tables_render_header_cells_and_body_rows() {
        let (html, _) = markdown_to_html_with_toc(
            "| Tier | Who writes it | Examples |\n|---|---|---|\n| 1 | The model | the bash tool |\n| 2 | The user | hooks |\n",
            None,
            &MarkdownOptions::default(),
        );

        assert!(
            html.contains(
                "<thead><tr><th>Tier</th><th>Who writes it</th><th>Examples</th></tr></thead>"
            ),
            "header cells are malformed: {html}"
        );
        assert!(
            html.contains(
                "<tbody><tr><td>1</td><td>The model</td><td>the bash tool</td></tr>\n<tr><td>2</td><td>The user</td><td>hooks</td></tr>"
            ),
            "body rows are malformed: {html}"
        );
        assert!(!html.contains("<th></th>"), "empty header cell: {html}");
    }

    #[test]
    fn test_table_alignment_is_preserved() {
        let (html, _) = markdown_to_html_with_toc(
            "| Left | Center | Right |\n|:---|:---:|---:|\n| a | b | c |\n",
            None,
            &MarkdownOptions::default(),
        );

        assert!(html.contains("<th align=\"left\">Left</th>"), "got: {html}");
        assert!(
            html.contains("<th align=\"center\">Center</th>"),
            "got: {html}"
        );
        assert!(
            html.contains("<th align=\"right\">Right</th>"),
            "got: {html}"
        );
    }

    #[test]
    fn test_header_only_table_still_renders_header() {
        let (html, _) = markdown_to_html_with_toc(
            "| Name | Value |\n|---|---|\n",
            None,
            &MarkdownOptions::default(),
        );

        assert!(
            html.contains("<thead><tr><th>Name</th><th>Value</th></tr></thead>"),
            "header-only table is malformed: {html}"
        );
        assert!(html.contains("<tbody></tbody></table>"), "got: {html}");
        assert!(!html.contains("<tbody><td>"), "loose body cell: {html}");
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
    fn test_ordered_list_closes_with_ol() {
        let (html, _) =
            markdown_to_html_with_toc("1. first\n2. second\n", None, &MarkdownOptions::default());
        assert!(html.contains("<ol>"), "got: {html}");
        assert!(html.contains("</ol>"), "got: {html}");
        assert!(!html.contains("</ul>"), "got: {html}");
    }

    #[test]
    fn test_ordered_list_with_start_attribute_closes_with_ol() {
        let (html, _) =
            markdown_to_html_with_toc("3. third\n4. fourth\n", None, &MarkdownOptions::default());
        assert!(html.contains("<ol start=\"3\">"), "got: {html}");
        assert!(html.contains("</ol>"), "got: {html}");
        assert!(!html.contains("</ul>"), "got: {html}");
    }

    #[test]
    fn test_unordered_list_still_closes_with_ul() {
        let (html, _) =
            markdown_to_html_with_toc("- one\n- two\n", None, &MarkdownOptions::default());
        assert!(html.contains("<ul>"), "got: {html}");
        assert!(html.contains("</ul>"), "got: {html}");
        assert!(!html.contains("<ol"), "got: {html}");
    }

    #[test]
    fn test_ordered_list_nested_in_unordered_is_balanced() {
        let (html, _) = markdown_to_html_with_toc(
            "- outer one\n- outer two\n  1. inner a\n  2. inner b\n",
            None,
            &MarkdownOptions::default(),
        );
        // Every `<ol>` must be closed by `</ol>`, every `<ul>` by `</ul>`,
        // regardless of nesting order.
        let ols = html.matches("<ol").count();
        let ol_ends = html.matches("</ol>").count();
        let uls = html.matches("<ul").count();
        let ul_ends = html.matches("</ul>").count();
        assert_eq!(ols, ol_ends, "unbalanced <ol>/</ol>, got: {html}");
        assert_eq!(uls, ul_ends, "unbalanced <ul>/</ul>, got: {html}");
        assert_eq!(ols, 1, "expected one ordered list, got: {html}");
        assert_eq!(uls, 1, "expected one unordered list, got: {html}");
    }

    #[test]
    fn test_unicode_heading_id() {
        let (html, _) =
            markdown_to_html_with_toc("## Café Notes\n", None, &MarkdownOptions::default());
        // Transliterated to ASCII.
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
