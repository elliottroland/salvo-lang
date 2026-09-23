# Refinement types via qualifiers — the option space (working document)

Status: **DECIDED, complete** — four rounds landed 2026-09-23, the final word
("keep provenance, document its range") included; the sequence is approved
and every question is closed. What remains is the owed propagation (header
note) and then this file's deletion per its charter.

Written 2026-09-23, in a **read-only session** (another agent held the
workspace), from the user's sketch of that day: explore a simple refinement- or
dependent-type layer for Salvo built on qualifiers, so that provably safe code
needs fewer assertions — the motivating example being a loop from
`list.size() - 1` down to `0`, inside which `get(list, i)` should answer an
element rather than an optional. This document lays the design out in
options/trade-offs/recommendations style, per DESIGN_DOC.md, **and the calls
are the user's** (AGENTS.md's first invariant).

**Second sitting, same day:** the user signalled liking the local-tied
qualifier axis ("similar to how we tie a qualifier to a named function" —
RT-2's direction, noted there as a signal rather than a decision) and asked
for a wider set of worked sites to think the feature through against. That is
the **catalog** section, inserted between §2 and RT-1: each entry names the
site, sketches the claim, and says which RT decisions it exercises.
**Third sitting:** the user named `KeyOf` and `Span` the catalog's most
interesting entries; the catalog gained two worked sketches — user-written
`KeyOf` with the four syntax expansions it fixes, and `InRange` spelling out
the constant-slot fork. **Later the same sitting the first decisions landed**
(see "The decided list"): the argument-block syntax (`KeyOf<K, V>(map: Map<K,
V>)`, `proj(list)`), all-or-none type generics, the dependent `qualifies` and
`is KeyOf(m)` expansions, and constants sequenced after locals. Spellings in
the catalog and in RT sections written *before* that round still read
`Q<value>` / `proj[from: list]`; they are kept for the argument trail — **the
decided spelling is the round-bracket form**, and the two worked sketches
have been updated to it.

**Propagation owed.** This session may write only this file. Until the user
decides, *nothing has propagated*: no COMPLETED.md decision-log entry, no
ROADMAP.md section (the decisions-pending table below is written so it can be
lifted into "Decisions waiting on the user" nearly verbatim), no
LANGUAGE_SPEC.md / backend-spec rules, and no fresh labels — the candidate
labels named below (`[qual-value-arg]`, `[qual-depend]`, `[col-idx]`, …) are
**proposals, not citations**. Delete-when-decided charter: once the user has
made the calls, fold the outcomes into COMPLETED.md's log and the specs, turn
the table's rows into ROADMAP.md plans, and **delete this file**.

Sources: LANGUAGE.md "Qualifiers continued", "Deductions", "Assertions";
LANGUAGE_SPEC.md "Qualifiers" ([qual-of], [qual-subject], [qual-predicate],
[is-qualifies], [qual-ctor-fn], [qual-ctor-predicate], [qual-refn],
[qual-refn-narrow], [qual-refn-conflict], [qual-erasure], [qual-generic]),
"Deductions" ([deduce-syntax], [deduce-gained], [deduce-reapply],
[deduce-infer]), "Shared fate" ([fate-link], [readonly-return]), the ordering
round ([cmp-carry], [cmp-binder], [implicit-param], [implicit-resolve]), the
collections rules ([col-of-nonempty], [col-sorted-list], [col-bounds]), the
assertion rules ([assert-op], [assert-narrow], [assert-trap]),
[fn-overload-rank], [iter-protocol], [backend-parity]; ROADMAP.md "Deductions
and qualifier reasoning" (D4), "Assertions — what is left" (A-6), "Variance on
generic parameters", "L7 remainders"; COMPLETED.md's log entries of 2026-09-22/23
(the ordering round, `+Q`, [deduce-gained], [qual-refn-narrow], the assertions
round A-1…A-7); `std/heap.sv` and `std/core/{list,range}.sv` as the live
precedents; and the user's message of 2026-09-23. Every label cited above was
verified present in the specs before citing.

## §0. The stated intent

The user's sketch, numbered so the decisions below can be judged against it.
It is exploratory ("I would like to explore the possibility") — it is used
here to guide the options and their cost, not to fix answers.

1. **Something resembling a simple refinement or dependent type system**, by
   leveraging qualifiers — not a new subsystem beside them.
2. **Building on what qualifiers already carry**: implicit generics landed in
   qualifiers (`Sorted<T, ?cmp>`, `Heap<T, ?Ordered<T>>` — an ordering
   *identity* lives in the type), and the question is whether the same axis
   can carry more.
3. **The goal is fewer assertions in otherwise safe code**: where the program
   already guarantees a fact, the type system should carry it, so nothing is
   asserted twice and nothing can fail.
4. **The worked example**: looping from `list.size() - 1` down to `0`, the
   body should not need `get(list, i)!` — `i` is within bounds by
   construction of the loop.
5. **Kinship with preconditions** in other languages — the same machinery
   should read as "this function requires an in-range index", stated in the
   signature.

How the points meet the record: (1) and (2) fit the recorded direction outright
— qualifiers are the language's one mechanism for claims, and the ordering
round already put non-type information (a function identity) into them
[cmp-carry]. (3) is the assertion ladder's own charter (LANGUAGE.md
"Assertions": *prove it* is rung 1, *assert it* is rung 4 — this work grows
rung 1). (4) collides with three fixed points (§1: erasure, the
subject-sees-only-itself rule of `qualifies`, and mutation stripping being
per-parameter), and those collisions are decision sections RT-2, RT-3 and
RT-6. (5) meets [qual-refn-narrow], which already names extra qualifiers on a
refinement's parameter a **precondition** — RT-7 folds it.

## §1. Fixed points — already decided, inherited here

- **Qualifiers erase** [qual-erasure]. Only compile-time consequences survive:
  overload choice, casts, predicate calls, union arm choice. Whatever this
  design proves must land as one of those — concretely, as *overload choice*
  (a total `get` beside the optional one). It follows that the win is a
  **source-level** win: fewer `!`s, fewer `None` arms to handle. The emitted
  code still uses the host's checked indexing (see §2's backends section), so
  this is about program text and failure surface, not machine performance.
- **The assertion ladder is decided and built** (A-1…A-7, 2026-09-23):
  `expr!`, `assert!(cond)` with permanent then-narrowing [assert-narrow],
  `unreachable!()`, one trap text on both backends [assert-trap]. Rung 4
  exists and stays; this document is about moving specific sites up to rung 1.
- **A state claim is stripped by mutation of its subject** [deduce-syntax]: a
  parameter the body mutates lists exhaustively what survives, and the claim's
  *owner* re-establishes across calls it does not own, via refinements
  [qual-refn]. **Collision**: an index claim is invalidated by mutating *the
  list*, not the index — stripping today is strictly per-parameter, and no
  rule invalidates a claim held by one value when *another* value is mutated.
  That is RT-3.
- **`qualifies` sees only its subject** [qual-predicate]: exactly one
  parameter, of the `of` type. **Collision**: "is a valid index of `xs`"
  cannot be tested by a function that sees only the `Int`. That is RT-6.
- **Qualifier slots hold function identities, resolved by name**
  [cmp-carry] [implicit-resolve]: `Sorted<T, ?cmp>` names the ordering the
  list was sorted by, fixed at construction, erased at run time, re-supplied
  at each operation by the binder [cmp-binder]. This is the "implicit generics
  in qualifiers" of §0(2). **Collision**: the slot mechanism resolves *named
  functions*; an index claim must name a *local value* (the list the index is
  valid for), which no slot can hold today. That is RT-2.
- **Types already name parameters, in two places.** A derived return is
  `proj[from: list] T` [readonly-return], and a deduction entry names the
  parameters it is about [deduce-syntax]. So "a type mentions another
  parameter of the same signature" has precedent; what is new is doing it in
  *parameter position* and inside a qualifier.
- **The checker already tracks value-to-value relationships as flow state**:
  fate links [fate-link] — directed, transitive, per-projection, merged at
  branch joins. And the L7 remainders record **"parameterized compiler
  qualifiers as checked signature vocabulary"** as the leading design for
  folding links into the type presentation (ROADMAP.md "L7 remainders"). A
  dependent index claim is arguably that design's first customer.
- **Qualifier-ranked overloads are live precedent**: `first(list: NonEmpty
  List<T>) -> proj[from: list] T` beside `first(list: List<T>) -> proj T?`,
  ranked by [fn-overload-rank] [col-of-nonempty]. The whole consuming surface
  of this design (RT-5) is more of exactly this.
