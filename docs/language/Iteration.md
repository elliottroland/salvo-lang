# Iteration

## Iterating anything: `Yield`

`params` groups are how Salvo says what a Rust programmer would say with a
trait bound. The standard library's own example is iteration:

```
params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

fn map<It, T, U>(it: Mut It, f: (T) -> U, ?Yield<It, T>) -> Mut List<U> => it: Mut, f {
    let out = mut_list_of<U>()
    for x in it {
        add(out, f(x))
    }
    return out
}
```

A `for` over a **type parameter** works because of the spread: the position says
"this call supplies a `next` for `It`", which is as much of a declaration as a
`: Yield<self, T>` clause is, so the loop drives by calling that parameter and
takes the element type from its result. Nothing about `for` is special-cased for
std — this is how any combinator of your own reads.

There is no `Yield` *type* and nothing implements it: `map` needs a `next` for
whatever `it` is, and the call site supplies one. The subject is a **pass** — a
position in a sequence — so a container is iterated by writing its `iter`:

```
let doubled = map(iter(xs), double)         // a list
let letters = filter(iter("hello"), keep)   // a string's characters
let capped = map(iter(counter(3)), double)   // a pass of your own
```

A type of your own joins in by declaring an `iter` that hands back a pass:

```
struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListYield<Int> => bag {
    return iter(bag.items)
}

let total = reduce(iter(bag), 0, (acc, n) -> acc + n)
```

Inference runs *through* the group: `It` comes from the subject, and `T` — the
element type — is read from `It`'s `: Yield<self, T>` clause. That is what lets
the lambda be written bare (`n -> n * 2`) with no annotation anywhere.

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
let out = map_to(mut_list_of<Int>(), iter(xs), double)
let kept = filter_to(map_to(mut_list_of<Int>(), iter(xs), double), iter(ys), is_even)
```

The destination is moved in and returned, which is what makes the nested form
work. If you want to keep hold of one across the call, rebind it:
`let sink = map_to(sink, iter(xs), double)`.

**Iterating a container consumes it**, because the pass holds it: `iter(xs)`
moves `xs` into the pass it builds. A `for` straight over the container does
not — that is data, and the backends walk it in place — so the copy is only
needed where you walk the same container twice through `iter`:
`map(iter(copy(xs)), f)`.

`Yield` is the pattern, not the only instance: `core.compare` declares
`Ordered`, `Eq` and `Hashed` the same way
([Comparison, equality and hashing](Comparison-and-Hashing.md)), and
`core.list` declares `Locate` for algorithms generic over what a *position*
is ([Mutable handles](Mutable-Handles.md)). A type joins any of them by
declaring one function.

## Obligations: `params` groups on a type

A `params` group can also be an obligation on a **struct**, so the
requirement travels with the type rather than with each function that takes
it (`export struct ArrayYield<T> : Yield<self, proj T>`). See
[Implicit parameters](Implicit-Parameters.md).
