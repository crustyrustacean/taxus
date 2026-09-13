// taxus-generator/src/build/pipeline/shortcodes.rs

//! Shortcode expansion (stage 3, emit): `{{ name(arg="x") }}` inline
//! shortcodes and `{{% name %}}…{{% /name %}}` block shortcodes in
//! Markdown content, rendered by Tera with a deliberately minimal
//! context. See the book's
//! [Shortcodes](https://crustyrustacean.github.io/taxus/shortcodes.html) chapter.

use crate::content::Page;
use crate::error::{GeneratorError, Result};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;
use tera::{Context, Tera};

use super::internal_links::{code_block_ranges, in_ranges};

/// Remove complete shortcode spans from Markdown text, leaving
/// everything else byte-for-byte intact.
///
/// The text paths — summaries, word counts, reading times, the search
/// index — consume `raw_content` and must not see shortcode markup:
/// `{{ image(alt="a sunset") }}` is not prose. This helper blunts those
/// spans out *without* expanding them (expansion belongs to the HTML
/// path in `process_content`; here only cleanliness matters).
///
/// What counts as a span:
///
/// - inline: `{{ name( … ) }}` — an opening `{{ name(` finds its matching
///   `)` followed by optional whitespace and `}}`, with quote-awareness
///   so parentheses inside string arguments do not end the span
/// - block: `{{% name %}}` through the first `{{% /name %}}`
///
/// Anything else — plain braces, `{{}}`, a name without a call or block
/// pair, an unclosed span — is left alone. Malformed shortcodes are the
/// expansion pass's error to report; this helper only ever removes what
/// is unambiguously a complete shortcode.
pub(crate) fn strip_shortcode_spans(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;

    while i < content.len() {
        // Inline span: `{{ name(` … `}}`
        if bytes[i..].starts_with(b"{{")
            && let Some(open_end) = inline_open_len(&content[i..])
            && let Some(total) = inline_span_len(&content[i + open_end..])
        {
            out.push(' ');
            i += open_end + total;
            continue;
        }

        // Block span: `{{% name %}}` … `{{% /name %}}`
        if bytes[i..].starts_with(b"{{%")
            && let Some(open_end) = block_open_len(&content[i..])
            && let Some(total) = block_span_len(&content[i + open_end..])
        {
            out.push(' ');
            i += open_end + total;
            continue;
        }

        // Not a shortcode start: copy one byte (multi-byte UTF-8 is
        // preserved because we only advance by ASCII-length jumps).
        let step = utf8_step(bytes, i);
        out.push_str(&content[i..i + step]);
        i += step;
    }

    out
}

/// Length of a syntactically valid shortcode name (letters, digits,
/// `-`, `_`; must start with a letter).
fn name_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut n = 0usize;
    while n < bytes.len()
        && (bytes[n].is_ascii_alphanumeric() || bytes[n] == b'-' || bytes[n] == b'_')
    {
        n += 1;
    }
    if n > 0 && bytes[0].is_ascii_alphabetic() {
        Some(n)
    } else {
        None
    }
}

/// If `s` starts an inline shortcode (`{{ name(`), the byte length of
/// that opening through the `(`.
fn inline_open_len(s: &str) -> Option<usize> {
    let after_braces = &s[2..];
    let rest = after_braces.trim_start();
    let ws = after_braces.len() - rest.len();
    let n = name_len(rest)?;
    let after_name = &rest[n..];
    if after_name.starts_with('(') {
        Some(2 + ws + n + 1)
    } else {
        None
    }
}

/// If `s` starts at the position just after `{{ name(`, the total length
/// through the closing `) }}` — quote-aware so `(`/`)` inside string
/// arguments do not terminate the span early.
fn inline_span_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut quote: Option<u8> = None;

    while i < bytes.len() {
        match quote {
            Some(q) => {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2; // escaped char inside a string
                } else if bytes[i] == q {
                    quote = None;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            None => match bytes[i] {
                b'"' | b'\'' => {
                    quote = Some(bytes[i]);
                    i += 1;
                }
                b')' => {
                    // `)` then optional whitespace then `}}`
                    let after = &s[i + 1..];
                    let trimmed = after.trim_start();
                    if trimmed.starts_with("}}") {
                        return Some(i + 1 + (after.len() - trimmed.len()) + 2);
                    }
                    return None;
                }
                _ => i += 1,
            },
        }
    }
    None
}

