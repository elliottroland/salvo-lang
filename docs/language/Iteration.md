# Iteration

## Overview

Salvo has one loop over sequences, `for x in e`, and a library of functions
over sequences in [std.core.seq](../../lib/std/core/seq.sv). This page is
about using them: what a `for` can walk, the eager functions (`map`,
`filter`, `reduce`), the lazy adaptors (`mapping`, `taking`, …), and writing
functions of your own that take any iterator. How `for` works underneath,
and how to write an iterator, is the next page,
[Iterators](Iterators.md).

```salvo
let xs = List(1, 2, 3, 4, 5)

for x in xs {
    println("${x}")                                  // 1, 2, 3, 4, 5
}
for i in range(3) {
    println("${i}")                                  // 0, 1, 2
}

let squares = map(xs, (x) -> x * x)                  // [1, 4, 9, 16, 25]
let odd = filter(iter(xs), (x) -> x % 2 == 1)        // [1, 3, 5]
let total = reduce(xs, 0, (acc, x) -> acc + x)       // 15
for x in taking(mapping(iter(xs), (x) -> x * 10), 2) {
    println("${x}")                                  // 10, 20
}
```

Two words recur. An **iterator** is a value holding a position in a
sequence; it is what the sequence functions take. A **source** is a value
an iterator can be minted from, such as a list or a string: its `iter`
function answers a fresh iterator over it. A `for` takes either, and the
sequence functions take an iterator, so a container is passed to them as
`iter(xs)`. The most common case keeps a short spelling: `map`, `filter`
and `reduce` also take a `List` directly.

