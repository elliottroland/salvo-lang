# Compile-and-run test data

Each `<area>/*.sv` here is a program — or several — that every backend it
names compiles and runs, and what it does must be what its directives say
([test-run-data] in LANGUAGE_SPEC.md; the format is `salvo_testkit::run`,
the runner `run_data.rs`). One test, `every_program_runs`, compiles and runs
every area's programs and every example under `examples/` as one plan, so
the batches are full (user decision 2026-10-29); a new area directory is
picked up by listing. `SALVO_RUN_AREAS=actors,types` runs only those areas
(`examples` names the examples), for the inner loop.

```
//--- a_program
// ARGS: alpha beta
import std.args

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    for arg in args() {
        println(arg)
    }
}
//= stdout
//= alpha
//= beta
```

- **`//--- name`** starts a program. Comments before the first one are the
  file's preamble, and comments after a program's outcome (a description of
  the next one) are allowed.
- **`// BACKENDS: rust kotlin`** limits a program to those backends (both
  when absent). Use it only for what one backend cannot do yet, and say why
  in a comment: the default is parity.
- **`// ARGS: a b`** is the program's command line.
- **The outcome** comes last: `//= exit N` (0 when absent) or
  `//= trap <text>` (the program stops at a trap whose message starts
  `salvo: <text>` — `unreachable at main:2:25`, the module and the line
  within the program's own file), then `//= stdout`, after which every `//=`
  line is one line of the exact stdout (`//= ` and one space stripped,
  trailing spaces kept; `//=` alone is an empty line). No `//= stdout`: the
  program prints nothing. Before `//= stdout`, a `//= stdin` block is what
  the program reads on standard input, and a `//= stderr` block its exact
  standard error (not compared without one); such a block ends at the first
  line that is not `//=`, or at the next header. A header ending ` -n`
  (`//= stdout -n`) drops its block's final newline.
- **`// FILE: name`** starts another file: a module, a backend's companion
  (`helpers.kt`), a platform file (`platform/main.rs`) or a `salvo.toml`
  (the program is then read through its manifest, as `salvo compile` reads a
  project — which is how a platform root is named). Before the first, the
  file is `main.sv`. The outcome lines land in the last file, so a
  `salvo.toml` is never the last.
- **Stamps**: a program whose generated code, outcome and toolchain are
  unchanged is not compiled again (`SALVO_E2E_FRESH=1` ignores the stamps;
  `SALVO_SKIP_E2E=1` skips the runner).
- **Working directory**: each run starts in a fresh, empty directory of its
  own, so a file a program writes at a relative path (`path("out.txt")`)
  never meets another run's — the two backends run a program at once.
- Every program here is also built to IR by `salvo-core`'s `corpus_tests`
  ([ir-build]).
