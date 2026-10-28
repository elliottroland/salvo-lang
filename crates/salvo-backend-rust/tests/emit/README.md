# Emit data (Rust)

Each `<area>/*.sv` here is emitted by the Rust backend over the real std, and
what it emits must be what its annotations say ([test-emit-data] in
LANGUAGE_SPEC.md; the format and runner are `salvo_testkit::data`, the test
binary `emit_data.rs`, one test per area listed in its `areas!` line). The
Kotlin backend's are in `crates/salvo-backend-kotlin/tests/emit/`, same
format.

```
fn twice(n: Int) -> Int {
    //~ emits: pub fn twice(mut n: i32) -> i32 {
    //~|     return i32::wrapping_mul(n, 2i32);
    return n * 2
}
```

- **`//~ emits: text`**: the program's own emitted files (not std's modules
  or the runtime's) hold `text`, exactly, spacing included. `//~|` lines
  continue it, one emitted line each. `//~ emits(main.rs): text` looks in
  one emitted file (any, std's or the runtime's included); `//~ emits 2:
  text` asks for exactly two occurrences; `//~ !emits: text` for none. An
  empty `//~ emits(path):` asserts only that the file is emitted.
- **`//~ refused[id]`**: emission fails, with exactly these ids (one
  annotation per error reported).
- An annotation sits by the construct it is about, but only its program is
  checked, not its line. `// FILE:` and `//---` split files and programs as
  in the checker's data; a program here must check clean (a checker
  diagnostic is the checker's data, `salvo-core/tests/check/`).
- **What is asserted is the point of the test**, not the whole module: one
  construct's spelling. A whole module as the contract is a snapshot
  (`golden_*` in `codegen_tests.rs`). What a program *does* is run data
  (`salvo-cli/tests/run/`).
- Every program here that is not refused is also built to IR by
  `salvo-core`'s `corpus_tests` ([ir-build]).
