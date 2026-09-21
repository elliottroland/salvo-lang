# Ordering, equality and hashing — the design round

Raised 2026-09-21 by `demo/heap.sv`'s `Heap<T canbe ordered>` TODO
(HEAP_QUALIFIER.md item 1) and grown, through one session's worth of user
decisions, into a redesign of how comparison, equality and hashing work in
the language. This document is the round: the adopted model, the decisions
already made, the questions still open, and the build plan. **Deleted when
the last step lands**, with the decisions in COMPLETED.md's log and the
rules in LANGUAGE_SPEC.md (the OPTIONALS.md pattern).

Status: **adopted — every call made 2026-09-21** (the canonical-placement
question settled in the evening, the last four calls the same night;
decisions 1–14 below). **Building: step 1 of the plan landed 2026-09-21**
(the three groups + the intrinsic canonicals; see the build plan and
COMPLETED.md's log). The spec now carries [cmp-groups] and
[cmp-hash-values]; everything the later steps change ([col-equality],
[col-hashed-ordered], [op-order] et al.) still describes the current
language, because the operators do not route through the groups yet.

## The model in one page

Comparison, equality and hashing become **params groups** — the iteration
precedent (`params Yield<It, T>` [implicit-group]) applied to three more
capabilities, with no traits entering the language:

```
params Ordered<T> { fn cmp(a: T, b: T) -> Int }
params Eq<T>      { fn eq(a: T, b: T) -> Bool }
params Hashed<T>  { fn hash(a: T) -> Long }
```

- **Canonical implementations are top-level fns `@`-scoped to the type**
  (user decision 2026-09-21, revising the same day's in-body-member form
  — now in the rejected list): declared in the type's own file, spelled
  with the type's name —

  ```
  struct Person {
      name: Str
      age: Int
  }

  fn eq@Person(a: Person, b: Person) -> Bool {
      return a.name == b.name
  }

  fn cmp@Person(a: Person, b: Person) -> Int {
      if eq(a, b) { return 0 }
      return cmp(a.age, b.age)
  }
  ```

  An `@`-scoped fn is an ordinary overload (bare calls and dot-notation
  `a.eq(b)` reach it [fn-dot]) with two extra properties. It **travels
  with the type**: importing the type imports every fn `@`-scoped to it
  in its file, so the canonical is in scope wherever the type is usable —
  closing the visibility hole in [implicit-resolve]'s per-call-site
  locality (a module importing `Person` but not its file's `cmp` could
  otherwise resolve `?cmp` to some *other* visible `cmp(Person, Person)`,
  or to nothing). And it is the **default selection for implicit
  parameters** of the same name and shape. Canonical status is thereby
  decoupled from the obligation clause: `fn cmp@Person` alone makes
  `SortedSet<Person>` work, no `: Ordered<self>` written — the clause
  remains what it is today, the check at the declaration (and the
  `default` generator, whose generated fns are `@`-scoped). The
  declaration spelling **is** the disambiguation spelling:
  `find_first(persons, cmp = cmp@Person)`, `Heap<cmp@Person>` — the same
  `@` the language already uses for scope selection
  (`size@core.list(xs)`), extended from modules to types on both sides.
  Struct bodies stay fields-only. For primitives, canonicals stay
  intrinsic std overloads — `cmp(Str, Str)` keeps the code-point contract
  that absorbs today's `__salvoCompare` parity rule [kt-ordered].
- **`default` on compiler-known obligation groups** generates the
  well-behaved structural implementation:

  ```
  struct Point : default Ordered<self>, default Hashed<self> { x: Int, y: Int }
  struct Custom : Eq<self> { … }    // obligation: an eq(Custom, Custom) must
                                    // exist — any visible overload satisfies
                                    // the check; @-scoping it (eq@Custom) is
                                    // what makes it canonical (decision 8)
  ```

  A user also declaring the generated member is the ordinary same-module
  duplicate (remedy: "remove `default`"); a bare obligation with no
  implementation gets the reverse hint. **The `default` forms bring `eq`
  with them** (everything generated is structural, so consistency is by
  construction); hand-written implementations are declared piece by piece.
- **Operators resolve through the groups.** `<` `<=` `>` `>=` rewrite to
  `cmp(a, b) op 0`; `==`/`!=` to `eq(a, b)` — at a concrete type via the
  unique visible overload [implicit-resolve], at a generic `T` via an
  enclosing `?cmp`/`?eq` [implicit-forward] (colouring, like effects; the
  missing-implicit error already names the remedy). **No candidate is an
  error** — this closes the today-silent comparisons on unconstrained `T`
  (HEAP_QUALIFIER.md item 7) and makes equality **opt-in**, overturning
  [col-equality]'s universality. Numerics keep the native fast path.
- **Construction-time binding with the identity in the type** (option G of
  the exploration): a structure that *holds* an ordering names it as a
  fn-valued type argument, fixed at construction:

  ```
  qualifier Heap<T, ?cmp: (T, T) -> Int> of List<T>
  fn empty_heap<T>(?cmp: (T, T) -> Int) -> Mut List<T> as Heap<?cmp>
  fn heap_push<T>(heap: Heap<?cmp> Mut List<T>, elem: T) => heap: Heap<?cmp> Mut

  intrinsic type SortedSet<T, ?cmp: (T, T) -> Int = cmp> canbe Mut
  Set<T, ?hash = hash, ?eq = eq>
  Map<K, V, ?hash = hash, ?eq = eq>
  ```

  `Heap<min_by_age>` and `Heap<max_by_age>` are different types that refuse
  to mix; a shared binder (`fn heap_merge<T>(a: Heap<?cmp> Mut List<T>, b:
  Heap<?cmp> List<T>)`) forces two parameters to carry the *same* ordering.
  Dropping such a qualifier is **fail-safe**: the operations demand it, and
  a plain value never regains it by subtyping [qual-constructive] — you
  lose access, never correctness. [implicit-resolve]'s per-call-site
  locality stops being a hazard, because the resolved identity lands in
  the type.
- **The load-bearing restriction**: a fn bound into a type must have
  **static identity** — named, top-level, capture-free (what
  [implicit-resolve]'s default path produces). A capturing lambda would
  smuggle runtime state into a type — refused, with the error naming why.
- **`canbe ordered` and `canbe hashed` are deleted.** What survives in
  `canbe` on structs: `Mut` and the linearity forms.

## Lowering

- **Rust.** Each bound fn gets a generated zero-sized marker struct and a
  trait impl (`pub struct __Cmp_by_age; impl SalvoCmp<Person> for
  __Cmp_by_age { … }`); a fn generic over an ordering takes a hidden
  marker generic (`fn heap_push<T, C: SalvoCmp<T>>`), monomorphized to a
  direct call. Values stay their own type — a heap is still `Vec<T>`;
  sendability is untouched because there is no comparator *value*, only a
  type. At the one boundary where Rust demands ordering *in the element
  type* (`BTreeSet`/`BTreeMap` take no comparator), the emitter wraps with
  `#[repr(transparent)] struct OrdBy<C, T>(T, PhantomData<C>)` whose
  `impl Ord` delegates to `C::cmp` — wrap on insert, `.0` on read,
  physically free. `default Ordered`/`default Eq`/`default Hashed` keep
  today's derives (`PartialOrd, Ord` / `PartialEq` / `Hash`) — consistent
  by construction, and the canonical case never pays the wrapper.
  **Invariant: the only real `Ord` impls are derived ones** — a
  hand-written `cmp` never becomes the type's `Ord` (an impl inconsistent
  with `Eq` corrupts BTree invariants); user comparisons always travel the
  marker path.
- **Kotlin.** `TreeSet`/`TreeMap` already take a comparator (the backend
  already builds them over `__salvoCompare` [kt-ordered]); the bound fn
  replaces it at the construction site. `Set`/`Map` need a Salvo-runtime
  hash container (JVM `LinkedHashSet` keys off `hashCode`/`equals` with no
  pluggable slot) — the Rust side already has one (`SalvoSet`/`SalvoMap`,
  [rs-collections]). Markers lower to stateless singletons or direct
  calls; not zero-cost like Rust's ZSTs, behaviorally identical.
- **Bound spelling on Kotlin**: an orderable-`T` constraint must mean
  "accepted by the language's comparison", **never** `T : Comparable<T>` —
  `String.compareTo` is UTF-16 code-unit order and Salvo's `Str` order is
  code point; natural ordering would diverge from Rust [backend-parity].

## Decided (all user decisions, 2026-09-21)

1. **The group model + construction-time binding (G) is the direction**;
   the ordering-qualifier-on-elements idea (option F) is rejected — a
   droppable qualifier silently reverting to canonical order is fatal —
   and comparator-in-the-value (option D) is rejected for sendability
   (`Rc<dyn Fn>` fields [rs-fn-field]).
2. **`default` on compiler-known obligation groups** is the one-token
   spelling; `canbe ordered`/`canbe hashed` are deleted. Whitelist:
   `default` only where the compiler has a generator (`Ordered`, `Eq`,
   `Hashed`); `default Yield<…>` errors naming the groups that have one.
3. **`self` in obligation arguments is the existing syntax**
   ([group-self], `struct Lines : Yield<self, Str>`) — nothing new needed.
4. **Lowering split by author**: `default` inherits today's validation
   (no float fields, no fn fields) and derive-based lowering; hand-written
   implementations travel the marker path. Generated members carry the
   struct's `export` (for hand-written `@`-scoped fns, export is explicit
   and must match — decision 10).
