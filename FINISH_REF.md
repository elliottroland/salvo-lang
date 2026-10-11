# Finishing `ref`: a `ref(c)` result is always a locator

Status: **in progress** — decided by the user 2026-10-10, built one step at a
time (below), each step committed on its own. This document is the working
plan *and* the handoff: it is kept current as each step lands, so a fresh
session resumes from **"Resume here"**. When the last step lands it is retired
into COMPLETED.md's decision log, and anything still open moves to ROADMAP.md
(the design-document rule, AGENTS.md). ROADMAP.md item 15 points here.

## Resume here

**Next step: 1** (`to_ref`). Steps done: 0.

Before resuming: read AGENTS.md, then this file top to bottom, then
`git log --oneline -10` to see which step commits exist. The step list below
is the source of truth for what is done; a step's **Status** line is updated
in the same commit that finishes it.

## The decisions (user, 2026-10-10)

1. **On Rust, a `ref(c)` result has exactly one rendering: the path** into
   `c`, with its own `Walk` [rs-path] — whatever its type arguments are, and
   whoever calls it. No mint has a second, borrow-answering face; no call site
   chooses between faces. A read through a handle walks the path (`walk_ref`,
   or the inline place with `&`); a write walks it mutably. Kotlin is
   unaffected (a handle is the element there).
2. **No reader is copied into a path version.** A reader's borrow becomes a
   position by **`p.to_ref(c)`**, a std intrinsic the backend monomorphizes per
   (container, element) pair; on Rust it recovers the position from the
   borrow's address (option (A) below). Every function is emitted once.
3. **`to_ref`'s shape**:

       // core.ref: the backend's, per (container, element) pair.
       export intrinsic fn to_ref<C, T>(p: proj(c) T, c: C) [] -> ref(c) T => p, c

       // The optional form, plain Salvo.
       export fn to_ref<C, T>(p: (proj(c) T)?, c: C) [] -> (ref(c) T)? => p, c {
           when p {
               is None { return None }
               else { return p.to_ref(c) }
           }
       }

   `p` first, so it reads `p.to_ref(c)`. Over a `Mut` element kind the result
   is a mutable handle; over any other, a handle with nothing to write.
