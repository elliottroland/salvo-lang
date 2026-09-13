# Collections — the option space (working document)

Status: **OPEN**. Written 2026-09-12 by a read-only session, companion to
FILE_SYSTEM.md (whose FS-6 wants a map for `MemRawFs`, and which flags "std
has no map type" three times). Same style as OBLIGATIONS.md: options,
trade-offs, recommendations — the calls are the user's. Nothing here is
scheduled; the sequencing question is C-0.

Sources: LANGUAGE.md ("Qualifiers", "Auto-qualifiers and `Mut`",
"Constructive qualifiers", "State and provenance", "Refinements",
"Backends"), LANGUAGE_SPEC.md ([type-canbe-mut], [qual-erasure],
[str-drop-mut], [op-no-none], [iter-for-native], [proj-field],
[linear-generics]), std/core (list.sv, array.sv, string.sv, iterator.sv,
seq.sv conventions), BACKEND_SPEC.kotlin.md ([struct-decl] data classes),
BACKEND_SPEC.rust.md (struct derives, [rs-fn-field]), ROADMAP.md (operator
typing due before phase 4; intrinsic linear containers deferred), and the
user's stated intent (below).

## 0. The stated intent, and sequencing

The user's sketch (2026-09-12):

1. Expand the collections before the filesystem: **List, Set, Map,
   SortedSet, SortedMap (by key)** are the familiar, useful five.
2. Could **`Sorted` be an intrinsic qualifier** on Set and Map, the way
   `Mut` works in general (without being an auto-qualifier like `Mut` is)?
3. **Linked lists**: implementable in Salvo proper, or another qualifier on
   `List`?
4. Interactions with the rest of the language should be laid out —
   qualifiers especially, with `NonEmpty` the natural candidate.

**Sequencing (C-0).** Set/Map over *non-linear* elements needs nothing from
phase 3, so this work can run before or alongside phase 4 — and FS-6's
in-memory test filesystem is its first internal customer. One decision it
does share with phase 4: **operator typing** (what `==`, `<` mean and for
whom) is already due before phase 4, and C-2 below is the same question
wearing a different hat — the two should be decided together rather than
letting a key-equality rule and an operator rule diverge.

## 1. Fixed points — already decided, inherited here

- **`intrinsic type` + `intrinsic fn` is how std declares containers**
  (list.sv verbatim): each backend maps the type natively and lowers each
  call at the resolved argument type. New containers follow the pattern.
- **`canbe Mut` is the mutability story** [type-canbe-mut]: `Mut Set<T>`,
  `Mut Map<K, V>` mirror `Mut List<T>`. Kotlin maps them to the `Mutable*`
  interfaces; Rust maps both forms to one type with mutability in bindings.
  Mutators take `Mut`, everything else takes plain, a `Mut` reaches the
  plain surface by dropping.
- **Qualifiers are erased — except where a backend says otherwise.** `Mut`
  is today "the one qualifier a target may render as a different type"
  (LANGUAGE.md), and where the types really differ the *drop is a recorded
  conversion* ([str-drop-mut]: `Mut Str` → `Str` emits `.toString()` on
  Kotlin). This precedent is exactly the hook C-4's `Sorted` proposal hangs
  on: representation-affecting qualifiers with drop-conversions exist,
  singular, and the question is whether the mechanism generalizes.
- **std takes a pass** [seq-pass]: iteration surfaces are passes; the S-Seq
  combinators then work over Set/Map iteration for free. Passes over
  containers *borrow* them ([proj-field], ListYield's shape).
- **`for` keeps a native fast path** [iter-for-native] per container the
  backends recognize.
- **Intrinsic containers of linear elements are deferred** (phase 3 decision,
  2026-09-12: "Intrinsic containers (`List<linear T>`) stay deferred").
  Set/Map inherit the deferral — see C-8 for why keys are worse than a
  deferral.
- **Structs today**: Kotlin emits `data class` (structural `equals`/
  `hashCode` for free, [struct-decl]); Rust derives `Clone, Debug` — **not**
  `PartialEq`/`Eq`/`Hash`/`Ord`. Whatever C-2 decides about struct keys has
  emitter work on the Rust side.
- **Operator typing is open** ([op-no-none]: "operand typing beyond `None` —
  numeric towers, promotion, `Bool` for `&&`/`||` — is still open"). `==`
  works on `Str` today (string.sv prose); what it means on structs is
  undecided.
- **Backend parity**: divergences are closed by restriction or faithful
  emission, never silently ([backend-never-wrong] and the parity principle).
  Iteration order (C-3) is where collections meet this head-on.
- **Generated code uses no third-party crates/libraries**; what a backend
  cannot get from its standard library it ships as a static runtime file
  (`runtime/throw.kt`, `runtime/seq.rs` — and FILE_SYSTEM.md's FS-1 extends
  the same mechanism). Relevant because Rust's std has no insertion-ordered
  map (C-3).

## 2. What other languages teach

Filtered for the decisions they force, as in FILE_SYSTEM.md §2.

### Java

- The `Collection`/`List`/`Set`/`Map` + `SortedSet`/`SortedMap`/
  `NavigableMap` interface hierarchy is the user's named mental model, and
  its *interface/implementation* split (interface `SortedSet`, class
  `TreeSet`) is what the `Sorted`-as-qualifier idea reproduces more cheaply:
  `Sorted Set<T>` **is a** `Set<T>` by qualifier drop, no subtype hierarchy
  needed.
- `HashMap` vs `LinkedHashMap` vs `TreeMap` is the representation triangle
  every design must place itself in: arbitrary / insertion-ordered / sorted.
  Java makes you pick a class; Python later showed a language can just pick
  insertion-ordered as *the* semantics (below).
- `equals`/`hashCode` as universal methods is the piece Salvo deliberately
  lacks: there is no universal equality, and [op-no-none] left operand
  typing open. Java's lesson is mostly negative — mutable keys whose
  hashCode changes corrupt the map silently; C-5/C-8 can rule that out
  statically.
- `LinkedList` implements `List` and `Deque`, and forty years of profiling
  says: nobody has ever been happy with it (cache-hostile, and the O(1)
  insert needs an iterator positioned there, which the `List` interface
  doesn't give you). The *useful* thing it carries is the `Deque` interface,
  whose good implementation is `ArrayDeque`. This is C-9's whole argument.

### Kotlin

- **Read-only by default, `Mutable*` to write** — Salvo's `canbe Mut` is
  this, checked rather than conventional. The mapping is exact:
  `Set<T>`→`Set`, `Mut Set<T>`→`MutableSet`, etc., and `MutableSet` IS-A
  `Set` so the drop renders nothing (unlike `Mut Str`).
- `mutableMapOf()` is a **LinkedHashMap**: Kotlin chose insertion-ordered as
  the default, paying a pointer per entry for deterministic iteration.
- `TreeSet`/`TreeMap` accept a **`Comparator`** — the JVM side of C-2's
  comparator option is easy. (Rust's side is not; see below.)
- Data classes give structural equality to every struct for free — Kotlin
  struct keys "just work", which is exactly why C-2 must be decided
  explicitly: the *Rust* side is where the work and the restrictions live,
  and letting Kotlin's permissiveness set expectations is how parity breaks.

### Rust

- `HashMap` requires `K: Hash + Eq`; `BTreeMap` requires `K: Ord` — the
  requirement is *in the type of the container*, which is C-2's question:
  where does Salvo state what a key must support?
- **`BTreeMap` takes no comparator** (Ord is a property of the key type,
  full stop), so "sorted by a caller-supplied comparison" is genuinely
  painful in generated Rust (newtype wrapper implementing `Ord` per call
  site). C-2's comparator option dies mostly on this rock.
- **`f64` is neither `Eq` nor `Ord` nor `Hash`** (NaN), so `Double` keys are
  *unrepresentable* in Rust's native maps while Kotlin accepts `Double` keys
  happily. A forced restriction (C-2): allow it and the backends diverge on
  day one.
- `HashMap` iteration order is deliberately randomized-ish (HashDoS
  seeding); Rust's honest answer to determinism is `BTreeMap` or an external
  `indexmap` — and generated code has no external crates, hence C-3's
  runtime-file option.
- `String: Ord` is **byte-wise UTF-8**, which equals code-point order.
  Kotlin's `String.compareTo` is **UTF-16 code-unit** order, which does
  *not* equal code-point order for supplementary-plane characters. `Sorted
  Set<Str>` diverges between backends on astral characters unless the Kotlin
  runtime compares by code point (C-2's parity note; the same choice —
  characters over encoding units — was already made once for string
  indexing).
- `VecDeque` is the good deque; `std::collections::LinkedList` is the
  documented "you probably don't want this" (no cursor API on stable, cache-
  hostile) — the second half of C-9's argument.

### Python, Go, and the rest

- **Python made insertion order part of `dict`'s contract** (3.7+), and a
  generation of programmers now assumes it. **JS `Map`** likewise. This is
  the strongest argument that C-3 should pick insertion-ordered rather than
  "unspecified": the modern default *is* deterministic.
- **Go randomizes map iteration on purpose** to stop programs depending on
  it — the honest version of "unspecified", and evidence that "unspecified"
  in practice means "specified by whatever the implementation does until
  someone depends on it".
- **Clojure/Scala persistent collections**: structural sharing makes
  immutable-by-default cheap. Salvo's answer to the same pressure is
  different — borrows (`proj`), `Mut` opt-in, and `copy` opt-in — so
  persistent implementations are *not* needed; noted to close the question
  rather than open it.
- **Swift/Rust `Set`/`Dictionary` value semantics + copy-on-write**: Salvo's
  move/borrow model gets the same safety without hidden copies; nothing to
  import.

## 3. The decisions

### C-1 — Which types, and what they are

The five named, mapped:

| Salvo | Kotlin | Rust | notes |
|---|---|---|---|
| `List<T>` (exists) | `List`/`MutableList` | `Vec<T>` | unchanged |
| `Set<T>` | `Set`/`MutableSet` (impl per C-3) | per C-3 | new |
| `Map<K, V>` | `Map`/`MutableMap` (impl per C-3) | per C-3 | new |
| `Sorted Set<T>` or `SortedSet<T>` | `TreeSet` | `BTreeSet` | per C-4 |
| `Sorted Map<K, V>` or `SortedMap<K, V>` | `TreeMap` | `BTreeMap` | per C-4; sorted **by key** |

Deliberately not in v1: `Deque`/`Stack`/`Queue` (C-9 recommends recording
`Deque` as the next type, not building it), multiset/bag, bidirectional map,
persistent variants. `T[]` arrays and tuples exist and are unchanged; a
`Map` entry surfaces as a tuple `(K, V)` ([type-tuple] — native tuples on
both backends).

**DECISION C-1**: the five (minus C-4's spelling question), nothing else in
v1?

---

### C-2 — What may be a key: equality, hashing, ordering

The load-bearing decision. A `Set<T>` needs to ask "have I seen this
element?"; a `Sorted` anything needs "which comes first?". Salvo has no
universal `equals`/`hashCode`/`compareTo`, no operator typing yet, and
structs derive neither `PartialEq` nor `Hash` nor `Ord` on Rust. Options:

**O-K1 — intrinsic key types only, v1.** Keys (Set elements, Map keys) must
be one of: `Int`, `Long`, `Str`, `Char`, `Bool`. Explicitly excluded:
`Double`/`Float` (no `Eq`/`Ord`/`Hash` on Rust — a *forced* exclusion, and
independently a good idea NaN-wise), structs (Rust derive work + the
mutable-key question), unions, tuples (representable later — both backends
hash/compare tuples natively — but each addition is API surface to test).
Enforced where `Set<T>`/`Map<K, V>` is *instantiated*, like [linear-generics]
checks instantiations. Values (`V`) are unrestricted (modulo C-8).

- For: every backend requirement is met natively; zero new machinery; covers
  FS-6 (`Map<Long, …>` handle tables) and the overwhelmingly common string-
  and id-keyed cases. Restriction is the parity-honest move
  [backend-never-wrong].
- Against: no struct keys (`Map<Point, T>` refused), and the refusal wants a
  good diagnostic naming the remedy (key by a `Str`/`Int` id field).

**O-K2 — struct keys via structural equality.** Extend O-K1: a struct whose
fields are (recursively) key-eligible is key-eligible. Kotlin: free (data
class). Rust: emit `#[derive(PartialEq, Eq, Hash)]` (and `Ord` for sorted
use) on such structs — derivable mechanically, but `Ord` derives *field
order* as significance, which must then be a documented language rule
("struct ordering is lexicographic by declaration order"), and a struct
with a `Double` field or a fn-typed field ([rs-fn-field]) silently loses
eligibility, so eligibility is a computed, propagating property.

- For: `Map<UserId, …>` with a one-field struct key — the pattern the
  provenance-qualifier docs *already recommend* ("if you want a distinct
  type usable as a distinct map key, use a one-field struct") — works.
  That prose is currently a promise the language doesn't keep; O-K2 keeps it.
- Against: derive management on Rust, an eligibility relation the checker
  must own, and the declaration-order-is-significant rule for `Ord`.

**O-K3 — comparator/hasher as data (`params` groups).** A `params Ord<T>
{ fn compare(a: T, b: T) -> Int }` group, `?Ord<K>`-spread into functions
that need it, Salvo-authored for any type. This is the language's own
idiom (Yield, ToStr are groups) — and it dies on the backends: Kotlin's
`TreeMap` takes a comparator, but Rust's `BTreeMap` does not, and `HashMap`
custom hashing means per-key-type wrapper newtypes in generated code. The
honest version is a *Salvo-implemented* sorted map (a B-tree or sorted
`List` written in std, taking `?Ord<K>`), which is a real library project —
possible, unscheduled, and it forfeits the native containers.

**Recommendation: O-K1 now, O-K2 as the recorded follow-up** (it is pure
extension: same rule, wider eligibility), O-K3 only if a
sort-by-arbitrary-criterion customer appears — and note that *sorting a
List by a key function* (`sort_by(xs, (x) -> Long)`) covers most such
customers without touching the container types.

Two parity notes to pin whichever option wins:

- **`Str` ordering is code-point order** on both backends: Rust's byte-wise
  UTF-8 `Ord` already is; the Kotlin runtime must compare by code point
  rather than `String.compareTo`'s UTF-16 code-unit order (divergence is
  real for supplementary-plane characters, and "characters, not encoding
  units" is the decision string indexing already made).
- **Key mutation**: a key stored in a hash/tree structure must not change
  under it (Java's classic silent corruption). Under O-K1 keys are
  immutable types, so the hazard is unrepresentable; under O-K2, keys are
  *stored* (moved) and struct mutation needs a `Mut` path to the struct,
  which a stored key never yields — worth one deliberate test, then it is
  a selling point: the corruption bug is a compile error.

**DECISION C-2**: O-K1 / O-K2 / O-K3, `Double` exclusion confirmed, and the
code-point ordering rule. Decide alongside the operator-typing question —
"what `==` means" and "what a key is" should be one rule, not two.

---

### C-3 — Iteration order: the parity crux

Every program that prints a map or writes a set to a file observes
iteration order. Kotlin's defaults are insertion-ordered; Rust's are
arbitrary *and seeded per process*. Leaving this unspecified means the two
backends observably diverge on nearly every program that touches a Set —
[backend-never-wrong]-adjacent and impossible to test around. Options:

**O-I1 — insertion-ordered, both backends.** The Python/JS/Kotlin
semantics: iteration yields entries in first-insertion order. Kotlin:
already the default (`LinkedHashSet`/`LinkedHashMap`). Rust: **std has no
insertion-ordered map**, so the backend ships one as a runtime file
(`runtime/ordmap.rs`: the standard indexmap design — a `Vec` of entries
plus a `HashMap` from key to index; tombstone-or-swap choices documented;
a few hundred lines, no dependencies).

- For: deterministic, parity-exact, matches the modern default programmers
  assume, makes golden tests trivially stable. The runtime-file mechanism
  is established (seq.rs, throw.kt; FS-1 extends it).
- Against: the Rust side is a real (small) data-structure project with its
  own tests; iteration order becomes *semantics*, so it can never be
  changed later without breaking programs (Python is stuck with it too —
  deliberately).

**O-I2 — sorted always** (plain `Set` = `BTreeSet` on Rust, `TreeSet` on
Kotlin). Deterministic and parity-exact with zero new code — but it makes
every key need `Ord` (C-2 tightens), makes `Sorted` (C-4) meaningless as a
distinction, and pays O(log n) on every operation to give an ordering most
callers didn't ask for.

**O-I3 — unspecified order.** Honest only if enforced (Go randomizes to
*keep* it honest); in a two-backend language "unspecified" means "your
tests pass on one backend", and the golden/e2e test suite itself — which
asserts exact stdout — could not even cover Set/Map iteration.

**Recommendation: O-I1.** It is the only option that is deterministic,
keeps `Sorted` meaningful, and keeps C-2 at hash-requirements rather than
ord-requirements. The runtime `ordmap.rs`/`ordset.rs` is the cost, and it
is bounded and testable (property-test it against `HashMap` + insertion
log). `to_str` of a Set/Map (interpolation) inherits the same order for
free.

**DECISION C-3**: O-I1 / O-I2 / O-I3.

---

### C-4 — `Sorted`: a qualifier, or types of their own

The user's proposal: `Sorted` as an intrinsic qualifier on Set and Map,
"similar to how Mut works in general (it needn't be an auto-qualifier)".
Assessment first, then options.

**The proposal fits the existing machinery better than it first looks.**
The pieces it needs all have precedent:

- *A qualifier a backend renders as a different type*: `Mut` is exactly
  this, and LANGUAGE.md already says "the **one** place a qualifier
  survives erasure" — the question is whether that sentence gains a second
  member. The lowering mechanism (type + qualifier set → native type) is
  [type-canbe-mut]'s, reused.
- *Dropping the qualifier where representations differ is a conversion*:
  [str-drop-mut] (`StringBuilder` → `.toString()`). `Sorted Set<T>` →
  `Set<T>`: on Kotlin `TreeSet` IS-A `Set`, drop renders nothing (the
  `MutableList` case); on Rust `BTreeSet` ≠ O-I1's ordset, so drop is a
  rebuild — **O(n), silently, at a widening**. That is the proposal's one
  real cost, and where the options below differ.
- *Constructed, not predicated*: constructive qualifiers exist and their
  constructors live in the qualifier's file — `fn sorted_set<T>(…) ->
  Set<T> as Sorted`. `x is Sorted` on a non-union value is then a compile
  error (no predicate), which is right: sortedness-of-representation is
  not discoverable from bits, it is how the value was built. (This makes
  intrinsic `Sorted` behave like a *provenance*-flavored claim even though
  it reads like a state claim — see the C-6 note on the *other* Sorted.)
- *Not an auto-qualifier*: correct — `Mut`'s auto-machinery (language-
  managed acquisition/loss) is not wanted; `Sorted` is gained at
  construction and kept. What is needed is only `canbe`-style opt-in
  (`intrinsic type Set<T> canbe Mut, Sorted` — the opt-in list already
  exists syntactically for `Mut`).

**What `Sorted` buys over separate types** — one function surface.
`add`, `remove`, `contains`, `size`, `iter`, `keys` are written once
against `Set<T>`/`Map<K,V>` and a `Sorted` value reaches them by drop (or
by plain subsumption where the deduction keeps qualifiers — the usual
[deduce-*] rules apply, and mutating calls that only *add* can promise
`Sorted` back by refinement or by taking `Mut Sorted Set<T>` — see the
refinement note below). Sorted-only functions say `Sorted` in the
parameter: `min(set: Sorted Set<T>)`, `range(map: Sorted Map<K,V>, from:
K, to: K)`. Java needed an interface hierarchy for this; here it is a
qualifier.

The options:

**O-S1 — `Sorted` as the second intrinsic representation qualifier**
(the proposal). `intrinsic type Set<T> canbe Mut, Sorted`; constructors
`sorted_set(...)`/`sorted_map(...)`; lowering table gains (Set, {Sorted})
→ `TreeSet`/`BTreeSet` etc.; drop recorded as conversion where needed
(Rust rebuild). Mutating members must state what they do to `Sorted` the
way anything does ([deduce] strips undeclared state qualifiers — but
`Sorted`-the-intrinsic is representational: `add(m: Mut Map)` on a `Sorted
Map` must not strip the *representation*. The clean rule: **intrinsic
representation qualifiers are not droppable by deduction, only by
widening** — they behave like `Mut` on this axis too, which is the
precedent's own behavior.)

- For: the one-surface economy above; the user's instinct matches the
  machinery; teaches one general concept (representation qualifiers)
  instead of two container families.
- Against: the silent O(n) rebuild at a Rust-side drop (mitigable:
  *refuse* the implicit drop on Rust and require an explicit conversion fn
  — but then Kotlin and Rust accept different programs, or the refusal
  must be portable and Kotlin's free IS-A is forfeited: pick one). And it
  widens the "one place erasure is violated" sentence into a *category*,
  which is spec surface forever.

**O-S2 — separate intrinsic types** `SortedSet<T>`/`SortedMap<K, V>` (the
Java spelling without the interfaces). Every shared function is declared
twice (overloads — the three `size` overloads are precedent and it is
mechanical); no subsumption (`SortedSet` where `Set` is wanted needs an
explicit conversion, honest about the O(n) on Rust, wasteful on Kotlin
where it is free).

- For: no new qualifier machinery; every cost is visible; each type's key
  bound is stated independently (`SortedSet<T>` requires ord-eligible,
  `Set<T>` only hash-eligible — under O-S1 this becomes "the `Sorted`
  qualifier tightens the instantiation bound", a qualifier-conditional
  *requirement*, which is novel).
- Against: 2× surface forever; the "is-a" relation everyone knows from
  Java is absent; combinators taking `Set<T>` exclude sorted sets.

**O-S3 — `Sorted` as an ordinary (erased) state qualifier + one
representation.** No representation change at all: containers are always
O-I1's insertion-ordered structures; `Sorted` is a plain constructive
state qualifier meaning "currently in sorted order" (constructors:
`sorted(list)`, `sorted_set(…)` which inserts in order), stripped by
mutation like any state claim, restorable by refinements. `min`/`range`
then *cannot* rely on tree structure — they are O(n)/O(n) or need
sorted-insertion discipline.

- For: zero backend work beyond C-3; `Sorted` composes with `List` too
  (see C-6 — a sorted List enabling binary search is real); no drop
  problem.
- Against: it isn't the feature the user asked for — no O(log n) sorted
  containers at all; `range` over a big map is a scan.

**Recommendation: O-S1, with the drop rule decided eyes-open** (suggest:
implicit drop *allowed* and recorded as conversion — the Kotlin cost is
zero, the Rust cost is O(n) at a widening the programmer wrote, matching
`Mut Str`'s existing precedent where dropping also pays a conversion), and
O-S3's *List* half adopted independently in C-6. If the qualifier-tightens-
the-key-bound consequence (C-2 interaction) proves unpleasant to specify,
O-S2 is the retreat that loses only economy, not capability.

**DECISION C-4**: O-S1 / O-S2 / O-S3; if O-S1: (a) confirm intrinsic
representation qualifiers as a category (spec: [type-canbe-q]
generalizing [type-canbe-mut]), (b) the drop-conversion rule, (c) that
`Sorted` requires ord-eligible keys at instantiation while plain
`Set`/`Map` require only hash-eligible.

---

### C-5 — The function surface

Strawman, mirroring list.sv's conventions (constructor moves elements in;
`get` returns a borrowed optional; mutators take `Mut` and list survivors
exhaustively):

```
intrinsic type Set<T> canbe Mut            // + Sorted per C-4
intrinsic fn set<T>(...elems: T[]) [] -> Set<T>
intrinsic fn mutable_set<T>(...elems: T[]) [] -> Mut Set<T>
intrinsic fn add<T>(set: Mut Set<T>, elem: T) [] -> Bool => set: Mut, !elem   // false if present
intrinsic fn remove<T>(set: Mut Set<T>, elem: T) [] -> Bool => set: Mut, elem
intrinsic fn contains<T>(set: Set<T>, elem: T) [] -> Bool => set, elem
intrinsic fn size<T>(set: Set<T>) [] -> Int => set

intrinsic type Map<K, V> canbe Mut         // + Sorted per C-4
intrinsic fn map_of<K, V>(...entries: (K, V)[]) [] -> Map<K, V>
intrinsic fn mutable_map<K, V>(...entries: (K, V)[]) [] -> Mut Map<K, V>
intrinsic fn get<K, V>(map: Map<K, V>, key: K) [] -> (proj[from: map] V)? => map, key
intrinsic fn put<K, V>(map: Mut Map<K, V>, key: K, value: V) [] -> None => map: Mut, !key, !value
intrinsic fn remove<K, V>(map: Mut Map<K, V>, key: K) [] -> V? => map: Mut, key
intrinsic fn contains_key<K, V>(map: Map<K, V>, key: K) [] -> Bool => map, key
intrinsic fn size<K, V>(map: Map<K, V>) [] -> Int => map
```

Notes and sub-decisions:

- **Constructor naming**: `map` collides with the S-Seq combinator, hence
  `map_of` (the one asymmetry; alternatives: `dict(...)`, or renaming
  nothing and letting overload resolution try — a `map` taking a variadic
  tuple array vs a `map` taking a pass + fn *do* differ, but leaning on
  that for the most common constructor in the language is fragile).
  Entries as native tuples read fine: `map_of(("a", 1), ("b", 2))`.
- **`put` returns `None`, not the old `V?`** — Java's return-the-old-value
  is occasionally handy and *forces a move of the old value out on every
  put* (ownership-wise the right shape, in fact: the map must not silently
  drop an owned old value... for non-linear values dropping is fine, and
  C-8 keeps linear values out, so plain `None` is safe). Sub-decision:
  offer `replace(map, k, v) -> V?` later if a customer wants the old value.
- **`remove` returning `V?` moves the value out** — this is FS-6's handle
  table (`remove(table, id)` hands the resource back) and, later, the
  cache-handle discharge shape from OBLIGATIONS.md, so its signature
  should be right from day one.
- **`get` borrows** (`proj[from: map] V` mirrors list's `get`); a caller
  that stores the result writes `copy` [copy-opt-in].
- **Sorted extras** (per C-4): `min`/`max` (`(proj[from: set] T)?`),
  `first_key`/`last_key`, `range(sorted, from, to)` as a pass — v1 can
  ship `min`/`max` only and defer `range` (it is the first *pass minted by
  an intrinsic* — design it when a customer exists).
- **`is_empty`**: skip — `size(x) == 0`, and NonEmpty (C-6) is the useful
  spelling.
- `union`/`intersect`/`difference`: writable in Salvo over passes + `add`;
  not intrinsic; defer to demand.

**DECISION C-5**: the surface above — names (`map_of`?), `put`'s return,
`remove`-returns-the-value, borrow shapes.

---

### C-6 — NonEmpty and friends: the state-qualifier dividend

This is where collections repay the qualifier machinery. `NonEmpty` is
LANGUAGE.md's *running example* (predicate + constructor + refinement on
`add`) but is not actually declared in std. Ship it, over each container:

```
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool { return size(list) > 0 }
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty, !elem
}
// and NonEmpty of Set<T>, NonEmpty of Map<K, V> similarly
// (refn on add/put; remove strips it via the ordinary Mut deduction)
```

What it buys immediately: `first(xs: NonEmpty List<T>) -> proj T` (no `?`),
`min(set: NonEmpty Sorted Set<T>) -> proj T`, a `fold` that needs no seed,
FS-7-style APIs returning `NonEmpty List<Str>` where emptiness was already
checked. The `is NonEmpty` test and the refinement flow are *already
implemented and tested* — this is pure std authorship.

Sub-decisions and notes:

- **Overload or optional-return pairs?** `first` on plain `List` returns
  `T?`; on `NonEmpty List` returns `T` — two overloads ranked by qualifier
  specificity (the overload machinery already ranks these; [fn-overload]).
  Recommend shipping the `NonEmpty` overloads for `first`/`min`/`max` only,
  not a parallel universe of them.
- **The other `Sorted`** (O-S3's half, independent of C-4): a plain state
  qualifier `Sorted of List<T>` — `qualifies` is writable (`is_sorted`
  scan), constructor `fn sort<T>(list: List<T>) -> List<T> as Sorted`, and
  `binary_search(xs: Sorted List<T>, x: T) -> Int?` becomes an honest
  signature. If C-4 picks O-S1, *this* Sorted-of-List and *that*
  Sorted-of-Set are one name with two mechanics (erased state claim vs
  representation) — acceptable (qualifiers are per-subject-type already),
  but say it in the docs; if it grates, name the List one `SortedOrder` or
  ship it later.
- **`Distinct of List<T>`** (no duplicates — what a Set gives you back as
  a List): cheap, useful for `to_list(set)` return type. Optional.
- **Predicate qualifiers can't index into containers** (field overrides are
  struct-only), so none of these carry payload facts; they are pure claims.
  That is enough for the signatures above.

**DECISION C-6**: ship std `NonEmpty` over List/Set/Map (+Array?), the
`NonEmpty` overload set, `Sorted of List` with `sort`/`binary_search`, and
whether the C-4/C-6 name collision on `Sorted` is acceptable.

---

### C-7 — Iteration: passes over Set and Map

List's pass walks by index (`get(items, at)`) — position-based, borrowing,
no native iterator object. Sets and maps have no index, so the pattern
doesn't transplant. Options:

**O-T1 — intrinsic passes holding native iterators.** `intrinsic type
SetYield<T>` whose backend representation wraps `Iterator`/`std::…::Iter`.
Rust's iterator borrows the container with a lifetime — an intrinsic
struct field holding `Iter<'a, T>` needs the lifetime plumbing that
[proj-field] emits for view structs; plausible but it is the first
*intrinsic* borrowing pass, and Kotlin's `Iterator` is trivially fine.
Mutation during iteration: the pass borrows the container ([proj-field]
fate rules), and mutating needs a `Mut` path the borrow excludes — the
ConcurrentModificationException class of bug is a *compile error*. That
guarantee is worth advertising loudly whichever option wins.

**O-T2 — eager snapshots**: `keys(map) -> List<K>` (copies keys),
`values(map) -> List<proj V>`?, `entries(map) -> List<(K, proj V)>`…
Copying keys of intrinsic types is cheap and honest (`copy` semantics per
backend); `values`/`entries` borrowing *into a freshly built List* is a
new shape for the borrow machinery (a List of projections — 2b's
`List<Proj T>` views exist, so it may just work, but it is the exotic
path).

**O-T3 — `for`-native only, combinators later**: `for (k, v) in
iter(map)` lowered natively per backend ([iter-for-native] already gives
`for` a per-container native path), with *no* general pass in v1 — Set/Map
don't feed combinators until O-T1 is designed properly.

**Recommendation: O-T1 for Set (elements borrow cleanly, one type
parameter), O-T1 for Map with entries emitted as `(proj K, proj V)`…
if the tuple-of-borrows shape checks out against the emitters — otherwise
O-T3 ships first and O-T1 follows.** The deciding fact is repo-internal
(how [proj-field] lifetimes generalize to an intrinsic pass), so this one
needs a prototype, not a document. Iteration *order* is C-3's answer in
all cases.

**DECISION C-7**: O-T1 / O-T2 / O-T3, prototype-informed.

---

### C-8 — Linear elements: keys never, values later

Phase 3 deferred `List<linear T>`; Set/Map add one sharper fact:

- **Linear keys are unsound, not just deferred.** `add(set, handle)` where
  the element compares equal to one already present *drops the duplicate*
  (or the insert is refused at runtime — either way an obligation is
  silently discharged or duplicated by data-dependent control flow the
  checker cannot see). Keys/elements should be excluded from linearity
  *permanently* by the C-2 key bound (intrinsic key types are never
  linear, so under O-K1 this costs nothing and even O-K2's struct keys
  stay non-linear by construction — a `linear struct` key is refused).
- **Linear *values* are the future customer, not the v1 scope.**
  `Map<Long, InStream>` is FS-6's handle table and OBLIGATIONS.md's cache
  example (`remove(cache, handle)` as a discharge with context) — the
  container-of-linear design (O-C3's intrinsic-container half) lands here
  when it lands. v1: `Map<K, V>` instantiation with linear `V` is refused
  by the existing [linear-generics]-style check, message pointing at the
  deferral. (FS-6's *runtime-file* handle table dodges this by living in
  native code — worth a cross-reference in FILE_SYSTEM.md when that
  session resumes.)

**DECISION C-8**: confirm keys permanently non-linear; values deferred
with the intrinsic-container question.

---

### C-9 — Linked lists: probably a Deque, probably not now

The user's question: Salvo-proper, or a qualifier on `List`? Three
readings, and a recommendation to do none of them yet:

**O-L1 — a representation qualifier (`Linked List<T>`).** Mechanically it
is C-4's O-S1 again (Kotlin `java.util.LinkedList`, Rust
`std::collections::LinkedList`). But the *reason* fails before the
mechanism does: both targets' linked lists are the discouraged corner of
their own libraries (cache-hostile; Rust's has no stable cursor API, so
the O(1)-middle-insertion that justifies a linked list is unreachable),
and `get(list, i)` — the surface List already promises — is O(n) on a
linked representation, so the qualifier would change complexity contracts
silently. A representation qualifier should never make the shared surface
*worse*; `Sorted` doesn't, `Linked` does.

**O-L2 — Salvo-proper**: `struct Node<T> { value: T, next: Node<T> |
None }`. This is a **recursive type**, now investigated and written up as
its own ROADMAP.md section ("Recursive types", 2026-09-12): direct field
and union-arm recursion is an undiagnosed backend divergence today (Kotlin
compiles it, Rust dies downstream with E0072, no Salvo diagnostic —
recorded as an open defect), while recursion *through `List<T>`* already
works end to end on both backends. So O-L2 is blocked on the Rust boxing
rule that section details — a language decision far bigger than linked
lists (trees, ASTs, JSON), wrong to smuggle in here.

**O-L3 — the honest replacement: `Deque<T>`.** Every workload people
reach for linked lists for (queues, BFS frontiers, sliding windows, LRU
order) is served better by `ArrayDeque`/`VecDeque`, which both targets
ship and are proud of. One intrinsic type, six functions
(`push_front/back`, `pop_front/back`, `peek` both ends), no new concepts.

**Recommendation: none in v1.** Record O-L3 as the next collection when a
customer appears (phase 5's mailboxes may be it); the recursive-types
question O-L2 surfaced now has its own ROADMAP.md section and open-defect
entry (the missing diagnostic), so nothing further is owed from here.

**DECISION C-9**: defer all three; adopt the two roadmap recordings?

---

### C-10 — Interpolation and conversion odds-and-ends

- `to_str` intrinsics for Set/Map mirroring List's (`[1, 2, 3]`,
  `{a: 1, b: 2}`?) — order per C-3, format a sub-decision (Kotlin and
  Rust debug-format maps differently; the intrinsic must pick one string
  and emit it identically).
- `to_list(set) -> List<T>` (order per C-3), `to_set(list) -> Set<T>`
  (dedup — returns plain `Set`, and `Distinct` on the reverse direction
  per C-6), `keys`/`values` per C-7's choice.
- Set/Map equality (`==` on containers) — punt to the operator-typing
  decision; nothing here requires it (Kotlin equals is deep, Rust
  `PartialEq` on containers exists, but *exposing* it is the operator
  question).

**DECISION C-10**: formats and converters, low stakes, decide at build
time.

## 4. Summary of recommendations

| decision | recommendation |
|---|---|
| C-0 sequencing | build before/alongside phase 4 (FS-6 is the first customer); decide C-2 together with operator typing |
| C-1 types | Set, Map (+ Sorted per C-4); no Deque/others in v1 |
| C-2 keys | intrinsic key types only (`Int`, `Long`, `Str`, `Char`, `Bool`); `Double`/`Float` excluded (Rust forces it); struct keys recorded as the O-K2 follow-up; `Str` ordering fixed at code-point order on both backends |
| C-3 iteration order | insertion-ordered on both backends; Rust side ships `runtime/ordmap.rs`/`ordset.rs` (indexmap design, no deps) |
| C-4 `Sorted` | the user's proposal, adopted: second intrinsic representation qualifier (`canbe Sorted`), constructive (`sorted_set(...)`), drop-as-conversion per the `Mut Str` precedent; `Sorted` tightens the key bound to ord-eligible; separate types (O-S2) as the retreat |
| C-5 surface | list.sv-style intrinsics; `map_of` constructor; `put -> None`; `remove -> V?` (moves the value out — the future discharge shape); `get` borrows |
| C-6 qualifiers | std ships `NonEmpty` (predicate + refinements + `first`/`min`/`max` overloads) and `Sorted of List` (state qualifier: `sort`, `binary_search`); name overlap with C-4 documented |
| C-7 iteration | intrinsic borrowing passes (native iterators) if the [proj-field] lifetime story generalizes — prototype first; `for`-native as the fallback ship vehicle |
| C-8 linearity | keys permanently non-linear (dedup would drop obligations); linear values deferred with intrinsic containers |
| C-9 linked lists | none: `Linked` qualifier breaks the surface's complexity contract, Salvo-proper is blocked on recursive types (now its own ROADMAP.md section + open defect); `Deque<T>` recorded as the next type when a customer appears |
| C-10 conversions | `to_str`/`to_list`/`to_set`, formats picked at build time |

The load-bearing calls are **C-2** (what may be a key — decide with
operator typing), **C-3** (iteration order — the parity crux, and the one
with a real Rust runtime cost), and **C-4** (whether `Sorted` makes
representation qualifiers a category). C-9's real yield was the
recursive-types question, now written up as its own ROADMAP.md section
("Recursive types") with the missing diagnostic recorded as an open defect.

## 5. Interactions to verify first (the test list)

1. **Iteration-order parity end to end**: build the same map on both
   backends, print it — identical output (the golden tests can only exist
   if C-3 lands deterministic).
2. **`Str` key ordering on supplementary-plane characters** in a
   `Sorted Set<Str>` — the Kotlin runtime's code-point comparator vs
   Rust's byte order, one test with astral characters.
3. **`Sorted` drop**: pass a `Sorted Set` where `Set` is wanted on both
   backends — Kotlin renders nothing, Rust emits the conversion; then
   mutate through the plain handle and confirm the `Sorted` original is
   unaffected (the conversion copied) — or is *consumed* (the drop moved) —
   whichever the C-4(b) rule says, verified observably.
4. **Refinement flow on `put`/`add`**: `+NonEmpty` survives into inferred
   deductions (the refill pattern from LANGUAGE.md, on a Map).
5. **Mutation during iteration is refused**: hold a Set pass, try `add` —
   the borrow/fate diagnostic, both backends compile-refuse identically.
6. **Linear refusals**: `Set<InStream>` instantiation refused;
   `Map<Str, InStream>` refused with the deferral message (once FS types
   exist; until then a local `linear struct`).
7. **`remove` hands the value out**: ownership-correct on both backends
   (no clone on Rust, no aliasing on Kotlin) — the future cache-discharge
   shape must be movement-clean from day one.
8. **Tuple entries**: `map_of(("a", 1))` construction and `for (k, v) in
   iter(m)` destructuring against [type-tuple] on both backends.