/// If `s` starts a block shortcode (`{{% name %}}`), the byte length of
/// that opening tag.
fn block_open_len(s: &str) -> Option<usize> {
    let after_braces = &s[3..];
    let rest = after_braces.trim_start();
    let ws = after_braces.len() - rest.len();
    let n = name_len(rest)?;
    let after_name = &rest[n..];
    let t2 = after_name.trim_start();
    if t2.starts_with("%}}") {
        Some(3 + ws + n + (after_name.len() - t2.len()) + 3)
    } else {
        None
    }
}

/// If `s` starts at the position just after a `{{% name %}}` opening,
/// the total length through the first `{{% /name %}}` (no nesting).
fn block_span_len(s: &str) -> Option<usize> {
    s.find("{{% /").map(|close| {
        let after_slash = &s[close + 5..];
        // Skip the name and require `%}}`; consume through it.
        let n = name_len(after_slash).unwrap_or(0);
        let rest = &after_slash[n..];
        let t = rest.trim_start();
        let extra = if t.starts_with("%}}") { 3 } else { 0 };
        close + 5 + n + (rest.len() - t.len()) + extra
    })
}

/// Advance one UTF-8 scalar from byte position `i`.
fn utf8_step(bytes: &[u8], i: usize) -> usize {
    let b = bytes[i];
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

// ===========================================================================
// Expansion engine
// ===========================================================================

/// Renders shortcodes with Tera over a minimal, deliberate context.
///
/// The registry is the filesystem: every `shortcodes/*.html` file is a
/// shortcode (loaded by [`Self::load_dir`]), plus two built-ins
/// (`image`, `youtube`) defined in code. A file colliding with a
/// built-in is an error — the built-ins are load-bearing names, and a
/// silent override is exactly the class of surprise this project
/// rejects everywhere else.
///
/// Tera is instantiated with the taxus filters (`term_slug`, contrib
/// `slugify`, …) but *without* the site functions (`get_section`,
/// `get_page`, `island`): shortcodes run before the tree-derived
/// context exists, and islands are a different tier (runtime
/// interactivity, not content-time markup).
pub struct ShortcodeRenderer {
    /// Templates keyed by shortcode name.
    tera: Tera,
    names: Vec<String>,
}

impl ShortcodeRenderer {
    /// An empty renderer: built-ins only.
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();
        crate::templates::renderer::register_filters_only(&mut tera);

        let builtins = builtin_templates();
        for (name, tpl) in &builtins {
            // Registered as {name}.html so Tera's autoescaping applies.
            tera.add_raw_template(&format!("{name}.html"), tpl)
                .map_err(|e| {
                    GeneratorError::Template(Box::new(crate::error::TemplateError::Render(
                        format!("builtin shortcode {name}: {e}"),
                    )))
                })?;
        }

        Ok(Self {
            names: builtins.iter().map(|(n, _)| n.to_string()).collect(),
            tera,
        })
    }

    /// Register one shortcode template (tests and `load_dir`).
    pub fn add_template(&mut self, name: &str, tpl: &str) -> Result<()> {
        if self.names.iter().any(|n| n == name) {
            return Err(GeneratorError::UnknownShortcode {
                // Reuse the variant's shape; the message says what's wrong.
                file: format!("shortcodes/{name}.html"),
                name: name.to_string(),
            });
        }
        // Tera autoescapes templates whose name ends in .html; register
        // under {name}.html so argument values are HTML-escaped while the
        // registry (and the syntax in content) stays bare.
        self.tera
            .add_raw_template(&format!("{name}.html"), tpl)
            .map_err(|e| {
                GeneratorError::Template(Box::new(crate::error::TemplateError::Render(format!(
                    "shortcode {name}: {e}"
                ))))
            })?;
        self.names.push(name.to_string());
        Ok(())
    }

    /// Load every `*.html` under `dir` as a shortcode named by its file
    /// stem. A missing directory is not an error — no shortcodes.
    pub fn load_dir(&mut self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut loaded = 0usize;
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| GeneratorError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("html"))
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| GeneratorError::Io {
                    path: path.clone(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "non-UTF-8 shortcode filename",
                    ),
                })?
                .to_string();
            let body = std::fs::read_to_string(&path).map_err(|e| GeneratorError::Io {
                path: path.clone(),
                source: e,
            })?;
            self.add_template(&name, &body)?;
            loaded += 1;
        }
        Ok(loaded)
    }

    fn knows(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }

    /// Render one shortcode invocation.
    fn render(
        &self,
        name: &str,
        args: &BTreeMap<String, Value>,
        body: Option<&str>,
        page: Option<&Page>,
        site_name: &str,
        base_url: &str,
    ) -> Result<String> {
        let mut ctx = Context::new();
        let mut arg_map = Map::new();
        for (k, v) in args {
            arg_map.insert(k.clone(), v.clone());
        }
        ctx.insert("args", &Value::Object(arg_map));
        if let Some(body) = body {
            ctx.insert("body", body);
        }
        if let Some(page) = page {
            let mut p = Map::new();
            p.insert(
                "title".into(),
                Value::String(page.frontmatter.title.clone()),
            );
            p.insert(
                "description".into(),
                page.frontmatter
                    .description
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            p.insert("draft".into(), Value::Bool(page.frontmatter.draft));
            if let Some(date) = page.frontmatter.date {
                p.insert("date".into(), Value::String(date.to_string()));
            }
            ctx.insert("page", &Value::Object(p));
        }
        ctx.insert("site_name", site_name);
        ctx.insert("base_url", base_url);

        self.tera
            .render(&format!("{name}.html"), &ctx)
            .map_err(|e| {
                GeneratorError::Template(Box::new(crate::error::TemplateError::Render(format!(
                    "shortcode {name}: {e}"
                ))))
            })
    }
}

