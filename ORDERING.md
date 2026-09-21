# Ordering, equality and hashing — the round's remainder

Raised 2026-09-21 by `demo/heap.sv`'s `Heap<T canbe ordered>` TODO
(HEAP_QUALIFIER.md item 1) and grown, through one session's worth of user
decisions, into a redesign of how comparison, equality and hashing work in the
language. **Decisions 1–14 were all made 2026-09-21 and four of the five build
steps landed the same night** — their record is in COMPLETED.md's decision log
(four entries: step 1, step 3a, step 3b, steps 2+4), and the rules they added
are in LANGUAGE_SPEC.md ([cmp-groups], [cmp-hash-values], [cmp-canonical],
[cmp-default], [op-order], [op-equality], with [col-equality] and
[col-hashed-ordered] rewritten).

**What is left is step 5**, and this document is now *only* that: the design of
fn-valued type arguments, kept whole so the work can be picked up without
re-deriving it. Delete this file when step 5 lands.

## What already works (so the remainder reads in context)

```
// The three capabilities, as params groups in core.compare.
params Ordered<T> { fn cmp(a: T, b: T) -> Int }
params Eq<T>      { fn eq(a: T, b: T) -> Bool }
params Hashed<T>  { fn hash(value: T) -> Long }

// A canonical implementation, @-scoped to its type and travelling with it.
fn cmp@Person(a: Person, b: Person) -> Int { return cmp(a.age, b.age) }

// Or generated, structurally, from the fields.
struct Point : default Ordered<self>, default Hashed<self> { x: Int, y: Int }

// The operators are those functions: `a < b` is `cmp(a, b) < 0`, `a == b` is
// `eq(a, b)`. Generic code asks for the capability and forwards it.
fn larger_of<T>(a: T, b: T, ?Ordered<T>) -> T { if a > b { return a } return b }
```

`canbe ordered` and `canbe hashed` are gone; being orderable is *having a
`cmp`*. What a structure cannot yet do is **hold** an ordering.

## Step 5 — construction-time binding with the identity in the type

Option G of the original exploration, adopted as decision 1: a structure that
*holds* an ordering names it as a fn-valued type argument, fixed at
construction.

```
qualifier Heap<T, ?cmp: (T, T) -> Int> of List<T>
fn empty_heap<T>(?cmp: (T, T) -> Int) -> Mut List<T> as Heap<?cmp>
fn heap_push<T>(heap: Heap<?cmp> Mut List<T>, elem: T) => heap: Heap<?cmp> Mut

intrinsic type SortedSet<T, ?cmp: (T, T) -> Int = cmp> canbe Mut
Set<T, ?hash = hash, ?eq = eq>
Map<K, V, ?hash = hash, ?eq = eq>
```

`Heap<min_by_age>` and `Heap<max_by_age>` are different types that refuse to
mix; a shared binder (`fn heap_merge<T>(a: Heap<?cmp> Mut List<T>, b: Heap<?cmp>
List<T>)`) forces two parameters to carry the *same* ordering. Dropping such a
qualifier is **fail-safe**: the operations demand it, and a plain value never
regains it by subtyping [qual-constructive] — you lose access, never
correctness. [implicit-resolve]'s per-call-site locality stops being a hazard,
because the resolved identity lands in the type.

