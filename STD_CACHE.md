# Caching std's front-half: the `check_program` seam

Design options for ROADMAP §5 step 3 — the actual speedup behind the LSP
responsiveness work. The soundness groundwork (`[deduce-std-explicit]`, total
deduction clauses on exported std fns) has landed; this document is the
*mechanism* that exploits it, and it needs a user decision before building
because it reshapes the front-half, not just memoizes it.

Written 2026-10-10. When a decision is made and the work lands, this document
dies into COMPLETED.md's decision log (the retired-design-doc rule) and
ROADMAP §5 step 3 is closed.

## Status

- **2026-10-10 — Option A chosen (user).** Build the skip-std-bodies seam.
  Option B (std as a precompiled library) is **recorded in ROADMAP §5 as a
  future item**, not built now. Option C is rejected. Sub-decisions for A are
  being worked through; see "Sub-decisions for A" at the end, updated per
  round.

## The problem, precisely

The language server re-runs the whole pipeline on every keystroke
(`crates/salvo-cli/src/analysis.rs`): build a `Program` of std + deps +
project, `Symbols::collect`, `resolve`, then `check_program`. Phases 1–2
(debounce, per-file parse cache) removed the parse cost and the per-keystroke
pile-up. What remains is `check_program`, and it is almost all std:
`salvo analyze --src demo` is ~460ms for 42 std files + 1 user file.

Three facts make the obvious cache impossible, and shape every option below.

1. **`Resolution<'p>` and `Symbols<'p>` borrow the `Program`.** Their scopes,
   `declared_in`, `decl_files` and the `FnEntry`/`&FnDecl` references are all
   `&'p` into one `Program`'s ASTs (`crates/salvo-core/src/resolve.rs`,
   `program.rs`). Neither survives the fresh merged `Program` a new analysis
   builds, so you cannot cache "std's resolution" and splice it in.
2. **`check_program`'s fixpoint reads the whole program.** `check_once` walks
   every file's bodies into a shared `Checked`; the per-round driving facts
   (`candidates`, `claims`, `mutations`, `mut_fields`, `parking`) accumulate
   across all files; `deduce::infer` builds its `fns` list from *all*
   `program.modules` and runs a call-graph fixpoint. There is no existing seam
   that says "these files are done, skip them."
3. **`Checked` is owned (no `'p`), but keyed by global file index.** `FnKey {
   file, item }`, `Key = (file, span)`, `DefSite { file, span }`, and
   `Resolution.scopes` are all indexed by position in `Program::files`. std is
   loaded first and in sorted order, so its indices are stable *as long as*
   nothing shadows std (`apply_std_shadow` removes std files) and the project
   is not itself `std = true` (`mark_std_tree`). Any cached std data must be
   remapped if those indices move — in the common LSP case they do not.

What `[deduce-std-explicit]` bought us: std's exported fns now have *written*
deduction clauses, which `deduce::infer` treats as fixed points (it overlays
written entries and never widens them from caller facts). So **std's contracts
no longer depend on the project** — a project call cannot change a std fn's
deductions. That is the invariant that makes any of the options below sound; it
is necessary but not sufficient (overload resolution and the span-keyed tables
still need handling).

## What a project's check actually needs *from* std

Distinguishing this is what separates the cheap options from the complete ones.

- **To check project files**, the checker needs std's *declarations* (for
  resolution and overload candidates — already rebuilt each analysis, cheaply,
  by `resolve`) and std fns' *contracts* (deductions, effects, return types,
  whether a member parks) at call sites. It does **not** need std's
  per-expression `expr_ty`/`coerce`/`is_tests` tables to check a project file.
- **To emit std** (`salvo build`, not the LSP), the backend needs std's full
  span-keyed `Checked` — every `expr_ty`, coercion, union-arm identity, etc.
- **To hover/goto *inside* a std file** (the LSP, only when the user opens a
  std file to read it), the LSP reads std's `def_refs`/`fn_refs`/`expr_ty` for
  that one file. Rare, and recomputable lazily.

So the LSP's hot path needs only std's *contracts*, while `salvo build` and
"hover in std" need std's *full tables*. The options differ mainly in which of
these they serve and at what rebuild cost.

## Option A — Skip-std-bodies seam in `check_program`

**Idea.** Add a mode to `check_program` that takes a set of "trusted" files
(std) plus their precomputed contracts, and in `check_once` **does not walk the
bodies of trusted files** — it only publishes their contracts into the tables a
call site reads, then checks the untrusted (project + dependency) files against
them. `deduce::infer` seeds std fns' deductions from the cache and does not
re-derive them (sound: they are written, hence fixed points).

**What is cached.** An owned `StdContracts` struct: per std `FnKey`, the
deduction list, effect list, return type, parking/send facts, overload
visibility keys — everything a *call site* in a project file consults. Keyed by
the embedded-std content hash (constant per compiler binary) and gated on "no
std shadow, project not `std = true`" so the file indices still line up. Built
once by checking std in isolation (a std-only `Program`, as
`salvo-backend-kotlin/src/stdlib.rs::std_library_files` already does for
emission), then re-indexed into the merged program (std occupies the leading
indices, so the remap is the identity in the common case and a shift otherwise).