/// The built-in shortcode templates (v1: `image`, `youtube`).
///
/// `image` resolves `@/`-style refs to content-relative URLs (where
/// co-located assets live) and passes absolute URLs through — the `@/`
/// rewrite happens in [`rewrite_image_args`] before rendering, because
/// join semantics are a content concern, not template logic. Args are
/// HTML-escaped by Tera's default autoescaping.
fn builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "image",
            r#"<figure class="image-shortcode">
  <img src="{{ args.src }}"{% if args.alt %} alt="{{ args.alt }}"{% endif %}{% if args.class %} class="{{ args.class }}"{% endif %} loading="lazy">
</figure>"#,
        ),
        (
            "youtube",
            r#"<figure class="youtube-shortcode">
  <iframe
    src="https://www.youtube-nocookie.com/embed/{{ args.id }}"
    {% if args.title %}title="{{ args.title }}"{% else %}title="YouTube video"{% endif %}
    loading="lazy"
    allowfullscreen
    referrerpolicy="strict-origin-when-cross-origin"
    width="560" height="315"
    frameborder="0"></iframe>
</figure>"#,
        ),
    ]
}

/// `@/path` image sources resolve to the content-relative path — the
/// exact destination stage 5 copies co-located assets to.
fn rewrite_image_args(args: &mut BTreeMap<String, Value>) {
    if let Some(Value::String(src)) = args.get_mut("src")
        && let Some(stripped) = src.strip_prefix("@/")
    {
        *src = format!("/{stripped}");
    }
}