4. **Explicit.** The `proj → ref` promotion at a mint's return stays only for
   **places** (a search loop's element over the container, a field path) and
   **mints** (a delegating mint's `return at(c, l)`). A reader's result becomes
   a handle only by writing `to_ref`: std's `at` becomes `return get(list,
   index).to_ref(list)`; the user's `fn at(a: A, index: Int) -> ref(a) Mut
   Str?` becomes `return a.b.c.get(index).to_ref(a)`. A returned reader result
   without it is a checker error naming `to_ref`.
5. **Std's `at` bodies are rewritten** with `to_ref`, for uniformity. The
   backend's inline position forms for std's platform readers (`get` →
   bounds-checked index, `get_at` → the index, `Map`'s `get` → `slot_of`,
   `get_present` → `slot_of(..).expect(..)`) are `to_ref`'s fast path over
   them, so nothing gets slower.
6. **Zero-sized elements**: `to_ref` refuses them (a loud backend error) —
   their addresses cannot be told apart, and they hold no state.

## The sequence

Each step ends with `cargo build` warning-free, `cargo test`, a warm
`cargo nextest run`, a commit, and this file's Status lines and "Resume here"
updated in that commit. The last step also runs the fresh suite.

### Step 0 — Propagate this plan

**Status: done** (commit "Plan finishing ref: one rendering for a ref
result, and to_ref").

ROADMAP.md item 15 points here and carries the two items found on the way
(the E0308 defect until step 3 lands; the type-argument inference gap, which
is not part of this work); AGENTS.md's documentation map lists this file.

### Step 1 — `to_ref`

**Status: not started.**

Add `to_ref` (both forms, decision 3) to `std/core/ref.sv`, beside
`NotSame`. Nothing about the promotion rule changes yet.

- **Checker**: `to_ref`'s first parameter's type names a *later* parameter
  (`proj(c)` with `c` second). Check that a forward qualifier argument
  resolves; if it does not, make it (or order the intrinsic's parameters
  `(c, p)` internally and keep `p.to_ref(c)` as the written form). The
  argument must be a projection of `c` — the checker already proves that for
  a derived return; reuse that. Check `when … else` over an optional works
  in std.
- **Kotlin**: the identity.
- **Rust**: `to_ref` mints a handle. Its path is computed per (container,
  element) pair by a generated `locate(&C, &E) -> Option<P>` from the same
  enumeration the path types come from:
  - a contiguous level (`Vec`, the map's slot slab): `as_ptr_range()`
    contains the address, and the offset divided by the element size is the
    index — O(1), integers only, no `unsafe`;
  - a struct level: which field's range holds the address;
  - a level with its own allocation (each team's `roster` in a map of
    teams): a scan of that level, then O(1) in the buffer that holds it;
  - `VecDeque`: its two slices; the map slab needs an occupied-slots
    iterator (a small host addition);
  - the fast path: when `p` is a call of a std platform reader with an
    inline position form, use that form instead of `locate`;
  - zero-sized element types: a loud error.
  `to_ref` must work both in ordinary code (`let h = find(xs).to_ref(xs)!`)
  and inside a mint's path computation (`loc_expr` / `mint()` in `paths.rs`).
- **Tests** (both backends, compile-and-run): `to_ref` over a flat list, the
  `League` (map of rosters plus a bench), a `Grid`, a `Deque`, a reader with
  real logic (a search through a heap of candidates), and the optional form;
  a checker test that a `to_ref` of a projection of *another* container is
  refused.

### Step 2 — Explicit: the promotion only for places and mints

**Status: not started.**

- **Checker**: a mint returning a reader's result (not a place, not a mint)
  without `to_ref` is an error naming `p.to_ref(c)` as the remedy.
- **Sweep** (no compatibility shims): every `at` and every other mint body in
  `std/`, the examples (source and regenerated output), the corpora, inline
  `.sv` sources in Rust tests, `docs/language/` (Mutable-Handles.md, notably
  the `a.b.c.get(index)` example), LANGUAGE_SPEC.md and both backend specs.
  The user's `demo/` is theirs: list it, do not edit it.
- **Rust**: std's `at` bodies now go through `to_ref`'s fast path; the
  generated code should be unchanged or equivalent — diff the examples.

### Step 3 — One rendering for a `ref` result

**Status: not started.**

- **Mint recognition by `ref` alone**: a function, effect member, fn type or
  implicit-group member is a mint when its result has a `ref` arm
  (`salvo_ir::build::refs::has_ref`), `Mut` or not. Sites: `Shared::is_mint`,
  `member_lends_mut` (and its name), `fn_ty_lends_mut`, the handle-binding
  test in `let_stmt`, `mint()`'s arms in `paths.rs`.
- **One face**: a mint renders only as its path function; a `ref` member of a
  fn type or implicit renders as a locator closure answering the path type
  (`&mut dyn FnMut(&C, &L) -> Option<__Pk>`, `__Pk: Walk<C, T>` in generic
  code), never as a borrow-answering closure. This fixes the E0308 (see
  "Background").
- **Reads through a locator**: a handle used only for reading is materialized
  with `&` at each use (inline place, or `Walk::walk_ref`); never bound as a
  long-lived `&`, since a write through another handle into the same
  container may come between two reads [fate-poison].
- **Tests**: the probe shapes in "Background", both backends — `head_of` at
  `T = Mut X` and `T = Str` (Rust: the path function only), a `Pick`-style
  group through one and two implicit layers at both kinds of `T`; a read-only
  handle held across a write through another handle.

### Step 4 — Retire `__loc`

**Status: not started.**

Remove what produced locator twins: the second face of a mint, the
per-call-site choice (`mut_lend` / `mut_lend_call`), the path copy of a
reader (`compute_locs` / `Shared::locs`, `fn_decl`'s loc mode for non-mints,
`loc_call`), the `__loc` naming in `member_loc_sig`, and the emitted
`get_platform__loc` functions (`platform_loc_body`) — std readers' position
forms are inline (step 1). Done when `grep -rn '__loc' crates std examples`
is empty (the decision log aside). Regenerate the examples and both host
projects, review the snapshot diffs.

### Step 5 — Specs, docs, the record; retire this file

**Status: not started.**

- BACKEND_SPEC.rust.md: [rs-loc] rewritten (locator closures stay, twins go);
  [rs-path] states a `ref` result is always the path; `to_ref`'s rendering;
  the effect-member and fn-value bullets lose their `Mut` conditions.
- LANGUAGE_SPEC.md: a `[ref-to-ref]` rule; [ref-handle] states what a mint
  may return (a place, a mint, or `p.to_ref(c)`).
- docs/language/Mutable-Handles.md: `to_ref`; wiki sync.
- COMPLETED.md: a decision-log entry (the reasoning in "Background", the
  probe table, the removal of mints' second face and of reader copies,
  `to_ref`), gotchas, test counts.
- ROADMAP.md: item 15's pointer removed; leftovers (the follow-ups below)
  recorded.
- Retire this file. Fresh `SALVO_E2E_FRESH=1 cargo nextest run`.

## Background (the reasoning, kept for the record)

### Why: `Mut`-gated rendering is `proj Mut` coming back

Before this work the Rust backend decided a function's rendering from the
**`Mut` written on its declared return** (`is_mint`, `member_lends_mut`,
`fn_ty_lends_mut`, the `has_ref(..) && is_mut(..)` tests around `let_stmt`): a
return written `ref(c) Mut T` rendered as its path function only; a return
written `ref(c) T`, with the `Mut` arriving through a type argument, rendered
as a borrow function **plus** a `__loc` twin, each call site picking one.
That is the earlier `proj Mut` idea — "a borrow that is mutable when its type
is" — coming back in through `ref`. It works for named functions (a call site
can pick a face) and cannot work for a function passed as a value or an
implicit: a generic function is one Rust function, so its fn-typed parameter
has one type. Probes (2026-10-10, scratch programs, both backends):

| Shape | Kotlin | Rust before this work |
|---|---|---|
| `fn head_of<T>(xs: List<T>) -> ref(xs) T?`, called directly, `T = Mut Counter` and `T = Str` | correct | correct (borrow face or `__loc` per call) |
| `?Ref` (`at: -> ref(c) Mut T?`) through one and two generic layers, and a generic consumer writing through it | correct | correct |
| a group `Pick` whose member is `-> ref(c) T?`, through an implicit, `T = Mut Counter` | correct | refused: "a locator returns something other than a handle of its anchor [rs-loc]" |
| the same, `T = Str`, read only | correct | rustc E0308: the implicit typed `&mut dyn FnMut(&C, &L) -> Option<T>` — an *owned* `T`, the borrow dropped — with an unused `Walk` type parameter |

### Readers forwarded from a mint: three options

A mint's body may return the result of a user reader (`fn find(xs) ->
proj(xs) T?`, then `return find(xs)`). How the backend turns that borrow into
a position:

- **(a)** refuse it, requiring `find` to be declared `ref` — not taken: an
  asymmetry (std readers promotable, user readers not), and every ordinary
  caller of `find` re-walks on read.
- **(b)** copy the reader into a path version (today's `__loc`) — first
  chosen, then set aside: path mode copies the functions whose *result* a
  mint returns, transitively (`compute_locs` / `forwarded_callees`), changing
  their returns and their `for` loops over the anchor, and refusing anything
  else (`loc_expr`). Generalizing it — locals computed by other means,
  containers of borrows, fn values and implicits, effect handlers, generic
  code — would mean a second analysis of every such reader, a parallel copy
  of arbitrarily complex and nested functions.
- **(A)** recover the position from the borrow — **chosen**: the reader runs
  as itself and returns its `&T`; `to_ref` turns the address back into a path
  (one generated `locate` per type pair, not per reader). Correct by
  construction (the checker proves the borrow derives from the container); a
  value has one home and buffers never overlap, so exactly one path matches;
  `locate` goes by the address the reader returned, never by value. Its cost
  is proportional to the data (a scan per separately allocated nested level),
  not to the code. Example (`League`): test the bench's range (O(1)), then
  scan the teams' slots for the roster holding the address.

### Considered and set aside: a raw pointer under `ref`

A `*mut T` in the handle, resolved on demand, would make handles O(1) and
delete the path machinery, recursive types included. Set aside, at least for
now: **provenance** (a pointer made from a reader's `&T` may not be written
through; a writable one needs the buffer's own pointer plus the index —
`locate` again — or *exposed provenance*, the least precisely specified part
of Rust's memory model); **aliasing** (any `&C` held across a write through
the pointer is undefined behaviour); **the backstop goes** (a lowering
mistake becomes silent undefined behaviour instead of a rustc error; the e2e
suite would need Miri); and **validity rests on fate poisoning alone**.
Recursive types will get paths as lists, which stays in safe Rust.

## Follow-ups, after this lands (to discuss; not part of the sequence)

1. **Generalizing the two-function choice.** With one face everywhere, reads
   through a `ref` pay a re-walk where a borrow would have been direct. A
   borrow face could come back as a *backend* optimization — a generic body
   knows per use whether it writes, so a fn value or implicit could carry both
   faces — strictly as an optimization over a design already correct with one
   face, and only once measured.
2. **Whether `ref` merges into `proj Mut` after all.** With `ref`'s rendering
   independent of `Mut`, the remaining distinction is permission (may this
   result be written through, and is it a place?) versus kind (`Mut` on the
   value).