5. **Hashing gets the same treatment** (`Set<T, ?hash>`), accepting the
   two costs `Ordered` did not have: a callable `hash` makes the value
   observable — first read as forcing one language-defined algorithm in
   both runtimes, **revised by decision 13**: backend-native hashing,
   divergence accepted; and the consistency contract (`eq(a,b)` ⇒ equal
   hashes) is trusted, with a **warning** on the shape-detectable mistake
   of binding a custom `?eq` with the default `?hash`.
6. **Equality is opt-in and joins the triad** (`?eq`, `params Eq`,
   `: default Eq`): `==` on structs with no resolvable `eq` is an
   **error** — overturning [col-equality]'s "every struct, structurally"
   (a 2026-09-12 decision). At generic `T`s this is [implicit-forward],
   no new rule. New capability: a struct with a fn-typed field can
   declare a custom `eq` ignoring it and become comparable/hashable/a key.
7. **The `default` forms bring `eq` with them**; hand-written ones do not
   (no consistent `eq` is derivable from a custom `hash`; defaults come
   as a coherent bundle, custom implementations piece by piece).
8. **Canonical implementations are `@`-scoped top-level fns**
   (`fn cmp@Person(a: Person, b: Person) -> Int`, declared in the type's
   file): importing the type imports its `@`-scoped fns, and they are the
   default selection for implicit parameters of the same name and shape —
   so the canonical is in scope wherever the type is, by construction.
   Decoupled from the obligation clause, which stays the
   check-at-declaration and the `default` generator (generated fns are
   `@`-scoped); [group-obligation]'s "the obligation adds no scope" stays
   true. Reference and disambiguation reuse the existing scope-selector
   spelling (`cmp = cmp@Person`), extended from modules to types. This
   **knowingly overturns** [implicit-resolve]'s recorded note that "Salvo
   needs no qualified-name syntax for defaults … nothing ties it to the
   type's declaration": the tie and the syntax now exist, because
   canonical visibility must not depend on which of the type's file's
   names a module happened to import. Signatures write the type's own
   name — forced at top level anyway, and [group-self]'s rejection of a
   magic `Self` stands. Supersedes the same day's in-body-member form
   (rejected list). Noted possible follow-on, **separately decided**:
   migrating qualifier bodies' `fn qualifies` to the same shape
   (`fn qualifies@Positive`); handler members stay put — they interact
   with handler state, a different question.
