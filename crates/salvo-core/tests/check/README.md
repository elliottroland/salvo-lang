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
  type rather than an id. `T` must parse as a type (brackets balance, so an
  effect list may appear), and the checker judges it where the `type?` is,
  exactly as `type!(place: T)` would there ([type-assert]): its names are
  resolved in the scope at the `type?`, qualifier and union arm order do
  not matter, and a claim is compared by the value it is about, not by its
  spelling. So `Idx(cursor.str)` holds of a `pos` destructured from
  `cursor` (which `type?` prints `Idx(str)`), and `Idx(str)` does not hold
  where that `str` is another value's. An annotation naming something that
  is not in scope at the `type?` fails the file, saying so.
- **Exactly**: every diagnostic the program reports — errors, warnings and
  `type` reports, parse, checker and IR analyses — must be annotated, at
  the line its span starts on, as many times as it is reported. A file with
  no annotations checks clean, so a stray `type?` fails it. Prefix a local
  nobody reads with `_`, or the unused warning must be annotated (a local
  only named in `type?` or `type!(x: T)` is unread).
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