- **Mint-only qualifiers are live precedent**: `Sorted` has no `qualifies`
  ("deciding whether a list happens to be sorted compares its elements, which
  nothing can do over an unconstrained `T`") and is minted by `sort` alone
  [col-sorted-list]. A claim can be useful with no runtime test at all.
- **Preconditions exist, narrowly**: a refinement may write a narrower
  parameter than the declaration it refines, and the extra qualifiers are a
  precondition — applies only where the argument already carries them
  [qual-refn-narrow]. §0(5) generalizes the *posture*, not the rule.
- **Out-of-range stays a defined runtime event**: `get` answers `None` past
  the end, `swap` answers `false` [col-bounds], and the A-6 decision makes a
  bare subscript trap with Salvo's own message. Nothing here changes the
  untotal surface's meaning; the design adds a *total* surface beside it.
- **"Salvo assumes it can see everything"** (user decision 2026-09-03): facts
  are reached by *declaring* them, never by the compiler recognizing a
  pattern. A checker that silently understands `range(xs.size() - 1, -1, -1)`
  would breach this; a std function that *says* it yields valid indices would
  not. This cuts hard through RT-1 and RT-4.
- **Backend parity** [backend-parity] and **never silently wrong**
  [backend-never-wrong]: whatever is proven must hold identically on both
  backends, and an unproven use must stay an optional/trap, never an
  unchecked read. No option below may ever lower to `get_unchecked` or its
  JVM equivalent.
- **D4 is open** (predicate `is` on union subjects, ROADMAP.md): index claims
  on union-typed values inherit that limit as-is; nothing below reopens it.
- **Sequencing**: the variance section is "sequenced after the refinement
  work and the open defects" (user direction 2026-09-23) — where "the
  refinement work" is the *refn* rounds already landed ([qual-refn-narrow],
  [deduce-gained]), not this document. This design is currently unsequenced.

## §2. What other languages teach

### The liquid family — LiquidHaskell, F*, Flux (and Dafny beside them)

The pattern: types carry logical predicates over program values
(`{v:Int | 0 <= v && v < len xs}`), and an SMT solver discharges the
obligations; inference ("liquid inference") reconstructs most annotations from
templates. Dafny reaches the same facts through `requires`/`ensures` contracts
and loop invariants, also SMT-discharged. The user's exact example is these
systems' opening demo: array bounds proved from loop arithmetic.

Two lessons worth stealing and one warning:

- **Flux's key insight is Salvo-shaped**: refinement of *mutable* data is
  tractable exactly because Rust's ownership rules limit aliasing — the
  refinement system leans on ownership rather than fighting it (Lehmann et
  al., *Flux: Liquid Types for Rust*, 2023). Salvo's deductions and fate links
  play the same role: mutation is visible in signatures, so invalidation has
  somewhere to hook (informs RT-3).
- **The predicates that pay are boring**: the workhorse fragment is linear
  arithmetic over lengths and indices. Nobody needs quantifiers to delete
  `get(xs, i)!`.
- **The warning is the solver**: an external SMT dependency, solver-timeout
  flakiness, and error messages phrased as failed verification conditions.
  Salvo's compiler is self-contained Rust with diagnostics that name
  remedies; an SMT bolt-on would be the least Salvo-like component in the
  repository (informs RT-1, option C).

### Constraint domains without SMT — Wuffs, and DML/ATS before it

Wuffs (Google's language for untrusted file formats) proves **all** bounds and
overflow checks at compile time with **interval arithmetic plus "facts"** —
flow-scoped boolean notes refreshed by explicit `assert` statements in source —
and no SMT solver (wuffs `doc/note/interval-arithmetic.md`, `facts.md`,
`bounds-checking.md`). Dependent ML and ATS did the same a generation earlier
with type indices constrained to linear integer arithmetic, solved by a
built-in decision procedure. The lesson: a *deliberately weak* arithmetic
domain, checked by syntactic flow rules, covers the bounds-check use case
without a prover — at the price that the programmer restates facts the flow
lost (Wuffs code is dense with `assert` lines a human must supply). Informs
RT-1 option B directly, including its cost: Wuffs accepts that density because
its niche is codecs; Salvo's charter is a general-purpose language where that
density would land on every user.

### Preconditions as contracts — Eiffel, Ada/SPARK, Kotlin's `require`

Eiffel's `require` clauses check at run time and blame the caller; Ada's
predicate subtypes (`subtype Index is Integer range 0 .. N`) check at run time
in plain Ada and prove statically under SPARK; Kotlin's `require(i < size)` is
a library function that throws. The shared shape: **the precondition lives in
the signature, the check has a static tier and a runtime tier, and the runtime
tier blames the right party.** Salvo already has all three ingredients
separately — a qualifier on a parameter is the signature statement
[qual-refn-narrow], `is`/`assert!` are the runtime tier [assert-narrow], and
diagnostics name remedies. Informs RT-6 and RT-7: the design should make the
static tier and the runtime tier the *same vocabulary*, the way Ada does, not
two systems.

### Indices by construction — Idris's `Fin n`, and Rust's iterator posture

Idris types an index as `Fin n` — an integer *statically smaller than `n`* —
and a `Vect n a` lookup takes one, so out-of-bounds is untypeable; indices are
obtained from constructors and from functions that answer them
already-bounded. Rust reaches the same end without dependent types by
**refusing the index**: the idiomatic loop is `for x in xs.iter()` /
`xs.iter().rev()` / `.enumerate()`, and the standard library grows vocabulary
(`windows`, `chunks`, `zip`) precisely so that raw indexing is rare; the
bounds check disappears because no index ever exists. Both inform RT-4: the
strongest form of "the loop's index is in bounds" is a loop that either gets
its indices from the list itself (Idris) or never materializes them (Rust).
Salvo's passes [iter-protocol] can do either.

### Flow typing that stops at tags — TypeScript, Kotlin smart casts

Both narrow on tags, `typeof`/`is`, and nullability — and **neither tracks a
single arithmetic fact** (`if (i < arr.length)` narrows nothing in either).
Both are regarded as dramatically useful anyway. The lesson for RT-1: a
nominal-only evidence model is not a consolation prize; it is where two of the
most-used type systems in industry deliberately stopped, because tags cover
most of the safety and none of the solver cost. Salvo is *ahead* of both here
already — [assert-narrow] and predicate qualifiers are more than either has.

### What Kotlin and Rust do — the backends

Every decision must land in both [backend-parity], and here that is cheap:
**qualifiers erase** [qual-erasure], so a proven-total `get` lowers to exactly
what the optional one lowers to minus the `None` path — Kotlin `list[i]`
(JVM bounds check, `IndexOutOfBoundsException` the claim guarantees dead),
Rust `&xs[i]` (panic likewise dead). Neither backend may ever receive an
*unchecked* access: the host's check stays as the last line, per
[backend-never-wrong], and both JIT and LLVM routinely elide checks they can
see are dead, so the residual cost is theirs to remove. Nothing in this design
needs a new lowering rule in either backend — the entire feature is
checker-side, which is its strongest engineering property.

## The catalog — worked sites for dependent claims and checked preconditions

Added at the user's request (second sitting): variety to think the feature
through against. Grouped by mechanism; each entry names the site, sketches the
claim in illustrative spelling (`Q<value>` — the real spelling is RT-2's
question), and says which decisions it exercises. Group A calibrates what the
language already covers; groups B–D are the feature; group E marks the
boundary where the recommended (nominal, RT-1 A) model stops.

### A. Already expressible — the baseline this feature builds on

Worth listing so the new machinery is judged by what it *adds*:

- **`NonEmpty`** — `first`/`min`/`max` answering elements, the ranked-overload
  pattern every entry below reuses [col-of-nonempty].
- **`Positive` / `NonNegative`** predicate qualifiers — LANGUAGE.md's own
  examples; `is` and `assert!` already establish them [is-qualifies]
  [assert-narrow].
- **`Sorted<T, ?cmp>` and `Heap<T, ?cmp>`** — a claim carrying a *function
  identity*, so `binary_search` searches by the ordering that sorted the list
  [col-sorted-list]. The precedent the whole dependent axis extends.
- **Provenance**: `Authenticated Request` — a statically checked "this went
  through the checkpoint" precondition, no dependency needed [qual-subject].
- **Nominal unit tags**: `Meters Int` vs `Seconds Int` by plain qualifiers;
  std's `Instant`/`Tick` separation is the same idea as types. No new
  machinery — listed because refinement-type pitches often lead with units,
  and Salvo already has that story.

### B. Dependent claims over collections — the `Idx` family

1. **`KeyOf<map>` — the double-lookup site.** Today:
   `if contains_key(m, k) { get(m, k)! … }` — the `Bool` throws away what it
   just proved. With a dependent claim, `k is KeyOf<m>` (RT-6 B: a dependent
   `qualifies` is exactly `contains_key`) narrows `k`, and a total overload
   `get(map: Map<K, V>, key: KeyOf<map> K) -> proj[from: map] V` answers the
   value. Kept by `put` (a refn: writing never removes a key), stripped by
   `remove`/`drain` (RT-3). The `keys(map)` pass and a `for` over the map
   mint it for every key they yield (RT-4). Exercises every RT at once —
   arguably a better v1 driver than `Idx`, because map lookups outnumber
   descending index loops in ordinary code.
2. **`Idx<list>`** — the motivating example; as analyzed throughout.
3. **`InsertionPoint<list>` — one subject, two different bounds.** A
   `binary_search` miss is a valid *insertion* index — `0..=size`, one more
   than `Idx`'s `0..size`. `insert_at(list, at: InsertionPoint<list> Int,
   elem: T)` total; every `Idx` is an `InsertionPoint` but not the reverse.
   Exercises RT-5 (producers mint; the search's found/miss arms mint two
   *different* claims — a union whose arms carry different dependent
   qualifiers, touching D4's territory) and shows claims about one subject
   forming a small lattice.
4. **Range pairs — `substr`, `slice`.** `substr(str, start, end) -> Str?` and
   `slice(data, start, end) -> Bytes?` are optional because *three* facts must
   hold: both ends in range and `start <= end`. The pairwise fact is
   arithmetic — out of reach under RT-1 A as separate claims on two `Int`s —
   but **parse-don't-validate closes it**: mint the pair as one value,
   `span(data, start, end) -> (Span<data>)?`, and overload
   `slice(data: Bytes, at: Span<data>) -> Bytes` total. The optionality moves
   to one mint site instead of recurring at every use. Exercises RT-2 (a
   claim on a *tuple*), RT-5, and is the pattern for every multi-part
   precondition the nominal model cannot decompose.
5. **Parallel collections — one index, two lists.** Struct-of-arrays layouts,
   a `scores` list beside a `names` list: `for i in indices(names)` should
   index `scores` too. Needs a relation *between two lists* (`SameLen<names>
   List<Int>`), minted where they are built together or tested once
   (`assert!(scores is SameLen<names>)`), kept by paired `add`s, stripped by
   either's shrink. Exercises RT-3 hardest (invalidation now watches two
   values) — and is honestly served meanwhile by `zip`/`enumerate` (RT-4 C),
   which is the answer to give until the machinery earns this.
6. **Arena/graph indices — ids as the data model.** A graph of nodes holding
   edge lists of node ids, an AST arena, a compiler's span-into-source-buffer
   (`Span<src>` — this repository's own `salvo-syntax` pattern): every hop is
   `get(nodes, id)!` today. `NodeId<graph> Int` stored *inside* node structs
   makes traversals `!`-free — and immediately raises the storage question:
   a dependent claim on a **struct field's** type names a value the struct
   does not contain. Exercises RT-2 beyond its v1 (claims crossing not just
   signatures but *data*), so it is the case to keep in view when choosing
   how far the dependency axis eventually reaches; append-only arenas (no
   strip sites) are where it pays first.

### C. Dependent claims beyond collections

7. **A reading and the clock that minted it.** `time` keeps `Instant` and
   `Tick` apart so a deadline cannot be measured against a steppable clock —
   but two *`Tick`s from different tickers* still compare freely, and under
   `ManualTime` plus a real `Ticker` that is a live confusion. A dependent
   claim (`From<ticker> Tick`) would extend the module's own posture to
   plural clocks. Exercises RT-2 with a non-collection dependency (the linked
   value is a handler), and RT-7 (this wants to be *declarable*, since std
   cannot foresee every clock-like resource).
8. **Handles and their pool/registry.** A texture id valid for the renderer
   that produced it, a statement handle valid for its database connection, a
   worker index valid for `pool(n)`: `ValidIn<conn> Stmt`. Same shape as the
   arena case; invalidation on the *owner's* reset/close (RT-3), and the
   linear-types overlap is worth noting — where the handle is linear, the
   obligation story already exists, and the dependent claim adds only the
   "right owner" fact.

### D. Non-dependent checked preconditions — the free round (RT-7's note)

9. **`NonZero Int` divisors — pairs with A-6.** The trap-policy row makes
   division by zero trap with Salvo's message; a
   `div(a: Int, b: NonZero Int) -> Int` overload moves the failure to the
   signature. **Wrinkle worth recording**: under RT-1 A even the literal `2`
   is not `NonZero` — nothing minted it. Either constant literals get a
   narrow establishment rule (a literal is the one place the checker *can*
   see the value; a special case, but a principled one), or the vocabulary
   stays for variables and literals keep plain `/`. This wrinkle applies to
   every predicate qualifier over constants and deserves its own line in any
   build plan.
10. **Bounded scalars — ports, percentages, RGB.** `InRange<0, 65535>` wants
    a slot holding a *constant*, which is a third slot kind (functions
    [cmp-carry], locals (RT-2), now literals) — const generics through the
    same door. Cheaper alternative that works today: named qualifiers
    (`qualifier Port of Int` with `qualifies`), one per meaningful range —
    less general, zero new axes. The catalog notes the fork; nothing below
    depends on it.
11. **Parse, don't validate — the string family.** `Email Str`, `Url Str`,
    `Hex Str`, `Utf8 Bytes` minted by their parsers ([qual-ctor-predicate]:
    the constructor asserts, callers skip the re-check); `Sanitized Str` /
    `Escaped Str` for injection safety, where **provenance** [qual-subject]
    is the honest axis (being escaped is about where the string came
    *through*, and string contents cannot prove it). All expressible today;
    listed because it is the highest-value *precondition* material in
    ordinary services, and because a std or example-directory showcase of it
    costs nothing from this design.
12. **Typestate-lite — protocol order.** "Write only after handshake":
    `Connected Socket` minted by `connect`, stripped by `close`'s exhaustive
    deduction, required by `send(s: Connected Socket, …)`. State claims plus
    linearity already carry this (streams do it now — `InStream` is linear
    and forward-only); dependent claims add nothing here, which is itself
    useful calibration: not every precondition wants the new axis.

### E. The boundary — what stays out of reach under RT-1 A, on purpose

13. **Length arithmetic.** `concat(xs, ys)` having size `m + n`, matrix
    dimensions through multiplication, `chunks(n)` yielding `n`-sized pieces:
    the Idris `Vect`-arithmetic tier. No option short of RT-1 C reaches it;
    the nominal model's answer is producer vocabulary (a `Matrix` struct that
    *owns* its invariant) rather than proofs.
14. **Index arithmetic — with two redeeming producers.** `i - 1` on an
    `Idx<xs>` is plain (the descending *hand-rolled* loop stays out of
    reach; `rev_indices` is the answer), **but** two arithmetic-looking facts
    are producer-mintable without any arithmetic in the checker:
    `index_mod(list, i) -> Idx<list> Int` (`i % size` lands in range whenever
    the list is `NonEmpty` — a ring buffer's whole index story), and
    `read_to(s, buf, max) -> Ok Int | Err FsError`, whose count is by
    construction at most the buffer's length — the intrinsic *declares* what
    it returns, so the fill-a-buffer loop can slice without a check. The
    boundary is thus not "no arithmetic facts" but "arithmetic facts enter
    only where a declaration states them" — RT-1 A's thesis in one line.
15. **Loop invariants proper** (Dafny's `invariant` clauses, Wuffs's
    re-asserted facts): out of scope under every option except C, and the
    explicit non-goal that keeps this design small.

### Worked sketch — user-written `KeyOf`, and the syntax inventory it fixes

Added third sitting (the user named `KeyOf` and `Span` the most interesting
entries and asked what the syntax must grow for a *user function* to determine
`KeyOf(map)`). The sketch, updated to the decided argument-block spelling,
written as if in `core.map` — or user code, which is RT-7 Option B:

```
export qualifier KeyOf<K, V>(map: Map<K, V>) of K {
    // Dependent predicate: subject first, then one parameter per value slot.
    fn qualifies(key: K, map: Map<K, V>) -> Bool {
        return contains_key(map, key)
    }

    // Growth never removes a key; the claim's owner says so.
    refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf
}