9. **Ambiguity around a canonical is rejected consistently — explicit
   calls and implicit resolution alike** (user decision 2026-09-21): when
   an `@`-scoped canonical is among the fitting candidates, no scope rank
   silently wins — a second fitting candidate on *any* rung is an error
   naming both selector spellings (`cmp@Person`, `cmp@my.module`). This
   carves the canonical case out of [fn-overload-scope]'s
   Own-beats-Import silence, resolving the explicit/implicit divergence
   the in-body draft had. The intent is wider — **no scope-based silent
   winners anywhere** — but that is deferred to its own ROADMAP item
   ("Overload resolution — no silent scope winners"), to be designed as a
   consistency pass across features; decision 9 is its first installment,
   and general overload resolution stays untouched by this round.
10. **`export` on an `@`-scoped fn is explicit and must match the
   type's** (user decision 2026-09-21): no inheritance — an exported type
   with an unexported canonical, or the reverse, is a compile-time error
   naming the mismatch. Explicitness keeps [mod-export]'s "the public
   surface is exactly what the module writes down" literally true where
   auto-import might have blurred it.
11. **The binder binds bare in the signature** (user decision 2026-09-21,
   evening): all `?name` occurrences in one signature denote one binding
   — bound by an explicit implicit parameter when one is declared
   (`empty_heap`: resolution fills it, the return type publishes it),
   otherwise by **capture** from the argument types (`heap_push`). Its fn
   type is never written at the fn — the slot it fills (the qualifier's
   `?cmp` declaration) states it. Two occurrences captured from two
   arguments must carry the same identity: `heap_merge`'s unification,
   a mismatch a plain type error ("two heaps ordered differently"); two
   independent orderings are two names. Sub-rules adopted with it: a bare
   `Heap` (slot unnamed) is legal where the body never needs the
   identity, and a deduction keeping the qualifier keeps the identity
   automatically (the fn could not change it — it lives in the type); the
   binder is callable in the body and forwards like any implicit
   [implicit-forward] — the existing colouring, not a new one.
   Justification recorded with the call: the bare binder is *an indirect
   way of declaring a fn in the parameter scope*, so it does not belong
   in the generics list — with the reservation that the generics-list
   spelling could be revisited long-term if the bare form disappoints.
