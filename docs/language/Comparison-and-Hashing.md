# Comparison, equality and hashing

Three capabilities are declared exactly this way, in `core.compare`:

```
params Ordered<T> { fn cmp(a: T, b: T) -> Int }
params Eq<T>      { fn eq(a: T, b: T) -> Bool }
params Hashed<T>  { fn hash(value: T) -> Long
                    fn eq(a: T, b: T) -> Bool }
```

`Hashed` is a **pair**, and says so: a hash container buckets by `hash` and
confirms the bucket hit by `eq`, so a `hash` without its `eq` is useless and a
pair that disagrees is a container that loses values. `Ordered` is deliberately
not a pair — no sorted collection consults equality, since it decides membership
by `cmp` — which is what lets one program hold a `Set<Person>` keyed by every
field beside a heap ranked by age, without either being a lie.

A group that spreads two overlapping capabilities asks for the shared member
once: `?Eq<T>, ?Hashed<T>` brings one `eq`. And a spread asks for *every* member
it declares, whether the body uses it or not — so a function that only needs an
ordering writes `?cmp: (T, T) -> Int` rather than a group.

So "orderable" is not a property of a type — it is *an ordering being in
scope*. `cmp` answers a negative number when `a` sorts first, zero when the two
tie and a positive number otherwise; `eq` decides equality; `hash` digests.
Each is an ordinary overloadable function, which is the whole mechanism: no
trait, no interface value, nothing either backend knows about.

The intrinsic types come with theirs — `cmp` for `Int`, `Long`, `Byte`, `Char`,
`Bool` and `Str`, `eq` for those plus `Double` and `Float`, `hash` for the ones
`cmp` covers:

```
let order = cmp("ab", "b")      // negative: strings compare by code point
let yes = eq(1.5, 1.5)          // true
```

`Double` and `Float` have an `eq` and no `cmp`: `NaN` ties with nothing, so
there is no total order to promise — the same reason a sorted collection of
them is refused.

A type of your own joins in by declaring the function. A function **declared on
a type** travels with it, and there are two ways to declare one on a struct —
inside its body, or as your file's answer to an obligation it states:

```
struct Person {
    name: Str,
    age: Int

    fn cmp(a: Person, b: Person) -> Int {
        return cmp(a.age, b.age)
    }
}
```

```
struct Person : Ordered<self> { name: Str, age: Int }

fn cmp(a: Person, b: Person) -> Int {
    return cmp(a.age, b.age)
}
```

Both declare `cmp` *on* `Person`, and either way it is an ordinary function:
`cmp(p, q)` and `p.cmp(q)` reach it, and it takes part in overload ranking like
anything else. What attachment adds is that it **travels with the type** — a
module that imports `Person` gets its `cmp` too, so "orderable" arrives with the
type instead of depending on which of its file's names you happened to import —
and that an implicit `?cmp` resolves to it by default.

The obligation form is also a check: `: Ordered<self>` says at the declaration
that a matching `cmp` exists, rather than failing at some distant use.

**Visibility.** A function inside the body takes the struct's own — writing
`export` on it is an error, because it is contained in the struct. A *detached*
fulfilment is its own declaration, so if the type is exported it must say
`export` too; otherwise the capability would be unreachable in the modules that
can use the type.

**Naming one.** `cmp@Person` names the `cmp` declared on `Person` — at a call,
and as a value:

```
let by_age = cmp@Person(ada, bob)      // the one declared on the type
let by_name = cmp@main(ada, bob)       // this module's own
let younger = min_of(ada, bob, cmp = cmp@Person)
```

You need the selector exactly when two `cmp`s fit: ambiguity is always refused
rather than resolved by scope (see [Functions](Functions.md)).

For the everyday case — compare the fields, in order — you do not write the
bodies at all. `auto` asks the compiler for them:

```
struct Point {
    x: Int,
    y: Int

    auto fn cmp(a: Point, b: Point) -> Int
    auto fn eq(a: Point, b: Point) -> Bool
    auto fn hash(value: Point) -> Long
}
```

An `auto fn` has no body: the compiler writes one from the fields. Ordering is
lexicographic by field declaration order (so field order is significant), and
everything generated is structural, so a generated `cmp`, `eq` and `hash` agree
with each other by construction.

On an obligation clause, `auto` is shorthand for exactly those declarations —
one per member of the group:

```
struct Point : auto Ordered<self>, auto Hashed<self> { x: Int, y: Int }
```

The point of `auto` being a modifier on the function is that a type can mix.
Suppose `Person` should be ordered by name, with equality meaning "same rank":

```
struct Person : Hashed<self> {
    name: Str,
    age: Int

    auto fn cmp(a: Person, b: Person) -> Int
    auto fn hash(value: Person) -> Long

    fn eq(a: Person, b: Person) -> Bool {
        return cmp(a, b) == 0
    }
}
```