// The total overload, ranked above the optional one [fn-overload-rank].
export fn get<K, V>(map: Map<K, V>, key: KeyOf(map) K) -> proj(map) V

// Use sites.
if k is KeyOf(m) { let v = get(m, k) }      // total; no `!`
assert!(k is KeyOf(m))                      // hoisted; narrows the rest of the scope
```

Exactly **four syntax expansions** carry it; the rest is checker semantics:

1. **A value-argument block** — `(map: Map<K, V>)` after the type generics
   (**decided**, third sitting): round brackets, resembling a function's
   parameter list; `?` prefixes *implicit* (fn-identity) slots only
   (`Heap<T>(?Ordered<T>)`), local references are unprefixed. Type generics
   are **all-or-none** — written in full or all inferred, never partially
   dropped.
2. **An expression in the block at use sites** — `KeyOf(m) K`, and in a
   signature the identifier must name a **sibling parameter**
   ([readonly-return]'s rule for `proj(list)`), in a local annotation a
   visible local. (**Decided** with the block syntax.)
3. **`qualifies` grows dependency parameters** — [qual-predicate]'s "exactly
   one param of the `of` type" becomes "subject first, then one per value
   slot, matching the slot's type". The predicate body is ordinary Salvo
   (`contains_key`), which is RT-6 B's point: the runtime tier costs
   nothing. (**Decided**.)
4. **`is` with a filled block** — `k is KeyOf(m)`, lowering to
   `KeyOf.qualifies(k, m)` through the existing `predicate_tests` path;
   [assert-narrow] rides along unchanged. (**Decided**.)

Checker semantics behind them (no new surface): value identity in the type —
`KeyOf(m) ≠ KeyOf(m2)`, presumably bound to the **fate root** [fate-link] so
`let m2 = m` does not orphan claims; cross-value stripping on any `Mut` use of
`m` (RT-3); the `keeps` refn entry kind — a refinement speaking about claims
*other values* hold whose slot points at this parameter, a new AST shape
beside `+Q`/`-Q` [qual-refn]; and overload resolution matching the argument's
claimed slot against the sibling argument's identity.

**A gotcha with precedent**: the total `get` cannot delegate to the optional
one naively — `get(map, key)!` in its body re-resolves against the
still-claimed key, picks itself, and recurses. `first(NonEmpty)` dodged its
twin by delegating to `get(list, 0)!` rather than `first@core.list`
[col-of-nonempty]. Here the remedy is dropping the claim first (qualifiers are
droppable — user decision 2026-09-23): rebind the key at plain `K`, then call
the optional overload. Whatever build plan lands should test this shape on
day one.

### Worked sketch — `InRange`, the constant slot spelled out

Catalog entry 10, expanded at the user's request, respelled to the decided
block form. Without new machinery, a bounded scalar is one qualifier per
range, bounds hardcoded in the body (`qualifier Port of Int { fn qualifies(n:
Int) -> Bool { return n >= 0 && n <= 65535 } }`, likewise `Percent`, …) —
workable, but each range is a fresh unrelated declaration. A **constant
slot** puts the bounds in the type, one declaration for all ranges:

```
qualifier InRange(lo: Int, hi: Int) of Int {
    fn qualifies(n: Int) -> Bool { return n >= lo && n <= hi }   // slots in scope
}