/// Expand every shortcode use in `content` outside code constructs.
///
/// Runs between internal-link resolution and Markdown rendering, so
/// shortcode output becomes part of the Markdown stream (and is wrapped
/// in paragraphs/blocks correctly). Uses inside fenced blocks, indented
/// blocks, or inline code spans are left verbatim (#6's immunity, via
/// the same parser-derived ranges).
///
/// # Errors
///
/// - [`GeneratorError::UnknownShortcode`]: a name with no built-in and
///   no `shortcodes/{name}.html`, reporting the content file and name.
/// - A malformed args list or unterminated block is a render error
///   naming the file and position.
pub fn expand_shortcodes(
    content: &str,
    source_file: &Path,
    renderer: &ShortcodeRenderer,
) -> Result<String> {
    expand_shortcodes_in_page(content, source_file, renderer, None, "", "")
}

/// [`expand_shortcodes`] with the page context (frontmatter) and site
/// identity available to shortcode templates as `page` / `site_name` /
/// `base_url`.
pub fn expand_shortcodes_in_page(
    content: &str,
    source_file: &Path,
    renderer: &ShortcodeRenderer,
    page: Option<&Page>,
    site_name: &str,
    base_url: &str,
) -> Result<String> {
    let code_ranges = code_block_ranges(content);
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;

    while i < content.len() {
        // Inline: `{{ name(args) }}`
        if bytes[i..].starts_with(b"{{")
            && let Some(open_end) = inline_open_len(&content[i..])
            && let Some(total) = inline_span_len(&content[i + open_end..])
            && !in_ranges(&code_ranges, i)
        {
            let name = &content[i + 2..i + open_end - 1];
            let name = name.trim();
            if !renderer.knows(name) {
                return Err(GeneratorError::UnknownShortcode {
                    file: source_file.display().to_string(),
                    name: name.to_string(),
                });
            }
            let arg_src = &content[i + open_end..i + open_end + total - 2];
            // Trim the closing `)` and whitespace up to `}}`.
            let arg_src = arg_src.trim_end().strip_suffix(')').unwrap_or(arg_src);
            let mut args = parse_args(arg_src, source_file)?;
            if name == "image" {
                rewrite_image_args(&mut args);
            }
            let html = renderer.render(name, &args, None, page, site_name, base_url)?;
            out.push_str(&html);
            i += open_end + total;
            continue;
        }

        // Block: `{{% name %}}` … `{{% /name %}}`
        if bytes[i..].starts_with(b"{{%")
            && let Some(open_end) = block_open_len(&content[i..])
            && let Some(total) = block_span_len(&content[i + open_end..])
            && !in_ranges(&code_ranges, i)
        {
            let name = block_name_at(&content[i..i + open_end]);
            if !renderer.knows(&name) {
                return Err(GeneratorError::UnknownShortcode {
                    file: source_file.display().to_string(),
                    name,
                });
            }
            // Body: from the end of the open tag to the start of the
            // closing `{{% /name %}}`. `block_close_start` locates it.
            let close_start = block_close_start(&content[i + open_end..])
                .expect("block_span_len matched, so a close exists");
            let body = &content[i + open_end..i + open_end + close_start];
            let html = renderer.render(
                &name,
                &BTreeMap::new(),
                Some(body),
                page,
                site_name,
                base_url,
            )?;
            out.push_str(&html);
            i += open_end + total;
            continue;
        }

        let step = utf8_step(bytes, i);
        out.push_str(&content[i..i + step]);
        i += step;
    }

    Ok(out)
}

