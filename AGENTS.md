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
   referenced from that backend's crate and define files; core crates
   reference backend-neutral labels only.
3. **`PROGRESS.md`** — the living handoff document: what is built, key design
   decisions (condensed history), known limitations, the roadmap (currently:
   toward full linear types, with marked language-design decision points),
   the test inventory, and hard-won gotchas. **Consult it before starting
   any task** to find the current roadmap phase and avoid re-solving
   problems already documented under "Gotchas / lessons learned".

Do not begin implementation work without reading these. PROGRESS.md tells
you *where we are and why*; LANGUAGE.md tells you *what correct means*;
LANGUAGE_SPEC.md (+ the backend spec, when applicable) tells you *precisely
which rules are in play and where they live in the code*.

## Documentation map

| File | What it is | When to load |
|---|---|---|
| `LANGUAGE.md` | Narrative language spec — source of truth | Always |
| `LANGUAGE_SPEC.md` | Labeled rules + compiler decisions (backend-neutral) | Always |
| `BACKEND_SPEC.kotlin.md` | Kotlin interpretation of the rules + `kt-` rules | Only when working on the Kotlin backend (`salvo-backend-kotlin`, `std/**/*.kotlin.sv`) |
| `BACKEND_SPEC.rust.md` | Rust interpretation of the rules + `rs-` rules (deductions → borrows) | Only when working on the Rust backend (`salvo-backend-rust`, `std/**/*.rust.sv`) |
| `PROGRESS.md` | Status, milestones, design decisions, test inventory, gotchas | Always |
| `AGENTS.md` | This file — how to work on the repo | Always |
| `README.md` | Public-facing overview and quick start | Rarely (keep in sync on user-visible changes) |

## Keep the documents up to date

PROGRESS.md is the handoff point between sessions. After completing
meaningful work (a milestone, a sub-item, a design decision, a new gotcha):

- Update the status snapshot, completed-milestone notes, and test counts.
- Record new design decisions and *why* they were made.
- Move finished items out of "Remaining leftovers" and the roadmap; add
  newly discovered
  leftovers to the appropriate milestone section.
- Add lessons learned to "Gotchas / lessons learned".

The spec files carry the rule labels; keep them in sync with the code:

- Adding or changing a feature → add/update the rule in LANGUAGE_SPEC.md
  and tag the implementation and tests with its label. Backend-specific
  behavior goes in the backend spec, either as a repeated core rule with
  backend sub-bullets or as a new backend-prefixed rule.
- Renaming or removing a rule → `grep -rn '\[old-label\]'` and update every
  reference; a label in code that no longer exists in a spec is a bug.
- Fixing a LANGUAGE.md spec bug → also check whether a LANGUAGE_SPEC.md
  rule states the old behavior, and note the fix in PROGRESS.md.

## Repository layout

```
crates/
├── salvo-cli/            # binary "salvo": clap CLI, backend registry, embeds std/ via include_dir
├── salvo-syntax/         # lexer, parser, AST, spans, diagnostics (no deps)
│   └── tests/corpus/     # LANGUAGE.md-example .sv files + insta snapshots
├── salvo-core/           # SourceSet, Program, Symbols, resolve.rs, types.rs, check.rs
├── salvo-backend/        # Backend trait, BackendRegistry, BackendError
└── salvo-backend-kotlin/ # Kotlin emitter (emit.rs) + golden/kotlinc tests
└── salvo-backend-rust/   # Rust emitter (emit.rs) + golden/rustc tests
std/core/                 # Salvo stdlib (.sv) + backend define files (*.kotlin.sv, *.rust.sv)
```

This is a plain Cargo workspace (not a Brazil package). See PROGRESS.md
"Workspace layout" for architectural details and how to add a new backend.

## Build, test, verify

```bash
cargo build                 # must stay warning-free
cargo test                  # full suite; see PROGRESS.md "Test inventory" for the current count
INSTA_UPDATE=always cargo test   # accept insta snapshot changes — only after reviewing diffs
```

- **Always run `cargo build` and `cargo test` before presenting changes.**
- **Temporary files stay inside the repository**: put scratch files,
  debug scripts, and throwaway output in the repo-local `tmp/` directory
  (gitignored) — never in `/tmp` or elsewhere outside the repo. Tests
  use `env!("CARGO_TARGET_TMPDIR")`. Clean up `tmp/` contents when done.
- Some tests invoke `kotlinc` (or `rustc` for the Rust backend) to compile
  and run emitted code with exact stdout assertions; they skip gracefully if
  the toolchain is not on PATH. If you have it, treat those tests as required.
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
  means, std API shape, items marked **DECISION** in PROGRESS.md's
  roadmap) must be *presented to the user* before implementation: state
  the options, trade-offs, and a recommendation, then wait for the call.
  Record the outcome in PROGRESS.md as a user decision (there is
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
    LANGUAGE.md / LANGUAGE_SPEC.md / BACKEND_SPEC.*.md snippets, README,
    and the syntax references in PROGRESS.md (it is the handoff document,
    not a historical archive — record the change in the decision log
    instead of leaving stale syntax in prose).
  - **Flag what you cannot rewrite confidently.** Anything whose intent
    is ambiguous under the new rules, sketches of unimplemented features
    (`experiments/`), or a site that *looks* like the changed construct
    but might be a different one: list it for the user to update by
    hand rather than guessing. Precedent: the `with` → `canbe` rename
    left `experiments/refinements.sv` alone because its `with` was the
    qualifier-compatibility clause, not an opt-in.
  - A transitional *error* naming the replacement is permitted but not
    expected (it is a diagnostic, not compatibility); still accepting the
    old form never is. Precedent: the `with` → `canbe` rename shipped
    without one — the user had it removed, since nothing outside this
    repository writes Salvo yet. Default to a plain parse error.
- **Never emit silently wrong code.** Unsupported constructs must produce a
  codegen/checker *error*, not incorrect output ([backend-never-wrong];
  the remaining deliberate cuts are listed in PROGRESS.md's history).
- **The checker is lenient by design**: anything it cannot type is
  `Ty::Unknown` and must pass through without cascading errors (Kotlin
  interop relies on this). Coercions/unwraps fire only where the checker's
  side tables say so.
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
  note it in PROGRESS.md (there is precedent — several LANGUAGE.md/std
  inconsistencies were fixed this way during M0–M8).
- Backend-prefixed rule labels (`kt-…`) may only be referenced from that
  backend's crate and its define files; `salvo-core`/`salvo-syntax`
  reference backend-neutral labels only. A new backend gets its own
  `BACKEND_SPEC.<backend>.md` and prefix.

## Workflow for a typical task

1. Read PROGRESS.md → identify the current roadmap phase and the
   leftovers.
2. Read the relevant LANGUAGE.md sections for the feature, and grep the
   affected `[rule-labels]` in LANGUAGE_SPEC.md (plus the backend spec if
   the task touches a backend crate or its define files).
3. Check "Gotchas / lessons learned" for traps in the area you're touching.
4. Implement across the pipeline in order: syntax → resolve/types/check →
   emit. Add or extend tests at each layer you touch, tagged with the rule
   labels they verify.
5. If the change alters syntax or rules, sweep the repo for every
   affected example and rewrite it (no compatibility shims — see the
   invariant); keep a list of sites you deliberately left alone and
   report them to the user at the end.
6. `cargo build` (warning-free) + `cargo test`; run kotlinc/rustc e2e tests if
   available.
7. Update PROGRESS.md with what changed, decisions made, and new gotchas;
   update/add the spec rules for any feature-level change.
