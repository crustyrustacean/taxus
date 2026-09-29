; Hand embedded script and style bodies to their own grammars.
;
; This is the feature that makes HTML highlighting worth the grammar:
; a `html` code block highlights its markup *and* the JavaScript and
; CSS inside it, because those bodies are parsed as the languages they
; actually are rather than as opaque HTML text.

((script_element
  (raw_text) @injection.content)
 (#set! injection.language "javascript")
 (#set! injection.include-children))

((style_element
  (raw_text) @injection.content)
 (#set! injection.language "css")
 (#set! injection.include-children))

; A `style="…"` attribute value is CSS too. `combined` keeps the
; attribute's surrounding quotes and whitespace as HTML while the value
; itself is highlighted as CSS.
((attribute
  (attribute_name) @_name
  (quoted_attribute_value (attribute_value) @injection.content))
 (#match? @_name "^(?i)style$")
 (#set! injection.language "css")
 (#set! injection.include-children))
