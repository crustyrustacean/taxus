// taxus-generator/src/build/pipeline/shortcodes.rs

//! Shortcode expansion (stage 3, emit): `{{ name(arg="x") }}` inline
//! shortcodes and `{{% name %}}…{{% /name %}}` block shortcodes in
//! Markdown content, rendered by Tera with a deliberately minimal
//! context. See the book's
//! [Shortcodes](https://crustyrustacean.github.io/taxus/shortcodes.html) chapter.

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
// Phase A lands the span machinery with tests but no callers yet:
// Phase B's expansion engine and the text-path integrations consume
// these. `expect` (not `allow`) so the cleanup is compiler-enforced —
// remove the attribute with the first caller.
#[cfg_attr(not(test), expect(dead_code))]
fn strip_shortcode_spans(content: &str) -> String {
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
#[cfg_attr(not(test), expect(dead_code))]
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
#[cfg_attr(not(test), expect(dead_code))]
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
#[cfg_attr(not(test), expect(dead_code))]
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
#[cfg_attr(not(test), expect(dead_code))]
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
#[cfg_attr(not(test), expect(dead_code))]
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
#[cfg_attr(not(test), expect(dead_code))]
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

#[cfg(test)]
mod tests {
    // The tests for `strip_shortcode_spans` are written first (RED):
    // they define the contract before the implementation exists.

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
