# Finishing `ref`: positions are preserved, never recovered

Status: **in progress** — decided by the user 2026-10-10, built one step at a
time (below), each step committed on its own. This document is the working
plan *and* the handoff: it is kept current as each step lands, so a fresh
session resumes from **"Resume here"**. When the last step lands it is retired
into COMPLETED.md's decision log, and anything still open moves to ROADMAP.md
(the design-document rule, AGENTS.md). ROADMAP.md item 15 points here.

The user asked for the steps to be worked through **without stopping between
them**, committing after each; the suite need not be green between steps (the
last step runs the full and fresh suites).

## Resume here

**Next step: 4.** Steps done: 0, 1, 2, 3.

Where things stand (2026-10-10, after step 3):
- `__loc` is gone: `mut_lend` / `mut_lend_call`, `compute_locs` / `Shared::locs`,
  the non-mint loc twins and the `__loc` suffix are deleted; a mint renders as
  its path fn only. The two remaining `__loc` greps are *negative assertions*
  in `crates/salvo-backend-rust/tests/codegen_tests.rs` (kept as guards).
- `cargo test` is green (76 result lines ok).
- Step 4 left: BACKEND_SPEC.rust.md ([rs-loc] at ~line 751 and 764 still
  describe `{name}__loc` twins; [rs-path]; effect-member / fn-value bullets
  lose their `Mut` conditions; std's `at`s now take any element), COMPLETED.md
  decision-log entry + gotchas + test counts (+ mention the
  `check_platform_boundary` generics fix and std's `at` widening to any `T`),
  ROADMAP.md (drop item 15's pointer here; keep 15.4-15.7 and the follow-ups;
  the opt-in slot-map container), docs/language pages that mention locators,
  AGENTS.md map row for FINISH_REF.md (delete with the file), then warm
  `cargo nextest run --no-fail-fast` and `SALVO_E2E_FRESH=1 cargo nextest run`,
  then delete FINISH_REF.md.

Before resuming: read AGENTS.md, then this file top to bottom, then
`git log --oneline -10` to see which step commits exist. The step list below
is the source of truth; a step's **Status** line is updated in the commit that
finishes it, together with this section.

## The decisions (user, 2026-10-10)

1. **`ref` and `proj` relate in one direction only.**
   - `ref(c) T` is a **position** in `c`; whether it can be written follows
     from `T`'s kind (`Mut`) and from `c` being lent mutably.
   - `proj(c) T` is a **borrow that may not be a position**: a key, either of
     two sources, a view, a derived value.
   - `ref → proj` is free (`ref(c) T <: proj T`, already the subtype rule).
   - **`proj → ref` does not exist.** A handle comes only from where the
     position is known: a **place** (a field path, a search loop's element
     over the container, a bound handle) or a **mint** (a function or member
     returning `ref`, or `x!` of one). A mint returning a reader's result is
     a checker error naming the remedy: declare the reader `-> ref(…)`.
   - Because `ref`'s permission follows `T`'s kind, one `-> ref(xs) T?`
     declaration serves read-only and mutating callers alike.
2. **On Rust, a `ref(c)` result has exactly one rendering: the path** into
   `c` [rs-path], whatever its type arguments and whoever calls it. No mint
   has a borrow-answering second face; no call site chooses between faces.
   Kotlin is unaffected (a handle is the element there).
3. **Std's mints are primitives**, not readers plus a conversion: `at` on
   `List`, `Deque` and `Map` (optional and total) is a `platform fn` beside
   `get`, its position form the backend's (a bounds-checked index, the index,
   `slot_of`, `slot_of(..).expect(..)`), its Kotlin host implementation the
   element.
4. Read-only callers of a `ref`-returning reader re-walk the path on Rust
   where a borrow would be direct; recovering that is the "two-function
   choice" follow-up, a backend optimization — not part of this work.

## The sequence

Each step ends with `cargo build` warning-free, a commit, and this file's
Status lines and "Resume here" updated in that commit. The last step also runs
`cargo test`, a warm `cargo nextest run` and the fresh suite.

### Step 0 — Propagate this plan

**Status: done** (commit "Plan finishing ref: one rendering for a ref
result, and to_ref"; the plan was reworked in the commit "Rework the ref plan:
positions are preserved, never recovered").

### Step 1 — No conversion: std's mints are primitives, and the checker rule

**Status: done** (commit "Make std's at functions platform primitives and refuse
a mint that returns a reader").

- **Std**: `at` (both forms) on `List`, `Deque` and `Map` becomes a
  bodyless `platform fn`; Kotlin host files (`std/platform/core/*.kt`) gain
  their `at`s (the same as the `get`s); the Rust backend's position forms
  (`platform_loc_body`) answer `at` as they answered `get` inside `at`.
- **Checker** ([ref-handle]): the `proj → ref` promotion at a mint's return
  (a fn or handler member whose declared result has a `ref` arm) accepts
  only a place, a bound handle, a search loop's element, a mint call, `None`,
  and `x!` / present / branches of those. A returned reader call — directly,
  or through a local bound from one — is an error naming `-> ref(…)` on the
  reader as the remedy.
- **Sweep** (no shims): every mint body in std, the examples, the corpora,
  inline `.sv` sources in tests, `docs/language/`, LANGUAGE_SPEC.md and both
  backend specs that returns a reader's result: rewrite the reader as `->
  ref(…)` or the mint as a place. The user's `demo/` is theirs — list it.
- Also in this commit: the backend fix found on the way — a mint's body whose
  tail `when` returns in every arm answers through its returns
  (`diverges` covers `Switch`; `fn_body` in locator mode).

### Step 2 — One rendering for a `ref` result

**Status: done** (commit "Render every ref result as its path").

- **Mint recognition by `ref` alone**: a function, effect member, fn type or
  implicit-group member is a mint when its result has a `ref` arm
  (`salvo_ir::build::refs::has_ref`), `Mut` or not. Sites: `Shared::is_mint`,
  `member_lends_mut` (and its name), `fn_ty_lends_mut`, the handle-binding
  test in `let_stmt`, `mint()`'s arms in `paths.rs`, `compute_locs`' seeds.
- **One face**: a mint renders only as its path function; a `ref` member of a
  fn type or implicit renders as a locator closure answering the path type,
  never as a borrow-answering closure (fixes the E0308, "Background").
- **Reads through a locator**: a handle used only for reading is materialized
  with `&` at each use (inline place, or `Walk::walk_ref`), never held as a
  long-lived `&`.
- **Tests**, both backends: `fn head_of<T>(xs: List<T>) -> ref(xs) T?` at
  `T = Mut X` and `T = Str` (Rust: the path function only); a `Pick`-style
  group (`fn pick(c: C, l: L) -> ref(c) T?`) through one and two implicit
  layers at both kinds of `T`; a read-only handle held across a write
  through another handle.

### Step 3 — Retire `__loc`

**Status: done** (commit "Retire the __loc locator twins").

With no conversion, nothing needs a locator twin. Remove: the second face of
a mint, the per-call-site choice (`mut_lend` / `mut_lend_call`), the path
copies of readers (`compute_locs` / `Shared::locs`, `fn_decl`'s loc mode for
non-mints, `loc_call`), the `__loc` naming in `member_loc_sig`, and the
emitted `get_platform__loc` functions (`platform_loc_body` serves the `at`
primitives directly). Done when `grep -rn '__loc' crates std examples` is
empty (the decision log aside). Regenerate the examples and both host
projects; review the snapshot diffs.

### Step 4 — Specs, docs, the record; retire this file

**Status: not started.**

- BACKEND_SPEC.rust.md: [rs-loc] rewritten (locator closures stay, twins go);
  [rs-path] states a `ref` result is always the path; std's `at` primitives;
  the effect-member and fn-value bullets lose their `Mut` conditions.
- LANGUAGE_SPEC.md: [ref-handle] states the one-way relation and what a mint
  may return; docs/language/Mutable-Handles.md likewise; wiki sync.
- COMPLETED.md: a decision-log entry (the reasoning in "Background", the
  probe table, the explored-and-dropped `to_ref`, the region revisited),
  gotchas, test counts.
- ROADMAP.md: item 15's pointer removed; leftovers recorded (the follow-ups
  below; the opt-in slot-map container).
- `cargo test`, warm `cargo nextest run`, fresh `SALVO_E2E_FRESH=1 cargo
  nextest run`; retire this file.

## Background (the reasoning, kept for the record)

### Why: `Mut`-gated rendering was `proj Mut` coming back

Before this work the Rust backend decided a function's rendering from the
**`Mut` written on its declared return** (`is_mint`, `member_lends_mut`,
`fn_ty_lends_mut`, the `has_ref(..) && is_mut(..)` tests around `let_stmt`): a
return written `ref(c) Mut T` rendered as its path function only; a return
written `ref(c) T`, with the `Mut` arriving through a type argument, rendered
as a borrow function **plus** a `__loc` twin, each call site picking one. That
is the earlier `proj Mut` idea coming back in through `ref`. It works for
named functions and cannot work for a function passed as a value or an
implicit. Probes (2026-10-10, scratch programs, both backends):

| Shape | Kotlin | Rust before this work |
|---|---|---|
| `fn head_of<T>(xs: List<T>) -> ref(xs) T?`, called directly, `T = Mut Counter` and `T = Str` | correct | correct (borrow face or `__loc` per call) |
| `?Ref` (`at: -> ref(c) Mut T?`) through one and two generic layers, and a generic consumer writing through it | correct | correct |
| a group `Pick` whose member is `-> ref(c) T?`, through an implicit, `T = Mut Counter` | correct | refused: "a locator returns something other than a handle of its anchor [rs-loc]" |
| the same, `T = Str`, read only | correct | rustc E0308: the implicit typed `&mut dyn FnMut(&C, &L) -> Option<T>` — an *owned* `T`, the borrow dropped |

### Why no conversion: the cost of recovering a discarded position

A reader returns a bare borrow, forgetting where the element is; any
`proj → ref` conversion has to recover the position. Explored on
2026-10-10, in order, and set aside:

- **Copying readers into path versions** (today's `__loc` for readers): a
  parallel copy of every reader a mint forwards from, transitively, bounded
  only by a refusal list (`loc_expr`). Arbitrarily complex and nested readers
  would need a second analysis.
- **`p.to_ref(c)`, recovering the position from the borrow's address**: a
  generated `locate` per (container, element) pair (O(1) per contiguous
  level by address arithmetic, a scan per separately allocated level). Built
  partway and reverted. What sank it, once the user's refinements were in:
  - the container should come from the borrow's provenance, which a
    function boundary loses for keys (a reader `-> proj(m) Str` may return a
    key or a value) — so the result must be `ref(c) T | NoRef`, `NoRef` a
    singleton because `T` may be `X?`;
  - parity then forces **Kotlin** to check membership too (an identity scan
    per level), where `to_ref` had been the identity — the user's objection;
  - every caller must discharge `NoRef`.
- **Raw pointers under `ref`** (`salvo_ref` resolving on demand): provenance
  (a pointer from a reader's `&T` may not be written through; the fix needs
  the index, or exposed provenance), aliasing (`&C` held across a write is
  undefined behaviour), the rustc backstop lost (Miri needed), validity
  resting on fate poisoning alone.
- **The region store, revisited** (values extracted into a central store,
  placeholders left behind, the store keyed by address so accesses can tell
  a placeholder from a value): code that does not know it is in a region —
  host code, Rust's own algorithms (`sort`, `contains`, slices), derives,
  the wire encoder — reads the placeholder; every element read everywhere
  would pay a thread-local lookup; a reallocation inside host code has no
  hook, and a Rust *move* (a struct passed, returned or reallocated) changes
  addresses with no event at all. Kotlin needs none of it.

What remains is to never throw the position away: a reader that should yield
handles returns `ref`. The one cost — re-walks on read-only uses on Rust — is
an optimization question (follow-up 1).

### Parked: an opt-in slot-map container

If recursive types or performance ever need O(1) handles, the sound form is a
container that *is* a slot map (elements in an arena it owns, handles stable
keys) — the group-borrowing design's Part 3 container, as an opt-in type,
not a region over ordinary containers. Recursive types are planned with paths
as lists instead.

## Follow-ups, after this lands (to discuss; not part of the sequence)

1. **The two-function choice.** A direct-borrow face for read-only uses of a
   `ref`-returning function, as a backend optimization: a generic body knows
   per use whether it writes, so a fn value or implicit could carry both
   faces — strictly an optimization over a design already correct with one
   face, and only once measured.
2. **Whether `ref` merges into `proj Mut` after all**, now that the relation
   is one-way and permission follows the kind.

## Found on the way (recorded in ROADMAP item 15, not part of this work)

- Overload ranking: `f<T>(x: T)` and `f<T>(x: T?)` both fit a `Str`, and the
  optional form wins (it is the more specific *pattern*); an exact-fit-first
  rule fixes it (prototyped 2026-10-10, reverted: it is a ranking rule, the
  user's call).
- Rust: a read-only fn iterating a field `List<Mut T>` of a kept parameter
  renders `iter_mut()` over `&` (E0596); the loop element is typed as a
  handle, so the fix belongs with per-binding mutation inference.
- Rust: `walk_pair` (the `Walk` impls) over a `VecDeque` uses slice
  splitting, which `VecDeque` lacks; latent until a program has `Deque`
  handles and a generic handle fn.
- Type arguments are not inferred from an in-scope implicit (already ROADMAP
  item 15.4).