**The load-bearing restriction** (decision 12): a fn bound into a type must have
**static identity** — named, top-level, capture-free. Module fns, `@`-scoped
canonicals and intrinsics (via [implicit-intrinsic]'s adapters) qualify;
capture-free lambdas and fn-valued locals are refused, with the error naming why
("a lambda or local has no identity a type can carry; declare it as a `fn`").
Everything a type can *print* it can carry, and widening later is purely
additive.

**The binder binds bare in the signature** (decision 11): all `?name`
occurrences in one signature denote one binding — bound by an explicit implicit
parameter when one is declared (`empty_heap`: resolution fills it, the return
type publishes it), otherwise by **capture** from the argument types
(`heap_push`). Its fn type is never written at the fn; the slot it fills (the
qualifier's `?cmp` declaration) states it. Two occurrences captured from two
arguments must carry the same identity: `heap_merge`'s unification, a mismatch a
plain type error ("two heaps ordered differently"); two independent orderings are
two names. Sub-rules adopted with it: a bare `Heap` (slot unnamed) is legal where
the body never needs the identity, and a deduction keeping the qualifier keeps
the identity automatically (the fn could not change it — it lives in the type);
the binder is callable in the body and forwards like any implicit
[implicit-forward]. Justification recorded with the call: the bare binder is *an
indirect way of declaring a fn in the parameter scope*, so it does not belong in
the generics list — with the reservation that the generics-list spelling could be
revisited long-term if the bare form disappoints.

### Lowering

- **Rust.** Each bound fn gets a generated zero-sized marker struct and a trait
  impl (`pub struct __Cmp_by_age; impl SalvoCmp<Person> for __Cmp_by_age { … }`);
  a fn generic over an ordering takes a hidden marker generic
  (`fn heap_push<T, C: SalvoCmp<T>>`), monomorphized to a direct call. Values
  stay their own type — a heap is still `Vec<T>`; sendability is untouched
  because there is no comparator *value*, only a type. At the one boundary where
  Rust demands ordering *in the element type* (`BTreeSet`/`BTreeMap` take no
  comparator), the emitter wraps with
  `#[repr(transparent)] struct OrdBy<C, T>(T, PhantomData<C>)` whose `impl Ord`
  delegates to `C::cmp` — wrap on insert, `.0` on read, physically free.
  **Invariant: the only real `Ord` impls are derived ones** — a hand-written
  `cmp` never becomes the type's `Ord` (an impl inconsistent with `Eq` corrupts
  BTree invariants); user comparisons always travel the marker path.
  - *A cheaper first cut, noted while building steps 1–4*: because qualifiers
    erase, the **qualifier** case needs no markers at all — a captured binder can
    be lowered as an ordinary **implicit parameter** filled from the argument's
    type at the call site, which the existing machinery already emits (an
    adapter closure). That is behaviourally identical and costs an indirect call
    instead of a monomorphized one; the markers are the optimization, and the
    *container* case (below) is what genuinely needs them.
- **Kotlin.** `TreeSet`/`TreeMap` already take a comparator (the backend builds
  them over `__salvoCompare` [kt-ordered]); the bound fn replaces it at the
  construction site. `Set`/`Map` need a Salvo-runtime hash container (JVM
  `LinkedHashSet` keys off `hashCode`/`equals` with no pluggable slot) — the Rust
  side already has one (`SalvoSet`/`SalvoMap`, [rs-collections]). Markers lower
  to stateless singletons or direct calls; not zero-cost like Rust's ZSTs,
  behaviorally identical.
- **Bound spelling on Kotlin**: an orderable-`T` constraint must mean "accepted
  by the language's comparison", **never** `T : Comparable<T>` —
  `String.compareTo` is UTF-16 code-unit order and Salvo's `Str` order is code
  point; natural ordering would diverge from Rust [backend-parity].

### The build, in the order that keeps the tree green

1. **Identities in types**: a `Ty` variant for a named fn (a one-variant
   addition — measured while preparing this step: exactly **four** `match` sites
   in the workspace are exhaustive over `Ty`, so the sweep is trivial), with
   equality, display (`Heap<cmp@Person>` prints by name), substitution and
   unification; the static-identity check and its diagnostic.
2. **The written forms**: `?name` and `name@Type` in a type-argument position
   (the parser accepts neither today), and `?name: (T, T) -> Int` slots in a
   qualifier's generics list.
3. **The binder**: one binding per signature; explicit-implicit or captured;
   unification across occurrences; the identity substituted into a call's result
   type (`empty_heap` publishing what resolution chose), and into the deduction
   clause's kept qualifier.
4. **`demo/heap.sv`** as the worked example, on both backends — the file this
   round came from. It also needs HEAP_QUALIFIER.md's items 2 (`+Heap`, D2) and 4
   (`swap`) to compile in full; the ordering half is this step.
5. **The keyed containers**: `SortedSet`/`SortedMap`/`Set`/`Map` gain their
   defaulted fn parameters, with the Rust marker/`OrdBy` machinery and the Kotlin
   runtime hash container. Also the two spec-wording questions this raises —
   membership semantics for keyed containers (a `SortedSet<T, f>` deduplicates by
   `f`; `Set<T, ?hash, ?eq>` members are `eq`-distinct, to be specified as
   definitions) and whether `Sorted List<T>` [col-sorted-list] is parameterized
   (`Sorted<?cmp>`) in the same change or stays canonical-only at first (once
   orderings are plural, `add_sorted` under a different `cmp` than the sort's
   silently breaks the claim).

### Rejected along the way (recorded so they are not re-explored)

- **Comparator in the value** (struct with a fn field): not sendable
  ([rs-fn-field] is `Rc<dyn Fn>`); fn fields bar the structural equality.
- **Ordering-overriding qualifier on elements** (`Desc Point`): erased
  qualifiers make it silently wrong at generic boundaries; a non-erased version
  is a second lowering model for values; and even the erased-with-markers
  version dies on droppability — a forgotten qualifier silently reverts the
  order. Its zone analysis and marker lowering survive inside G.
- **A separate `order` declaration species**: same lowering as G with a whole
  new declaration form; G reuses `params` + implicits instead.
- **Bound-only (`<T canbe ordered>`) as the whole answer**: its machinery is
  subsumed — the group + operator resolution covers every use, and "orderable"
  is "an ordering is in scope". (This is what the four landed steps did.)
- **Per-call `?cmp` for state-carrying structures**: build-under-one-order-
  pop-under-another. Still right for stateless algorithms
  (`sort(list, cmp = …)`); G is exactly it with resolution moved to the one
  construction site.
- **Canonical members in the struct body** (`struct Person : Eq<self> { … fn
  eq(…) }`): superseded within the session by `@`-scoped top-level fns, which
  landed. It tied canonical status to writing the obligation clause, put fns
  inside struct bodies, and needed a new `Type.member` reference form where
  `@`-scoping reuses the selector the language already has.
