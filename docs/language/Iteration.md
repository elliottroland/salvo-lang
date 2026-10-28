# Iteration

## Iterating anything: `Next`

`fns` groups are how Salvo says what a Rust programmer would say with a
trait bound. The standard library's own example is iteration:

```
fns Next<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Next<It, T>) -> Mut List<U> => it: Mut, f {
    let out = Mut List<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}
```

A `for` over a **type parameter** works because `for x in it` is the call
`next(it)`, and the spread brought in a `next` that takes an `It`: inside `map`
it is a parameter, which a call reaches before any declared `next`, so the loop
drives by calling it and takes the element type from its result. Nothing about
`for` is special-cased for std — this is how any combinator of your own reads.

There is no `Next` *type* and nothing implements it: `map` needs a `next` for
whatever `it` is, and the call site supplies one. The subject is an
**iterator** — a position in a sequence — so a container is iterated by writing
its `iter`, and an `iter fn` is called:

```
let doubled = map(iter(xs), double)         // a list
let letters = filter(iter("hello"), keep)   // a string's characters
let capped = map(countdown(3), double)      // an `iter fn` of your own
```

A type of your own joins in with an `iter` — an `iter fn` whose `state` is its
iterator struct, or a fn handing one back — and may declare `: Iter<self, It,
T>`, naming that struct, to have the pair checked at the type
([Iterators](Iterators.md)):

```
struct Bag : Iter<self, ListNext<Int>, Int> {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListNext<Int> => bag {
    return iter(bag.items)
}

let total = reduce(iter(bag), 0, (acc, n) -> acc + n)
```

Inference runs *through* the group: `It` comes from the subject, and `T` — the
element type — from the `next` that takes an `It` (or `It`'s `: Next<self, T>`
clause). That is what lets the lambda be written bare (`n -> n * 2`) with no
annotation anywhere.

The other spread, `?Iter<C, It, T>`, is for a combinator over a **source**
rather than an iterator: it brings in the `iter` that mints an `It` and the
`next` that drives it, so `total(xs)` and `total(bag)` both work and the body
writes `for x in c` — `next(iter(c))`, through the two parameters:

```
fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let sum = 0
    for n in c {
        sum = sum + n
    }
    return sum
}
```

`map`, `filter` and `reduce` are **eager**: they return a `Mut List<U>`.
Chaining works because a list has an `iter` like anything else. Each also has a
`List` overload, so `map(xs, f)` — no `iter` — keeps the short spelling for the
type people map most. One named variant covers the rest:

- `map_to` and `filter_to` put their results in a collection you provide, given
  first because it is what the call is about, and **hand it back** so a chain
  can carry on from it. Appending goes through an `?add` implicit parameter, so
  the destination is anything with an `add` — not just a `List`.

```
let doubled = map(xs, double)                       // Mut List<Int>
let out = map_to(Mut List<Int>(), iter(xs), double)
let kept = filter_to(map_to(Mut List<Int>(), iter(xs), double), iter(ys), is_even)
```

The destination is moved in and returned, which is what makes the nested form
work. If you want to keep hold of one across the call, rebind it:
`let sink = map_to(sink, iter(xs), double)`.

**Iterating a container borrows it**: `iter(xs)` hands back an iterator that
holds a view of `xs`, so `xs` stays usable and cannot be mutated while the
iterator lives. A `for` straight over the container allocates nothing at all —
that is data, and the backends walk it in place.

`Next` and `Iter` are the pattern, not the only instances: `std.core.compare`
declares `Ordered`, `Eq` and `Hashed` the same way
([Comparison, equality and hashing](Comparison-and-Hashing.md)), and
`std.core.list` declares `Ref` for algorithms generic over what a *position*
is ([Mutable handles](Mutable-Handles.md)). A type joins any of them by
declaring one function.

## Obligations: `fns` groups on a type

A `fns` group can also be an obligation on a **struct**, so the
requirement travels with the type rather than with each function that takes
it (`export struct ArrayNext<T> : Next<self, proj T>`, `struct Bag :
Iter<self, Int>`). See [Implicit parameters](Implicit-Parameters.md).
