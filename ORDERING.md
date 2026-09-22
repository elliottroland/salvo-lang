# Ordering, equality and hashing — the round's remainder

Raised 2026-09-21 by `demo/heap.sv`'s `Heap<T canbe ordered>` TODO
(HEAP_QUALIFIER.md item 1) and grown, through one session's worth of user
decisions, into a redesign of how comparison, equality and hashing work in the
language. **Decisions 1–14 were all made 2026-09-21. Steps 1–4 landed the same
night; step 5's own first four build items landed 2026-09-22** — their record is
in COMPLETED.md's decision log, and the rules they added are in
LANGUAGE_SPEC.md ([cmp-groups], [cmp-hash-values], [cmp-canonical],
[cmp-default], [op-order], [op-equality], [cmp-carry], [cmp-binder], with
[col-equality] and [col-hashed-ordered] rewritten).

**What is left is step 5's build item 5: the keyed containers.** This document
is now *only* that, kept whole so the work can be picked up without re-deriving
it. Delete the file when it lands.

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

// And a structure may HOLD an ordering: a fn slot on the declaration, the
// resolved identity in the type, one `?cmp` binding per signature.
qualifier Heap<T, ?cmp: (T, T) -> Int> of List<T>
fn empty_heap<T>(?cmp: (T, T) -> Int) -> Mut List<T> as Heap<?cmp>
fn heap_push<T>(heap: Heap<?cmp> Mut List<T>, elem: T) -> Mut List<T> as Heap<?cmp>
```

`Heap<min_by_age>` and `Heap<max_by_age>` are different types that refuse to
mix; a shared binder forces two parameters to carry the *same* ordering.
Dropping such a qualifier is fail-safe. A fn bound into a type must have static
identity — named, top-level, capture-free — so a lambda is refused with the
error naming why. All of that is built and tested ([cmp-carry], [cmp-binder]);
the *qualifier* case needed no backend work at all, because a qualifier erases
and the identity lowers as the implicit parameter it is resolved as.

## What is left: the keyed containers

```
intrinsic type SortedSet<T, ?cmp: (T, T) -> Int = cmp> canbe Mut
Set<T, ?hash = hash, ?eq = eq>
Map<K, V, ?hash = hash, ?eq = eq>
```

A container is where an identity stops being erasable: the *value* has to carry
it, because insertion and lookup happen inside the container.

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

### The two questions to answer first (**DECISION**, stated in ROADMAP.md)

1. **Membership semantics for a keyed container**, to be specified as
   definitions: a `SortedSet<T, f>` deduplicates by `f`; a
   `Set<T, ?hash, ?eq>`'s members are `eq`-distinct. The consequence is that
   membership depends on the slot, which is what makes a "sorted set by age"
   mean what it says.
2. **Whether `Sorted List<T>` [col-sorted-list] is parameterized** (`Sorted<?cmp>`)
   in the same change or stays canonical-only at first: once orderings are
   plural, `add_sorted` under a different `cmp` than the sort's silently breaks
   the claim.

## Rejected along the way (recorded so they are not re-explored)

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
  is "an ordering is in scope".
- **Per-call `?cmp` for state-carrying structures**: build-under-one-order-
  pop-under-another. Still right for stateless algorithms
  (`sort(list, cmp = …)`); G is exactly it with resolution moved to the one
  construction site.
- **Canonical members in the struct body** (`struct Person : Eq<self> { … fn
  eq(…) }`): superseded within the session by `@`-scoped top-level fns, which
  landed. It tied canonical status to writing the obligation clause, put fns
  inside struct bodies, and needed a new `Type.member` reference form where
  `@`-scoping reuses the selector the language already has.
- **A new `ast::Type` variant for an identity** (considered 2026-09-22 while
  building the written forms): rejected for two extra fields on `TypeRef`
  (`at`, `binder`). A variant would have needed arms in a dozen exhaustive
  matches across the checker, both emitters and the LSP for a node only ever
  written in one position; the fields cost one re-accept of every parser
  snapshot.