/// The name inside a `{{% name %}}` opening tag of known length.
fn block_name_at(open_tag: &str) -> String {
    open_tag[3..]
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// Byte offset of the `{{% /name %}}` closing tag that `block_span_len`
/// found, relative to `s` (the open tag consumed by the caller). The
/// body is everything before this offset.
fn block_close_start(s: &str) -> Option<usize> {
    let close = s.find("{{% /")?;
    let after_slash = &s[close + 5..];
    let n = name_len(after_slash)?;
    let rest = &after_slash[n..];
    let t = rest.trim_start();
    if t.starts_with("%}}") {
        Some(close)
    } else {
        None
    }
}

/// Parse `k="v", n=3, b=true` into a Tera value map.
fn parse_args(src: &str, _source_file: &Path) -> Result<BTreeMap<String, Value>> {
    let mut args = BTreeMap::new();
    let bytes = src.as_bytes();
    let mut i = 0usize;

    loop {
        // skip whitespace and commas
        while i < bytes.len()
            && (bytes[i] == b' '
                || bytes[i] == b','
                || bytes[i] == b'\n'
                || bytes[i] == b'\t'
                || bytes[i] == b'\r')
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // key
        let key_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
        {
            i += 1;
        }
        if i == key_start {
            return Err(GeneratorError::Template(Box::new(
                crate::error::TemplateError::Render(format!(
                    "shortcode args: expected a key at byte {i} in `{src}`"
                )),
            )));
        }
        let key = src[key_start..i].to_string();
        // `=`
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(GeneratorError::Template(Box::new(
                crate::error::TemplateError::Render(format!(
                    "shortcode args: expected `=` after `{key}`"
                )),
            )));
        }
        i += 1;
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        // value: quoted string, number, or bool
        let (value, next) = parse_value(src, i)?;
        args.insert(key, value);
        i = next;
    }

    Ok(args)
}

fn parse_value(src: &str, i: usize) -> Result<(Value, usize)> {
    let bytes = src.as_bytes();
    let bad = |msg: String| -> GeneratorError {
        GeneratorError::Template(Box::new(crate::error::TemplateError::Render(format!(
            "shortcode args: {msg} in `{src}`"
        ))))
    };
    if i >= bytes.len() {
        return Err(bad("expected a value".into()));
    }
    match bytes[i] {
        b'"' | b'\'' => {
            let quote = bytes[i];
            let mut j = i + 1;
            let mut val = String::new();
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' if j + 1 < bytes.len() => {
                        val.push(bytes[j + 1] as char);
                        j += 2;
                    }
                    c if c == quote => {
                        return Ok((Value::String(val), j + 1));
                    }
                    _ => {
                        // Multi-byte safety: advance a full scalar.
                        let step = utf8_step(bytes, j);
                        val.push_str(&src[j..j + step]);
                        j += step;
                    }
                }
            }
            Err(bad("unterminated string".into()))
        }
        b't' | b'f' => {
            if src[i..].starts_with("true") {
                Ok((Value::Bool(true), i + 4))
            } else if src[i..].starts_with("false") {
                Ok((Value::Bool(false), i + 5))
            } else {
                Err(bad(format!("invalid literal at byte {i}")))
            }
        }
        c if c.is_ascii_digit() || c == b'-' || c == b'+' => {
            let start = i;
            let mut j = i;
            let mut is_float = false;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit()
                    || bytes[j] == b'.'
                    || bytes[j] == b'-'
                    || bytes[j] == b'+'
                    || bytes[j] == b'e'
                    || bytes[j] == b'E')
            {
                if bytes[j] == b'.' {
                    is_float = true;
                }
                j += 1;
            }
            let text = &src[start..j];
            if is_float {
                text.parse::<f64>()
                    .map(|f| {
                        (
                            Value::Number(
                                serde_json::Number::from_f64(f)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            ),
                            j,
                        )
                    })
                    .map_err(|_| bad(format!("invalid number `{text}`")))
            } else {
                text.parse::<i64>()
                    .map(|n| (Value::from(n), j))
                    .map_err(|_| bad(format!("invalid number `{text}`")))
            }
        }
        other => Err(bad(format!(
            "unexpected character `{}` at byte {i}",
            other as char
        ))),
    }
}

#[cfg(test)]
mod tests {
    // Tests are written first (RED): they define the contract before
    // the implementation exists.

    use super::strip_shortcode_spans;

