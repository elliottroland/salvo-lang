# Dependent qualifiers

## Overview

Most claims are about a value on its own: a `NonEmpty List<T>`, a
`Positive Int`. A **dependent** claim is about a value's relationship *to
another value*: that this `Int` is a valid index of *that* list, that this
key is present in *that* map. The relationship is what makes it useful. Once
the compiler knows it, an operation that would otherwise answer "maybe" can
answer outright.

Take reading a list element. Without any knowledge about the index, `get`
has to allow for an index out of range, so it answers an optional:

```salvo
let xs = List(3, 4, 5)
let i = 1
let maybe = get(xs, i)            // an Int?: `i` might be out of range
```

Test the index against the list once, and the claim goes into `i`'s type:

```salvo
if i is Idx(xs) {
    let sure = get(xs, i)         // an Int: `i` is an `Idx(xs) Int` here
}
```

`Idx(xs)` reads "a valid index of `xs`". It names one particular list, so an
index proven for `xs` says nothing about another list `ys`. And because the
fact depends on `xs`, changing `xs` can make it false: after a change to
`xs` that is not known to keep every index valid, the compiler forgets the
claim and `get` is back to answering an optional.

In Rust you would either check the bounds at each access (`xs.get(i)`,
answering an `Option`) or index directly and accept a possible panic
(`xs[i]`). A dependent claim does the check once, carries the result in the
type, and lets the total operation compile to the direct access. TypeScript
narrowing works on one variable at a time; a dependent claim narrows a
variable *relative to another one*. This is refinement typing at its
smallest: prove a fact once, carry it in the type, and let mutation of what
it depends on take it away.

The rest of this page shows how such a qualifier is declared, how the claim
is tied to the value it depends on, how signatures demand it, how a claim
survives mutations that cannot break it, and what std declares.

## Declaring one

The qualifier declares the values it depends on as **value slots**,
unprefixed entries in its block, and its `qualifies` takes them as parameters
after the subject:

```salvo
// std.core.map: the claim that a key is present in one particular map.
qualifier KeyOf<K, V>(map: Map<K, V>) for K {
    fn qualifies(key: K, map: Map<K, V>) -> Bool {
        return contains_key(map, key)
    }
}
```

A use fills the slot with a **place**, meaning a variable or a field chain,
and the test is the ordinary `is` with the slot filled:

```salvo
if k is KeyOf(m) {
    // `k` is a `KeyOf(m) Str` here
}
assert!(k is KeyOf(m))     // or hoisted: narrows the rest of the scope
```

The slot must be filled: `k is KeyOf` alone is an error naming the filled
form. std's declaration is [`KeyOf`](../../lib/std/core/map.sv#qualifier-keyof).

## The claim is bound to an identity

