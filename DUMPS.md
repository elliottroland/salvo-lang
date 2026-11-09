# DUMPS.md — working document: dumps of the expanded program

Working doc for the user's 2026-11-09 request (resolve / type-check dumps show
what each step produced, expansions included). Retire it when done (AGENTS.md:
what is open → ROADMAP.md, the record → COMPLETED.md).

## The request, in short

- A shared AST printer: expanded `salvo_syntax::ast` modules → Salvo text,
  recording each printed node's output range (keyed by node identity:
  `Span::file` + `Span::node`, which every numbered node carries).
- The resolve dump (`names/dump.rs`) and type-check dump (`round_trip.rs`)
  rebuilt on it: they print the expanded, desugared module and place their
  annotations through the printer's node map instead of source offsets.
- Generated code marked with a leading `//` comment (stamped fn, hidden fn
  for a comptime call, …).
- The round trip keeps working on the printed expanded program.
- Shared spelling helpers with `ir/dump.rs`.
- Staging: (a) printer + stability test; (b) resolve dump; (c) type-check
  dump + round trip; (d) IR-dump sharing, docs, wiki, records.

## Design

- `crates/salvo-syntax/src/print.rs`: `print_module(&Module) -> Printed`
  with `Printed { text, nodes: HashMap<(u32, u32), NodeRange> }`, the range
  of every node with a numbered span (inner range, and for expressions the
  outer range including the parentheses the printer added by precedence).
- The parser drops grouping parentheses, so the printer parenthesizes by
  precedence (the parser's `infix_bp`).
- Annotation is post-hoc: each dumper builds edits at printed offsets
  (insert after a name, replace a range, wrap an expression) and renders
  them, as `round_trip.rs`'s `Writer::render` already does — only the
  offsets now come from the node map.
- Struct-attached fns (`scoped_to`, hoisted to module level by the
  desugaring) are printed back inside their struct's body, the only
  spelling that re-parses as attached.

## Towards `salvo fmt` (user remark, 2026-11-09)

Formatters such as gofmt and rustfmt parse and pretty-print; they keep
comments by attaching each to the nearest node. This printer is the core a
`salvo fmt` would need, but `fmt` would print the *unexpanded* tree and
would also need comments (the lexer keeps them in `lexed.comments`) and
blank lines between items. So the printer lives in `salvo-syntax`, works on
any `Module` (expanded or not, where the AST can still hold the written
form), and has no dependency on the checker. Comment attachment is not
built here.

## Status

(updated as work proceeds)

- [x] (a) printer + stability test (`print.rs`, `salvo_core::printed`, `print_tests.rs`; commit "Print a module's tree back as Salvo")
- [x] (b) resolve dump (`names/dump.rs` on `printed::print_file`; commit "Dump the resolved names of the expanded tree")
- [x] (c) type-check dump + round trip (`round_trip.rs` on the printer; commit "Dump the checker's choices in the expanded tree")
- [ ] (d) IR dump sharing, docs, wiki, records

## Findings / agent's calls

(appended as they come up)

- `name@[site]` in a stamped body prints as the bare name: printed into the
  site's file, that is what it means (agent's call).
- A lowered `?:`, `?.`, `?>` prints back as the operator: the `when` it
  lowers to has arm holes with no spelling (agent's call).
- `as!`/`as?` print as the desugared `if`, the block opening with a
  `// desugared from …` comment (recognized by the `as_N_v` name).
- A message-arm struct (`message_arm`) prints as a one-line comment: the
  re-expansion declares it again.
- `FieldDefault` (a comptime copy's `field.default`) has no spelling:
  `#{the default of T.f}` in dumps; `print_file_as_source` inlines the
  struct field's default expression (scope may differ; fine on the suite).
- The `iter fn` minter's `deductions: Some([])` (all-scalar params) has no
  spelling; prints as no clause.
- A variable's binding position `x#{l:c}` is now the line and column in
  the printed module text (the header line not counted), since an expanded
  node's span is its template's (agent's call).
- **The round trip cannot write stamped code as source** (found on the
  examples: `cli` and `collections`). A stamped body's names resolve in its
  template's file (`std.args`' private `add_one`, `is_command`, …), and its
  unrolled copies are scopes spliced flat (`let c` twice in one block). So
  `print_file_as_source` writes a `fn … by x` declaration as its original
  text and leaves out what an obligation's `by` or a comptime call stamped
  (the re-expansion makes it again); the dump shows all of it. Recorded as a
  finding; the stamped code's choices are therefore not round-tripped (as
  before), only dumped.
- Notes over generated fns print as `//` lines, so a re-parse reads them as
  docs.
