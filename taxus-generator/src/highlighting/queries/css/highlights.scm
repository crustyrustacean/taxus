; CSS highlight query.
;
; Uses only the 24 capture names in `engine.rs::HIGHLIGHT_NAMES`.

; At-rules and keywords

(at_rule) @keyword

; Selectors

(class_selector) @type
(id_selector) @type
(attribute_selector) @attribute
(pseudo_class_selector) @attribute
(pseudo_element_selector) @attribute

; Declarations and values

(property_name) @property
(tag_name) @tag
((plain_value) @constant.builtin
 (#match? @constant.builtin "^(?i)(inherit|initial|revert|unset|auto|none)$"))

(plain_value) @string
(color_value) @string.special

; Functions and units

(function_name) @function
(integer_value) @number
(float_value) @number
(unit) @type

(important) @keyword

; Punctuation

[
  "{"
  "}"
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

[
  ","
  ":"
  ";"
] @punctuation.delimiter