fn listen(port: InRange(0, 65535) Int) [Net] -> None
fn set_volume(pct: InRange(0, 100) Int) -> None

if n is InRange(0, 100) { set_volume(n) }
```

`InRange(0, 65535) Int` and `InRange(0, 100) Int` are distinct types the way
`Heap(min_by_age)` and `Heap(max_by_age)` are [cmp-carry] — same mechanism,
different block content. What makes constants a **third slot kind** is that
each kind carries different rules: implicit (`?`) slots resolve by name and
erase to an implicit argument [cmp-binder]; local slots (RT-2) bind by value
identity, are flow-tracked, and invalidate (RT-3); constant slots are
compile-time known, need no flow tracking and no invalidation, and open two
doors the others do not — **constant subtyping** (`InRange(10, 20)` fitting
where `InRange(0, 100)` is expected, a cheap literal comparison at check
time) and **literal establishment** (`listen(8080)` provably fine on sight —
the entry-9 wrinkle resolved for free in exactly this kind). This is const
generics arriving through the qualifier door. **Decided** (third sitting):
constants are sequenced **after** locals; note the declaration reuses the one
block — the slot kinds are told apart by `?` (implicit) and, for constants
vs locals, by what fills them, a distinction RT-8 must keep clean.

## The decided list — third sitting, 2026-09-23 (user decisions)

1. **The argument block.** A qualifier's value-level arguments — implicits,
   local references, and (later) constants — live in a **round-bracket block
   after the type generics**, resembling a function's parameter list:
   `Heap<T>(?cmp: (T, T) -> Int)`, `Heap<T>(?Ordered<T>)`,
   `KeyOf<K, V>(map: Map<K, V>)`. The `?` prefix marks **implicit** slots
   only (resolved by name [implicit-resolve]); local references are
   unprefixed. This replaces mixing value slots into the `<…>` list.
2. **All-or-none type generics.** A use site writes **all** of a
   declaration's type arguments or **none** (all inferred); dropping some
   but not others — what `Heap<min_by_age>` did — stops being legal. So:
   `Heap(min_by_age)` or `Heap<Person>(min_by_age)`, never a partial list.
3. **`proj` respelled to match**: `proj(list)` — round brackets, `from:`
   dropped — in all three positions (`-> proj(list) T`,
   `=> .items: proj(list)`, `=> proj(list)`).
4. **RT-6 is decided: Option B.** `qualifies` grows one parameter per value
   slot (subject first); `k is KeyOf(m)` and `assert!(k is KeyOf(m))` test
   and narrow through the existing predicate path.
5. **RT-2 is decided in substance**: the local-tied axis, with the spelled
   block as its surface from the start (Option A's spelling over Option B's
   hidden-link-only v1), the link machinery underneath. Granularity
   questions moved to RT-8.
6. **Constants come after locals** (sequencing): the constant slot kind
   (`InRange(0, 65535) Int`) is adopted in principle but built as its own
   later round.

**Consequences to carry into the build:** the respell sweep — every
`Sorted<T, ?cmp>`-shaped generic list and every `proj[from: x]` in std, the
specs, examples, corpus tests and inline test sources is rewritten
(backwards compatibility is not owed; AGENTS.md's invariant); constructor
return positions respell too (`as Sorted<T>(?cmp)` — the `?cmp` there still
names the fn's own implicit parameter [cmp-binder]). Fn signatures are
untouched: their implicit parameters already live in round brackets.

**Confirmations owed (implied by this round, not yet stated):** RT-1 Option A
(the whole round presumes nominal evidence — no checker arithmetic), and
RT-7 Option B in place of the recommended A (the sketch the user approved is
*user-written* `KeyOf`, so declarability is assumed; §RT-7's std-first
recommendation is superseded if so). One word each suffices.
*Both confirmed in the second round, below.*

### The decided list — second round, later the same evening (user decisions)

7. **RT-1 = A confirmed** (declarations-only evidence) and **RT-7 = B
   confirmed** (dependent qualifiers are user-declarable).
8. **RT-8 decided as recommended**: slots are filled by **places**
   (identifiers and field chains, fate provenance's domain), bound to
   **fate roots**.
9. **RT-3 and RT-11 decided: both sides** — preservation entries are legal
   in refns *and* in a fn's own deduction clause (checked against the body
   like [deduce-gained]) — **spelled `preserve`, not `keeps`**. The user's
   two reasons, recorded: "kept" already means *not consumed* in deduction
   vocabulary, and the entry's parameter (the map) is not the party holding
   the qualifier — other values' claims are what survive. Amended during the
   build (2026-09-23): **`preserve`**, imperative, agreeing with `defer` on
   send fns rather than the third-person `preserves`. So:
   `refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf`.
10. **RT-10 decided: the full return-type spelling (B)** — and a respell
    with it: **`-> T as Q` is replaced by `-> +Q T`** (the user has meant to
    make this change since `as Q` landed; it joins the step-1 respell
    round). One trust rule everywhere, the same two spellings deductions
    already have [deduce-reapply]: **`+Q` establishes** — trusted, legal
    only in the qualifier's own file — and **plain `Q` reports** — checked
    against the body. So `binary_search` writing `-> (Idx(list) Int)?`
    *proves* the claim (or, living in `Idx`'s file, writes
    `-> (+Idx(list) Int)?` and is trusted).
11. **RT-9: keep [qual-no-dup] for now.** The user sketched the eventual
    lift as an explicit **`with` self-compatibility** —
    `qualifier KeyOf<K, V>(map: Map<K, V>) of K with KeyOf<K, *>(map:
    Map<K, *>)` or similar — i.e. a claim declares *itself* stackable across
    different argument fillings, reusing the compatibility clause
    [qual-with] instead of a blanket by-arguments dedup. Recorded in RT-9 as
    the candidate design when the restriction is lifted.
12. **RT-5 decided as a starting list** — `get`/`swap`/`substr`/`slice`
    total, `binary_search`/`span` minting — explicitly extendable later.
13. **RT-13 decided: `Span` is a struct.** The tuple restriction was
    re-verified on request — real, enforced, and load-bearing; see RT-13.
14. **The sequence is approved**, with step 1 gaining the `as Q` → `+Q T`
    respell and step 0 detailed in place (see "The proposed sequence").
    **RT-12 stays open**: the user asked to work through the examples first;
    the section now carries them and its residual sub-decisions.

### The decided list — third round (user decisions)

15. **RT-12b and RT-12c decided as recommended**: declaration-side slots
    (`self.field` in Yield clauses, `p.field` in `next` returns, proj-link
    substitution at the mint), and the `iter fn` sugar carrying the same
    element spelling.
16. **RT-12a sharpened, one call outstanding**: the user identified
    `Emitted`, `Ok`, `Err`, `Thrown` as protocol tags and asked whether that
    is a third qualifier kind or provenance. The analysis (in RT-12a) lands
    on **provenance — no third kind**: reclassify the four as
    `provenance qualifier` in std. Awaiting the user's yes/no on the
    reclassification.

### The decided list — fourth round (user decision)

17. **RT-12a decided: the reclassification is accepted** — `Emitted`, `Ok`,
    `Err`, `Thrown` become `provenance qualifier` in std; no third qualifier
    kind. The user then asked whether "provenance" remains the right word or
    should become "protocol"; the recorded recommendation is **keep
    "provenance"** (it names the mechanism, true of both the authority and
    the protocol-role families; "protocol" would mislabel `Authenticated`;
    "origin" is the plainer respell if ever wanted; the connotation gap is
    documentation's to close). One word from the user settles the name.
18. **The word stays "provenance"** (user decision, closing the design):
    keep the keyword, and **document its range** — LANGUAGE.md's provenance
    section teaches the two families side by side, authority
    (`Authenticated`, `EnvironmentId`) and protocol role (`Ok`, `Err`,
    `Thrown`, `Emitted`), as two uses of one claim kind, using the examples
    this design produced. The documentation change rides step 1 with the
    reclassification it explains.

## RT-1. The evidence model — where "in bounds" comes from — ✅ confirmed 2026-09-23 (A)

**The load-bearing decision**; every later section assumes its answer. §0(3)
asks for fewer assertions; the question is what may count as *proof*.

- **Option A — nominal claims only (extend the existing machinery, add no
  arithmetic).** A fact enters the type system exactly the ways it already
  does: minted by a constructor (`-> T as Q` [qual-ctor-fn]), promised by a
  checked deduction ([deduce-gained]), kept by a refinement [qual-refn],
  tested by `is` / `assert!` [is-qualifies] [assert-narrow]. The checker never
  learns that `i < xs.size()` from a comparison; it learns "this `Int` is a
  valid index of `xs`" because a declared function *said so* — `indices(xs)`,
  `rev_indices(xs)`, `binary_search`'s found arm. Cost: index *arithmetic*
  stays outside the proof — `i - 1` on a claimed index is a plain `Int`, and
  a site the vocabulary does not cover still writes `!`. Trade-off: zero new
  checker theory (the work is std vocabulary plus the dependency axis of
  RT-2/RT-3), diagnostics stay in the language's own words, and it is exactly
  the posture "Salvo assumes it can see everything" prescribes — facts by
  declaration, not recognition.
- **Option B — a bounded arithmetic domain (Wuffs-shaped).** The checker
  tracks interval/relational facts for `Int` locals from comparisons and
  literals (`if i >= 0 && i < xs.size() { … }` establishes the claim in the
  then-branch; `for i in range(0, xs.size())` derives it from `range`'s
  arguments), invalidated when `xs` is mutated. Cost, itemized because it is
  the expensive option: a fact language over calls (`xs.size()` is a call —
  it needs a purity/stability notion no rule currently provides); loop
  back-edge reasoning (today's flow state re-checks loops without widening;
  intervals need widening or severe restriction); a second, structural kind
  of flow fact beside qualifier sets and union arms, reaching diagnostics,
  the LSP hover, and [type-unknown-lenient]'s one-error posture; and a
  standing tension with the no-recognition principle — `range` is *ordinary
  Salvo* (`std/core/range.sv`), so reading bounds out of `range(0,
  xs.size())` means either special-casing a std function or building
  postcondition vocabulary strong enough to state "emits integers in
  `[start, end)`", which is refinement types all the way down. Trade-off: it
  is the only option that proves the user's example *as written*, with
  `range` arithmetic and no new std calls.
- **Option C — shape-changing: SMT-backed refinements (the liquid family).**
  Full predicates over values, solver-discharged, liquid-style inference.
  Named because §2 shows it is where "reduce the need for assertions" ends if
  followed to the horizon, and it should be rejected *explicitly* rather than
  drifted away from: an external solver in a self-contained Rust compiler,
  solver time in a ~13s test suite, verification-condition diagnostics in a
  language whose errors name remedies, and a proof tier that could never be
  [backend-parity]-relevant since it all erases. If Salvo ever wants this, it
  wants it as a *separate analyzer*, not as the checker.

**Recommendation (the user's call):** Option A, with one deliberate seam: RT-6
defines how a *runtime-tested* claim is established from a comparison-shaped
`qualifies`, which is the one place Option B's flavor enters — as a checked
test, not as checker arithmetic. Option A covers the motivating example via
RT-4, costs no new theory, and leaves Option B adoptable later behind the same
vocabulary (a future arithmetic pass could *establish the same qualifier*
without changing a single signature).

## RT-2. The dependency axis — how a claim names the value it is about — ✅ decided 2026-09-23

**Decided (third sitting): the local-tied axis, spelled as the argument block
from the start** — `KeyOf(m) K` in annotations, sibling-parameter references
in signatures, fate-link machinery underneath. See "The decided list";
granularity questions moved to RT-8. The options below are kept for the
argument trail.

An index claim is a **relation**: `i` is in bounds *of `xs`*. Folds §0(2) and
the [cmp-carry] collision from §1. The claim's subject is the `Int`; the
question is how the type names the *other* participant. (Vocabulary note: this
is neither of [qual-subject]'s axes — not a claim about the `Int`'s contents
alone, not provenance of a handle. It behaves as a state claim whose truth
depends on another value's contents; RT-3 prices that.)

**User signal (2026-09-23, second sitting):** the user likes tying a qualifier
to a *local* the way one is tied to a named function today — i.e. this
section's direction, with overloading and refns doing the consuming — recorded
as a signal guiding the analysis, not yet a decision.

- **Option A — a value argument in the qualifier**: `Idx<xs> Int`, extending
  the slot mechanism from function identities [cmp-carry] to in-scope values.
  `qualifier Idx of Int` with a declared value slot; `get(list: List<T>,
  index: Idx<list> Int)` in signatures — precedented by `proj[from: list]`
  naming a sibling parameter [readonly-return]. Cost: types now mention
  locals, so type equality becomes value-identity-sensitive
  (`Idx<xs> ≠ Idx<ys>`), substitution and printing must carry binding sites,
  and the slot grammar (`<…>` holding an expression, or a restricted
  identifier) needs deciding. This is the genuinely dependent-type step, and
  the syntax is permanent surface.
- **Option B — a hidden link, fate-link style**: the surface type is plain
  `Idx Int`; *which* list it indexes is flow state, recorded at the mint
  (like [fate-link] records provenance) and consulted at use sites — a
  `get(xs, i)` resolves to the total overload only when `i`'s recorded link
  is `xs`. Inside one function this is exactly the machinery the checker
  has; across a **signature** the link must be spelled, which is the L7
  "parameterized compiler qualifiers as checked signature vocabulary" design
  arriving at its first customer — and it can arrive *later*: a v1 where
  `Idx` does not cross function boundaries (parameters never carry it; it is
  established and consumed within one body) is still enough for the worked
  example. Cost: a claim that silently fails to survive a call reads as a
  bug until the signature vocabulary lands; the diagnostic must say "an index
  claim does not cross a call yet — assert or pass the list".
- **Option C — shape-changing: no dependency axis at all.** Do not refine the
  index; make the *access* total instead — every element access happens
  through passes and paired vocabulary (`enumerate`, `reversed`, `zip`), and
  a raw `Int` index never carries a claim (Rust's posture, §2). Cost: sites
  that genuinely need an index — two lists advanced together by one index,
  an index answered by `binary_search` and used later — stay at `!`.
  Trade-off: zero type-system change; entirely std work; and honest about
  where the remaining `!`s are.

**Recommendation (the user's call):** B as the mechanism with A as its
eventual spelling — start link-local (no signature crossing), and take the L7
parameterized-qualifier surface as its own later round, where `Idx<list>` and
`proj[from: list]` should be designed as **one** notation rather than two.
Option C is not a rival but the floor: RT-4 recommends shipping its vocabulary
regardless, because it deletes most `!` sites before any type theory runs.

## RT-3. Invalidation — what mutating the list does to the index's claim — ✅ decided 2026-09-23

**Decided (second round, with RT-11)**: Option A — conservative cross-value
stripping (any call taking the linked value as `Mut` strips claims linked to
it), with `preserve` entries as the opt-back (spelling and both-sides rule
in RT-11). Sliceable via Option C, as the sequence's step 2 does. The trail:

The §1 collision: [deduce-syntax] strips claims on the *mutated parameter*;
an index claim lives on a **different value**. Without an answer, the design
is unsound: mint `Idx` from `indices(xs)`, `clear(xs)`, then `get(xs, i)`
resolves total and reads out of a shorter list.

- **Option A — conservative cross-value stripping, refinements opt back.**
  Any call that takes the linked list as `Mut` strips every index claim
  linked to it (the link from RT-2 says which). The claim's owner then writes
  refinements for the calls that in fact preserve validity — `add` (growth
  keeps every existing index valid), `swap`, `replace`-shaped writes — the
  exact posture [qual-refn] exists for, extended so a refinement entry can
  speak about *claims held by other values that depend on this parameter*
  (new spelling needed; candidates: `=> list: keeps Idx` or a dedicated
  clause). Loops that only read — the worked example — never strip. Cost:
  the refinement extension is new spec surface; forgetting one makes std
  *less useful*, never unsound.
- **Option B — invalidation classes on the qualifier.** `Idx` declares what
  invalidates it ("shrinking the list"), and mutators declare a class
  ("grows", "shrinks", "reorders"). Cost: a taxonomy nobody else needs,
  duplicated on every mutator, and a second vocabulary where refinements
  already are one; a wrong class is *unsound*, where a missing refinement in
  A is merely conservative. Named because it is the shape SPARK-like systems
  use, and rejected on that comparison: Salvo already has the per-call
  instrument.
- **Option C — shape-changing: no invalidation, because no persistence.**
  An index claim is *ephemeral*: it exists only between the expression that
  established it and the next statement — effectively, only compositions like
  `get(xs, binary_search(xs, e)!…)` or the `for` binding's own scope where
  the checker can see no intervening mutation. Cost: sharply less useful
  (hold an index in a variable across any call and it is plain again);
  benefit: no cross-value machinery at all. This is a coherent *first slice*
  of A rather than a true rival.

**Recommendation (the user's call):** A, shipped as C-then-A if slicing is
wanted: the worked example needs only "no mutation in the loop body", and the
refinement extension can follow with the std audit of RT-5.

## RT-4. The loop — how §0(4) actually gets its claim

The user's example, under RT-1 Option A: the claim must come from a
declaration, so the loop's index source must *say* it yields valid indices.

- **Option A — index passes in std**: `indices(list) -> …` and
  `rev_indices(list)` (the descending case *is* the motivating one), passes
  [iter-protocol] whose emitted element carries the claim linked to `list`.
  The `for` binding then holds `Idx Int` each iteration, RT-3 keeps it while
  the body does not shrink the list, and `get(xs, i)` resolves total (RT-5).
  What it needs mechanically: an `iter fn` whose element type carries a
  qualifier *established by the pass itself* — the element is already
  `Emitted Int` [iter-protocol], so this is a second state qualifier on the
  arm (`Emitted Idx Int`), which needs either a `with` declaration or the
  constructor-flavored form (`-> Emitted Int as Idx | Finished`) extended to
  arm position; and the link (RT-2) must flow from the pass's own list
  parameter to the caller's argument, which is a derived-return-shaped rule
  ([readonly-return] does precisely this for `proj`). Cost: those two
  extensions; benefit: ordinary Salvo, in std, no compiler recognition.
- **Option B — teach the checker `range` arithmetic**: recognize
  `range(0, xs.size())` / `range(xs.size() - 1, -1, -1)` and bound the
  emitted values. Rejected under RT-1 Option A by the no-recognition
  principle — `range` is ordinary Salvo source, and the alternative
  (postcondition vocabulary strong enough to state "emits values in
  `[start, end)`") is RT-1 Option B by the back door. Listed because it is
  the only option that makes the example compile *unmodified*; the user
  should see explicitly that the recommendation asks the loop to be
  rewritten from `range(xs.size() - 1, -1, -1)` to `rev_indices(xs)` — one
  call, and arguably the clearer sentence.
- **Option C — shape-changing: don't index — enumerate.** Ship `reversed(xs)`,
  `enumerate(xs)`, `enumerate_rev(xs)` (element-plus-index pairs) and let the
  common loops never hold a bare index at all (§2, Rust's posture; pairs
  with RT-2 Option C). The worked example becomes
  `for (i, x) in enumerate_rev(xs)` — and if the body only wanted `x`, the
  refinement layer was never needed. Cost: none to the type system; does not
  cover the sites that index *another* list with `i` — unless `enumerate`'s
  `i` also carries the claim, in which case this is Option A wearing
  friendlier vocabulary.

**Recommendation (the user's call):** C and A together, as one std round:
`reversed`/`enumerate`/`enumerate_rev` delete most index loops outright, and
`indices`/`rev_indices` plus claim-carrying `enumerate` indices serve the rest.
B is recommended against by name.

## RT-5. The consuming surface — what a proven index unlocks — ✅ decided 2026-09-23

**Decided (second round): the recommended list as a starting set, extendable
later** — `get`/`swap`/`substr`/`slice` total, `binary_search`/`span`
minting. The trail:

Folds §0(4)'s payoff. Pure precedent-following; listed as a decision because
it fixes std's shape.

- **Option A — overload the accessors, mirror `NonEmpty`**:
  `get(list: List<T>, index: Idx Int) -> proj[from: list] T` beside the
  optional `get` [col-idx candidate], ranked by [fn-overload-rank] exactly as
  `first(NonEmpty List<T>)` is [col-of-nonempty]; likewise a total
  `swap(list, i: Idx Int, j: Idx Int) -> None` beside the `Bool` one
  [col-bounds], and a total positional read for `Bytes`. **Producers mint**:
  `binary_search(list: Sorted<T, ?cmp> List<T>, elem: T) -> (Idx Int)?` — a
  found index *is* a valid index, and today's `Int?` throws that fact away.
  Cost: each overload is a new ranked pair to keep coherent, and the
  `binary_search` change touches its signature (backwards compatibility is
  explicitly not owed).
- **Option B — total-by-claim without overloads**: keep one `get` and make
  its return type conditional on the argument's claim (a type-level `when`).
  Rejected on sight for the record: Salvo resolves by overload ranking, and
  conditional return types are a second resolution mechanism duplicating the
  first.
- **Shape-changing note**: if RT-2 Option C were chosen (no index claims),
  this section reduces to `enumerate`-family additions only.

**Recommendation (the user's call):** A. It is what the `NonEmpty` precedent
was built to be repeated for.

## RT-6. The runtime tier — testing a dependent claim — ✅ decided 2026-09-23

**Decided (third sitting): Option B** — dependent `qualifies` (subject first,
one parameter per value slot), `is` with a filled block, `assert!` narrowing
unchanged. See "The decided list". The options below are kept for the
argument trail.

Folds the [qual-predicate] collision (§1): `qualifies` sees only its subject,
so `Idx` cannot be a predicate qualifier today. What replaces rung-4 `!` at
sites the static tier cannot reach?

- **Option A — mint-only, like `Sorted`**: `Idx` has no `qualifies`; where
  nothing minted the claim, the site keeps `get(xs, i)!` — which is precisely
  the assertion ladder working as designed (the `!` now *means* "unprovable
  here"). Cost: no `assert!(i is Idx…)` spelling, so no way to hoist one
  check above a loop that then indexes many times.
- **Option B — dependent `qualifies`**: allow a predicate whose signature
  takes the subject *plus* the claim's linked value —
  `fn qualifies(i: Int, list: List<T>) -> Bool { return i >= 0 && i <
  size(list) }` — with `i is Idx<xs>` / `assert!(i is Idx<xs>)` lowering to a
  call with both arguments [is-qualifies] and narrowing per [assert-narrow].
  This is the Ada lesson from §2: one vocabulary, a static tier and a runtime
  tier. Cost: `is` grammar grows the value argument; the extra parameter must
  be resolvable at the test site (it is — the link names it); D4's union
  limits apply unchanged.
- **Option C — shape-changing: comparison-shaped establishment.** The narrow
  seam named in RT-1: `assert!(i >= 0 && i < xs.size())` — or the same test
  as an `if` — establishes `Idx` when it *textually matches* the qualifier's
  declared predicate. This makes ordinary guards prove the claim with no new
  `is` syntax, and is the smallest true "refinement" flavor on offer. Cost:
  textual matching is fragile (`i < xs.size()` vs `xs.size() > i`), and its
  failure mode — a guard that looks right but does not establish — is
  invisible; if wanted, it should be a later convenience over Option B's
  semantics, never the primary path.

**Recommendation (the user's call):** B — it completes the ladder (prove /
require / handle / assert all speak `Idx`), and Option A's posture remains
available per-qualifier by simply not writing the predicate.

## RT-7. Scope — std-only claims, or user-declarable dependent qualifiers — ✅ confirmed 2026-09-23 (B)

Folds §0(5). Everything above can ship with `Idx` (and perhaps `KeyOf` for
maps) as std's, with the dependency axis kept internal — or the axis can be
declarable surface, so users write their own relations (`SmallerThan<cap>`,
`ValidHandle<pool>`) and state preconditions in signatures, the general form
the user's point 5 gestures at.

- **Option A — std-first, mechanism internal.** Ship `Idx`/`KeyOf`, keep the
  value-slot / link machinery compiler-internal (the position `proj` is in
  today: real, checked, not user-writable). Cost: users with the same shaped
  problem wait; benefit: the surface grammar (RT-2 Option A's spelling) is
  decided once, later, with usage evidence — the path `proj` itself is on.
- **Option B — declarable from the start.** `qualifier Q<…value slot…> of T`
  as public grammar, dependent `qualifies` (RT-6 B) as its test, refinements
  (RT-3 A) as its keep-list. Cost: the spelling becomes permanent immediately,
  and every RT above must be answered in its general form before anything
  ships.
- **Shape-changing alternative — preconditions without dependency**: note
  that *non-dependent* preconditions in signatures already exist and cover
  much of point 5 — a parameter typed `NonEmpty List<T>` or `Positive Int`
  is a precondition today, testable and assertable. A round that merely
  *documents and stdlib-fills* that (more predicate qualifiers: `Positive`,
  `NonNegative`, `InRange`… as LANGUAGE.md already sketches) delivers
  precondition ergonomics with zero new mechanism, independent of everything
  above.

**Recommendation (the user's call):** A, with the shape-changing note taken
regardless — it is nearly free and orthogonal.

*Post-decision note (third sitting): the approved `KeyOf` sketch is
user-written, so Option B is implied — listed under "confirmations owed".*

## RT-8. Slot arguments — what may fill one, and what identity it binds — ✅ decided 2026-09-23

**Decided (second round) as recommended**: places (identifiers and field
chains), bound to fate roots. The trail:

- **What may fill a local slot.** Options: **(a)** bare identifiers only —
  smallest, but `KeyOf(self.cache)` and `Idx(state.items)` are everyday
  shapes; **(b)** identifiers and field chains — exactly fate provenance's
  domain ("bare identifiers and field/index/`!` chains over one"
  [fate-link]), so the checker's existing notion of a *place* is reused
  verbatim; **(c)** arbitrary expressions — rejected on sight: an expression
  has no identity to link, so `KeyOf(make_map())` could never be consulted
  again.
- **What identity the claim binds.** Options: **(a)** the fate **root** —
  `let m2 = m` leaves `KeyOf(m)` usable with `m2` (they share fate; reads
  never invalidate); **(b)** the binding itself — simpler to print, but every
  rebinding orphans claims for no soundness gain.
- **Constants in the same block** (when their round comes): a literal fills a
  slot declared with a plain type the way a local does; the two are told
  apart syntactically (literal vs place). No `?` on either — `?` stays
  implicit-resolution's marker alone (decided).

**Recommendation (the user's call):** (b) places, bound to (a) fate roots —
both inherit machinery that exists and has its invariants tested.

## RT-9. The same claim twice — `KeyOf(m1) KeyOf(m2) K` — ✅ decided for now, 2026-09-23

**Decided (second round): Option A — [qual-no-dup] stands.** ("The recorded
lift trigger" means: the concrete use case written down here so that, when it
shows up in real code, it justifies reopening the restriction — the two-map
key, below.) The user also sketched the **candidate lift design**, recorded
for that day: an explicit `with` **self-compatibility with wildcard
arguments** —

```
qualifier KeyOf<K, V>(map: Map<K, V>) of K with KeyOf<K, *>(map: Map<K, *>)
```

— that is, a claim declares *itself* stackable across different argument
fillings, reusing the compatibility clause [qual-with] rather than a blanket
by-arguments dedup. This keeps the opt-in explicit (stacking stays refused by
default), scopes the change to declarations that want it, and gives the
flat-`quals` representation a narrower change to absorb (keyed entries only
for self-`with` qualifiers). The wildcard grammar (`*` in type and argument
position) is new surface to design when the trigger fires. The trail:

[qual-no-dup] forbids one qualifier twice on a type, and the flat `quals`
list *dedupes*, which is what makes `ok(ok(x))` inexpressible [qual-group].
A key present in two maps is a real shape (moving entries between them), and
under value arguments "the same qualifier" is no longer one claim — `KeyOf`
of `m1` and of `m2` are independent facts. Kept options: (A) no-dup for v1,
re-test per map at the two-map site; (B) dedupe by (name, arguments) —
superseded as the lift design by the self-`with` sketch above.

## RT-10. Dependent claims in return position — ✅ decided 2026-09-23

**Decided (second round): Option B, plus the `-> T as Q` → `-> +Q T` respell**
(the decided list, item 10): one trust rule in every position — `+Q`
establishes (trusted, qualifier's own file), plain `Q` reports (checked
against the body) — matching what deductions already spell [deduce-reapply].
The trail:

The producers the catalog leans on need to *answer* claimed values:
`binary_search(list: Sorted(…) List<T>, elem: T) -> (Idx(list) Int)?`,
`span(data, start, end) -> (ValidFor(data) Span)?`, a constructor
`-> K as KeyOf(map)`.

- **Validation**: the named value must be a **kept parameter** — the rule
  [readonly-return] already enforces for `proj(list)`; the caller
  substitutes its argument's identity at the call site, so `binary_search(xs,
  e)` answers `Idx(xs) Int?`. A moved parameter in a slot is the same error
  proj gives.
- **Option A — constructor `as` only** (`-> K as KeyOf(map)`): smallest, but
  `binary_search`'s claim sits *inside* an optional, which `as` cannot reach
  ([qual-ctor-simple]: constructor returns are simple types).
- **Option B — full return-type spelling** (a dependent qualifier legal
  anywhere in the return type), `as`-form included: what the catalog's
  producers actually need.

**Recommendation (the user's call):** B; A alone strands the two best
producers.

## RT-11. The `preserve` entry — how preservation is written — ✅ decided 2026-09-23

**Decided (second round): both sides — refns and own clauses — spelled
`preserve`** (not `keeps`: "kept" already means *not consumed* in deduction
vocabulary, and the entry's parameter is not the party holding the claim).
So: `refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf`,
and the same entry legal in a fn's own clause, checked against the body like
[deduce-gained]. This also settles RT-3's instrument. The trail:

- **Option A — `keeps Q` in refn entries**:
  `refn put(map: Mut Map<K, V>, key: K, value: V) => map: keeps KeyOf` —
  "claims of `KeyOf` held by other values, whose slot is this parameter,
  survive this call". Owner-scoped like every refn [qual-refn-scope];
  conflicts judged per [qual-refn-conflict].
- **Option B — also legal in a fn's own deduction clause**
  (`=> map: keeps KeyOf` on a user fn that only `put`s into the map):
  checked against the body like [deduce-gained] — every `Mut` use of the
  parameter must itself keep the claim (be refined `keeps`, or not touch
  it). This is what lets preservation survive one frame outward, the exact
  job [qual-refn-infer] does for `+Q`.
- **Shape-changing alternative — no `keeps`, invalidation classes on the
  qualifier** (RT-3 Option B's spelling): rejected there, listed here for
  the trail.

**Recommendation (the user's call):** A and B together — they are one rule
observed from two sides, and B is what makes user code over maps not
re-assert after every helper call.

## RT-12. Claims on a pass's elements — the worked examples (open)

RT-4's mechanics. The user asked to work the examples before deciding
(second round); this section was rewritten to carry them. First, the
constraints [iter-protocol] fixes:

- `next`'s shape must be **exactly** `Emitted T | Finished`, and `for` reads
  the element type from the `: Yield<self, T>` clause — so the claim must
  ride **inside `T`**, not restructure the union.
- A walking pass **already emits a qualified element**:
  `: Yield<self, proj T>`, its `next` returning `Emitted (proj T)`
  [yield-proj] — a qualifier inside the element type is precedented (for a
  compiler qualifier).
- The pass **borrows its source** ([proj-field]): `iter(xs)` links the pass
  to `xs`, and **mutating `xs` while a pass over it lives is already
  refused** [proj-infer]. Worth stating loudly: for pass-minted claims,
  invalidation needs no new machinery — the source cannot change under a
  live pass anyway.

### Example 1 — `indices` / `rev_indices` (closes §0(4))

Written in `core.list` — `Idx`'s own file, so `+` establishment is legal
(RT-10's trust rule):

```
export struct IndexYield<T> {
    list: List<T>       // borrowed by the pass [proj-field]
    i: Int
    step: Int
} : Yield<self, Idx(self.list) Int>

