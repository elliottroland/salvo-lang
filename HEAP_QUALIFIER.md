# The `Heap` qualifier — a plan for `demo/heap.sv`'s TODOs

`demo/heap.sv` is a user-space binary-heap qualifier over `List<T>` — the
exercise being whether the language can express it *outside* std. It cannot
yet, and each TODO in the file marks one of the reasons. This document takes
each one apart: what the file writes, what exists today (with rule labels and
code evidence), and the options with a recommendation. Items needing a
language-design call are marked **DECISION** per AGENTS.md's first invariant.

Started in a read-only session (2026-09-21): "today" claims are read from
the code, not verified by compiling — the verification steps are listed per
item. `demo/` is referenced by no test or build target, so the file
currently compiles nothing and breaks nothing.

**The one-line summary, as it started**: std's own `Sorted<T> of List<T>` is the
same shape as `Heap` and dodged every hard problem here by being `intrinsic` — no
body to validate, ordering delegated to the backend. `Heap` was the test of
whether ordinary Salvo could do what only intrinsics could, and the TODOs were
exactly the gaps.

**Where it ended (2026-09-22): it can.** `demo/heap.sv` compiles and runs on both
backends, and `Sorted` is no longer the privileged one — it carries its ordering
like `Heap` does, and its operations are ordinary Salvo over three primitives.
Items 1–4 are closed, and **what is left is two pieces of sugar**: `!is` (item 5)
and `+=` (item 6), both decided and both a parser change. Delete this file when
they land.

Two of the original TODOs are already resolved and carry no plan (2026-09-21):

- **`export` not highlighted in the editor** — operational, not code: the
  tmLanguage grammar covers `export qualifier`/`export fn` (lookahead rule,
  `vscode/syntaxes/salvo.tmLanguage.json:116`) and reloading the extension
  fixed it. The LSP publishes no semantic tokens, so the grammar is the sole
  highlight source; semantic tokens remain a possible ROADMAP entry under
  Tooling, but nothing here needs them.
- **Eliding `NonEmpty` to reach `core.list`'s `remove_first`** — no new
  syntax: a qualified value already fits a less-qualified parameter, and the
  scope problem (a bare call would rank the demo's own overload first and
  recurse) is what the **scope selector** exists for. The demo's fn body is
  `return remove_first@core.list(list)!`. Neither proposed spelling should
  happen: `^` was just re-homed into `is ^Q` [qual-lift], and `-NonEmpty`
  duplicates what subtyping permits silently. To do only as part of the demo
  rewrite: apply the selector and verify routing.

---

## Inventory

