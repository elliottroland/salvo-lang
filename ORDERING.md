# Ordering, equality and hashing — the round's remainder

Raised 2026-09-21 by `demo/heap.sv`'s `Heap<T canbe ordered>` TODO
(HEAP_QUALIFIER.md item 1) and grown, through two sessions' worth of user
decisions, into a redesign of how comparison, equality and hashing work in the
language. **Decisions 1–14 were made 2026-09-21 and 15–22 on 2026-09-22, and
everything decided has been built except one item.**

**What is left is step 5 of the sequence below — `Sorted<?cmp>` — and nothing
else.** It is written out in full under "Step 5 in full": the problem, the four
signatures, the one way it differs from the keyed containers, the snag to expect
with two ways out and a recommendation, the file and line references, and what
done looks like. Start there. **Delete this file when it lands**, leaving the
record in COMPLETED.md.

What has landed, all with COMPLETED.md log entries and rules in LANGUAGE_SPEC.md
([cmp-groups], [cmp-hash-values], [cmp-canonical], [cmp-auto], [op-order],
[op-equality], [cmp-carry], [cmp-binder], [col-membership], [col-keyed-slots],
with [col-equality] and [col-hashed-ordered] rewritten): the three capabilities as
params groups, `@`-scoped canonicals, `auto fn` generation, the operators through
the groups (2026-09-21); then identities in types, the written forms, the `?cmp`
binder, group spreads in slot lists with aliasing, and the keyed containers keeping
their keys by the ordering or hash their type names — on both backends
(2026-09-22).

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
2. ✅ **`Hashed<T>` gains `eq`; overlapping spreads merge; a spread resolves
   every member** — **landed 2026-09-22**. Decisions 16, 18, 19. The merge is in `collect_implicits`' duplicate
   check; the group change is `std/core/compare.sv` plus whatever asked for
   `?Hashed<T>` and relied on getting only `hash`.
3. ✅ **Group spreads in slot lists, aliasing, and the ambiguity refusal** — **landed 2026-09-22**.
   Decisions 21 and the alias half of the operator rule: `?Group<T>` in a
   qualifier's or type's generics list, `?slot: alias` at a use site, alias
   provenance recorded so the operator resolution can refuse two candidates for
   one capability.
4. **The keyed containers**, in two halves because only the second needs the
   runtimes:
   - ✅ **4a, the declarations** — **landed 2026-09-22**: the four types' slots,
     defaulted to the canonical implementation, with membership named per
     container ([col-membership], [col-keyed-slots]). The default is **not
     materialized**, so `Set<Str>` is unchanged; a non-canonical identity is
     refused with a message naming what it waits for.
   - ✅ **4b, the runtimes** — **landed 2026-09-22**, in five slices (COMPLETED.md
     has each): the Rust sorted pair behind a boxed store with ZST markers, the
     pattern rule, the emitter wiring, the Rust hash pair (which closed a
     pre-existing defect), and Kotlin's two halves. Originally described as: Rust's `SalvoSet`/`SalvoMap` keyed by the slots'
     functions rather than by the host's `Hash`/`Eq`, a sorted pair that carries
     its comparator, the Kotlin hash container, `binary_search`'s confirm
     (`cmp == 0`, fixing the host-`==` defect), and std's own signatures gaining
     binders so a fn can be generic over the identity. **A `fn` pointer beats the
     marker design here**: an identity is a named top-level fn, so
     `cmp: fn(&T, &T) -> i32` is Copy and Send, needs no hidden marker generic,
     and is what Kotlin has to do anyway — the markers stay recorded as the
     optimization.
5. **`Sorted<?cmp>`** — the list claim carrying its ordering. **This is all that
   is left of the round**, and it is written out under "Step 5 in full" below.

## Step 5 in full — `Sorted<?cmp>` (decision 22; the round's last item)

**The problem.** `Sorted` is a *claim on a list*, minted by `sort` and demanded by
`add_sorted` and `binary_search`:

```
export qualifier Sorted<T> of List<T> with NonEmpty            // std/core/list.sv:172
export intrinsic fn sort<T>(list: List<T>) [] -> List<T> as Sorted => list
export intrinsic fn add_sorted<T>(list: Mut Sorted List<T>, elem: T) [] -> None
export intrinsic fn binary_search<T>(list: Sorted List<T>, elem: T) [] -> Int?
```

