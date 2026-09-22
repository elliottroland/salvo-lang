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

**The one-line summary**: std's own `Sorted<T> of List<T>`
(`std/core/list.sv:160–195`) is the same shape as `Heap` and dodges every
hard problem here by being `intrinsic` — no body to validate, ordering
delegated to the backend. `Heap` is the test of whether ordinary Salvo can do
what only intrinsics can today, and the TODOs are exactly the gaps.

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
| 2 | `+Heap` — re-asserting the claim after mutation (lines 20–22) | **DECISION** (= ROADMAP **D2**) | written entry fails body validation |
| 3 | `val: proj proj T?` (line 28) | defect | ✅ **fixed 2026-09-22**: a hover-only display bug in the LSP, not the type system |
| 4 | `swap` / set-at-index on `Mut List<T>` (lines 39–40) | std API (**DECISION** on shape) | neither exists for `List` |
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

## 2. Re-asserting `Heap` after mutation — **DECISION** (this is ROADMAP D2)

**What the file writes:** `fn heap_push<T>(heap: Heap Mut List<T>, elem: T) => heap: Heap Mut`,
with the comment asking for `+Heap` in deductions.

**Today:** the written exhaustive entry `heap: Heap Mut` is *shape*-legal
(it keeps only qualifiers the parameter declares, [deduce-syntax]) but fails
**body validation**: "promising … a qualifier the body may remove, is an
error" [deduce-infer]. The body calls `add(heap, elem)`, whose exhaustive
`=> list: Mut` strips `Heap` (the removal set is computed against what the
argument carries). `NonEmpty` comes back via its refinement
(`refn add … => list: +NonEmpty`); nothing brings `Heap` back.

**Why each existing mechanism fails here:**
- **A refinement** (`refn add(…) => list: +Heap` inside `Heap`) would be a
  *false* claim: appending to the tail of a heap-ordered list breaks the
  heap property in general. Refinements state what *someone else's* call
  preserves; here nothing but `heap_push` as a whole preserves it. The TODO
  itself sees this ("running the qualifies again would be another O(n)").
- **`add_sorted`'s trick** (writing the keep in an intrinsic's clause) works
  only because an intrinsic has no body to validate. `heap_push` has one.
- **A `qualifies` at exit** — `Heap` as written has no body (constructive,
  [qual-constructive]), and even with one, an O(n) re-check per push defeats
  the point. Also `qualifies` needs the elements compared, which the ordering round
  supplied.

**What works *today*, as a stopgap:** the consume-and-return shape —

```
fn heap_push<T>(heap: Heap Mut List<T>, elem: T) -> Mut List<T> as Heap => !heap
```

— a constructor fn [qual-ctor-fn], trusted by construction, same-file rule
satisfied [qual-ctor-same-file]. Cost: callers write `h = heap_push(h, e)`,
and the demo's own `as_heap` helper shows the shape was already being
groped for. Worth rewriting the demo this way regardless, so it compiles
while D2 waits.

**The real fix is D2** ("Decisions waiting on the user", unscheduled):
`+Q` in a function's own deduction clause needs an *establishment rule* —
what entitles a body to claim it established `Q`. The heap gives D2 its
motivating example and suggests a narrow, honest rule:

> A fn declared **in the qualifier's own file** may write the qualifier in
> a parameter's exhaustive entry (or as `+Q`), trusted — the same trust and
> the same scoping as constructor fns [qual-ctor-same-file] and
> refinements-by-the-owner [qual-refn]. Outside that file it stays
> rejected.

That is weaker than general `+Q` (no establishment proof, but the same
party we already trust to mint the claim), and it makes `=> heap: Heap Mut`
in `heap.sv` legal as written. Options to put to the user:
- **A.** The same-file trusted keep/`+Q` above (recommended — consistent
  with every existing trust boundary: `as Q`, `refn`, field overrides).
- **B.** Full D2 with an establishment rule (verify the body ends every
  path having called something that establishes `Q`) — much heavier, and
  the machinery (a per-qualifier establishment relation) doesn't exist.
- **C.** Stay with consume-and-return permanently — zero compiler work,
  but `Mut`-parameter APIs (the language's own idiom for mutators) can
  never preserve a user qualifier, which will keep hurting.

**Effort (A):** moderate — deduce.rs validation carve-out, spec rule under
[deduce-syntax]/[qual-refn], tests; no emitter work (qualifiers erase,
[qual-erasure]).

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

## 4. `swap` / set-at-index for `Mut List<T>` — std API (**DECISION** on shape)

**Today:** `List` has `get`, `add`, `remove_first`, `remove_at`, `size`, …
— no `swap`, no `set`. `set` exists for `Mut Bytes` and `Mut Str`
(std/core/bytes.sv:67, string.sv:112), where elements are Copy scalars and
the old value can vanish. For `List<T>` with `T canbe linear`, overwriting
index `i` must put the old value *somewhere* — the TODO's own observation
("we would need to _take_ from the list as well").

**Proposed additions to `core.list` (all `intrinsic`, both backends):**

```
// The heap's actual need — total, no values enter or leave, linear-safe:
export intrinsic fn swap<T canbe linear>(list: Mut List<T>, i: Int, j: Int) [] -> None
    => list: Mut, i, j

// The general write — replace, answering the displaced value:
export intrinsic fn set<T canbe linear>(list: Mut List<T>, index: Int, elem: T) [] -> T?
    => list: Mut, index, !elem
```

- `swap` with either index out of bounds: propose **no-op is wrong** — but
  what instead is a call for the user. Options: (a) answer `Bool`
  (like `Set.add`), (b) answer `None` and treat out-of-range as a no-op,
  (c) leave it partial and document. `get`'s precedent is the honest
  optional; `swap` has no value to make optional, so (a) reads best.
- `set` answers `T?` — `None` when `index` is out of range, in which case
  **`elem` was consumed but not stored**: for a linear `T` that is a
  dropped obligation, so either `set` must answer `T?` where out-of-range
  *returns `elem` itself* (awkward: caller can't tell "displaced old
  value" from "rejected new value"), or out-of-range on a linear element
  is refused differently. Simplest honest shape: `set` requires the index
  in range as a documented contract and answers the displaced `T`
  (non-optional), with the out-of-range behavior being the same as the
  backends' (panic/exception) — but "never silently wrong" argues against.
  **This corner is the decision**; the heap only needs `swap`, so `set`
  can also simply wait.
- Rust lowering: `Vec::swap` is exactly `swap`; `std::mem::replace(&mut v[i], e)`
  is `set`. Kotlin: an `also`-captured `list[i] = e` pair. Both trivial in
  each backend's `intrinsics.rs`.
- Naming/`bytes` parity: `Bytes.set` returns `None` (Copy world); `List.set`
  answering the old value diverges by name. Alternative name: `replace`.

**Recommendation:** ship `swap` now (unblocks the heap, no semantic
corners), put `set`/`replace`'s out-of-range-with-linear-element question
to the user only when something needs it.

**Effort:** small — one intrinsic in std + two backend lowerings + e2e
cases (a new `KotlinCase` registry entry and a Rust runner test per
AGENTS.md).

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
3. **#2 (D2)** — the establishment rule; after it, `heap_push` keeps its
   natural `Mut`-parameter shape. (Meanwhile: rewrite the demo to
   consume-and-return `as Heap`, which works today.)
4. **#4 (`swap`)** — small std intrinsic; after the above the heap
   actually compiles and runs, making it a real e2e case.
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
