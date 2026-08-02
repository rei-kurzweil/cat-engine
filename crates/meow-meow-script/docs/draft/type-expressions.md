# Type-expression grammar and AST

Status: canonical draft.

## Grammar

```text
TypeExpression = Union
Union          = Intersection ("|" Intersection)*
Intersection   = Postfix ("&" Postfix)*
Postfix        = Primary ("?")*
Primary        = Ident
               | "[" TypeExpression "]"
               | "[" TypeExpression ";" ConstLength "]"
               | "{" RecordTypeFields? "}"
               | "fn" "(" TypeList? ")" ":" TypeExpression
               | "(" TypeExpression ")"
TypeList       = TypeExpression ("," TypeExpression)*
RecordTypeFields = Ident ":" TypeExpression
                   ((",")? Ident ":" TypeExpression)* (",")?
ConstLength    = integer literal accepted by the static-layout evaluator
```

Precedence is primary/postfix and fixed-array forms first, then `&`, then `|`.
Thus `A | B & C?` means `A | (B & (C?))`. `[T]` is a variable-length,
slice-like sequence type. `[T; N]` has statically known length and layout.
`T?` is nullable postfix syntax. `fn(A, B): R` is the function-type form.

The initial AST has named/primitive, sequence, fixed array, anonymous record,
nullable, intersection, union, parenthesized, and function variants. Every
node carries a full span; field names and fixed lengths carry their own spans.
The parser preserves parentheses needed for source diagnostics, while the
canonical unparser adds only those required by precedence.

Dependency: tokenizer support for `:`, `?`, `&`, `|`, and type-context `;`.
Smoke test: parse and unparse one nested expression containing every variant.
Exit gate: the canonical output reparses to an equivalent AST and every bad
delimiter/operator reports the narrowest useful span.