export fn next<T>(p: Mut IndexYield<T>) -> Emitted +Idx(p.list) Int | Finished
=> p: Mut { … }

export fn rev_indices<T>(list: List<T>) -> Mut IndexYield<T> => proj(list) {
    return IndexYield { list: list, i: size(list) - 1, step: -1 }
}
```

Then `for i in rev_indices(xs) { … get(xs, i) … }` is total — the founding
example, closed. What each line exercises: the Yield clause's element names a
**field of `self`** (RT-8's place grammar extended to declaration position;
at the mint the checker translates `p.list` to the caller's `xs` through the
proj link — the same substitution RT-10 performs for return types);
`+Idx(p.list)` in `next`'s return is RT-10's trusted form in arm position;
the `for` binding gets `Idx(xs) Int`, fresh each iteration [fate-link].

### Example 2 — `enumerate_rev` (pairs; the tuple rule obeyed)

A tuple cannot be qualified (RT-13), but its **components** can — "qualify
the parts instead" is the checker's own diagnostic text:

```
} : Yield<self, (Idx(self.list) Int, proj T)>
// next: -> Emitted (+Idx(p.list) Int, proj T) | Finished
```

`for (i, x) in enumerate_rev(xs)` destructures; `i` carries the claim, `x`
the borrow. A body that only wanted `x` got the claim for free; one that
indexes a *sibling* list with `i` meets RT-9 (today: re-test per list).

### Example 3 — `keys(map)`: the pass now, the container later

The pass form is Example 1 over a map (`: Yield<self, KeyOf(self.map) K>`,
established `+` in `KeyOf`'s file). But std's existing `keys(map)` answers a
**`List<K>` snapshot**, whose claimed form is `List<KeyOf(map) K>` — a
dependent claim in a *type argument*, which walks straight into the
invariance sharp edge the variance DECISION records (`List<NonEmpty
List<Int>>` is not a `List<List<Int>>`), sequenced after this work.
Recommendation inside the example: claim the pass, leave `keys`'s snapshot
unclaimed until the variance round.

### Example 4 — user-written: `even_indices(xs)`

RT-7 B lets a user write all of Example 1 for a qualifier *they* own. For
std's `Idx` they cannot write `+` (not the owner), so their pass must
**prove** the claim — which it can, by yielding only values obtained from an
already-claimed source (wrap `indices(xs)` and filter: a kept `Idx` is still
an `Idx`). A user pass that *computes* indices arithmetically cannot prove
them and stays unclaimed — the RT-1 boundary surfacing exactly where it
should.

### The residual sub-decisions

- **RT-12a — protocol tags are provenance — ✅ decided 2026-09-23 (fourth
  round).** The user agreed `Emitted` is a protocol tag — as are `Ok`,
  `Err`, `Thrown` — asked whether that is a *third* qualifier kind or
  just provenance, and **accepted the reclassification**: provenance, no
  third kind. On the follow-up naming question ("is *provenance* still the
  right word, or *protocol*?") the analysis recommends **keeping
  "provenance"**: the kind's semantics are "established by what the value
  passed through, not what is in it", true of both families
  (`Authenticated` came through the checkpoint, `Ok` through `ok()`), while
  "protocol" would mislabel the authority family; the connotation gap is
  closed in documentation (teach authority and protocol role side by side
  as two uses of one kind), with "origin" recorded as the plainer respell
  if "provenance" ever reads badly. The analysis: the kind test is
  content-dependence [qual-subject], and the tags pass every provenance
  criterion: mint-only with no body (`emitted`/`ok`/`err`/`thrown` are the
  mints); content-independent ("this came through `ok()`" is an origin
  statement, not a contents claim); should survive mutation (an
  `Ok Mut List<T>` through `add` must stay `Ok` — the state classification
  would strip it, which is wrong on the merits); composes freely without
  `with` — the RT-12a goal, plus a bonus fix: `Ok NonEmpty List<T>` stops
  needing `with` too. The connotation gap (provenance ≈ authority, tags ≈
  protocol role) is vocabulary, not semantics — `Thrown`'s documented
  forgeability already severs tag from authority. The discriminator that
  keeps the taxonomy honest: **mint-only ≠ provenance** — `Sorted`/`Heap`
  are mint-only *state* (contents claims; mutation must strip), the
  protocol tags are mint-only *provenance* (origin claims; it must not).
  **Now decided**: respell `provenance qualifier` onto `Emitted` (core.iterator),
  `Ok`/`Err` (core.result), `Thrown` (core.throw); no `Emitted` special
  case. Build caveats: sweep-check that nothing relies on exhaustive lists
  stripping these tags (relying on it would be a bug by this analysis), and
  union-arm machinery is unaffected (arm identity is by qualified type,
  kind-independent; `is Ok` on a union subject stays the arm test
  [is-narrowing]).
- **RT-12b — the declaration-side slot — ✅ decided 2026-09-23 (third
  round)**: `self.field` in a Yield clause and `p.field` in `next`'s return
  are legal slot fillers, with proj-link substitution at the mint. RT-8's
  grammar reused, as the examples assume.
- **RT-12c — the sugar — ✅ decided 2026-09-23 (third round)**: the
  `iter fn`/`state` form carries the same element spelling; engineering,
  with the early probe that the generated struct can host the borrowed
  field.

## RT-13. `Span` is a struct — the tuple restriction stands — ✅ decided 2026-09-23

**Decided (second round): (a), a struct.** The user asked whether the tuple
rule is real; re-verified: LANGUAGE.md states it outright ("Qualifiers CANNOT
apply to tuples, only to the types which make them up"), the checker enforces
it (`check.rs`: "a qualifier cannot apply to a tuple type; qualify the parts
instead"), and it is **load-bearing**, not incidental: tuple elements are
read-only *because* no tuple can carry `Mut` — LANGUAGE_SPEC.md's tuple rules
lean on [qual-union-arm] for exactly that ("so no tuple value can be `Mut`
and there is nothing through which to write"). Lifting it would reopen tuple
mutability, far beyond this customer. The trail:

Qualifiers cannot apply to tuples [qual-union-arm], so the catalog's
`(Span(data))?` cannot be a qualified *tuple*. Options: **(a)** `Span` is an
ordinary struct (`struct Span { start: Int, end: Int }`, in `core.bytes` or a
shared home) with `qualifier ValidFor(data: Bytes) of Span` — and the same
claim name declared over `Str` too, which [qual-overload] already supports
(same name, different subjects); `substr(str, at: ValidFor(str) Span) ->
Str`, `slice(data, at: ValidFor(data) Span) -> Bytes` total, minted by
`span(data, start, end) -> (+ValidFor(data) Span)?`. **(b)** Lift the tuple
restriction — a change motivated by nothing else in the language, priced far
beyond this customer.

Chosen: (a). It also gives `Span` a place for future members (`len(span)`),
which a tuple never grows.

## The proposed sequence — ✅ approved 2026-09-23 (second round)

Each step lands whole (build + tests + spec rules + sweep) before the next;
step 0 is independent and can go any time. Only RT-12's sub-decisions
(12a–12c) gate step 5.

0. **Plain iteration vocabulary** (RT-4 C) — ✅ **built 2026-09-23**
   (COMPLETED.md's log, "Refinement types, step 0"; ROADMAP.md carries the
   as-built note). The flagged probe answered: a tuple element with a `proj`
   component is refused — not by the qualifier rule but by
   [fate-derived-readonly] (a tuple literal cannot *store* a projection) —
   so the element is the view struct `Enumerated<T> { index: Int, elem:
   proj T }`, and **step 5's claimed respell lands on its `index` field**
   (`Idx` on a struct field, RT-2's grammar), not on a tuple component. Two
   build finds recorded in the log: the written opaque lend
   (`=> p: Mut, proj[from: p]`) is required because generic instantiation
   hides the borrow from inference, and [rs-proj-lends] grew the
   borrowing-struct elision case (inner-lifetime tie). The original plan
   text follows:
   - `reversed(list: List<T>)` — a **pass** walking backwards,
     `: Yield<self, proj T>` [yield-proj]: it borrows and copies nothing
     (unlike Kotlin's `reversed()`, which copies — ours is a pass, not a
     list; a caller wanting the list writes `to_list(reversed(xs))`).
     Struct + `next` in `core.list`, the `Range` shape.
   - `enumerate(list: List<T>)` — a pass of `(Int, proj T)` pairs, index
     ascending from 0.
   - `enumerate_rev(list: List<T>)` — the pairs in reverse order, index
     descending `size-1 .. 0` — §0(4)'s loop with the element already in
     hand.
   - Scope cuts: list-direct forms only; combinator forms over any pass
     (`enumerate(p, ?Yield<It, T>)`, the `core.seq` pattern) can follow
     later. Names are proposals (`reversed`/`enumerate`/`enumerate_rev`).
   - One flag for the builder: a **tuple element with a `proj` component**
     (`(Int, proj T)`) — [yield-proj] covers bare `proj T` elements; the
     tuple-wrapped form wants a probe test before the signatures are
     committed. If it does not hold, the fallback is a two-field struct
     element (RT-13's lesson), or landing `reversed` alone first.
   - In step 5 these signatures are *respelled* to carry claims
     (`Idx(self.list) Int`); step 0 deliberately does not wait for that.
1. **The respell round** (decided): argument blocks on qualifiers,
   all-or-none type generics, `proj(list)` — **and `-> T as Q` → `-> +Q T`**
   (decided with RT-10). **Plus the tag reclassification** (RT-12a):
   `provenance qualifier` onto `Emitted`/`Ok`/`Err`/`Thrown` — the one
   entry in this round that is *semantic*, not mechanical (the tags start
   surviving exhaustive stripping and composing without `with`), so it
   carries the sweep-check that nothing relied on the old stripping, its
   own tests, and the **provenance range documentation** (decision 18:
   LANGUAGE.md teaches authority and protocol role side by side as two
   uses of one kind). Everything else is respelling with no semantic
   change;
   the sweep touches std, specs, examples, corpus, inline test sources.
   Kept separate so the mechanical diff does not hide the semantic rounds
   behind it.
2. **Local slots, one function at a time**: declaration syntax, dependent
   `qualifies` + `is`/`assert!` (decided), place-filled slots and fate-root
   binding (RT-8, decided), conservative cross-value stripping (RT-3's C
   slice: any `Mut` use of the linked value strips). **Driver:
   `Span`/`substr`/`slice`** (RT-13, decided) — immutable subjects, so
   invalidation stays theoretical — plus `KeyOf` local-only.
3. **Signatures**: parameter position, return position with the one trust
   rule (RT-10, decided), the std total overloads and minting producers
   (RT-5, decided), `binary_search`'s new return, and the
   delegation-recursion regression test.
4. **Preservation**: `preserve` in refns and own clauses (RT-11, decided),
   the std audit (which mutators preserve `KeyOf`/`Idx`), inference
   propagation limits per [qual-refn-infer].
5. **Pass minting** (RT-12 — sub-decisions 12a–12c pending): `keys` (pass
   form), `indices`, `rev_indices`, claim-carrying `enumerate`/
   `enumerate_rev` respell — closes §0(4), the founding example.
6. **Constants** (decided: after locals): `InRange`, literal establishment,
   constant subtyping.

## Decisions pending — the table

After two decided rounds (both 2026-09-23), **the only open questions are
RT-12's sub-decisions**, gating step 5 of the approved sequence; steps 0–4
and 6 are fully unblocked.

| # | Question | Status |
|---|---|---|
| RT-1 | What counts as proof | ✅ Confirmed: A — declarations only, no checker arithmetic, SMT rejected |
| RT-2 | How a claim names its value | ✅ Decided: local-tied, argument-block spelling, links underneath |
| RT-3 | What mutation does to dependent claims | ✅ Decided: conservative cross-value strip; `preserve` opts back |
| RT-4 | How the loop gets the claim | ✅ In substance: step-0 vocabulary approved + RT-12's pass form; only 12a–12c remain |
| RT-5 | std total overloads / minting producers | ✅ Decided: starting list (`get`/`swap`/`substr`/`slice`; `binary_search`/`span`), extendable |
| RT-6 | The runtime tier | ✅ Decided: dependent `qualifies`, `is KeyOf(m)`, `assert!` narrowing |
| RT-7 | User-declarable or std-only | ✅ Confirmed: B — user-declarable |
| RT-8 | What fills a slot, what identity it binds | ✅ Decided: places (identifiers + field chains), fate roots |
| RT-9 | Same claim, different arguments | ✅ Decided: [qual-no-dup] stands; lift design recorded (self-`with` wildcard) |
| RT-10 | Return position + trust | ✅ Decided: full spelling; `+Q` trusted / plain checked; `as Q` → `+Q T` respell |
| RT-11 | The preservation entry | ✅ Decided: `preserve`, in refns and own clauses |
| RT-12a | Protocol tags: third kind or provenance? | ✅ Decided: provenance, no third kind; tags respelled `provenance qualifier`; **the keyword stays "provenance"**, its range documented (both families, side by side) |
| RT-12b | `self.field` / `p.field` as declaration-side slots | ✅ Decided: yes, with proj-link substitution at mint |
| RT-12c | The `iter fn` sugar carrying claimed elements | ✅ Decided: same spelling; early probe of the generated struct |
| RT-13 | The `Span` shape | ✅ Decided: a struct + `ValidFor(data)`; tuple restriction re-verified and stands |
| — | Argument block, all-or-none generics, `proj(list)`, `+Q T` | ✅ Decided (the decided list) |
| — | Constants after locals; the sequence | ✅ Decided / approved |