`auto Hashed<self>` is not available here, because it would generate the `eq`
this type writes by hand — and a bare `: Hashed<self>` is still the promise,
checked at the declaration, that a `hash` and an `eq` exist.

The compiler can write `cmp`, `eq` and `hash`, and says so if you write `auto`
on anything else. An `auto fn` names its type with `@`, because that is where
the fields come from. A field it cannot compare (a `Double`, a function) is an
error at the declaration, where the mistake is, not at a distant
`SortedSet<Point>`; so is a `canbe Mut` struct, which could change while a
collection holds it.

Writing a member by hand *and* asking for it with `auto` is a duplicate: keep
one.

A *generic* function has to ask, because nothing about an opaque `T` is
knowable:

```
fn min_of<T>(a: T, b: T, ?Ordered<T>) -> T {
    if cmp(a, b) <= 0 {
        return a
    }
    return b
}

let smaller = min_of("pear", "apple")   // the call site supplies `cmp`
```

The group spread is what makes `cmp(a, b)` legal inside, the call site fills it
with the canonical overload for the type it instantiates, and a function that
forwards to another needing the same capability declares it too — the colouring
implicit parameters already have.

**A hash value means something within one execution and nowhere else.** Each
backend hashes with its host's own algorithm, so the same value digests
differently on Kotlin and on Rust — deliberately, the way the two backends
generate different random numbers. What holds everywhere is that equal values
hash equal. So a hash is a bucket, not data: do not print it, persist it, or
compare it across runs.

### A structure that holds an ordering

`sort(list, cmp = …)` picks an ordering per call, which is right for an
algorithm that does its work and hands the result back. A structure that
*stays* ordered — a heap, a `SortedSet`, a `Sorted` list — cannot work that
way: building it under one ordering and reading it under another is a corrupt
structure. Such a structure names the ordering it holds as a **type
argument** (`Heap<T>(?cmp) List<T>`), so two built under different orderings
are different types and refuse to mix. That machinery — slots on a qualifier,
and the function identities that fill them — is
[Implicit parameters](Implicit-Parameters.md).

A keyed container is the same story, and its **constructor asks for the
identity**: `set_of` takes `?Hashed<T>`, `sorted_set_of` takes `?Ordered<T>`,
and the pair that resolved lands in the type the call answers. So the ordinary
case needs nothing —

```
let names: Mut Set<Str> = mut_set_of()          // the host's own identity
let people: Mut Set<Person> = mut_set_of()      // `Person`'s declared pair
```

— and a container keyed by *something else* passes it, which is the one way to
choose:

```
fn age_hash(p: Person) -> Long => p { return to_long(p.age) }
fn same_age(a: Person, b: Person) -> Bool => a, b { return a.age == b.age }

// two people of an age are one member here, and the type says so
let byage: Mut Set<Person>(age_hash, same_age) =
    mut_set_of(hash = age_hash, eq = same_age)
```

An annotation that names a pair does not *choose* it — it has to agree with
what the constructor resolved, because what fills the slot is what the
container is actually keyed by.

A hash and an equality are only meaningful **together**: equal values must hash
equally, and a container looks up by the hash first, so a custom equality beside
somebody else's hash would quietly do nothing rather than fail. `Hashed` says so
in its own declaration:

```
params Hashed<T> => eq with hash {
    fn hash(value: T) -> Long
    fn eq(a: T, b: T) -> Bool
}
```

`=> eq with hash` means the two are one decision, so a call supplies both or
neither — writing one and leaving the other to be found is an error that names
the fix:

```
// Refused: `eq` is written here, `hash` would be resolved.
let s: Mut Set<Point> = mut_set_of(eq = same_age)

// Written out — now the pair is stated rather than assembled.
let s: Mut Set<Point> = mut_set_of(hash = age_hash, eq = same_age)
```

A function that *holds* an identity says the same thing by taking the whole
group: `fn collect<T>(x: T, ?Hashed<T>)` forwards both members to whatever it
builds, where taking only `?hash` would leave the equality to be found. Taking
the two as separate implicit parameters instead (`?hash: …, ?eq: …`) is how a
signature opts out of the pairing, since the relation travels with the group.

A **generic** function builds one the same way, by declaring the capability and
letting the constructor's own slots be filled from it:

```
fn gather<T>(a: T, b: T, ?Hashed<T>) -> Set<T> => !a, !b {
    let s: Mut Set<T> = mut_set_of()
    add(s, a)
    add(s, b)
    return s
}
```

The container is keyed by whatever the caller's `T` brought with it — a declared
pair for a struct that has one, the host's own identity for a primitive — and a
function that only *forwards* the capability to a builder needs to say nothing
extra either. Both backends key the container by the functions themselves, which
is what the two runtimes have always taken.

One thing does not work yet, and says so: a keyed container over a **tuple or a
list**, whose identities are the hosts' own and Salvo cannot name them yet — the
same reason `(1, 2) == (1, 2)` does not resolve.