| # | TODO (file location) | Kind | Status today |
|---|---|---|---|
| 1 | `Heap<T canbe ordered>` — ordering bound on `T` (lines 6–7) | **DECISION** — grew into its own design round, now complete (COMPLETED.md's log) | ✅ **done**: the bound half landed 2026-09-21 (a `cmp` in scope *is* the bound) and the holding half 2026-09-22 (`Heap<T, ?cmp: (T, T) -> Int>`, [cmp-carry] [cmp-binder]). `demo/heap.sv` is rewritten to it and its ordering half compiles; what is left there is items 2 and 4 |
| 2 | `+Heap` — re-asserting the claim after mutation (lines 20–22) | **DECISION** (= ROADMAP **D2**) | ✅ **landed 2026-09-22**: `=> heap: +Heap<T, ?cmp> Mut`, trusted in the qualifier's own file |
| 3 | `val: proj proj T?` (line 28) | defect | ✅ **fixed 2026-09-22**: a hover-only display bug in the LSP, not the type system |
| 4 | `swap` / set-at-index on `Mut List<T>` (lines 39–40) | std API (**DECISION** on shape) | ✅ **`swap` landed 2026-09-22**, answering `Bool`; the positional write stays deliberately absent |
| 5 | `!is NonEmpty` + fall-through narrowing + overload routing (lines 51, 54) | **DECISION** (small) | narrowing and routing **verified working** 2026-09-22; only `!is` is missing |
| 6 | `+=` (line 76) | **DECISION** (small) | only `++`/`--` exist [inc-dec] |
| 7 | *(steering, 2026-09-21)* `<`/`>` on unconstrained `T` compiles silently | defect / posture gap | **fixed 2026-09-21** (the ordering round's step 2: the comparison resolves `cmp`, and a `T` with no `?Ordered<T>` is an error naming the remedy) |

Suggested order: **3 ✅, the ordering round's steps ✅ (which subsumed 1 and 7),
then 2, 4, 5, 6** — rationale at the end.

---

## 1. Constraining `Heap` to orderable `T` → the ordering round (**done**)

The 2026-09-21 design discussion grew this item into a redesign of how
comparison, equality and hashing work in the language, and it now lives in
its own round document, ORDERING.md — which has since been built out in full
and deleted (the OPTIONALS.md pattern), its record in COMPLETED.md's decision
log. In brief:

**Landed 2026-09-21**, which changes what this item still needs: the *bound*
half is done — `Ordered`/`Eq`/`Hashed` are params groups, the operators resolve
through them, and "orderable `T`" is now spelled `?Ordered<T>` in the signature
rather than as a bound on the type parameter. What the heap still waits for is
the *holding* half — which landed 2026-09-22: a heap remembers which ordering
built it by naming it in a fn slot, and `demo/heap.sv` is written that way now.
In outline, as decided:

- `Ordered`/`Eq`/`Hashed` become **params groups** (the `Yield` precedent);
  canonical implementations for structs are **top-level fns `@`-scoped to
  the type** (`fn cmp@Person(…)`, in the type's file), imported with the
  type and the default for implicits — `cmp = cmp@Person` selects one
  explicitly, and any ambiguity around a canonical errors, explicit or
  implicit (user decisions 2026-09-21) — with intrinsic `cmp`/`eq`/`hash`
  overloads for primitives; `: auto Ordered<self>` (etc.) on the
  obligation clause generates the structural one, and the `default` forms
  bring `eq` with them.
- Operators resolve through the groups; **equality becomes opt-in**
  (overturning [col-equality]); comparisons on unconstrained `T` become
  errors (item 7).
- Structures that *hold* an ordering bind it at construction as a
  fn-valued type argument with **static identity**
  (`Heap<?cmp>`, `SortedSet<T, ?cmp = cmp>`, `Set<T, ?hash, ?eq>`) —
  zero-cost markers on Rust, identities in the type, differently-ordered
  structures refuse to mix.
- `canbe ordered` / `canbe hashed` are deleted.

Decisions (fourteen made 2026-09-21, eight more 2026-09-22), lowering, rejected
options and the build plan are in COMPLETED.md's decision log. For *this* plan:
the heap's declaration line and its comparisons are unblocked.

---

## 2. Re-asserting `Heap` after mutation — **landed 2026-09-22** (ROADMAP D2)

**The decision** (the user's): option A — a fn in the qualifier's own file may
keep a claim it re-establishes — **with the refinement that the re-application is
spelled differently** from a claim checked as usual, so a reader can tell which is
which. `+Heap<T, ?cmp>`, on the model of a `refn`'s additions:

```
export fn heap_push<T>(heap: Heap<T, ?cmp> Mut List<T>, elem: T) -> None
    => heap: +Heap<T, ?cmp> Mut, !elem {
    add(heap, elem)     // strips the claim, as any mutating call must
    …                   // sift it back
}
```

A plain `Heap` in that position still fails body validation, which is what makes
the `+` mean something. The rule is [deduce-reapply]; what it refuses: another
file's qualifier, a compiler qualifier (`+Mut`), a qualifier the parameter does
not declare (re-establishing is not adding), arguments that disagree with the
parameter's, and `+Q` inside a `=>[f]` group (a callback's contract is not yours
to vouch for — that is what a `refn` is for). No emitter work: qualifiers erase.

**What it unblocked, and what it cost.** `demo/heap.sv` reached *zero* errors —
and then failed to *run*, twice, on defects the checker could not see:

- **Implicit arguments were dropped for a callee the checker had not walked yet.**
  A call read the callee's implicit-parameter list out of a table filled as the
  walk went, so a callee declared *later in the file* — or in a file sorted after
  the caller's — looked like a fn with no implicits: the call was accepted with
  none filled, and the emitted call was short an argument on **both** backends.
  Order-dependent, silent in Salvo, fatal in the target. Fixed by a program-wide
  signature pre-pass before any body is checked.
- **A forwarded implicit needed an adapter.** `drain(heap: Heap<Int, ?cmp> …)`
  holds `&mut dyn FnMut(i32, i32) -> i32` (a Copy scalar goes by value) while the
  generic `heap_pop<T>` it calls wants `FnMut(&T, &T)` (a type variable is never
  known to be Copy) — the disagreement [rs-fn-param-convention] already records
  one level out, for a lambda. The Rust emitter now bridges them, with the modes
  read off the one function that renders a fn type's parameters, so the two
  answers cannot drift.

Both are in COMPLETED.md's log. The heap now runs on both backends and is an e2e
case on each, from verbatim one source.

---

## 3. `heap.get(i)` hovers as `proj proj T?` — **fixed 2026-09-22**

**It was a hover bug, and the plan's hypothesis was wrong.** Reproduced in six
lines (`fn probe(xs: List<Str>) => xs { let first = xs.get(0) }`, hover on
`first`), and the checker's own type was right the whole time: an error at the
same binding prints `proj Str?`, one `proj`. Only the *hover* doubled it.

**The cause**: `lsp.rs`'s hover writes the compiler qualifier itself for a
fate-linked variable — `format!("proj {ty}")` [fate-link] — and `get` is declared
`-> (proj[from: list] T)?`, so the borrow was already on the type line. The fate
link and the declared projection are the same claim about the same value, said
twice.

**The fix**: `Ty::presents_proj()` (types.rs) answers whether a type's *rendering*
leads with a `proj` — a `proj` qualifier of its own, or an optional printed as
`X?` whose `X` leads with one — and the hover prefixes only when it does not. The
detail line still names the roots, which is the part the type does not carry.
Tested at both levels: a unit test on the predicate (including the buried-in-a-
multi-arm-union case, where a prefix is still informative) and an LSP hover
assertion in `diagnostics_hover_and_shutdown`.

**What the plan had guessed, recorded so nobody re-chases it**: that
`Ty::qualify`'s `sort` + `dedup` keeps two `proj` quals whose `from` args differ
(types.rs), and that a call applies projections along two routes. The first half is
true as written — `dedup` compares whole `Qual`s — but nothing reachable produces
two differing `proj` quals on one type: the language's own spelling for a borrow of
several sources is *one* qual with several arguments (`proj[from: a, b]`
[deduce-syntax]), and every `proj` in a checked type comes from lowering a written
one. So the dedup is latent at most, and merging `from` lists by name is a fix
waiting for a defect. The lesson: a type *shown* wrongly is not evidence that the
type *is* wrong — compare a diagnostic's rendering against the hover's before
reaching for the type system.

---

## 4. `swap` for `Mut List<T>` — **landed 2026-09-22**

**Shipped**, with the user's call on the one open corner:

```
export intrinsic fn swap<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> Bool
    => list: Mut, i, j
```

`false` when either index is out of range, and then nothing moved — chosen over a
silent no-op (a swap that quietly does nothing is a reordering bug with no symptom
at the call) and over the hosts' own behaviour (`Vec::swap` panics where a JVM list
throws, so the same program would fail differently per backend). `canbe linear`
because the exchange is **total**: no value enters the list and none leaves, so
nothing can be dropped — which is what makes it the one positional write a list of
obligations can have.

**The positional write stays deliberately absent**, and that was already settled
in `core.list`'s own prose before this plan existed: `replace(list, index, elem)`
has nowhere to put the displaced value when the index misses, and every available
answer either drops it (a silent leak) or confuses "displaced" with "bounced".
`swap` escapes the question rather than answering it. So no decision was needed
here after all.

**It closed a pre-existing defect on the way.** `[col-bounds]` — the rule
`core.list` had been *referencing* without it being written down anywhere — is now
in LANGUAGE_SPEC.md, and writing it exposed that the Rust backend broke its own
posture for a **literal** index: the lowerings cast straight to `usize`, a literal
takes its type from the cast target, and `(-1) as usize` is rustc's E0600. So
`get(xs, -1)` type-checked in Salvo and then failed to *build* — a
[backend-never-wrong] violation, with a variable holding `-1` working fine. The
cast now goes through `i64` at every index position (list/array/`Bytes` `get`,
`char_at`, `swap`), which moved every Rust golden and example by one `as i64`.

Tested by one e2e case per backend from verbatim one source, covering `swap` in
range, in place, past the end and negative, plus a literal negative read on three
surfaces.

---

## 5. `!is` and fall-through narrowing to the `NonEmpty` overload

**What the file writes:**

```
if heap !is NonEmpty {
    return None
}
// TODO: `heap` should be `NonEmpty` here, routing to the NonEmpty overload
return heap_pop(heap)
```

**Today, in layers:**
1. **`!is` does not parse.** No token, no arm in `parse_comparison`
   (parser.rs:2630ff) — `heap !is NonEmpty` is a parse error. The written
   form today is `!(heap is NonEmpty)`.
2. ✅ **The narrowing works** (verified 2026-09-22). `analyze_cond`'s
   `UnaryOp::Not` arm swaps then/else narrows, and [is-narrow-guard] installs a
   condition's else-narrows after an `if` whose branches all exit; a predicate
   `is` puts "subject + Q" in its then-narrows [is-qualifies], so through `Not`
   it lands exactly on the fall-through. Hovering the demo's own
   `return heap_pop(heap)` reports `Heap<T, ?cmp> Mut NonEmpty List<T>`,
   "narrowed here by an `is` test". The "no else information" note on
   [is-qualifies] had made this look untested; it now has two tests in
   `guard_tests.rs`.
3. ✅ **Overload routing comes free, self-delegation included** (verified
   2026-09-22). Rank prefers the signature demanding more of its argument
   [fn-overload-rank], and the trap the std comment warns about — a same-name
   call picking *itself* — does not fire with a narrowed argument: a program of
   the demo's exact shape (`pick` guarding, then calling `pick`) prints
   `empty` / `nonempty with 7` on **both** backends rather than recursing. The
   checker test pins the routing and a negative twin pins that it is the guard
   doing the work.

**Plan:** layers 2–3 are done and needed no compiler change — no narrow or rank
gap existed. What is left is the **DECISION**: add `!is` as sugar? Options:
  - **A.** `expr !is Type` sugar → `Unary Not (Is …)` in the parser's
    comparison tier. Kotlin precedent, reads well, ~20 lines + tests. No
    binding form (`!is Q name` binds nothing — there is no narrowed value).
  - **B.** Keep `!(x is Q)` as the only spelling. Zero work, but the demo's
    intuition (author reached for `!is` without checking) is evidence for A.
- Recommendation: **A**. The verification has landed, so this is now the whole
  of item 5.

**Effort:** small.

---

## 6. `+=` — **DECISION** (small)

**Today:** `++`/`--` exist ([inc-dec], ast.rs:954) — "a step of one on a
place". Compound assignment does not; `i_child = i_child + 1` is the
current spelling. (The `while` loop's `i = i_parent` is fine as is.)

**Options:**
- **A.** Add `+=` `-=` `*=` `/=` on places, desugared in the parser to
  `place = place op expr` (the desugar pass already exists for other
  sugar). Numeric-operand rules come free from [op-arith]; places and
  their reset/narrowing behavior come free from the assignment path.
  Decide the set: all four, or `+=`/`-=` only (matching `++`/`--`'s
  step-semantics reading).
- **B.** Don't: `++` covers the step-of-one case, everything else is rare.
  But the demo hit the want inside twenty lines of ordinary index
  arithmetic, which is evidence it will keep being hit.

**Recommendation:** A, all four arithmetic compounds, as pure desugar (no
new checker or emitter surface). One parser change + [op-arith]-tagged
tests + spec rule.

**Effort:** small.

---

## 7. Comparisons on unconstrained `T` compile silently → fixed by the round

**Confirmed.** `check_comparison_operands` and `check_arith` bail out for
any operand where `op_lenient` holds, and `op_lenient` includes
`Ty::Var(_)` — an unconstrained generic — alongside `Unknown`/`Never`
(check.rs:21137). The comment calls it "a documented leftover, matching the
equality slice". So every `parent <= val`, `>` and `<=` in `heap.sv` passed
unchecked, against the language's own posture that "with no bounds, nothing
about a `T` is knowable" ([call-resolve], AGENTS.md). Downstream it is
worse than a missing diagnostic: the Rust emitter would produce `a <= b` on
a bound-less generic — code `rustc` rejects at best ([backend-never-wrong]
survives only by accident of the target refusing it).

**Fixed 2026-09-21** (the ordering round's step 2, landed): operators resolve through
the `Ordered`/`Eq` groups, so a comparison on a `T` with no `?cmp`/`?eq` in
scope is the ordinary missing-capability error naming `?Ordered<T>`, and
`Ty::Var` left `op_lenient` for comparisons. Equality went the same way —
opt-in. The demo's own comparisons will therefore be *checked* once its
declaration line parses; they need the `?cmp` the qualifier binds, which is
step 5.

---

## Suggested sequence and why

1. ✅ **#3 (`proj proj`)** — **fixed 2026-09-22**, and it was a hover bug rather
   than a type-system one, so it de-noised the file for free.
2. ✅ **The ordering round's steps** — subsumed #1 and #7, and the round is
   **complete** (2026-09-21/22): the declaration line and every comparison in
   `heap.sv` now check, and nothing here waits on anything.
3. ✅ **#2 (D2)** — **landed 2026-09-22**: `heap_push` and `heap_pop` keep their
   natural `Mut`-parameter shape, and `demo/heap.sv` compiles *and runs*.
4. ✅ **#4 (`swap`)** — **landed 2026-09-22**, which took `demo/heap.sv` down to
   **one** error: item 2's.
5. **#5 (`!is`)** — the guard narrowing and the routing are verified working
   (2026-09-22); only the sugar is left, and it is a parser change.
6. **#6 (`+=`)** — independent, small, any time.

The demo rewrite (whenever the heap first compiles) also folds in the
already-resolved point above: `remove_first@core.list(list)!` in the
`NonEmpty` overload's body — or drop that wrapper fn and call the selector
from `heap_pop` directly.

When the decisions land: record each in COMPLETED.md's log, move the
settled items out of this file, and fold the surviving rules into
LANGUAGE_SPEC.md with labels ([deduce-syntax]/D2's successor, `!is`, `+=`,
`swap` under the collections rules; the ordering rules are already there).
This document is a working plan, not a spec — it should be deleted once
`demo/heap.sv` compiles and its lessons are in the permanent documents.
