# Reorganizing the tests (working document)

**Status:** rounds 1–3 decided (§6, 2026-10-28): TD1–TD12, nothing open.
Sequence steps 1 (T6) and 2 (T3 with C2) are done (§7); step 3 (T2: the
label rename with `@compilerrules`, then diagnostic ids) is in progress.
Started at
the user's request while PIPELINE.md Part II was paused (its §II.12 "Where it
stands"): this run spent about half its logged time on tests (since Part II's
A1: 6.3 of 13.1 hours).

**Charter.** As with PIPELINE.md: the calls are the user's, recorded as
dated rounds in §6. When the work lands, the reasoning goes to COMPLETED.md's
decision log, what is undecided to ROADMAP.md, the rules (diagnostic ids, the
test-data format) to LANGUAGE_SPEC.md and AGENTS.md, and this file is
deleted.

**Goal (user, 2026-10-28).**

1. **Fewer tests.** Reorganize so the suite costs less, rather than relying
   only on stamps and caches. The largest win is the sheer number of tests:
   group the programs that must check clean, and group the programs whose
   expected errors do not exclude one another. Keep today's pattern: write
   code, then list every diagnostic it produces.
2. **One way to run a snippet.** Shared helpers for running Salvo code and
   collecting its diagnostics, in place of the near-identical prelude and
   helper at the top of most test files.
3. **std checked once.** Does every test check std? Should tests share one
   checked std instead of hand-written preludes?
4. **Salvo-level tests.** Move what can move into `.test.sv` files.
5. **Less brittle assertions** (a second wave). Messages should not be
   repeated in tests: assert a stable diagnostic id, as rustc's error codes
   allow, and keep the id → message text in one place.

---

## 1. The suite today (measured 2026-10-28, HEAD 18073516)

**Size.** 2,156 tests in 91 test binaries. One warm `cargo nextest run`
(right after a `salvo-core` rebuild, so the CLI stamps missed): 208s wall,
1,956s summed. By duration: 1,330 tests under 0.1s, 576 at 0.1–1s, 169 at
1–5s, 81 over 5s.

**Where the summed time goes:**

| binary | tests | summed s | what it is |
|---|---|---|---|
| `salvo-cli` run_tests | 71 | 673 | spawns `salvo run`, often on both backends |
| rust codegen_tests | 367 | 279 | emitted-text `contains` assertions; 252 `run_rust_files` compile-and-run sites |
| `salvo-cli` platform_tests | 23 | 271 | host projects, companions, gradle/cargo |
| kotlin codegen_tests | 115 | 194 | emitted text; 4 shards over `KOTLIN_CASES` (~250–300 cases) |
| `salvo-cli` test_command_tests | 13 | 140 | `salvo test` itself; `std_tests_pass` alone 58s |
| `salvo-cli` analyze_tests | 70 | 59 | |
| salvo-core corpus_tests | 2 | 57 | every codegen program built (stamped warm) |
| salvo-core, ~70 prelude-based files | ~1,100 | ~60 | the checker tests |
| salvo-core, real-std files | ~70 | ~140 | diag, carry, stamp_signature, ir_linear, flow, golden, route_deadlock |
| salvo-syntax parser_tests | 183 | 13 | |

So **the CLI and codegen binaries are about 84% of the summed time** in a
few hundred tests. The ~1,100 checker tests cost little time each (a prelude
program checks in milliseconds), but they are most of the *count*: under
nextest each is a process, and every one carries its own copy of a fixture.

