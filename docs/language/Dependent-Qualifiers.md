# Dependent qualifiers

Most claims are about a value on its own: a `NonEmpty List<T>`, a
`Positive Int`. A **dependent** claim is about a value's relationship *to
another value* — that this `Int` is a valid index of *that* list, that this
key is present in *that* map. The relationship is what makes it useful: once
the compiler knows it, the operation that would otherwise answer "maybe"
can answer outright.

## Declaring one

The qualifier declares the values it depends on as **value slots** —
unprefixed entries in its block — and its `qualifies` takes them as
parameters after the subject:

```
// core.map — the claim that a key is present in one particular map.
qualifier KeyOf<K, V>(map: Map<K, V>) of K {
    fn qualifies(key: K, map: Map<K, V>) -> Bool {
        return contains_key(map, key)
    }
}
```

A use fills the slot with a **place** — a variable or a field chain — and the
test is the ordinary `is`, with the slot filled:

```
if k is KeyOf(m) {
    // `k` is a `KeyOf(m) Str` here
}
assert!(k is KeyOf(m))     // or hoisted: narrows the rest of the scope
```

## The claim is bound to an identity

`KeyOf(m)` and `KeyOf(m2)` are different facts. The claim is tied to the
*identity* of the value that filled the slot — its fate roots, so an alias of
`m` is still `m`, but a different map is a different claim and the compiler
says so.

And because the claim is about the **map's** contents rather than the key's,
it is invalidated from the other side: any mutation of `m` strips `KeyOf(m)`
from every value holding it. Conservatively — reads keep it, mutating some
other map keeps it. Reassigning `m` strips it too, since the value the claim
described is gone.

This is refinement typing at its smallest: prove a fact once, carry it in the
type, and let mutation of what it depends on take it away.

## Consuming one: total operations

A signature can *demand* the claim, which is how an operation loses its
"maybe". `core.list` declares an index claim and a `get` that needs it:

```
// The claim: 0 <= index < size(list), about one particular list.
export qualifier Idx<T>(list: List<T>) of Int {
    fn qualifies(index: Int, list: List<T>) -> Bool {
        return index >= 0 && index < size(list)
    }
}

// The optional read — no claim, so it may answer nothing.
export intrinsic fn get<T>(list: List<T>, index: Int) -> (proj(list) T)?

// The total read — the claim did the checking, so there is no `None` arm.
export fn get<T>(list: List<T>, index: Idx(list) Int) -> proj(list) T
```

Both are called `get`; which one a call gets is ordinary overload resolution,
ranked by the qualifier ([Qualifiers](Qualifiers.md)):

```
let i = 0
println("${get(xs, i)!}")       // no claim: the optional read, asserted

if i is Idx(xs) {
    println("${get(xs, i)}")    // the claim is in `i`'s type: the total read
}
```

`swap(list, i: Idx(list) Int, j: Idx(list) Int)` is the same idea for a
mutation: two proven indices cannot be out of range, so there is no `Bool` to
check. And the passes that *produce* indices mint the claim, which is what
makes a loop total end to end:

```
for i in indices(xs) {
    total = total + get(xs, i)   // every element, no assertion anywhere
}
```

## Keeping a claim across a mutation: `preserve`

Stripping every claim on any mutation would make the machinery useless the
moment a map is written to. So the claim's *owner* can opt specific calls
back in with a **`preserve` entry** — either as a refinement on someone
else's function, or in a function's own clause:

```
// core.map, inside the qualifier that owns the claim: writing under a key
// never removes one.
refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf
```

In a function's own clause it is *checked*: every call in the body handing the
map to a mutator must itself preserve the claim, or the promise is refused.
Which is what lets this pass with no `!` anywhere:

```
let m = mut_map_of(("a", 1))
let k = "a"
assert!(k is KeyOf(m))
put(m, "b", 2)              // preserves KeyOf claims
let v = get(m, k)           // the total overload: an Int, not an Int?
```

`core.list` preserves `Idx` across `add` and `swap` for the same reason —
growth keeps every existing index valid, and an exchange moves no boundary.
The `update` family preserves it too, which is what makes a sequence of
in-place writes stay total ([Mutable handles](Mutable-Handles.md)).

## The other slot kinds

A slot does not have to hold a place. Three kinds exist, and a qualifier uses
one of them:

* **A place** — everything above.
* **A constant**, compile-time known, so nothing can invalidate it:
  `core.range` declares `qualifier InRange(lo: Int, hi: Int) of Int`, used as
  `InRange(0, 65535) Int`.
* **A function identity** — the ordering a structure is kept by:
  `Heap<T>(?cmp)`, described in
  [Implicit parameters](Implicit-Parameters.md).

## What std declares

| Claim | Says | Consumed by |
|---|---|---|
| `Idx<T>(list)` | a valid index of *this* list | total `get`, total `swap` |
| `KeyOf<K, V>(map)` | this key is present in *this* map | total `get` |
| `NotEq(i)` | this `Int` differs from *that* one | `update2`, two-handle calls |
| `SpanOf(str)` | `0 <= start <= end <= size(str)` | total `substr` |
| `InRange(lo, hi)` | within a constant range | your own signatures |

`SpanOf` is worth a second look, because it claims a *pair* whole:

```
struct Span { start: Int, end: Int }
qualifier SpanOf(str: Str) of Span

fn substr(str: Str, at: SpanOf(str) Span) -> Str     // total
```

A single claim over a two-field struct carries the cross-field fact
(`start <= end`) along with the bounds — the parse-don't-validate pattern for
a precondition that is not about one number.

All of it is **erased**: the claims reach the backends only as the arguments
of a lowered `qualifies` call, and a total operation compiles to the direct
access with no check at all.