`KeyOf(m)` and `KeyOf(m2)` are different facts. The claim is tied to the
*identity* of the value that filled the slot, which the compiler tracks as
its fate roots (see
[Shared fate](Deductions-and-Ownership.md#shared-fate-and-copy)). So an alias of `m`
is still `m`, but a different map is a different claim, and the compiler says
so.

Because the claim is about the **map's** contents rather than the key's, it
is invalidated from the other side: any mutation of `m` strips `KeyOf(m)` from
every value holding it. This is conservative. Reads keep the claim, and
mutating some other map keeps it. Reassigning `m` strips it too, since the
value the claim described is gone.

## Consuming one: total operations

A signature can *demand* the claim, which is how an operation loses its
"maybe". [`std.core.index`](../../lib/std/core/index.sv) declares an index
claim, [`Idx`](../../lib/std/core/index.sv#qualifier-idx), and each container
has a `get` that needs it:

```salvo
// The claim: 0 <= index < size(c), about one particular container.
export qualifier Idx<C>(c: C) for Int {
    fn qualifies(index: Int, c: C, ?size: (c: C) -> Int) -> Bool {
        return index >= 0 && index < size(c)
    }
}

// The optional read: no claim, so it may answer nothing.
export platform fn get<T>(list: List<T>, index: Int) -> (proj(list) T)?

// The total read: the claim did the checking, so there is no `None` arm.
export fn get<T>(list: List<T>, index: Idx(list) Int) -> proj(list) T
```

A signature fills the slot with a sibling parameter: `Idx(list)` names the
`list` parameter, and at each call it stands for whatever the caller passed.

One `Idx` serves every container with a `size`: a `List`, a `Deque`, an
`IntBuffer`. Its `qualifies` takes the `size` as an implicit parameter
([Implicit parameters](Implicit-Parameters.md)), filled where the claim is
*tested*, at the container the test names, so nothing about it shows in the
type. `Idx(xs) Int` reads the same whatever `xs` is.

Both functions are called `get`. Which one a call gets is ordinary overload
resolution, ranked by the qualifier ([Qualifiers](Qualifiers.md),
[Functions](Functions.md)):

```salvo
let i = 0
println("${get(xs, i) ?: unreachable!()}")       // no claim: the optional read, asserted

if i is Idx(xs) {
    println("${get(xs, i)}")    // the claim is in `i`'s type: the total read
}
```

`swap(list, i: Idx(list) Int, j: Idx(list) Int)` is the same idea for a
mutation: two proven indices cannot be out of range, so there is no `Bool` to
check. And the iterators that *produce* indices, such as
[`indices`](../../lib/std/core/index.sv#fn-indices), mint the claim, which is
what makes a loop total end to end:

```salvo
for i in indices(xs) {
    total = total + get(xs, i)   // every element, no assertion anywhere
}
```

## Keeping a claim across a mutation: `preserve`

Stripping every claim on any mutation would make the machinery useless the
moment a map is written to. So the claim's *owner* can opt specific calls
back in with a **`preserve` entry**, either as a refinement on someone else's
function ([Qualifiers](Qualifiers.md#refinements)) or in a function's own
deduction clause:

```salvo
// std.core.map, inside the qualifier that owns the claim: writing under a key
// never removes one.
refn put(map: Mut Map<K, V>, key: K, value: V) => map: preserve KeyOf
```

In a function's own clause the entry is *checked*: every call in the body
that hands the map to a mutator must itself preserve the claim, or the
promise is refused.

With `put`'s refinement, this passes with no `!` anywhere:

```salvo
let m = Mut Map(("a", 1))
let k = "a"
assert!(k is KeyOf(m))
put(m, "b", 2)              // preserves KeyOf claims
let v = get(m, k)           // the total overload: an Int, not an Int?
```

`Idx` is preserved across a list's `add`, `append`, `swap` and `replace`, a
deque's `add_first`, `add_last` and `replace`, and a buffer's `replace` and
`clear`, for the same reason: growth keeps every existing index valid, and an
exchange or a write in place moves no boundary. All of those refinements live
in `std.core.index`, beside the claim, since only the claim's owner may say
that a call keeps it. `std.core.list`'s
[`update`](../../lib/std/core/list.sv#fn-update) preserves it too, which is
what makes a sequence of in-place writes stay total
([Mutable handles](Mutable-Handles.md)).

## The other slot kinds

A slot does not have to hold a place. Three kinds exist:

* **A place**: everything above.
* **A constant**, known at compile time, so nothing can invalidate it.
  [`std.core.range`](../../lib/std/core/range.sv#qualifier-inrange) declares
  `qualifier InRange(lo: Int, hi: Int) for Int`, used as `InRange(0, 65535) Int`.
  A constant is an `Int`, a `Char` or a `Str` literal, of the slot's own type:
  `qualifier Prefix(p: Str) for Str` is written `Prefix("arn:") Str`, and
  `qualifier Initial(c: Char) for Str` is written `Initial('p') Str`. Two
  claims are the same fact only when their constants are equal.
* **A function identity**: the ordering a structure is kept by, as in
  `Heap<T>(?cmp)`, described in [Implicit parameters](Implicit-Parameters.md).

Places and constants are both *value* slots, and one qualifier may mix them.
A qualifier with function-identity slots has no value slots.

## What std declares

| Claim | Says | Consumed by |
|---|---|---|
| [`Idx<C>(c)`](../../lib/std/core/index.sv#qualifier-idx) | a valid index of *this* container | total `get`, `at`, `swap`, `replace`, `update` |
| [`KeyOf<K, V>(map)`](../../lib/std/core/map.sv#qualifier-keyof) | this key is present in *this* map | total `get`, `at` |
| [`NotSame(a)`](../../lib/std/core/ref.sv#qualifier-notsame) | this handle names a different element from *that* one | two live handles, two-handle calls |
| [`SpanOf(str)`](../../lib/std/core/string.sv#qualifier-spanof) | `0 <= start <= end <= size(str)` | total `substr` |
| [`InRange(lo, hi)`](../../lib/std/core/range.sv#qualifier-inrange) | within a constant range | your own signatures |

`SpanOf` is worth a second look, because it claims a *pair* whole:

```salvo
struct Span { start: Int, end: Int }

qualifier SpanOf(str: Str) for Span {
    fn qualifies(span: Span, str: Str) -> Bool {
        return span.start >= 0 && span.start <= span.end && span.end <= size(str)
    }
}

fn substr(str: Str, at: SpanOf(str) Span) -> Str     // total
```

A single claim over a two-field struct carries the cross-field fact
(`start <= end`) along with the bounds. This is the parse-don't-validate
pattern for a precondition that is not about one number. (A struct rather
than a tuple, because a qualifier cannot apply to a tuple.)

## At run time

All of it is **erased**. The claims reach the backends only as the arguments
of a lowered `qualifies` call, and a total operation compiles to the direct
access with no check at all.