**Cost recovered.** All of std's body-checking — nearly the whole 460ms. The
project still checks fully.

**What it does *not* serve.** std's per-expression tables are absent from the
project's `Checked`. Fine for the LSP's hot path (project diagnostics). For
"hover inside a std file", recompute that one file's tables on demand (a single
`Checker` pass over one std file is cheap). For `salvo build`, this mode is not
used — a build wants std's full tables and has the Kotlin std-classes cache
([kt-std-library]) downstream anyway.

**Risk.** The seam must publish *exactly* what a call site reads, or the
project check silently diverges from a full check. This is the
[backend-never-wrong]-adjacent danger: a missing contribution (e.g. a std
`params` group's `with` deduction, a std handler's parking flag) would change
project checking. Mitigation: a debug-only `SALVO_VERIFY_STD_CACHE` that runs
both the seam and a full check and asserts the project-file diagnostics and
`Checked` project-entries are identical — the same "verify, don't assume"
stance the Kotlin std cache takes.

**Effort.** Medium-large. One new mode parameter threaded through
`check_program_rounds`/`check_once`, a contract-publishing step, the
`deduce::infer` seeding, and the isolation+remap build. No change to what the
language *means*.

## Option B — std as a precompiled library with an owned interface

**Idea.** The deeper version of A: give the front-half a real library boundary.
std is checked once into an **owned, index-free summary** (contracts keyed by
*module path + item name*, not file index), serialized to the cache dir. A
project analysis never puts std files in its `Program` at all — it loads the
summary, and `resolve`/`check` consult it for any `core.*`/imported std name.

**Cost recovered.** Everything A recovers, plus std is not even parsed or
resolved per analysis (phases 1–2 already made that cheap, so the marginal win
over A is small), and the cache survives across compiler runs (written to disk,
keyed by the embedded-std hash).

**What it costs.** A large refactor: `resolve` and `check` must consult an
external symbol/contract source rather than assuming every name is in the one
`Program`. The index-free keying means `FnKey`-based tables need a parallel
path-based key for std entities. This is the "std-as-a-crate" boundary Salvo
has never had.

**Risk.** Higher — it touches the invariant that "Salvo sees everything in one
program" (though only for std, which is fixed). Worth it only if cross-run
persistence or multi-project workspaces make the per-run isolation of A too
slow, which today's numbers do not suggest.

**Effort.** Large. A front-half architecture change.

## Option C — Cache the whole `Checked`, recheck only on std-set change