12. **Fn-valued type arguments are named top-level fns only** (user
   decision 2026-09-21, evening): module fns, `@`-scoped canonicals, and
   intrinsics (via [implicit-intrinsic]'s adapters) — everything a type
   can *print* (`Heap<min_by_age>` and `Heap<cmp@Person>` display and
   compare by name). Capture-free lambdas and fn-valued locals are
   refused: "a fn bound into a type must be a named top-level fn — a
   lambda or local has no identity a type can carry; declare it as a
   `fn`." Widening later is purely additive.
13. **Hash values diverge per backend — accepted** (user decision
   2026-09-21, evening, revising decision 5's forced-algorithm reading):
   `hash` lowers to each backend's native hashing, on the analogy of the
   two backends generating different random numbers — the shape and the
   high-level guarantees are identical, the exact values are not. The
   contract, to be specified: `eq(a,b)` ⇒ `hash(a) == hash(b)` **within
   a single execution**; nothing more is promised (per-run divergence on
   one backend, e.g. a seeded hasher, is within contract). Consequences:
   no language-defined algorithm to choose or implement; a program must
   not print or persist a hash value and expect cross-backend identity —
   an example's `expected.txt` cannot contain one, the same posture as
   random. Revisiting value-level parity for hash *and* random together
   is a recorded ROADMAP item.
14. **The package is formally adopted** (user decision 2026-09-21,
   evening): decisions 1–13 plus the build plan below are *the* plan —
   equality opt-in and its repo sweep, the deletion of
   `canbe ordered`/`canbe hashed`, operators through the groups,
   `@`-scoped canonicals, and construction-time fn-valued type
   arguments. Building starts after the concurrent test-time session
   finishes; steps 1–4 land value before step 5's type-system work.

## Open questions — none

Every call is made (decisions 1–14 above; the last four — formal
adoption, the binder, the static-fn boundary, and the hash question —
landed the evening of 2026-09-21, the hash one by *dissolving*: decision
13 accepts backend divergence, so there is no algorithm to choose). What
remains is the build.

Mechanical items that travel with the build rather than needing calls:
the `@Type` grammar on both sides (declaration `fn cmp@Person(…)` and
reference `cmp@Person`). The selector's shapes are already distinguished
by [name-casing] as a *rule*: lowercase after `@` starts a module path
[fn-overload-at], capitalized is a type — except that the capitalized
reading is today an **effect selector** [effect-at] (`close@Fs(h)`).
Benign, since effects and types share the capitalized namespace under
[name-casing] and one name cannot be both in scope: resolve by what the
name declares, and [effect-at]/[fn-overload-at] get a third sibling
rather than a new grammar. One divergence to carry deliberately:
[effect-at] is a call form only, while [fn-overload-at] is "also valid as
a value" (`describe@main`) — `cmp@Person` follows the *module* precedent,
which with [implicit-override] (`cmp = cmp@Person`) is value-position
support for free. Remaining: auto-import wording under [mod-collision]
(the fns arrive as overloads riding the type, not as importable names of
their own); duplicate detection (two same-signature fns `@` one type are
ordinary duplicates); and emitter name-mangling for the lowered fn
(`cmp@Person` and `cmp@Point` in one module).

Two spec-wording questions travel with the build rather than needing a
call now: membership semantics for keyed containers (a `SortedSet<T, f>`
deduplicates by `f`; `Set<T, ?hash, ?eq>` members are `eq`-distinct — to
be specified as definitions, resolving the consistency-invariant hazard by
specification), and whether `Sorted List<T>` [col-sorted-list] is
parameterized (`Sorted<?cmp>`) in the same change or stays canonical-only
at first (once orderings are plural, `add_sorted` under a different `cmp`
than the sort's silently breaks the claim — the model wants uniform
application).

## Build plan (sketch — sequence within the change is the builder's)

1. ~~**Foundations**~~ — **landed 2026-09-21.** `std/core/compare.sv` holds
   the three `params` groups and the canonical `cmp`/`eq`/`hash` overloads
   for the intrinsic types (`hash` lowering to each backend's native
   hashing, decision 13). It needed no checker change — the groups ride
   [implicit-resolve]/[implicit-forward]/[group-obligation] as designed —
   but it did force one Rust-backend fix, since an implicit's *kept*
   position rendered by value and so **moved** what the contract keeps:
   kept non-`Mut` non-Copy positions now render `&T`
   ([rs-fn-param-convention]), which is what makes `?Ordered<T>` usable
   over anything but a Copy scalar. New rules [cmp-groups] and
   [cmp-hash-values]; details in COMPLETED.md's log entry.
2. ~~**Operators through the groups**~~ **+ 4. ~~the equality sweep~~** —
   **landed 2026-09-21 as one change** (they cannot land apart: the operator
   switch is what breaks every program that compared a struct). `<`-family →
   `cmp`, `==`/`!=` → `eq`, recorded per comparison for the emitters; numerics
   and intrinsic-type equality keep the native operator; `Ty::Var` left
   `op_lenient` (HEAP_QUALIFIER.md item 7 closed); `canbe ordered`/`canbe
   hashed` deleted, with key eligibility now asking whether the *function*
   exists; `eq@Bytes` added to std; the sweep of std, examples, tests and the
   spec ([col-equality], [col-hashed-ordered], [op-order] rewritten,
   [op-equality] added). Details in COMPLETED.md's log.
3. **`@`-scoped canonicals** — **landed 2026-09-21** (step 3a, split from
   `default` at the user's choice of landing order): `fn cmp@Person(…)`, the
   declaration-side parser, the same-file and export-match checks, auto-import
   with the type, default selection for implicits, decision 9's consistent
   ambiguity errors in both ranking paths, and the selector in call *and* value
   position. New rule [cmp-canonical].
   **`default` obligations landed the same night** (step 3b): the contextual
   `default` in the `:` clause, the `Ordered`/`Eq`/`Hashed` whitelist and the
   `self` rule, generation as a *desugaring* (so the generated `@`-scoped
   members are ordinary items and the duplicate check catches a hand-written
   collision), today's `canbe` validation inherited, and derive-based lowering on
   both backends. New rule [cmp-default]. What remains of the original step 3 is
   the **deletion of `canbe ordered`/`canbe hashed`**, which moved to the 2+4
   landing where its sweep happens. Original wording:
   parser (`default`
   in the `:` clause; `fn name@Type` declarations and the type side of
   the `@` selector), auto-import with the type, resolution (decision 8's
   default selection for implicits, decision 9's consistent ambiguity
   errors), generation (generated fns are `@`-scoped), duplicate/missing
   diagnostics, the explicit-export match check (decision 10); delete
   `canbe ordered`/`canbe hashed`
   and sweep ([col-hashed-ordered] sites: `std/time.sv`,
   `examples/collections`, corpus, spec snippets).
4. ~~**The equality sweep**~~ — landed with step 2 above. (Container membership
   through `?eq` defaults belongs to step 5, where the containers gain their fn
   parameters.)
5. **Fn-valued type arguments (G)**: identities in types (equality,
   unification, inference, display), the `?cmp` binder, marker/ZST
   emission on Rust, the `OrdBy` boundary newtype, Kotlin runtime hash
   containers; `SortedSet`/`SortedMap`/`Set`/`Map` gain their defaulted
   fn parameters; `demo/heap.sv` becomes the worked example.
6. **Spec**: rewrite [col-equality], [col-hashed-ordered], [op-order],
   [col-sorted]/[col-sorted-list] as needed; new rules for the groups,
   `default`, fn-valued arguments and the binder; rules for `@`-scoped
   canonicals — declaration, auto-import, default selection, consistent
   ambiguity (decisions 8–9) — deleting [implicit-resolve]'s "no
   qualified-name syntax for defaults / nothing ties it to the type's
   declaration" note and extending the scope-selector rule from modules
   to types; COMPLETED.md log entries per landed step.

Steps 1–4 need none of step 5's type-system work and already fix the
demo's item-7 hole; step 5 is the heap's unblock and the largest single
piece.

## Rejected along the way (recorded so they are not re-explored)

- **Canonical members in the struct body** (`struct Person : Eq<self> {
  … fn eq(a: Person, b: Person) … }`, the `qualifies`/handler-member
  precedent) — the first 2026-09-21 form of decision 8, superseded the
  same session by `@`-scoped top-level fns: it tied canonical status to
  writing the obligation clause (a canonical should not require one), put
  fns inside struct bodies (eroding fields-only structs and inviting
  methods), and needed a new `Type.member` reference form where
  `@`-scoping reuses the selector the language already has — the
  declaration spelling and the disambiguation spelling became the same
  token. Its explicit/implicit resolution divergence is what decision 9
  closed.

- **Comparator in the value** (struct with a fn field): not sendable
  ([rs-fn-field] is `Rc<dyn Fn>`); fn fields bar equality.
- **Ordering-overriding qualifier on elements** (`Desc Point`): erased
  qualifiers make it silently wrong at generic boundaries; a non-erased
  version is a second lowering model for values (the D5a objection); and
  even the erased-with-markers version dies on droppability — a forgotten
  qualifier silently reverts the order. Its zone analysis and marker
  lowering survive inside G.
- **A separate `order` declaration species**: same lowering as G with a
  whole new declaration form; G reuses `params` + implicits instead.
- **Bound-only (`<T canbe ordered>`) as the whole answer** (option A):
  its machinery (bound registry, instantiation checks) is subsumed — the
  group + operator resolution covers every use, and "orderable" is
  "an ordering is in scope".
- **Per-call `?cmp` for state-carrying structures**: build-under-one-
  order-pop-under-another. Still right for stateless algorithms
  (`sort(list, cmp = …)`); G is exactly it with resolution moved to the
  one construction site.

## Backend ground truth (grounding for all of the above; read 2026-09-21)

- Rust: `canbe ordered` derives `PartialOrd, Ord` (rust/emit.rs:1836);
  `SortedSet`/`SortedMap` are `BTreeSet`/`BTreeMap` (no comparator slot;
  one `Ord` per type); `Set`/`Map` are Salvo's own `SalvoSet`/`SalvoMap`
  ([rs-collections], insertion-ordered). `std::cmp::Reverse` is the
  wrapper precedent; `repr(transparent)` newtypes are free.
- Kotlin: `TreeSet`/`TreeMap` built over `__salvoCompare`, never natural
  ordering (kotlin/intrinsics.rs:128, 283, [kt-ordered]) because JVM
  `String.compareTo` is UTF-16 code-unit order [backend-parity];
  `Set`/`Map` are `LinkedHashSet`/`LinkedHashMap` keyed off JVM
  `hashCode`/`equals` (no pluggable hasher). No ZSTs on the JVM.
- Comparisons on unconstrained `T` are currently silent: `op_lenient`
  includes `Ty::Var(_)` (check.rs:21137, "a documented leftover").
- Today's hash values differ per backend and are unobservable only
  because iteration is insertion-ordered and nothing exposes a hash.
