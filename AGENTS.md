# AGENTS.md — Working on the Salvo compiler

Guidance for AI agents (and humans) contributing to this repository.

## Read these first, always

1. **`LANGUAGE.md`** — the Salvo language specification. It is the source of
   truth for syntax and semantics; parser, checker, and backends are all
   built to match it. When behavior is ambiguous, LANGUAGE.md decides.
2. **`LANGUAGE_SPEC.md`** — the labeled-rule companion to LANGUAGE.md:
   every feature as a short rule with a stable label like `[qual-erasure]`,
   plus the compiler decisions under it. Rule labels are referenced from
   compiler code and tests — `grep -rn '\[rule-name\]'` jumps between the
   rule, its implementation, and its tests. When you add or change a
   feature: update the rule (or add one), and tag the implementation and
   tests with its label.
   **When working on a specific backend, also load its
   `BACKEND_SPEC.<backend>.md`** (e.g. `BACKEND_SPEC.kotlin.md`): it
   repeats core rules with backend interpretation details and adds
   backend-prefixed rules (`kt-…`). Backend-prefixed labels must only be
   referenced from that backend's crate; core crates
   reference backend-neutral labels only.
3. **`ROADMAP.md`** — what is left to do: **"The sequence"** (the agreed order
   of work), **open defects** (reproduced bugs with their root cause), the
   phases not yet built, and the decisions and plans already made about each,
   with language-design decision points marked **DECISION**. **Read it before
   starting any task** — it is where the work is, it says what comes next, and
   it says which choices are the user's to make.
4. **`COMPLETED.md`** — the record: the decision log (newest first), the
   milestone history, every option explored and abandoned, defects found and
   closed, the test inventory, and **"Gotchas / lessons learned"**. Consult it
   to avoid re-solving a problem or re-deciding a question, and read its
   gotchas for the area you are about to touch.

Do not begin implementation work without reading these. ROADMAP.md tells you
*what is left and what has been decided about it*; COMPLETED.md tells you
*where we got to and why*; LANGUAGE.md tells you *what correct means*;
LANGUAGE_SPEC.md (+ the backend spec, when applicable) tells you *precisely
which rules are in play and where they live in the code*.

## Documentation map

| File | What it is | When to load |
|---|---|---|
| `LANGUAGE.md` | Narrative language spec — source of truth | Always |
| `LANGUAGE_SPEC.md` | Labeled rules + compiler decisions (backend-neutral) | Always |
| `BACKEND_SPEC.kotlin.md` | Kotlin interpretation of the rules + `kt-` rules | Only when working on the Kotlin backend (`salvo-backend-kotlin`, `std/**/*.kotlin.sv`) |
| `BACKEND_SPEC.rust.md` | Rust interpretation of the rules + `rs-` rules (deductions → borrows) | Only when working on the Rust backend (`salvo-backend-rust`, `std/**/*.rust.sv`) |
| `ROADMAP.md` | What is left: open defects, unbuilt phases, decisions and plans about them | Always |
| `COMPLETED.md` | The record: decision log, milestone history, abandoned options, closed defects, test inventory, gotchas | Always |
| `AGENTS.md` | This file — how to work on the repo | Always |
| `examples/README.md` | The worked examples: layout, how to regenerate them, and the conventions they must keep | When adding or touching an example, or when a language change invalidates one |
| `README.md` | Public-facing overview and quick start | Rarely (keep in sync on user-visible changes) |

## Keep the documents up to date

COMPLETED.md and ROADMAP.md are the handoff point between sessions, and the
split between them is by *tense*: what happened, and what has not happened yet.
After completing meaningful work (a milestone, a sub-item, a design decision, a
new gotcha):

- **COMPLETED.md**: add a decision-log entry at the top of the log — what was
  decided, by whom, what it took, and what fell out of building it. Update the
  test counts and the "what is built" paragraph. Add lessons to "Gotchas /
  lessons learned"; record an option you explored and rejected, with why.
- **ROADMAP.md**: delete what you finished, or move it to COMPLETED.md if the
  reasoning is worth keeping. Add newly discovered leftovers and any defect you
  reproduced but did not fix (with its repro). A decision the user has now made
  stops being a **DECISION** and becomes a plan.
- **Do not leave an item in both.** An entry that is still open belongs in
  ROADMAP.md only; the moment it lands, its record belongs in COMPLETED.md
  only, with a one-line pointer left behind if a reader would otherwise look
  for it in the old place.

The spec files carry the rule labels; keep them in sync with the code:

- Adding or changing a feature → add/update the rule in LANGUAGE_SPEC.md
  and tag the implementation and tests with its label. Backend-specific
  behavior goes in the backend spec, either as a repeated core rule with
  backend sub-bullets or as a new backend-prefixed rule.
- Renaming or removing a rule → `grep -rn '\[old-label\]'` and update every
  reference; a label in code that no longer exists in a spec is a bug.
