# Ordering, equality and hashing — the round's remainder

Raised 2026-09-21 by `demo/heap.sv`'s `Heap<T canbe ordered>` TODO
(HEAP_QUALIFIER.md item 1) and grown, through two sessions' worth of user
decisions, into a redesign of how comparison, equality and hashing work in the
language. **Decisions 1–14 were made 2026-09-21 and 15–22 on 2026-09-22.** What
has landed so far: steps 1–4 of the first plan (2026-09-21) and step 5's own
first four build items (2026-09-22) — their record is in COMPLETED.md's decision
log, and the rules are in LANGUAGE_SPEC.md ([cmp-groups], [cmp-hash-values],
[cmp-canonical], [cmp-auto], [op-order], [op-equality], [cmp-carry],
[cmp-binder], with [col-equality] and [col-hashed-ordered] rewritten).

**What is left is the five-step sequence below**, agreed 2026-09-22. Delete this
file when step 5 of it lands.

## What already works (so the remainder reads in context)

```
// The three capabilities, as params groups in core.compare.
params Ordered<T> { fn cmp(a: T, b: T) -> Int }
params Eq<T>      { fn eq(a: T, b: T) -> Bool }
params Hashed<T>  { fn hash(value: T) -> Long }

// A canonical implementation, @-scoped to its type and travelling with it.
fn cmp@Person(a: Person, b: Person) -> Int { return cmp(a.age, b.age) }

// Or generated, structurally, from the fields.
struct Point : auto Ordered<self>, auto Hashed<self> { x: Int, y: Int }

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
identity — named, top-level, capture-free. The *qualifier* case needed no
backend work at all, because a qualifier erases and the identity lowers as the
implicit parameter it is resolved as.

## The decisions of 2026-09-22 (all the user's)

**15. `default` becomes `auto`, at the function level.** A capability's
structural implementation is declared as a **bodiless `auto fn`**, `@`-scoped to
its type:

```
struct Person : Ordered<self>, Hashed<self> { name: Str, age: Int }

auto fn cmp@Person(a: Person, b: Person) -> Int
auto fn hash@Person(a: Person) -> Long

// …and a hand-written member beside the generated ones, which is the point:
fn eq@Person(a: Person, b: Person) -> Bool { return cmp(a, b) == 0 }
```

`auto Group<self>` on the obligation clause stays, as **sugar for an `auto fn`
per member**. The obligation clause itself goes back to being only a promise
(`: Ordered<self>`), checked as [group-obligation] already checks it. What this
buys is the thing `default` could not express: *some* members generated and
others written, which is what a type wanting a custom `eq` over a structural
`cmp` needs.

**16. A `params` group states every member it needs — visibly.** `Hashed<T>`
gains `eq`, because a hash container buckets by `hash` and confirms by `eq`: a
`hash` without its `eq` is useless, and a pair that disagrees is a silently
broken container on both hosts. `Ordered<T>` does **not** gain `eq` (decision
17). This is what makes [cmp-auto]'s "the `default` forms bring `eq` with
them" stop being a special rule: it becomes group membership, visible in
`core.compare`.

```
params Ordered<T> { fn cmp(a: T, b: T) -> Int }
params Eq<T>      { fn eq(a: T, b: T) -> Bool }
params Hashed<T>  { fn hash(value: T) -> Long
                    fn eq(a: T, b: T) -> Bool }
