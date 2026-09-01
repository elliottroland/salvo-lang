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
   decisions, known limitations, the milestone plan (M0–M8), the test
   inventory, and hard-won gotchas. **Consult it before starting any task**
   to find the current milestone and avoid re-solving problems already
   documented under "Gotchas / lessons learned".

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
| `BACKEND_SPEC.<backend>.md` | Same pattern for future backends (`rs-` for Rust, M8) | Only when working on that backend |
| `PROGRESS.md` | Status, milestones, design decisions, test inventory, gotchas | Always |
| `AGENTS.md` | This file — how to work on the repo | Always |
| `README.md` | Public-facing overview and quick start | Rarely (keep in sync on user-visible changes) |

## Keep the documents up to date

PROGRESS.md is the handoff point between sessions. After completing
meaningful work (a milestone, a sub-item, a design decision, a new gotcha):

- Update the status snapshot, completed-milestone notes, and test counts.
- Record new design decisions and *why* they were made.
- Move finished items out of "Remaining milestones"; add newly discovered
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
std/core/                 # Salvo stdlib (.sv) + backend define files (*.kotlin.sv)
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
- Some tests invoke `kotlinc` to compile and run emitted Kotlin with exact
  stdout assertions; they skip gracefully if `kotlinc` is not on PATH. If
  you have kotlinc, treat those tests as required.
- insta snapshot tests fail on first run by design; accept intentional
  changes with `INSTA_UPDATE=always` and *review every snapshot diff* —
  snapshots encode the parser AST and emitter output contracts.
- End-to-end sanity check:
  ```bash
  cargo run -- compile --backend kotlin --src ./some_dir --target ./out
  ```

## Non-negotiable invariants

- **Never emit silently wrong code.** Unsupported constructs must produce a
  codegen/checker *error*, not incorrect output. (See "Deliberate cuts" in
  PROGRESS.md.)
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
  output: update the insta snapshots deliberately and check the kotlinc
  end-to-end tests still pass.
- Keep the spec documents and the implementation consistent. If you find a
  spec bug, fix LANGUAGE.md (and any stale LANGUAGE_SPEC.md rule) *and*
  note it in PROGRESS.md (there is precedent — see "Fixed inconsistencies
  in LANGUAGE.md + std").
- Backend-prefixed rule labels (`kt-…`) may only be referenced from that
  backend's crate and its define files; `salvo-core`/`salvo-syntax`
  reference backend-neutral labels only. A new backend gets its own
  `BACKEND_SPEC.<backend>.md` and prefix.

## Workflow for a typical task

1. Read PROGRESS.md → identify the current milestone and its leftovers.
2. Read the relevant LANGUAGE.md sections for the feature, and grep the
   affected `[rule-labels]` in LANGUAGE_SPEC.md (plus the backend spec if
   the task touches a backend crate or its define files).
3. Check "Gotchas / lessons learned" for traps in the area you're touching.
4. Implement across the pipeline in order: syntax → resolve/types/check →
   emit. Add or extend tests at each layer you touch, tagged with the rule
   labels they verify.
5. `cargo build` (warning-free) + `cargo test`; run kotlinc e2e tests if
   available.
6. Update PROGRESS.md with what changed, decisions made, and new gotchas;
   update/add the spec rules for any feature-level change.
