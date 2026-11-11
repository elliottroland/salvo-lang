# Checker test data

Each `<area>/*.sv` here is checked over the real standard library, and what
it reports must be exactly what its annotations say ([test-data] in
LANGUAGE_SPEC.md; the runner is `salvo_testkit::data`, the test binary
`check_data.rs`). One test per area.

```
//~ error[op-promote:mixed-classes]
let _a = 1 + 2.5
```

- **An annotation** is a line of its own, `//~ error[id]` or
  `//~ warning[id]`, and names the next line that is not an annotation;
  several may stack on one line. Nothing follows the id: tests assert ids,
  never message text ([diag-id]; the text is `messages.rs`'s).
- **`//~ type[T]`** asserts the `type` diagnostic a `type?(place)` reports
  on the next line ([type-query]): the one annotation whose payload is a
  type rather than an id. `T` is parsed as a type (brackets balance, so an
  effect list may appear) and compared as a type, not as text: qualifier
  order, union arm order, `T?` against `T | None` and whitespace do not
  matter. A claim compares by its spelling here (`Idx(str)` is not
  `Idx(parsing.str)`); whether two spellings are one value is
  `assert!(place: T)`'s to say ([assert-type]).
- **Exactly**: every diagnostic the program reports — errors, warnings and
  `type` reports, parse, checker and IR analyses — must be annotated, at
  the line its span starts on, as many times as it is reported. A file with
  no annotations checks clean, so a stray `type?` fails it. Prefix a local
  nobody reads with `_`, or the unused warning must be annotated (a local
  only named in `type?` or `assert!(x: T)` is unread).
- **`// FILE: name.sv`** starts another file of the same program (before
  the first, the file is `main.sv`); a `.test.sv` name is a test annex.
- **`//--- name`** starts another program, checked on its own. Share a
  program unless something prevents it (user decision, TESTS.md TD3): a
  parse error, a program-wide analysis (deadlock, duplicate declarations), or
  a name another case declares.
- A failure lists, per program, what was annotated and not reported and what
  was reported and not annotated, with the message, ready to read.

What annotations cannot say — which function a call went to, what a table
records — stays in a Rust test beside the area (`op_tests.rs` for
`operators/`), over the same std (`salvo_testkit::front`).
