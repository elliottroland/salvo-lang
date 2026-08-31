# AGENTS.md — Working on the Salvo compiler

Guidance for AI agents (and humans) contributing to this repository.

## Read these first, always

1. **`LANGUAGE.md`** — the Salvo language specification. It is the source of
   truth for syntax and semantics; parser, checker, and backends are all
   built to match it. When behavior is ambiguous, LANGUAGE.md decides.
2. **`PROGRESS.md`** — the living handoff document: what is built, key design
   decisions, known limitations, the milestone plan (M0–M8), the test
   inventory, and hard-won gotchas. **Consult it before starting any task**
   to find the current milestone and avoid re-solving problems already
   documented under "Gotchas / lessons learned".

Do not begin implementation work without reading both. PROGRESS.md tells you
*where we are and why*; LANGUAGE.md tells you *what correct means*.

## Keep PROGRESS.md up to date

PROGRESS.md is the handoff point between sessions. After completing
meaningful work (a milestone, a sub-item, a design decision, a new gotcha):

- Update the status snapshot, completed-milestone notes, and test counts.
- Record new design decisions and *why* they were made.
- Move finished items out of "Remaining milestones"; add newly discovered
  leftovers to the appropriate milestone section.
- Add lessons learned to "Gotchas / lessons learned".

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
- Keep LANGUAGE.md and the implementation consistent. If you find a spec
  bug, fix LANGUAGE.md *and* note it in PROGRESS.md (there is precedent —
  see "Fixed inconsistencies in LANGUAGE.md + std").

## Workflow for a typical task

1. Read PROGRESS.md → identify the current milestone and its leftovers.
2. Read the relevant LANGUAGE.md sections for the feature.
3. Check "Gotchas / lessons learned" for traps in the area you're touching.
4. Implement across the pipeline in order: syntax → resolve/types/check →
   emit. Add or extend tests at each layer you touch.
5. `cargo build` (warning-free) + `cargo test`; run kotlinc e2e tests if
   available.
6. Update PROGRESS.md with what changed, decisions made, and new gotchas.