Coming from Rust, `next` plays the part of `Iterator::next` and `iter` the
part of `IntoIterator`, but there is no trait to implement; coming from
Kotlin or Java, the eager functions are the collection operations and the
adaptors are a `Sequence` or `Stream`. There is no iterator *type* either:
every iterator is a struct of its own, and functions accept any of them
through an implicit parameter
([Iterating anything: `Next`](#iterating-anything-next)).

## What a `for` can walk

```salvo
for x in xs { ... }             // a list: walked in place
for c in "hi" { ... }           // a string's characters
for x in iter(xs) { ... }       // the iterator a list's `iter` hands back
for x in range(0, 10) { ... }   // the iterator `range` mints
for n in countdown(3) { ... }   // an `iter fn` of your own
for x in bag { ... }            // a type of your own with an `iter`
```

std's containers (lists, arrays, strings, sets, maps, deques) are sources,
and [`range`](../../lib/std/core/range.sv#fn-range) mints an iterator over
evenly spaced `Int`s. A type of your own becomes something a `for` walks by
having a `next` (it is then an iterator) or an `iter` (it is then a source);
[Iterators](Iterators.md) shows both, and the `iter fn` that writes an
iterator for you.

**Iterating a container borrows it.** `iter(xs)` hands back an iterator that
holds a view of `xs`, so `xs` stays usable and cannot be mutated while the
iterator lives. A `for` straight over the container allocates nothing at
all: that is data, and the backends walk it in place.

**An iterator is advanced where it lives.** An iterator held in a variable
keeps the position a loop reached, so one loop can read part of it and
leave the rest for another, or for a function:

```salvo
let it = range(5)
for i in it {
    if i == 1 {
        break
    }
}
println("${count(it)} left")                         // 3 left: 2, 3 and 4
```

Only a *temporary* subject, such as `for x in iter(xs)`, is consumed by the
loop. [Iterators](Iterators.md#overview) has the precise rule.

## Eager functions: `map`, `filter`, `reduce`

[`map`](../../lib/std/core/seq.sv#fn-map),
[`filter`](../../lib/std/core/seq.sv#fn-filter) and
[`reduce`](../../lib/std/core/seq.sv#fn-reduce) are **eager**: they run the
whole pass when called, and `map` and `filter` return a `Mut List<U>`.
Chaining works because a list has an `iter` like anything else. Each also
has a `List` overload, so `map(xs, f)` (no `iter`) keeps the short spelling
for the type people map most.

```salvo
let doubled = map(iter(xs), double)         // a list
let letters = filter(iter(word), keep)      // a string's characters
let capped = map(countdown(3), double)      // an `iter fn` of your own
```

[`to_list`](../../lib/std/core/seq.sv#fn-to_list) drives an iterator to its
end and answers its elements as a list, and
[`count`](../../lib/std/core/seq.sv#fn-count) answers how many there were.

**`filter` borrows its source's elements.** Its answer is a list of views
of the elements the iterator walks, not copies, so it can live no longer
than the source: filter a string or list held in a variable, not a literal
written in the call. [std.core.seq](../../lib/std/core/seq.sv) gives each
function's details.

**Into a collection you provide.** One named variant covers the rest:
[`map_to`](../../lib/std/core/seq.sv#fn-map_to) and
[`filter_to`](../../lib/std/core/seq.sv#fn-filter_to) put their results in a
collection you provide, given first because it is what the call is about,
and **hand it back** so a chain can carry on from it. Appending goes through
an `?add` implicit parameter, so the destination is anything with an `add`,
not just a `List`.

```salvo
let doubled = map(xs, double)                       // Mut List<Int>
let out = map_to(Mut List<Int>(), iter(xs), double)
let kept = filter_to(map_to(Mut List<Int>(), iter(xs), double), iter(ys), is_even)
```

The destination is moved in and returned, which is what makes the nested
form work. If you want to keep hold of one across the call, rebind it:
`let sink = map_to(sink, iter(xs), double)`.

## Iterators over iterators: `mapping`, `taking`, …

A function whose name is a present participle answers an iterator over
another one: [`mapping(it, f)`](../../lib/std/core/seq.sv#fn-mapping),
[`filtering(it, keep)`](../../lib/std/core/seq.sv#fn-filtering),
[`taking(it, n)`](../../lib/std/core/seq.sv#fn-taking),
[`taking_while(it, keep)`](../../lib/std/core/seq.sv#fn-taking_while),
[`skipping(it, n)`](../../lib/std/core/seq.sv#fn-skipping) and
[`skipping_while(it, skip)`](../../lib/std/core/seq.sv#fn-skipping_while).
Nothing runs until the result is driven, and each step pulls one element
from the source, so they work on a source that never ends. `map` and
`filter` are their eager counterparts: they drive the source at once and
answer a list.

```salvo
iter fn naturals(n: Int) -> Emitted Int | Finished {
    state Naturals {
        at: Int = n
    }
    let v = at.copy()
    at = at + 1
    return emitted(v)
}

let odd_squares = filtering(mapping(naturals(1), (x: Int) -> x * x), (x: Int) -> x % 2 == 1)
for x in taking(odd_squares, 3) {   // 1, 9, 25
    println("${x}")
}
```

`to_list(it)` drives an adaptor to its end like any iterator. Each adaptor
is an ordinary iterator struct holding its source and the source's `next`,
the same thing you would write by hand ([Iterators](Iterators.md)).

An adaptor may wrap an iterator that owes, such as a file's `Lines`, and is
then an obligation itself ([Linear types](Linear-Types.md)). Drive it where
it lives, then [`detach`](../../lib/std/core/seq.sv#fn-detach) it: that ends
the adaptor and hands back the source, at the position it reached, for you
to read on or close:

```salvo
let first = taking(lines, 10)
for line in first { println(line) }
let rest = detach(first)          // the same `Lines`, ten lines in
close(rest)
```

## Iterating anything: `Next`

`fns` groups ([Implicit parameters](Implicit-Parameters.md#bundling-them-fns))
are how Salvo says what a Rust programmer would say with a trait bound. The
standard library's own example is iteration, and a combinator of your own
is written the way std's are:

```salvo
fns Next<It, T, [E]> {
    fn next(it: Mut It) [E] -> Emitted T | Finished => it: Mut
}

fn map<It, T, U, [E]>(it: Mut It, f: (T) [E] -> U, ?Next<It, T, [E]>) -> Mut List<U> => it: Mut, f {
    let out = Mut List<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}
```

A `for` over a **type parameter** works because `for x in it` is the call
`next(it)`, and the spread brought in a `next` that takes an `It`: inside
`map` it is a parameter, which a call reaches before any declared `next`, so
the loop drives by calling it and takes the element type from its result.
Nothing about `for` is special-cased for std; this is how any combinator of
your own reads. `E` is what the callback and the iterator's `next` perform,
which the caller supplies; a combinator that writes `?Next<It, T>` takes
only an iterator whose `next` performs nothing
([Implicit parameters](Implicit-Parameters.md#effects-in-a-group)).

There is no `Next` *type* and nothing implements it: `map` needs a `next`
for whatever `it` is, and the call site supplies one. The subject is an
**iterator**, so a container is iterated by writing its `iter`, and an
`iter fn` is called.

Inference runs *through* the group: `It` comes from the subject, and `T`,
the element type, from the `next` that takes an `It` (or `It`'s `:
Next<self, T>` clause). That is what lets a lambda be written bare (`(n) ->
n * 2`) with no annotation anywhere.

### Over a source: `Iter`

The other spread, `?Iter<C, It, T>`, is for a combinator over a **source**
rather than an iterator. It brings in the `iter` that mints an `It` and the
`next` that drives it, so `total(xs)` and `total(bag)` both work, and the
body writes `for x in c`, which is `next(iter(c))` through the two
parameters:

```salvo
fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let sum = 0
    for n in c {
        sum = sum + n
    }
    return sum
}
```

`total(bag)` learns `It` from the `iter` that fills the pair, and
`total(bag, iter = other, next = other_next)` from the one written there.
[Sources](Iterators.md#sources-iter) has the details.

### A type of your own

A type of your own joins in with an `iter`: an `iter fn` whose `state` is its
iterator struct, or a fn handing one back. It may also declare `: Iter<self,
It, T>`, naming that struct, to have the pair checked at the type
([Iterators](Iterators.md#sources-iter)):

```salvo
struct Bag<T> : Iter<self, ListIter<T>, T> {
    items: List<T>
}

fn iter<T>(bag: Bag<T>) -> Mut ListIter<T> => bag {
    return iter(bag.items)
}

let sum = reduce(iter(bag), 0, (acc, n) -> acc + n)
let also = total(bag)
```

### Obligations: `fns` groups on a type

A `fns` group can also be an obligation on a **struct**, so the requirement
travels with the type rather than with each function that takes it:
`export struct ArrayIter<T> : Next<self, proj T>` in std, or `struct Bag<T>
: Iter<self, ListIter<T>, T>` above. See
[A struct that requires them](Implicit-Parameters.md#a-struct-that-requires-them).

`Next` and `Iter` are the pattern, not the only instances: `std.core.compare`
declares `Ordered`, `Eq` and `Hashed` the same way
([Comparison, equality and hashing](Comparison-and-Hashing.md)), and
`std.core.list` declares `Ref` for algorithms generic over what a *position*
is ([Mutable handles](Mutable-Handles.md)). A type joins any of them by
declaring one function.
