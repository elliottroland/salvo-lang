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

A `params` group can also be stated as an **obligation** on a struct, with a
`:` clause between the generics and `canbe`:

```
params Step<It, T> {
    fn advance(it: Mut It) -> Emitted T | Finished => it: Mut
}

struct Countdown : Step<self, Int> canbe Mut {
    at: Int
}

fn advance(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 { return finished() }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}
```

Where `?Step<It, Int>` in a signature asks the *call site* to supply the
group's members, `: Step<self, Int>` on a declaration promises that the
members exist for this type — and the promise is checked **at the struct**:
declaring it without a visible `advance` matching
`fn advance(it: Mut Countdown) -> Emitted Int | Finished` is an error naming
that signature, where a misspelled member would otherwise surface as some
puzzling failure at a distant use site.

`self` is the shorthand for "the type being declared", written where a type
argument goes. The checker substitutes `Countdown` for it and looks for a
matching overload — by parameter types and return type, positionally, up to a
consistent renaming of type variables, so a generic struct satisfies a group
through its own type parameters. The member's parameter *names* belong to the
group; an implementation picks its own.

Note what `self` being an *argument* buys: the group itself is ordinary, so
**one declaration serves both uses**. The same `Step` spreads as
`?Step<It, Int>`, which is how a generic function reaches the member of a type
it does not know — a magic `Self` inside the group would have ruled that out,
since nothing would bind it in a signature. Outside an obligation `self` is
simply an unknown type.

Two things keep this a where-clause rather than a trait:

- **No value may have a group as its type.** `let p: Step<Countdown, Int>` is an error
  wherever a type can be written — parameter, return, field, `let`
  annotation, type argument, union arm. There is no erasure and no interface
  value; a group constrains a *named* type, and everything resolves
  statically.
- **A group is satisfied by functions, not by membership.** The obligation
  adds no scope and no dispatch: `advance` is an ordinary function, found and
  overloaded like any other. The clause only moves the check to the
  declaration.

One group is **designated**: the compiler knows `Yield<self, T>` by name and
reads the clause itself — it is what makes a type a pass, so `for` resolves
its `next` from the declaration ([Passes](Passes.md)).
It is an ordinary `params` group otherwise — declared in std, spreadable
with `?`. (Linearity used to be the second designated group; it is a
declaration modifier now, `linear struct` — see [Linear types](Linear-Types.md).)