Now that orderings are plural, the claim is **not enough**: nothing says *which*
ordering a `Sorted List<T>` is sorted by, so `add_sorted` under a different `cmp`
than the sort used silently breaks the claim — it inserts at a position that is a
lower bound for one ordering and nonsense for the other. The fix is the one the
whole round is about: the claim names its ordering.

```
export qualifier Sorted<T, ?cmp: (T, T) -> Int> of List<T> with NonEmpty

export fn sort<T>(list: List<T>, ?Ordered<T>) [] -> List<T> as Sorted<T, ?cmp> => list
export fn mut_sort<T>(list: List<T>, ?Ordered<T>) [] -> Mut List<T> as Sorted<T, ?cmp> => list
export fn add_sorted<T>(list: Mut Sorted<T, ?cmp> List<T>, elem: T) [] -> None
    => list: Mut Sorted, !elem
export fn binary_search<T>(list: Sorted<T, ?cmp> List<T>, elem: T) [] -> Int? => list, elem
```

`sort` **publishes** what it sorted by; the other two **capture** it, so the
insert position and the search are computed with the ordering the list actually
carries. A `Sorted` written bare stays legal and unconstrained (step 3's pattern
rule), which is what a body that only passes a sorted list along wants.

**What is different from step 4, and it is the crux.** A keyed *container* holds
its ordering at run time — that is what step 4 built. A **qualifier has nothing to
hold it in**: `Sorted List<T>` is a plain `Vec<T>`/`MutableList<T>` with a claim,
and qualifiers erase [qual-erasure]. So the ordering has to arrive **at each
operation**, as the implicit parameter the binder already lowers to — which is
exactly the "cheaper first cut" this document recorded for the qualifier case, and
which step 3 built and tested (`carry_tests.rs`). No markers, no runtime container,
no new type-system work.

**The snag to expect.** These four are `intrinsic fn`s, and **no `intrinsic fn`
takes an implicit parameter today** (checked 2026-09-22: 25 intrinsics, none with
a `?`). Their lowerings are templates over rendered arguments, so the bound `cmp`
has to reach the template. Two ways, both implementation choices:
  * **(a)** teach the intrinsic path to pass implicit arguments — the emitters
    already render them (`emit_implicit_args`), so the templates would receive them
    as trailing `args`; or
  * **(b)** make these four *ordinary Salvo fns* over a lower-level intrinsic
    (`sort_by(list, cmp)`, `lower_bound(list, elem, cmp)`), which keeps intrinsics
    implicit-free and puts the binding in Salvo where it reads. `filter_to`
    (std/core/seq.sv:94) is the precedent for an ordinary fn carrying implicits.
  * **(b) is the recommendation**: it needs no emitter surgery, and the bodies are
    three lines each.

**What it also finishes.** Today both backends sort and search by the **host's**
ordering — Rust `__v.sort()` (derived `Ord`), Kotlin `sortedWith(__salvoCompare)` —
so a *hand-written* `cmp@Person` is ignored by `sort`, `add_sorted` and
`binary_search` alike. That is the other half of the `binary_search` defect fixed
2026-09-22: its bound and confirm were made *consistent* with each other, and step
5 makes them consistent with the **ordering the claim names**. Expect the e2e case
to be: sort a list by a hand-written non-structural `cmp`, then `binary_search` for
an element — which answers wrongly today.

**Where things are**:
  * the declarations: `std/core/list.sv:172–205`;
  * the lowerings: `crates/salvo-backend-rust/src/intrinsics.rs` (`"sort"`,
    `"add_sorted"`, `"binary_search"` around lines 239–260) and
    `crates/salvo-backend-kotlin/src/intrinsics.rs` (around 180–205);
  * the machinery to lean on: `qual_slot_ty` and `collect_binder_slots` in
    `crates/salvo-core/src/check.rs` (a qualifier's slot type comes from matching
    its `of` type against the value), and `carry_tests.rs` for the shape of a test;
  * the rules: [cmp-carry], [cmp-binder], [col-sorted-list], [col-membership].

**Done looks like**: the four signatures above, the emitters using the bound
ordering, tests in `carry_tests.rs` (a list sorted by one ordering refusing an
`add_sorted` bound to another) and one e2e case per backend from verbatim the same
source, [col-sorted-list] updated to say the claim carries its ordering, a
COMPLETED.md log entry — and then **delete this file**, since it is the last item
in it.

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