    #[test]
    fn strips_inline_shortcode_span() {
        // The span is braces-to-braces; one space replaces it. The
        // text's own spaces around the span are preserved.
        let src = "before {{ youtube(id=\"abc\") }} after";
        assert_eq!(strip_shortcode_spans(src), "before   after");
        // Adjacent (no surrounding spaces): a single space appears.
        assert_eq!(strip_shortcode_spans("a{{ x() }}b"), "a b");
    }

    #[test]
    fn strips_block_shortcode_with_body() {
        let src = "intro\n{{% figure %}}\n![alt](img.png)\n{{% /figure %}}\noutro";
        assert_eq!(strip_shortcode_spans(src), "intro\n \noutro");
    }

    #[test]
    fn leaves_plain_braces_alone() {
        // Template-looking constructs that are not shortcodes (no name
        // call, no block pair) must pass through untouched — this is a
        // blunt text-path helper, not a parser.
        let src = "a { b } c {{}} d {{ not-a-shortcode }} e";
        assert_eq!(strip_shortcode_spans(src), src);
    }

    #[test]
    fn braces_inside_shortcode_args_do_not_confuse_stripping() {
        let src = r#"{{ quote(text="{{ literal }} braces") }}"#;
        assert_eq!(strip_shortcode_spans(src), " ");
    }

    #[test]
    fn multiple_spans_all_stripped() {
        // The block span covers open tag, body, and close — the body's
        // `y` goes with it.
        let src = "{{ a() }} x {{% b %}} y {{% /b %}} z {{ c(k=1) }}";
        assert_eq!(strip_shortcode_spans(src), "  x   z  ");
    }

    #[test]
    fn unclosed_span_left_alone() {
        // A malformed shortcode is the expansion pass's error to report,
        // not this helper's; stripping only removes complete spans.
        let src = "text {{ youtube(id=";
        assert_eq!(strip_shortcode_spans(src), src);
    }

    #[test]
    fn multibyte_text_around_spans_is_preserved() {
        let src = "café {{ note(t=\"中文\") }} naïve";
        assert_eq!(strip_shortcode_spans(src), "café   naïve");
    }
}

/// Tests for the expansion engine (Phase B), written first.
#[cfg(test)]
mod expansion_tests {
    use super::*;

    /// A renderer with the built-ins plus one custom template per test.
    fn renderer_with(custom: &[(&str, &str)]) -> ShortcodeRenderer {
        let mut r = ShortcodeRenderer::new().unwrap();
        for (name, tpl) in custom {
            r.add_template(name, tpl).unwrap();
        }
        r
    }

