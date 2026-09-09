# Salvo examples

Worked examples of the language as it is *today*. Each one is a program that
compiles and runs on both backends, checked in together with the code the
compiler generated for it and the output it prints.

| example | what it shows |
|---|---|
| [`iteration/`](iteration/) | every form of iteration: native container loops, a hand-written pass, `yield fn` generators, a combinator of your own, the sequence functions, and the lazy pair |
| [`defer-and-throw/`](defer-and-throw/) | `defer` at block exits, a linear resource released on every path, `throw`/`try` and the `Ok T \| Thrown M` outcome |
| [`qualifiers/`](qualifiers/) | where a qualifier claim comes from, what survives a call, and state versus provenance |

More will be added as features land. This tree replaced `experiments/`, which
held hand-written *prototypes* of designs not yet built; the prototypes' value
was the findings they produced, and those live in PROGRESS.md.

## Layout

Every example is one directory:

```
<example>/
├── README.md      what the program is chosen to show, and what to look for
├── salvo/         the Salvo source (the only hand-written code)
├── rust/          the generated Rust, exactly as `salvo compile` wrote it
├── kotlin/        the generated Kotlin, likewise
└── expected.txt   the stdout — byte-identical on both backends
```

The generated trees sit *beside* the sources rather than inside them: a `.rs`
or `.kt` file inside a source directory is indistinguishable from a
hand-written companion file [backend-companion], so it would be picked up by
the next build.

## Running and regenerating one

```bash
cargo run -- run --backend rust   --src examples/iteration/salvo
cargo run -- run --backend kotlin --src examples/iteration/salvo

# after a compiler or std change, refresh the checked-in output:
cargo run -- compile --backend rust   --src examples/iteration/salvo --target examples/iteration/rust
cargo run -- compile --backend kotlin --src examples/iteration/salvo --target examples/iteration/kotlin
cargo run -- run     --backend rust   --src examples/iteration/salvo > examples/iteration/expected.txt
```

`salvo run` needs the backend's own toolchain (`rustc` / `kotlinc`) on PATH.

## Conventions for adding or updating an example

- **The Salvo source must compile and run on *both* backends, to the same
  stdout.** That equality is the point of checking the output in: a divergence
  is a [backend-parity] defect, not an example bug.
- **Use `std` rather than rolling your own.** An example that hand-writes a
  list, a result type or a pass teaches the wrong thing and stops being
  evidence that std works. Write your own only where *that* is the subject
  (`iteration/` writes a pass by hand because the manual form is one of the
  forms it is showing).
- **Regenerate `rust/`, `kotlin/` and `expected.txt` in the same change as the
  source.** Stale generated code is worse than none: it is read as what the
  compiler does.
- **Old examples are not kept out of respect.** When a language change makes an
  example illustrate code that no longer works, rewrite it or delete it —
  there is no compatibility guarantee to document and no historical value in a
  program that does not compile. Deleting one is the expected outcome, not a
  loss; the decision log in PROGRESS.md is where history belongs.
- **Comment the source for a reader who knows some other language.** The
  comments are the explanation; the README says what to look for and why the
  shapes were chosen.
