; JavaScript highlight query.
;
; Uses only the 24 capture names in `engine.rs::HIGHLIGHT_NAMES`, so the
; shipped light/dark themes light these up with no style changes.

; Literals

(number) @number

(this) @variable.builtin
(super) @variable.builtin

[
  (true)
  (false)
  (null)
  (undefined)
] @constant.builtin

(regex) @string.special
(template_string) @string
(string) @string

; Keywords

[
  "as"
  "async"
  "await"
  "break"
  "case"
  "catch"
  "class"
  "const"
  "continue"
  "debugger"
  "default"
  "delete"
  "do"
  "else"
  "export"
  "extends"
  "finally"
  "for"
  "from"
  "function"
  "get"
  "if"
  "import"
  "in"
  "instanceof"
  "let"
  "new"
  "of"
  "return"
  "set"
  "static"
  "switch"
  "throw"
  "try"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
] @keyword

; Function and method definitions — named first, so the name reads as a
; function rather than a variable.

(function_declaration
  name: (identifier) @function)

(method_definition
  name: (property_identifier) @function)

; A call whose callee is an identifier we have not already classified.
; Ordered after definitions: later queries win in tree-sitter-highlight,
; so plain identifier calls read as function calls while bindings read as
; variables.
(call_expression
  function: (identifier) @function)

; A `function` or `class` expression's own name.
(variable_declarator
  name: (identifier) @variable
  value: [(function_expression) (arrow_function) (class)])

(class_declaration
  name: (identifier) @type)

; Properties and members

(property_identifier) @property
(member_expression
  property: (property_identifier) @property)
(assignment_expression
  left: (member_expression
    property: (property_identifier) @property))

; Types (JSDoc and TS-in-JS comments aside, capitalised identifiers read
; as types — same convention as the Rust query).
((identifier) @type
 (#match? @type "^[A-Z]"))

; Variables — identifiers not already claimed above.

(identifier) @variable

; Punctuation and operators

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "=="
  "==="
  "!="
  "!=="
  "<"
  "<="
  ">"
  ">="
  "&&"
  "||"
  "!"
  "??"
  "?"
  ":"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "=>"
  "..."
  "++"
  "--"
] @operator