- Fixing a LANGUAGE.md spec bug → also check whether a LANGUAGE_SPEC.md
  rule states the old behavior, and note the fix in COMPLETED.md.

## Repository layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols, resolve.rs, types.rs, check.rs
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
├── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
├── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests
└── salvo-testkit/        # dev-dependency: toolchain probing + the e2e content cache
std/core/                 # Salvo stdlib (.sv); each backend lowers its `intrinsic` declarations in src/intrinsics.rs
examples/<name>/          # worked examples: salvo/ source, rust/ + kotlin/ generated output,
                          #   expected.txt (identical on both backends), README.md — see examples/README.md
```

This is a plain Cargo workspace (not a Brazil package). See COMPLETED.md
"Workspace layout" for architectural details and how to add a new backend.

## Build, test, verify

```bash
cargo build                 # must stay warning-free
cargo test                  # the suite; toolchain tests are content-cached, so a
                            # re-run costs seconds. See "Test inventory" in COMPLETED.md
SALVO_E2E_FRESH=1 cargo test # FULL: every test, nothing taken from the cache
SALVO_SKIP_E2E=1 cargo test # inner loop only: skips the kotlinc/rustc tests
INSTA_UPDATE=always cargo test   # accept insta snapshot changes — only after reviewing diffs
cargo nextest run           # same tests, with per-test timings (diagnosis; slower)
```

Three speeds, and it matters which one you use:

| command | what it does | wall time |
|---|---|---|
| `SALVO_SKIP_E2E=1 cargo test` | skips every toolchain test | ~4s |
| `cargo test` | runs everything; skips only *re-verifying* unchanged generated code | ~8s warm, ~80s cold |
| `SALVO_E2E_FRESH=1 cargo test` | runs everything, ignoring the cache | ~70s |

- **Always run `cargo build` and `cargo test` before presenting changes**, and
  `SALVO_E2E_FRESH=1 cargo test` before anything that gets committed or
  handed over.
- **`cargo test` is complete, not partial.** A toolchain test that has
  already compiled and run *this exact generated code*, with *this exact
  toolchain*, is not repeated: `salvo-testkit` writes a stamp keyed by the
  content hash of the generated files, the expected output and the compiler
  version. Touch the emitter and every affected stamp misses, so the cache
  cannot hide a regression it caused. A stamp is written only after every
  assertion passes.
- **`SALVO_SKIP_E2E=1` is for the inner loop, never for the final check.**
  It skips the toolchain tests — and those tests still report as *passing*,
  so a skipped run looks identical in the summary. The wall time is the tell.
- **Temporary files stay inside the repository**: put scratch files,
  debug scripts, and throwaway output in the repo-local `tmp/` directory
  (gitignored) — never in `/tmp` or elsewhere outside the repo. Tests
  use `env!("CARGO_TARGET_TMPDIR")` (via `salvo_testkit::scratch`), and the
  stamp cache lives in `target/tmp/salvo-e2e-cache` — delete that directory
  to reset it.
- Some tests invoke `kotlinc` (or `rustc` for the Rust backend) to compile
  and run emitted code with exact stdout assertions; they skip gracefully if
  the toolchain is not on PATH. If you have it, treat those tests as required.
  - The availability probe is **cached per test binary** by
    `salvo_testkit::kotlinc()` / `rustc()` / `tool()`: `kotlinc -version`
    starts a JVM and costs about as much as a small compile, so a per-test
    probe made the *check* one of the most expensive things in the suite.
  - The gate and the cache both sit inside the runner helpers
    (`run_kotlin_files`, `run_kotlin_entry`, `run_rust_files`), so a new test
    that forgets its own guard still skips instead of failing.
  - The CLI tests cache per *test*, keyed on the `salvo` binary and the test
    binary rather than on generated text, since there the thing under test is
    a subprocess. Those stamps therefore miss on every compiler rebuild —
    correctly: they pay off in the re-run and test-editing loops.
- insta snapshot tests fail on first run by design; accept intentional
  changes with `INSTA_UPDATE=always` and *review every snapshot diff* —
  snapshots encode the parser AST and emitter output contracts.
- End-to-end sanity check:
  ```bash
  cargo run -- compile --backend kotlin --src ./some_dir --target ./out
  ```

## Non-negotiable invariants

- **Language-design decisions belong to the user.** Any choice that shapes
  the language surface or its semantics (new syntax, what an operation
  means, std API shape, items marked **DECISION** in ROADMAP.md) must be
  *presented to the user* before implementation: state
  the options, trade-offs, and a recommendation, then wait for the call.
  Record the outcome in COMPLETED.md as a user decision (there is
  precedent — see the decision log). Analysis engineering under decisions
  already made does not need re-approval.
- **Backwards compatibility is not a requirement** (user decision
  2026-09-03). Salvo is experimental and its features are still being
  worked out; compatibility shims would get in the way. So:
  - Change the syntax or the rules outright — no deprecation periods, no
    dual-accepting grammars, no version gates. The old spelling simply
    stops being the language.
  - **Rewrite every affected example** in the same change: `std/`,
    `crates/**/tests/corpus/*.sv`, inline `.sv` sources in Rust tests,
    `examples/**` (source *and* its checked-in generated code and expected
    output), LANGUAGE.md / LANGUAGE_SPEC.md / BACKEND_SPEC.*.md snippets,
    README, and the syntax references in COMPLETED.md and ROADMAP.md
    (neither is a historical archive — record the change in the decision log
    instead of leaving stale syntax in prose).
  - **An example that no longer works is deleted, not preserved.** There is
    no compatibility guarantee to document and no value in a checked-in
    program that does not compile, so rewriting it or removing it are both
    correct outcomes and removal needs no apology. History belongs in
    COMPLETED.md's decision log, not in a directory of dead programs.
  - **Flag what you cannot rewrite confidently.** Anything whose intent
    is ambiguous under the new rules, or a site that *looks* like the
    changed construct but might be a different one: list it for the user to
    update by hand rather than guessing. Precedent: the `with` → `canbe`
    rename left a sketch's `with` alone because it was the
    qualifier-compatibility clause, not an opt-in.
  - A transitional *error* naming the replacement is permitted but not
    expected (it is a diagnostic, not compatibility); still accepting the
    old form never is. Precedent: the `with` → `canbe` rename shipped
    without one — the user had it removed, since nothing outside this
    repository writes Salvo yet. Default to a plain parse error.
- **Never emit silently wrong code.** Unsupported constructs must produce a
  codegen/checker *error*, not incorrect output ([backend-never-wrong];
  the remaining deliberate cuts are listed in COMPLETED.md's history and in
  ROADMAP.md where they are still open).
- **Salvo assumes it can see everything** (user decision 2026-09-03).
  Every call, field read, subscript, and `for` subject must be justified
  by a declaration: an unresolved callee, a field on a non-struct, a
  subscript on a non-array, a non-iterable `for` subject — all errors
  ([call-resolve], [field-resolve], [index-resolve], [iter-resolve]).
  Target-language features are reached by *declaring* them — a member of a
  `platform effect` for customer code [platform-effect], an `intrinsic` for
  std [intrinsic-std-only] — never by writing an undeclared member and
  hoping the backend understands it. Generics are opaque under this rule
  too: with no bounds, nothing about a `T` is knowable.
- **What leniency remains is about inference, not visibility**: a type
  the checker could not *infer* is `Ty::Unknown` and must pass through
  without cascading errors ([type-unknown-lenient]) — one mistake, one
  diagnostic. Coercions/unwraps fire only where the checker's side
  tables say so. The emitters keep their syntactic fallbacks so a
  checker regression degrades to plain output rather than wrong output.
- **Checker and emitter must agree** on lowering rules (union wrapper arm
  identity, the ident-unwrap predicate, `is`-test lowering). If you change
  one side, change the other and the tests.
- Union arm identity is positional over the *declared* type's non-`None`
  arms, in declaration order. Do not reorder or dedupe in ways that change
  arm indices.
- When changing std (`std/`), the parser AST, checker lowering, or emitter
  output: update the insta snapshots deliberately and check the
  kotlinc/rustc end-to-end tests still pass.
- Keep the spec documents and the implementation consistent. If you find a
  spec bug, fix LANGUAGE.md (and any stale LANGUAGE_SPEC.md rule) *and*
  note it in COMPLETED.md (there is precedent — several LANGUAGE.md/std
  inconsistencies were fixed this way during M0–M8).
- Backend-prefixed rule labels (`kt-…`) may only be referenced from that
  backend's crate; `salvo-core`/`salvo-syntax` reference backend-neutral
  labels only. A new backend gets its own
  `BACKEND_SPEC.<backend>.md` and prefix.

## Workflow for a typical task

1. Read ROADMAP.md → pick the item, and check whether it is marked
   **DECISION** (if so, the call is the user's before any code).
2. Read the relevant LANGUAGE.md sections for the feature, and grep the
   affected `[rule-labels]` in LANGUAGE_SPEC.md (plus the backend spec if
   the task touches a backend crate).
3. Check COMPLETED.md's "Gotchas / lessons learned" for traps in the area
   you're touching, and its decision log for whether the question was already
   answered.
4. Implement across the pipeline in order: syntax → resolve/types/check →
   emit. Add or extend tests at each layer you touch, tagged with the rule
   labels they verify.
5. If the change alters syntax or rules, sweep the repo for every
   affected example and rewrite it (no compatibility shims — see the
   invariant); keep a list of sites you deliberately left alone and
   report them to the user at the end.
6. `cargo build` (warning-free) + `cargo test`; run kotlinc/rustc e2e tests if
   available.
7. Update COMPLETED.md (a decision-log entry, new gotchas, test counts) and
   ROADMAP.md (remove what you finished, add what you found);
   update/add the spec rules for any feature-level change.