**Shapes** (estimated by classifying test bodies; salvo-core's tests):
about 550 assert specific diagnostics (by message substring), about 380
assert that a program checks clean, about 130–230 test internals. Elsewhere:
compile-and-run (Rust and Kotlin cases, the CLI's `…_on_both_backends`),
emitted-text `contains` assertions (739 in the two codegen files), insta
snapshots (parser, goldens).

**Duplicated fixtures.** 61 test files define their own
`errors`/`check`/`program`-style helper (`errors` alone in 39), and 69 build
a `SourceSet` by hand. They differ only in: which std (a prelude, several
prelude files, or the real std), one file or several (or a `.test.sv`
annex), how far the pipeline runs (parse, resolve, check, IR analysis),
which severities, and how diagnostics render (bare message, `file:line:
message`). The codegen files and corpus_tests have three more near-copies of
a "std parsed once" front.

**std per test.** 73 hand-written preludes in about 55 files (≥1,450 lines).
Each operator change of 2026-10-28 had to touch them (58 preludes gained
`eq`/`cmp`, 35 gained `plus`/…), because a fixture that models std must
declare whatever the language now calls. 15 files load the real std from
`lib/`; there, std is parsed once per binary at best, then expanded,
resolved and **checked again for every program** (~0.25s each): an estimated
400–500s summed per fresh run. Nothing in the tests uses the language
server's `StdCache` / `check_project`, which checks only the project's files
over an already-checked std.

**`salvo test`.** 19 `.test.sv` files, 146 `test "…" { … }` blocks, all in
std, run on both backends. A test fails by a trap at run time; there is no
way to say "this must fail to compile with …" (the harness builds one program
from every block, so a block that does not compile breaks the module).

**Diagnostics.** `Diagnostic` (salvo-syntax) and `FileDiagnostic`
(salvo-core) carry severity, message, span, import suggestions and the
declaration; the IR and flow passes have their own diagnostic types. **None
carries a code.** Rule labels appear in some messages (`[op-unique-shape]`) but
not consistently; about 1,000–1,300 test assertions match message text
(about 2,500 `contains`/`starts_with` calls in all, many on emitted code), and
only 57 match a label.

**The per-piece method** (AGENTS.md and PIPELINE.md's practice) adds to
this: a temporary differential running two fresh full suites (old and new),
then a warm nextest before each commit (3.5–7 minutes after a core or std
change, since every stamp misses), then a final `cargo test` that repeats
it.

## 2. What other compilers do

The common pattern is to treat tests as **data files, not code**: one
runner, many source files, and the expectations written in the source at the
line they are about.

- **rustc's UI tests** (compiletest): each test is a `.rs` file; diagnostics
  are expected by annotations in comments at the line (`//~ ERROR` with a
  message fragment, `//~^` for the line above, `//~v` for the line below),
  and the full rendered output is also compared with a checked-in `.stderr`
  file, which a `--bless` run rewrites. Errors carry codes (`E0308`), whose
  explanations live in one place; a test can annotate the code rather than
  the text. ([rustc-dev-guide: UI tests](https://rustc-dev-guide.rust-lang.org/tests/ui.html),
  [compiletest](https://rustc-dev-guide.rust-lang.org/tests/compiletest.html),
  [error codes](https://rustc-dev-guide.rust-lang.org/diagnostics/error-codes.html),
  [test infra, March 2025](https://blog.rust-lang.org/inside-rust/2025/04/13/test-infra-mar-2025/))
- **Clang's `-verify`**: the compiler itself captures its diagnostics instead
  of printing them and compares them with `expected-error {{…}}` /
  `expected-warning` comments in the source (with `@+1`-style line offsets);
  a file that should produce none must say `expected-no-diagnostics`. Each
  Clang diagnostic has a unique ID beside its English text.
  ([Clang internals manual](https://clang.llvm.org/docs/InternalsManual.html),
  [the `expected-no-diagnostics` change](https://discourse.llvm.org/t/significant-change-to-verify-feature/24869))
- **Go's `errorcheck` tests**: a test file says how it is run in its first
  line, and each line that should fail carries `// ERROR "regexp"`.
  ([errchk](https://github.com/LilithGames/golangci-lint/blob/v1.45.0/test/errchk.go))
- **Kotlin's diagnostic tests**: `testData/diagnostics/*.kt` files mark the
  exact range a diagnostic covers with its *name*, `<!UNRESOLVED_REFERENCE!>x<!>`;
  a generator turns the directory into test methods, and codegen "box" tests
  are files whose `box()` must return `"OK"`.
  ([compiler plugin template](https://github.com/demiurg906/kotlin-compiler-course-examples))

What they share, and what Salvo lacks today: **a diagnostic id** asserted
instead of the message (rustc's codes, Clang's IDs, Kotlin's names); **many
cases per file**, since a well-recovering checker reports independent
mistakes independently; **one runner per kind of test**, so a new test is a
file, not a Rust function; and a **bless** mode that rewrites expectations
after an intended change. Content was rephrased for compliance with
licensing restrictions.

## 3. Options

### T1. Test data: `.sv` files with inline expectations, one runner

Checker tests become `.sv` files under a test-data tree (say
`tests/check/<area>/*.sv`), each holding many cases, with each expected
diagnostic written at its line:

```
fn f(a: Pt, b: Pt) [] -> Bool => a, b { return true }   //~ error[op-unique-shape]
fn g() -> Int { return h(1) }                           //~ error[call-resolve]
```

One Rust runner per tree discovers the files (as corpus_tests already does
for inline programs), checks each **once** over a shared std (T3), and
compares the produced diagnostics with the annotations: every expected one
present at its line, nothing unexpected. A file with no annotations asserts
that it checks clean (Clang's `expected-no-diagnostics`, here the default).

- **Grouping, as the user put it.** Clean programs merge freely (distinct
  names). Error programs merge when their errors do not exclude one another;
  the checker's "one mistake, one diagnostic" rule ([type-unknown-lenient])
  and per-declaration gating ([ir-analyze]) make errors in different
  declarations independent. What cannot merge: parse errors (one stops the
  file; they stay in parser tests or get a file each), errors whose
  *absence* elsewhere is the point, and program-wide analyses (deadlock,
  duplicate declarations) — those keep a file of their own. A split marker
  (`//--- second case`, like Kotlin's `// FILE:`) gives several isolated
  programs in one data file where isolation is needed.
- **Multi-file cases**: `// FILE: lib.sv` sections, so module and import
  tests stay one data file.
- **Expected output**: compile-and-run cases are data files too, with
  `//~ stdout` or a `.out` file beside, run on both backends by one runner
  that batches them (as the Kotlin shards do).
- **Count**: ~1,100 checker tests would become a few hundred cases in tens
  of files; the Rust test functions become one per data directory (or per
  file, for useful nextest names).

**Annotations above the code, or a blessed file (TD2).** The same cases in
the two styles. Annotations: each expectation sits on the line before the
code it checks (`//~v`, as rustc's compiletest spells "next line"); a case
with no annotation must check clean; any diagnostic not annotated fails the
file.

```
// tests/check/operators/shape.sv

//~v error[op-unique-shape:bare-type-param]
fn eq<T>(a: T, b: T) [] -> Bool => a, b { return true }

//~v error[call-resolve:no-callee]
fn g() -> Int { return h(1) }

fn fine(a: Int) -> Int { return a + 1 }
```

A blessed file: the `.sv` carries no expectations, and a `shape.diag` beside
it holds the full rendered output, written by a `--bless` run and reviewed
as a diff:

```
error[op-unique-shape:bare-type-param]: `eq` declares `==` over a
  bare type parameter `T`; a shape needs a concrete type
  --> shape.sv:4:1
error[call-resolve:no-callee]: no fn `h` is visible here
  --> shape.sv:7:24
```

| | annotations | blessed file |
|---|---|---|
| what is expected | stated by the author, at the code | whatever the compiler printed when blessed |
| message wording | not repeated; a rewording changes no test | in every file that shows it; a rewording reblesses many files |
| adding a case | add code and its annotation | lines shift, so every later location in the `.diag` changes |
| reviewing | reads as the intent | a diff of output; an accidental diagnostic looks like an intended one |
| what it catches beyond the id | nothing (fragment optional) | rendering: spans, notes, wording |

rustc keeps both, and its `.stderr` churn is the brittleness §Goal 5 wants
gone. **Decided (TD2, round 2):** annotations above the line, and no
blessed file: the tests are about which diagnostics appear, not their
wording. The wording lives in one messages file (T2).

### T2. A stable diagnostic id

Add a code to every diagnostic (`FileDiagnostic::code`, and the IR's and
flow pass's), set at the emission site; the message text lives once, there.
The natural id is the **rule label** it already cites (`unique-shape`,
`call-resolve`): stable, already cross-referenced to LANGUAGE_SPEC.md and the
code, and greppable. Tests (T1's annotations) assert the id and the line,
optionally a message fragment where the text itself is the point.

- **User-visible?** rustc prints `error[E0308]`; Salvo could print
  `error[op-unique-shape]`, and many messages already end with the label. Or the
  id could be internal, for tests and the language server only.
- **Granularity**: one label covers several messages (a rule has several
  ways to fail). Either a sub-id (`unique-shape/bare-type-param`) or "the
  label plus an optional fragment" in tests.
- **Cost**: ~240 `format!` sites in check.rs alone, plus resolve, comptime,
  the IR passes; mechanical, and checkable (a test that every diagnostic has
  an id whose label exists in LANGUAGE_SPEC.md).
- **Decided (TD1)**: the id is `<label>:<diagnostic-name>`, the label
  renamed to `<feature>-<rule>` (`op-unique-shape`), the name one per
  message (`op-unique-shape:bare-type-param`). Today LANGUAGE_SPEC.md
  defines ~665 distinct labels, and the code cites 679 distinct ones in
  ~10,600 places; the rename is a mapping table and a scripted sweep
  (TD7 asks how far it goes).
- **Decided (round 2): one messages file.** Every diagnostic's text comes
  from a function in a single file, named deterministically after its id:
  `op-unique-shape:bare-type-param` is
  `op_unique_shape_bare_type_param(…)`, so
  `grep op.unique.shape.bare.type.param` finds the id in specs and
  tests and the function together. Rewording a message touches that file
  only. Engineering under the decision:
  * Each function returns the id with the text (a `Message { id, text }`),
    so the emission site cannot pair a text with the wrong id; a unit test
    checks every function's name against its id (`-` and `:` to `_`) and
    every id's label against LANGUAGE_SPEC.md and the backend specs.
  * Rust has no overloading, so a diagnostic with several forms takes an
    argument (an `Option` or a small enum) rather than a second function,
    keeping function and id one to one. A form different enough to need
    its own name is its own diagnostic.
  * Where it lives: `salvo_syntax::messages`, re-exported by `salvo-core`,
    since the parser's diagnostics need it too and salvo-syntax is the crate
    everything depends on. Backend diagnostics (`kt-…`, `rs-…`) cannot
    live there under the layering rule, so each backend crate has its own
    `messages.rs` in the same form.
  * Messages lose the `[label]` suffix many carry today.
- **Decided (TD7, round 2): ids are hidden by default.** Users should not
  see compiler rules unless they look for them: the CLI prints the message
  only, and `error[op-unique-shape:bare-type-param]: …` under a new
  `-v`/`--verbose` flag. (Today `verbose` is an internal argument —
  `salvo compile` passes `true`, `run` and `test` `false` — with no flag; the
  flag would gate those notes too.)
- **The build plan (2026-10-28, step 3b)**, from a survey of every emission
  site (~1,000: check.rs ~540 through `Checker::error/warn/error_if/
  error_unresolved`; parser ~162 and lexer 13 through `Parser::error`,
  `Lexer::error`; comptime ~78; resolve 29; refine 16; IR ~32 through
  `flow::report_*`, `check::report_one`, `Lower::error`; the backends ~108
  through `ModuleEmitter::error` and direct pushes, all plain strings):
  * `salvo_syntax::messages::Message { id, text }`, defined by a
    `messages!` macro that also lists every `(fn name, id)` for the
    name-matches-id test; `Diagnostic` and `FileDiagnostic` gain
    `id: Option<&'static str>`.
  * The constructors take `impl IntoMessage`, implemented for `Message`
    and — during the migration only — for `String`/`&str` (no id). The last
    step deletes the string impls, so the compiler lists whatever was not
    converted; then `id` stops being optional where it can.
  * A message rewritten on its way out (`Checker::error`'s operator and
    `for` rewording, `origin::render`'s prefix, `shown`'s key rewriting)
    keeps its id.
  * Rendering: `error[id]: text --> file:line:col` when ids are shown, as
    rustc prints codes; the CLI's global `-v` sets a process-wide switch in
    `salvo_syntax::diag` (the backends render their errors to strings before
    the CLI sees them, so a switch is simpler than threading a flag);
    `analyze --format json` adds `"id"` under `-v` too. The language server
    never sends it (TD11).
  * Each message loses its `[label]` suffix as it moves; a test that matched
    a label matches the id instead (`salvo_testkit::front::Checked::ids`).
  * Order, one commit each, each green: (1, done) the machinery, `-v`, and the
    lexer, parser and desugar; (2, done) the core passes before the checker
    (expand, comptime, resolve, refine, deduce, unique_shape, platform,
    deadlock); (3, done) the checker; (4) the IR passes; (5) the backends and the
    CLI's own errors, then the string impls go.

### T3. std checked once per test process

Move the language server's std front (`StdFront`, `build_std_front` in
`salvo-cli/src/analysis.rs`) down into `salvo-core` (or a test-support crate
over it), and give every test helper a once-per-process checked std
(`OnceLock`) used through `check_project`. Then **real std becomes the
default fixture** and the hand-written preludes go, except where a test
genuinely needs a different std (std-shadow, `std = true`, annexes of std
modules, `check_std_deductions_total`, tests of std's own diagnostics), which
keep a full check or a small named fake std shared by name.

- Cost per program drops from ~0.25s (std re-checked) to the project's own
  check, and the preludes stop being a second, drifting copy of std's
  surface.
- Constraint: the cache's address-keyed rows require the program to share
  the very `Arc` modules, so the shared front hands out clones of the same
  `Arc`s, as the language server does.

### T4. Salvo-level tests

Move **runtime-behaviour** cases into `.test.sv` files run by `salvo test`
on both backends: the backends' runtime_tests (18), the std-behaviour
compile-and-run cases in the CLI's run_tests (console, streams, derived
parsers, qualifier constants), and the codegen cases whose point is output
rather than emitted shape (their emitted-text half stays, or becomes a
snapshot). Not movable: platform and host tests, `salvo test`'s own tests,
emitted-shape tests.

Negative tests ("this fails to compile with X") are T1's data files, not
`salvo test`: they are about the compiler, not the program. Whether `salvo
test` should also grow expected-diagnostic tests for users is a separate,
user-visible question.

### T5. The CLI and codegen tests

These binaries — CLI run, platform and `salvo test` tests, the two codegen
files, corpus, analyze, LSP, golden, the CLI's unit tests — are **~1,745s of
the 1,956s summed** in about 700 test functions (plus the 238
`KOTLIN_CASES`). T1–T4 as written barely touch
them, so they get their own steps (investigated 2026-10-28).

**Where their warm time goes.** Not the toolchains, mostly:

- **Stamps keyed on the binary.** The CLI tests' `e2e_stamp` and the corpus
  stamps are keyed on the `salvo` and test-binary fingerprints (size and
  mtime), so *every compiler rebuild* misses all of them, even when no
  generated byte changed. In the measured run ~1,100s of the 1,956s were
  those misses.
- **The front end before the stamp.** The codegen tests generate before
  they look at the stamp (`run_rust_files`, the `KotlinCase` builders), and
  each program re-checks std (~0.25s): ~14s per Kotlin shard even when
  every case hits.
- **Duplicates.** 214 programs appear verbatim in both codegen files, and
  215 run tests share name stems across them: each is generated, compiled
  and stamped twice, once per file.
- **Repeated subprocesses.** 48 run_tests cases run both backends (~13s
  each); `std_tests_pass` (58s) runs `salvo test --src lib` on both backends
  but gates on rustc only; `a_tests_output_is_shown_as_configured` (50s)
  runs `salvo test` seven times; analyze_tests and lsp_tests spawn a
  process per test (~0.85s each in analyze).

**What stays end-to-end.** About 30 run_tests cases test the CLI itself
(arguments, exit codes, output modes, project layout); the other ~33
(~420s) test language or std behaviour. Of platform_tests, ~8
boundary-behaviour cases (~150s) could batch with their companions; the rest
test host builds.

**Steps:**

- **C1. Content-keyed stamps.** A CLI test stamps on what it produced (the
  generated files, the expected output, the toolchain version, the test's
  own source, and a hash of `salvo-cli`'s sources for what the binary does
  after generating), not on the binary; a core change that leaves the
  output alone then hits. **Not a cheap first step** (found 2026-10-28,
  when starting it): a test can only key on content once it has generated
  it, so every CLI test would have to write its project and run
  `salvo compile` per backend *before* its stamp check — a rewrite of each
  of the ~100 bodies, most of which C3 and C7 move or delete anyway. So C1
  lands with them: the run runner keys on content by construction (as the
  codegen drivers do), and the CLI tests C7 keeps are restructured to
  compile first. The **corpus** stamps stay keyed on the test binary: what
  corpus_tests checks *is* the front end's output, so there is no cheaper
  content to key on; C2 makes it cheap instead. (An alternative for the
  CLI tests is a build cache in the CLI itself — skip `kotlinc`/`rustc`
  when the emitted files and the toolchain match a cached build, as
  [kt-std-library] does for std — but that is a user-visible feature, not
  a test change; not proposed.)
- **C2. The shared checked std (T3), used by the codegen tests**, and the
  stamp check moved ahead of everything but generation.
- **C3. A compile-and-run data runner.** `tests/run/**/*.sv` (T1's format,
  with `// FILE:` sections, companions, args, and the expected stdout, trap
  or exit code), run on both backends by one process, batched per backend.
  Into it go: the duplicated codegen programs (once, not twice), run_tests'
  ~33 behaviour cases, platform's boundary cases, the examples, and the
  corpus (as a stage of the same runner). Kotlin batches as the shards do;
  Rust batches one crate per batch (TD9). The format (TD8):
  ```
  // BACKENDS: rust kotlin
  // ARGS: a b
  fn main() -> None { … }
  //= stdout
  //= 3
  //= done
  ```
  with `//= exit 1` and `//= trap <id>` for the other outcomes, and
  `// FILE:` sections for companions and modules.
- **C4. Emitted-text tests** become emitted-module snapshots, or
  `//~ emits: …` annotations in the data files where one construct's
  spelling is the point (the second wave, brought forward for these files).
- **C5. Negative codegen tests** (a backend refusing a construct) become
  checker-style data files with ids (T1, T2).
- **C6. LSP and analyze**: ~10 spawned protocol tests stay; hover,
  definition, completion and analyze cases run in-process over the data
  files.
- **C7. Trim the CLI surface**: one both-backend parity test per feature;
  output modes asserted once per backend; `std_tests_pass` split per backend
  so each gates on its own toolchain.

**Estimate**: from ~1,745s summed to 150–250s, and from ~700 test
functions to about 100 (one per data directory plus the CLI's own). C3 is
the large one, and carries C1.

### T6. The per-piece method

Independent of the reorganization, and the quickest to change:

- The temporary differential runs every program the suite checks, twice,
  fresh. With T1's data files (and the corpus), the differential can run the
  data corpus only — a dump per file, old against new — instead of two full
  suites.
- The final `cargo test` after a green warm nextest repeats the same tests;
  AGENTS.md could require one of the two.

### Second wave: brittleness

With T2's ids in place: rewrite message-text assertions as id assertions;
replace the 739 emitted-text `contains` assertions in the codegen files by
snapshots of the emitted modules (insta, reviewed and blessed) where the
whole text is the contract, keeping `contains` only where one construct's
spelling is the point.

## 4. A possible sequence

1. T6 (method), at once: it costs nothing to change.
2. T3 (std once) and a shared fixture helper (one `salvo-testkit`-style
   builder: std real or fake, files and annexes, phase, rendered
   diagnostics), used by new tests first; C2 with it.
3. T2 (ids, with the label rename of TD7), so that T1's annotations have
   something stable to name.
4. T1 (check data files and runner), migrating the prelude-based checker
   files area by area, each migration deleting the Rust test file it
   replaces; C5 with it.
5. C3 with C1 (the run data runner, content-keyed), migrating the codegen
   run cases, the duplicated programs, run_tests' behaviour cases and the
   examples; T4 alongside, for what reads better as `.test.sv`.
6. C4, C6, C7 (the kept CLI tests content-keyed).
7. The second wave, for what is left.

Each step lands green, with the test inventory in COMPLETED.md updated; a
migration step proves it lost nothing by listing, per old test, the data
case that replaces it.

## 5. Decisions needed

TD1–TD12 are decided (§6, rounds 1–3); the entries below are kept as asked.

- **TD10 (T2).** The rename sweep rewrites every label cited in std's
  comments (1,124 citations in `lib/std`) anyway. Fold the user's planned
  `@compilerrule` doc-tag sweep into the same pass, so each citation is
  renamed and moved under the tag once? That would need the tag's shape
  first (a new [doc-tags] tag: hidden from hover unless asked for?). Or keep
  the two sweeps apart, rename first.
- **TD11 (T2).** The language server: send the id as the LSP diagnostic's
  `code` (editors show it dimmed after the message, in the problems list),
  or leave it out to match the CLI's default? Recommendation: leave it out;
  hover or a setting can show it later.
- **TD12 (T2).** The feature words of the rename. Today 726 labels are
  defined; 640 already start with a feature-ish prefix, but abbreviated or
  overlapping (`op-`, `cmp-`, `col-`, `qual-`, `kt-`), and 86 start with a
  word used once or twice (`unique-shape`, `elvis-guard`, `dead-code`). The
  proposal: one word per area, mostly docs/language's pages, every label
  renamed `<feature>-<rest>`, the old prefix dropped where it was the
  feature:

  | feature | today's prefixes |
  |---|---|
  | `operators` | op, cmp, elvis, inc, unique (`unique-shape`) |
  | `qualifiers` | qual, canbe, arm |
  | `types` | type, union, lit, narrow, is, rewrap, mixed, byte, bytes, buffer, checked, result, str, interp |
  | `structs` | struct, field, state |
  | `collections` | col, seq, elem, index |
  | `functions` | fn, call, param, default, lambda, free, main, bare, ident, name |
  | `implicits` | implicit |
  | `effects` | effect, handler, use, eff |
  | `actors` | actor, spawn, task, waitfor, remote, noremote, pool, route, monitor, protocol, wire, threadsafe, defer |
  | `deductions` | deduce, fate, copy, mut, readonly, ref, proj, flow, ownership |
  | `linear` | linear, obligation |
  | `control` | expr, if, when, loop, while, try, throw, terminal, cond, dead, unused, placeholder, var, let |
  | `iteration` | iter, for, yield |
  | `comptime` | comptime |
  | `modules` | mod, namespace, manifest, std, core |
  | `platform` | platform, host, intrinsic |
  | `testing` | test, assert, console |
  | `ir` | ir, node, origin, phase, facts |
  | `cli` | cli, args |
  | `lsp`, `docs`, `diagnostics`, `backends` | lsp, doc, diag, backend |
  | `files`, `time`, `network`, `streams`, `random`, `runtime` | fs, time, net + addr, stream, random, runtime |
  | `kotlin`, `rust` | kt, rs |

  So `[op-arith]` → `operators-arith`, `[op-unique-shape]` →
  `operators-unique-shape`, `[qual-erasure]` → `qualifiers-erasure`,
  `[kt-std-library]` → `kotlin-std-library`. The script writes the full
  726-row table for review before the sweep, and refuses a rename that
  collides. Open within it: `kt-`/`rs-` spelled out or kept short.

## 6. Rounds

**Round 3 (user, 2026-10-28).**

- **TD10.** Rolled in. The tag is `@compilerrules [<rule1>, <rule2>, ...]
  <comment>`, the comment optional (how the group of rules is implemented
  here); most uses only list rules. Hover hides it; go-to-definition shows the
  source, where it is.
- **TD11.** The language server does not send the id.
- **TD12.** Short feature words are fine (`op-arith`, `op-unique-shape`,
  `kt`, `rs`, `qual`); what matters is that every label **starts with its
  feature**. So the rename touches only the labels whose first word is not a
  feature (§7, the rename table), not the 640 that already lead with one.

**Round 2 (user, 2026-10-28).**

- **TD2.** Annotations above the line; no blessed file. Tests stay about
  which diagnostics appear, not their wording. The wording lives in a single
  messages file of functions named after the ids
  (`operators_unique_shape_bare_type_param`), so a rewording touches that
  file only and the id stays greppable (`operators.unique.shape.bare.type.param`).
  Engineering under it in T2.
- **TD7.** (a) one sweep renames every label. Users do not see compiler
  rules unless they look: ids print only under a new `-v` flag, otherwise
  the message alone. The user had already planned a sweep of std comments
  to a new `@compilerrule` doc tag (TD10).
- **TD8.** The recommended run-data format: header lines (`// BACKENDS:`,
  `// ARGS:`, `// FILE:`) and a trailing `//= stdout` block, with
  `//= exit N` and `//= trap <id>`.
- **TD9.** (a) one crate per Rust batch: std's modules once, each program a
  module, a dispatcher `main` running one program per process, a batch that
  fails to compile bisected.

**Round 1 (user, 2026-10-28).**

- **TD1.** A rule label is the diagnostic id, not a number, but the labels
  must be clearer: `<feature>-<rule>:<diagnostic-name>`, the label being
  what comes before `:` (`operators-unique-shape`). TD7 follows from it.
- **TD2.** The user asked how annotations above the code compare with a
  blessed file; answered by T1's example and table (decided in round 2).
- **TD3.** Yes: test in a shared file unless something prevents it, with
  exceptions where sensible (parse errors, program-wide analyses, behind a
  split marker).
- **TD4.** Yes: real std, checked once per process, is the default fixture.
- **TD5.** Yes: the general rule is `salvo test` where possible, but it is
  not universal; negative compile tests are compiler data files.
- **TD6.** The lighter option: the differential runs over the data corpus
  only, and one final run, not two — the warm nextest before a commit
  stands, and a `cargo test` after a green nextest is dropped.
- The user asked for a plan for the CLI and codegen tests: T5's C1–C7.

## 7. Work log

- **Survey (2026-10-28).** §1's numbers from `tmp/nextest.log` and a read
  of the test files; §2 from the rustc, Clang, Go and Kotlin sources linked
  there.
- **CLI and codegen investigation (2026-10-28).** T5's numbers, from
  `tmp/nextest.log` and a read of run_tests, platform_tests,
  test_command_tests, both codegen files and `salvo-testkit`'s stamps; the
  binary-keyed stamps (C1) were the main finding.
- **T6 (2026-10-28).** AGENTS.md: one final run (the warm nextest before a
  commit, else `cargo test`); PIPELINE.md's method: the differential over
  the data corpus. Committed with step 2.
- **C1 re-sequenced (2026-10-28)**, see T5: it cannot land before the
  tests generate ahead of their stamp check.
- **Step 2: T3 with C2 (2026-10-28).** `salvo_core::std_front` (moved from
  the CLI), emission over a front (`emit_program_over`,
  `check_for_emission_over`, `ir::dump_program_over`), and
  `salvo_testkit::front`; both codegen files, corpus and ten real-std checker
  files moved onto it. The differential over all 774 codegen programs found
  one gate missing (a std parsed again is not the cache's: `check_over` now
  requires shared modules) and then no difference. Measured: under `cargo
  test` Rust codegen 18 → 6.3s, Kotlin 16 → 5.5s, corpus 15 → 2.8s; under
  nextest 1,928 → 1,701 summed seconds, 208 → 186s wall. **Finding:** nextest
  runs each test in its own process, so the per-process front is rebuilt per
  test (~0.3s); it pays under nextest only where one test checks many
  programs. T1's grouping is what turns it into nextest time; the prelude
  files were not moved (T1 replaces them).
- **Step 3a: the rename and `@compilerrules` (2026-10-28).** Every rule
  label now leads with its feature (TD12): the 82 below were renamed
  (4,119 references, one scripted sweep over every tracked file except the
  user's `demo/`, OPERATOR_OVERLOAD.md and SLICES.md); the rest already did.
  `@compilerrules` is a doc tag (TD10, [doc-compilerrules]): hover leaves it
  out, a malformed one warns, and std's 644 documentation blocks that cited
  rules in their prose now cite them there (881 citations; two used a label
  as a noun and were reworded by hand). `docs_tests` checks that std's
  documentation cites rules only in the tag, and only rules a spec defines.
  Comments that document nothing (section headers, body comments, the
  platform host files) keep their inline citations: no user sees them. Six
  of those cite labels no spec defines (`backend-parity`, `cmp-canonical`,
  `col-duplicate-keys`, `col-insertion-order`, `mod-private`,
  `wire-encoding`), as before.

  | old | new |
  |---|---|
  | `arm-infer` | `qual-arm-infer` |
  | `bare-names` | `name-bare` |
  | `buffer-type` | `col-buffer` |
  | `byte-value` | `type-byte` |
  | `bytes-type` | `type-bytes` |
  | `canbe-optin` | `qual-canbe-optin` |
  | `checked-type` | `type-checked` |
  | `cond-bool` | `expr-cond-bool` |
  | `console` | `std-console` |
  | `core-layers` | `backend-core-layers` |
  | `dead-code` | `expr-dead-code` |
  | `defer-deduction` | `linear-defer` |
  | `eff-handle` | `effect-eff-handle` |
  | `elem-distinct` | `fate-elem-distinct` |
  | `facts-dump` | `cli-facts` |
  | `free-send-fn` | `actor-free-send-fn` |
  | `ident-resolve` | `name-ident-resolve` |
  | `if-else-none` | `expr-if-else-none` |
  | `inc-dec` | `op-inc-dec` |
  | `interface-pass` | `decl-interface-pass` |
  | `lambda-view` | `fate-lambda-view` |
  | `main-pool` | `actor-main-pool` |
  | `mixed-handler` | `actor-mixed-handler` |
  | `monitor-handler` | `actor-monitor-handler` |
  | `mut-lends` | `fate-mut-lends` |
  | `namespace` | `mod-namespace` |
  | `narrow-assign-reset` | `is-narrow-assign-reset` |
  | `noremote` | `net-noremote` |
  | `once-fn` | `fn-once` |
  | `origin` | `node-origin` |
  | `ownership-audit` | `deduce-ownership-audit` |
  | `path-type` | `fs-path-type` |
  | `phase-timings` | `cli-phase-timings` |
  | `placeholder` | `expr-placeholder` |
  | `random-default` | `std-random-default` |
  | `readonly-return` | `proj-readonly-return` |
  | `remote-backpressure` | `net-remote-backpressure` |
  | `result-errs` | `type-result-errs` |
  | `rewrap` | `type-rewrap` |
  | `route-stub` | `actor-route-stub` |
  | `safe-nav` | `op-safe-nav` |
  | `spawn-inherit` | `actor-spawn-inherit` |
  | `threadsafe-platform` | `platform-threadsafe` |
  | `unique-shape` | `op-unique-shape` |
  | `unused-var` | `var-unused` |
  | `while-value` | `loop-while-value` |
  | `wire-format` | `net-wire-format` |
  | `with-clause` | `effect-with-clause` |
  | `yield-proj` | `iter-yield-proj` |
  | `addr-capability` | `net-addr-capability` |
  | `addr-routable` | `net-addr-routable` |
  | `elvis` | `op-elvis` |
  | `elvis-guard` | `op-elvis-guard` |
  | `field-canbe-mut` | `struct-field-canbe-mut` |
  | `field-resolve` | `struct-field-resolve` |
  | `index-resolve` | `col-index-resolve` |
  | `for-elem-write` | `iter-for-elem-write` |
  | `for-iter` | `iter-for` |
  | `host-splice` | `platform-host-splice` |
  | `host-tool` | `backend-host-tool` |
  | `intrinsic-fn` | `platform-intrinsic-fn` |
  | `intrinsic-std-only` | `platform-intrinsic-std-only` |
  | `let-destructure` | `var-let-destructure` |
  | `let-infer` | `var-let-infer` |
  | `lit-adopt` | `type-lit-adopt` |
  | `lit-numeric` | `type-lit-numeric` |
  | `param-const` | `fn-param-const` |
  | `param-mode` | `fn-param-mode` |
  | `pool-fault-sink` | `actor-pool-fault-sink` |
  | `pool-retire` | `actor-pool-retire` |
  | `protocol-hash` | `actor-protocol-hash` |
  | `protocol-lock` | `actor-protocol-lock` |
  | `state-infer` | `handler-state-infer` |
  | `state-name` | `handler-state-name` |
  | `terminal-lambda` | `expr-terminal-lambda` |
  | `terminal-scope` | `expr-terminal-scope` |
  | `try` | `throw-try` |
  | `try-innermost` | `throw-try-innermost` |
  | `union-arm-identity` | `type-union-arm-identity` |
  | `union-spread` | `type-union-spread` |
  | `use-no-dup` | `effect-use-no-dup` |
  | `use-requires-use` | `effect-use-requires-use` |
- **Step 3b part 1 (2026-10-28).** The id machinery (`Message`,
  `messages!`, `IntoMessage`, the `id` field), `-v`, and the lexer's,
  parser's and desugarer's 179 messages with ids (labels judged per message
  where the text cited none; `syntax-expected` added for the generic
  ones). Parts 2–5 follow T2's order.
- **Step 3b part 2 (2026-10-28).** The core passes before the checker: 132
  messages with ids (comptime 70-odd, resolve, refine, deduce, deadlock,
  platform, naming). Labels for citation-less messages chosen from the
  code's own comments; deadlock reports are `actor-deadlock-cycle:*`. Four
  texts keep long space runs from their old literals (second wave).
- **Step 3b part 3 (2026-10-28).** The checker: 543 messages, ids through
  every rewording. Left: the IR passes (part 4), the backends and the CLI
  (part 5), then the text impl of `IntoMessage` goes.
