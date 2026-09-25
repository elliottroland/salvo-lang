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

A type of your own joins in by declaring the function — and the *canonical* one
for a type is written `@`-scoped to it, in the type's own file:

```
struct Person { name: Str, age: Int }

fn cmp@Person(a: Person, b: Person) -> Int {
    return cmp(a.age, b.age)
}
```

`cmp@Person` is an ordinary function: `cmp(p, q)` and `p.cmp(q)` both reach it.
The `@` adds two things. It **travels with the type** — a module that imports
`Person` gets its canonical too, so "orderable" arrives with the type instead of
depending on which of its file's names you happened to import — and it is what
an implicit `?cmp` resolves to by default. `export` on it is explicit and must
match the type's: a public type with a private canonical is a compile error, not
a silent hole.

Because a canonical is *the* implementation for its type, an ambiguity around
one is never settled by scope. If another `cmp` also fits, the call says which
it means, with the same selector the declaration was written with:

```
let by_age = cmp@Person(ada, bob)      // the canonical
let by_name = cmp@main(ada, bob)       // this module's own
let younger = min_of(ada, bob, cmp = cmp@Person)
```

A type can also promise the capability at its declaration,
`struct Person : Ordered<self>`, which checks there that a matching `cmp`
exists rather than failing at some distant use.

For the everyday case — compare the fields, in order — you do not write the
bodies at all. `auto` asks the compiler for them, on the function:

```
struct Point { x: Int, y: Int }

auto fn cmp@Point(a: Point, b: Point) -> Int
auto fn eq@Point(a: Point, b: Point) -> Bool
auto fn hash@Point(value: Point) -> Long
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
struct Person : Ordered<self>, Hashed<self> { name: Str, age: Int }

auto fn cmp@Person(a: Person, b: Person) -> Int
auto fn hash@Person(value: Person) -> Long

fn eq@Person(a: Person, b: Person) -> Bool {
    return cmp(a, b) == 0
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