**Idea.** Give up on splicing; instead, memoize at the analysis boundary. Hash
(std content + dep content + every project file's content). On a keystroke only
one project file changed, but the hash changes, so this *alone* saves nothing —
unless combined with incremental re-check of only the changed file's bodies,
which runs into the same whole-program fixpoint problem (a changed project file
can change a project fn's inferred deductions, which a project caller sees).

**Verdict.** Rejected as a standalone: the fixpoint is whole-program over the
*project*, so there is no file-granular project cache without the same seam
work as A. Recorded so it is not re-proposed.

## Recommendation

**Option A.** It recovers essentially all of the std cost, serves the LSP's hot
path exactly, degrades gracefully for the rare "hover in std" (lazy recompute)
and for `salvo build` (unused there), and does not change what Salvo means — it
is a performance seam guarded by a verify-not-assume check. Option B is A's
natural successor *if* cross-run persistence or workspace scale later demands
it, but it should not be the first step. Option C is not viable alone.

Open sub-decisions within A, for the design session:

1. **Where the seam lives.** A `trusted_files: &HashSet<usize>` +
   `std_contracts: Option<&StdContracts>` pair on `check_program`, or a new
   `check_project(program, resolution, symbols, std: &StdContracts)` entry
   point beside `check_program` that the LSP calls and `salvo build` does not.
2. **Exactly which facts `StdContracts` carries.** Enumerate every table a
   call site in a project file reads for a std callee (deductions, effects,
   return/throw, parking, overload/visibility keys, refinement entries that
   cross the boundary) — the correctness surface. This enumeration *is* the
   risky part and should be derived by auditing `check_once`'s reads, with the
   `SALVO_VERIFY_STD_CACHE` differential check as the backstop.
3. **Isolation build + remap.** Reuse `std_library_files`' std-only `Program`
   construction; decide whether to recompute per LSP session (simplest) or
   persist to disk keyed by the embedded-std hash (option-B-lite, a small add).
4. **Shadow / `std = true` fallback.** When the project shadows std or is
   itself `std = true`, the seam is **off** — fall back to today's full check.
   That keeps std-development (where the std bodies are the thing being
   checked) correct and simple.

## Sub-decisions for A

Grounded in an audit of what a project call site reads about a std callee
(2026-10-10). Two seam points, both now justified:

- **`check_once`**: do not walk the bodies of trusted (std) files; publish
  their contracts into the tables call sites read.
- **`deduce::infer`**: a std fn's entries are all `written`
  ([deduce-std-explicit]), and the fixpoint already *discards* the walked
  result of an all-written fn (the overlay `if c.written { *n = c.clone() }`).
  So std fns can be skipped in the `fns` loop outright — today they are walked
  and thrown away, pure waste.

**The `FnKey`-keyed callee facts (the cache's core).** Only seven `Checked`
tables are keyed by the callee's `FnKey`, i.e. are facts a project call reads
*about std*:
`deductions`, `fn_effects`, `fn_lends`, `param_mutations`, `iter_returns`,
`implicit_params`, `established_quals`. These plus the std fns' declared
signatures (return/throw types, send/parking flags — read from the AST via
`resolution`/`symbols`, which are rebuilt cheaply every analysis) are the
contract surface. `StdContracts` carries the seven tables' std entries; the
declarations come free from the always-rebuilt resolution.

Everything else is either (a) `Key = (file, span)`-keyed — a fact about an
expression *in a file*, so a project call site produces its own entry when the
project is checked, and std's entries are only needed to *emit* or *hover in*
std; or (b) whole-program accumulators (`parking`, `candidates`, …) that the
seam seeds from the cache and then only grows with project facts.

### A1 — Where the seam lives

- **(i) A new `check_project(program, resolution, symbols, std: &StdContracts)`
  entry beside `check_program`.** The LSP calls it; `salvo analyze`/`salvo
  build` keep calling `check_program` (full check). Cleanest separation, no
  mode flag threaded through the fixpoint, and the full path stays exactly as
  tested. Recommended.
- (ii) A `trusted: Option<(&HashSet<usize>, &StdContracts)>` parameter on
  `check_program`. Less duplication but threads a mode through every round.

### A2 — Structural, not a differential check (user decision 2026-10-10)

The cache is **not** a separate `StdContracts` type copied into `Checked` —
that would be two representations to keep in sync, policed by a runtime diff
(rejected). Instead **the cache *is* a `Checked`**: `check_project` seeds its
`out` from the std rows of a real full check and adds only project rows on top.
std rows are therefore always "whatever the real checker produced", never a
parallel computation, so there is nothing to keep in sync.

Two consequences:

- **One producer.** std's rows are produced only by the normal checker. The
  cache is captured from a genuine `check_program` (over a std-only program)
  the first time, then reused while only the project changes.
- **Soundness is a stated per-table invariant, by construction — not a diff.**

**The invariance audit (2026-10-10), the proof the design rests on.** The seven
`FnKey`-keyed tables are the only facts a project call reads *about* a std
callee; the audit confirmed each std row is a pure function of that std fn's own
declaration and body, never of its callers or of project code:

| table | computed from | caller-dependent? |
|---|---|---|
| `deductions` | the fn's own written total clause, a fixed point overlaid every round ([deduce-std-explicit]) | no — pinned by the clause |
| `fn_effects` | the fn's own declared effect list (+ its handler's deps) | no |
| `fn_lends` | a written projection clause, else inferred from the fn's own body | no |
| `param_mutations` | the fn's own body mutation events, keyed by `own_fn` | no |
| `iter_returns` | which iterator struct the fn's own body mints | no |
| `implicit_params` | the fn's own declared `?name`/`?Group` params | no |
| `established_quals` | the fn's own `+Q` clause entries | no |

Three (`deductions`, `fn_lends`, `iter_returns`) depend on the fn's **callees**
— but a std fn only calls std fns (a project name is not visible from a std
body), so the dependency is std-internal and still invariant under project
code. The audit also confirmed `check_call` reads **only** a resolved callee's
`FnKey`-keyed contract tables plus its AST declaration (via resolution/symbols)
— the one `expr_ty`-family read in the whole call path is of the *caller's own
argument*, never the std callee's body-span entries. So std bodies can be
skipped outright in the project hot path; nothing a project call needs is
produced only by walking a std body. **No differential check is required; the
invariant is the guarantee.**



### A3 — Build and index remap

Reuse `salvo-backend-kotlin/src/stdlib.rs::std_library_files`' std-only
`Program` construction to check std in isolation. In the common LSP case std
occupies the leading indices of the merged program unchanged, so the remap is
the identity; compute the offset defensively anyway. **Recompute per LSP
session** (held in the `Server`, like the parse cache), not persisted to disk —
persistence is Option B's job, and a per-session build is a one-time ~400ms at
startup, paid back on the first keystroke.

### A4 — Fallback

The seam is **off** — fall back to today's full `check_program` — when the
project shadows std (`apply_std_shadow` removed/replaced std files) or is itself
`std = true` (`mark_std_tree`). There the std bodies are the thing being
checked, and the index assumptions do not hold. Detected before building the
cache; the LSP already knows both conditions from the analysis.