```

**17. A sorted container's membership is `cmp(a, b) == 0`; a hash container's is
`eq`.** Each names its own basis, and that is what answers the question the
round left open. `Ordered` therefore does not carry an `eq`: the sorted
containers never consult one. Both hosts already collapse by the comparator —
`BTreeSet`/`BTreeMap` decide duplicates by `Ord` alone, and `java.util.TreeSet`/
`TreeMap` use the comparator handed to them (the JDK documents "inconsistent
with equals" as permitted) — so this is writing down what the backends do rather
than changing it, and it is the only rule implementable over a BTree.

The two may legitimately disagree, and unbundling is what lets both be right:
`Set<Person>` (`eq` over all fields) beside `SortedSet<Person, by_age>` (`cmp`
over age) says "same person" and "same rank" are different questions.
`Distinct List<T>` is a claim by `eq` while sortedness is a claim by `cmp` —
independent claims on one value. And `binary_search` is an existing **defect**
this fixes: both backends find the lower bound with the comparator and then
confirm the hit with the *host's* `==` (`__x == __e` on Rust,
`__l[it] == __e` on Kotlin), which is neither `eq` nor `cmp == 0` — and since
equality became opt-in, a struct with no `eq` at all still gets host equality
there. It becomes `cmp(x, e) == 0`, the only test consistent with the `Sorted`
claim the parameter demands.

**18. Overlapping spreads merge.** `?Ordered<T>` and `?Hashed<T>` in one
signature bring **one** `eq`, not two: it is one function position, named once.
[implicit-group]'s "two implicits of one name is an error" was written for a rare
mistake and now describes the common case; it applies only when the two
positions' *types* differ. The same answer holds for a qualifier's slot list —
`Heap<T, ?Ordered<T>, ?Hashed<T>>` carries one `eq` slot — because that decides
type identity.

**19. A spread resolves every member, used or not.** `?Ordered<T>` asks the call
site for exactly what the group declares; a body that needs less says so by
declaring the members it needs individually, or through a narrower group. The
rule stays predictable and the remedy is a spelling that already exists.

**20. The contracts stay trusted.** `auto fn hash@Person` beside a hand-written
coarser `eq@Person` breaks every `Set<Person>` and nothing catches it — the same
posture [cmp-groups] already takes, and the alternative (`auto` all-or-nothing
within a bundle) would refuse the very mixture decision 15 exists to allow.

**21. A `params` group may be spread into a slot list, and an alias remembers
what it came from.**

```
qualifier Heap<T, ?Ordered<T>> of List<T>     // sugar for ?cmp: (T, T) -> Int
```

At a use site the type argument is written and the slots are **optional**: a
signature chooses which of them it can call, and a slot it does not mention is
not constrained — the value keeps carrying it. That makes today's bare `Heap` the
empty case of one rule. Collisions are aliased with the syntax destructuring
already uses (`field: variable_name`):

```
fn f<T>(heap1: Heap<T, ?Ordered<T>> List<T>, heap2: Heap<T, ?cmp: cmp2> List<T>)
```

— `heap1` brings `?cmp` and `heap2`'s is `?cmp2`, and `heap2` brings no `eq`.
**An alias remembers the slot it came from**, so `t1 > t2` in that body is
refused: two in-scope candidates for the same capability are an ambiguity
whatever they are called, and the body must name the one it means. Aliasing is
how you get two orderings into one scope, not how you dodge the ambiguity
between them.

**22. `Sorted List<T>` is parameterized too** (`Sorted<?cmp>`), as its own step
after the containers.

## The sequence

Steps 1–3 are language surface with no emitter work; step 4 is where the backend
work is. Each step is one commit.

1. ✅ **`default` → `auto`, at the function level** — **landed 2026-09-22**
   (COMPLETED.md's log). The word, the bodiless `auto
   fn` (the third legal bodiless form after `intrinsic` and an effect member),
   the `@`-scoping requirement, the generable-member whitelist (`cmp`, `eq`,
   `hash`), the signature check against the member, and `auto Group<self>` as
   sugar. `FnDecl.structural` already exists and already drives both emitters'
   derive path, so a hand-written `auto fn` *is* a structural fn and needs no
   desugaring: the clause sugar keeps producing what it produces today. Groups
   unchanged. ~120 sites across std, both emitters, the checker, the parser,
   five test files, the specs and the editor grammar.
2. **`Hashed<T>` gains `eq`; overlapping spreads merge; a spread resolves every
   member.** Decisions 16, 18, 19. The merge is in `collect_implicits`' duplicate
   check; the group change is `std/core/compare.sv` plus whatever asked for
   `?Hashed<T>` and relied on getting only `hash`.
3. **Group spreads in slot lists, aliasing, and the ambiguity refusal.**
   Decisions 21 and the alias half of the operator rule: `?Group<T>` in a
   qualifier's or type's generics list, `?slot: alias` at a use site, alias
   provenance recorded so the operator resolution can refuse two candidates for
   one capability.
4. **The keyed containers.** `SortedSet<T, ?Ordered<T>>` (one slot),
   `SortedMap<K, V, ?Ordered<K>>`, `Set<T, ?Hashed<T>>`, `Map<K, V, ?Hashed<K>>`
   (two slots each), the two membership definitions, `binary_search`'s confirm,
   the Rust marker/`OrdBy` machinery and the Kotlin runtime hash container.
5. **`Sorted<?cmp>`** — the list claim carrying its ordering, and
   `sort`/`mut_sort`/`add_sorted`/`binary_search` binding it.

## Lowering the containers (step 4's design, as decided 2026-09-21)

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
- **Bundling `eq` into `Ordered<T>`** (proposed and dropped 2026-09-22, within
  one exchange): it was the answer to "what does a sorted container deduplicate
  by", and it dies on the fact that neither host's sorted container consults
  equality at all — the slot would be carried in every sorted type's identity
  and read by nothing. Naming each container's basis (decision 17) answers the
  question instead, and unbundling is what lets a program hold both a
  `Set<Person>` by all fields and a `SortedSet<Person, by_age>` by rank without
  either being a lie.
- **`auto` as an all-or-nothing bundle** (a checkable coherence rule): it would
  refuse `auto cmp` + `auto hash` + a hand-written `eq = cmp(a, b) == 0`, which
  is the mixture decision 15 exists to allow. Trust instead (decision 20).
