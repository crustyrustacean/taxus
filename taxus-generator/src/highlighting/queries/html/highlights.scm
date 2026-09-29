; HTML highlight query.
;
; Embedded JavaScript and CSS are handled by `injections.scm`, which
; hands `<script>` bodies to the javascript grammar and `<style>` bodies
; (and `style="…"` attribute values) to the css grammar. This file
; covers the markup itself.

(tag_name) @tag
(erroneous_end_tag_name) @tag

(attribute_name) @attribute
(quoted_attribute_value (attribute_value) @string)
(attribute_value) @string

(text) @variable

(comment) @comment

(doctype) @keyword
[
  "</"
  "/>"
  "<"
  ">"
] @punctuation.bracket

"=" @operator