    fn file() -> &'static std::path::Path {
        std::path::Path::new("blog/post.md")
    }

    #[test]
    fn expands_inline_shortcode_with_args() {
        let r = renderer_with(&[("note", "<aside>{{ args.text }}</aside>")]);
        let out = expand_shortcodes("a {{ note(text=\"hi\") }} b", file(), &r).unwrap();
        assert_eq!(out, "a <aside>hi</aside> b");
    }

    #[test]
    fn args_parse_strings_numbers_bools() {
        let r = renderer_with(&[(
            "echo",
            "{{ args.s }}|{{ args.n }}|{{ args.b }}|{{ args.f }}",
        )]);
        let src = r#"{{ echo(s="x", n=3, b=true, f=1.5) }}"#;
        let out = expand_shortcodes(src, file(), &r).unwrap();
        assert_eq!(out, "x|3|true|1.5");
    }

    #[test]
    fn expands_block_shortcode_with_body() {
        let r = renderer_with(&[("wrap", "<div>{{ body }}</div>")]);
        let src = "{{% wrap %}}inner *text*{{% /wrap %}}";
        let out = expand_shortcodes(src, file(), &r).unwrap();
        assert_eq!(out, "<div>inner *text*</div>");
    }

    #[test]
    fn page_and_site_context_available() {
        let r = renderer_with(&[("ctx", "{{ page.title }}@{{ site_name }}")]);
        let page =
            crate::content::Page::from_str("+++\ntitle = \"My Post\"\n+++\nbody", "blog/post.md")
                .unwrap();
        let out = expand_shortcodes_in_page(
            "{{ ctx() }}",
            file(),
            &r,
            Some(&page),
            "Test Site",
            "https://example.com",
        )
        .unwrap();
        assert_eq!(out, "My Post@Test Site");
    }

    #[test]
    fn unknown_shortcode_is_an_error_naming_file_and_name() {
        let r = renderer_with(&[]);
        let err = expand_shortcodes("{{ nope() }}", std::path::Path::new("docs/guide.md"), &r)
            .unwrap_err();
        match err {
            crate::error::GeneratorError::UnknownShortcode { file, name } => {
                assert_eq!(file, "docs/guide.md");
                assert_eq!(name, "nope");
            }
            other => panic!("expected UnknownShortcode, got {other}"),
        }
    }

    #[test]
    fn shortcodes_inside_code_constructs_are_not_expanded() {
        let r = renderer_with(&[("boom", "EXPLODED")]);
        // Fenced block:
        let fenced = "```\n{{ boom() }}\n```\n";
        assert_eq!(expand_shortcodes(fenced, file(), &r).unwrap(), fenced);
        // Inline code span:
        let inline = "run `{{ boom() }}` now\n";
        assert_eq!(expand_shortcodes(inline, file(), &r).unwrap(), inline);
        // But the same use outside code DOES expand:
        assert_eq!(
            expand_shortcodes("x {{ boom() }} y", file(), &r).unwrap(),
            "x EXPLODED y"
        );
    }

    #[test]
    fn image_builtin_resolves_internal_ref_and_renders_figure() {
        // The image builtin treats @/ refs as co-located assets: the
        // `@/` prefix resolves to the file's content-relative path,
        // which is exactly where stage 5 copies co-located assets to.
        let r = renderer_with(&[]);
        let src = r#"{{ image(src="@/blog/sunset.jpg", alt="A sunset") }}"#;
        let out = expand_shortcodes(src, file(), &r).unwrap();
        assert!(out.contains("<figure"), "figure wrapper, got: {out}");
        assert!(
            out.contains("src=\"/blog/sunset.jpg\""),
            "internal ref resolved to content path, got: {out}"
        );
        assert!(
            out.contains("alt=\"A sunset\""),
            "alt text escaped through, got: {out}"
        );
    }

    #[test]
    fn image_builtin_external_url_passthrough() {
        let r = renderer_with(&[]);
        let src = r#"{{ image(src="https://example.com/pic.png", alt="x") }}"#;
        let out = expand_shortcodes(src, file(), &r).unwrap();
        assert!(out.contains("https://example.com/pic.png"), "got: {out}");
    }

    #[test]
    fn youtube_builtin_emits_nocookie_lazy_embed() {
        let r = renderer_with(&[]);
        let src = r#"{{ youtube(id="dQw4w9WgXcQ") }}"#;
        let out = expand_shortcodes(src, file(), &r).unwrap();
        assert!(out.contains("youtube-nocookie.com"), "got: {out}");
        assert!(out.contains("dQw4w9WgXcQ"), "got: {out}");
        assert!(out.contains("loading=\"lazy\""), "got: {out}");
    }

    #[test]
    fn html_in_args_is_escaped() {
        let r = renderer_with(&[("note", "<aside>{{ args.text }}</aside>")]);
        let src = r#"{{ note(text="<script>alert(1)</script>") }}"#;
        let out = expand_shortcodes(src, file(), &r).unwrap();
        assert!(!out.contains("<script>"), "must be escaped, got: {out}");
        assert!(out.contains("&lt;script&gt;"), "got: {out}");
    }

    #[test]
    fn adjacent_and_multiple_uses() {
        let r = renderer_with(&[("x", "[{{ args.v }}]")]);
        let out = expand_shortcodes("{{ x(v=1) }}{{ x(v=2) }}", file(), &r).unwrap();
        assert_eq!(out, "[1][2]");
    }
}
