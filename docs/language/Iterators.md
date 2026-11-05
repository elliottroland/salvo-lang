# Iterators: how `for` works

## Overview

Iteration is ordinary Salvo, not a built-in protocol. An **iterator** is a
value that holds a position in a sequence, and it is advanced by a `next`
function that answers either an element or the end. A `for` loop calls that
`next` until it answers the end. Everything else, from std's lists to the
lazy adaptors, is built from this, and a type of your own takes part by
declaring the same functions. (Using loops and the sequence functions is
the previous page, [Iteration](Iteration.md).)

The whole protocol is declared in
[std.core.iterator](../../lib/std/core/iterator.sv):

```salvo
provenance qualifier Emitted<T> for T
struct Finished {}

fns Next<It, T, [E]> {
    fn next(it: Mut It) [E] -> Emitted T | Finished => it: Mut
}
```

An iterator of your own is a struct and a `next` over it:

```salvo
struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) [] -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    c.at = c.at - 1
    return emitted(c.at + 1)
}

fn main() [use] -> None {
    use StdConsole()
    let c = Mut Countdown { at: 3 }
    for n in c {
        println("${n}")         // 3, 2, 1
    }
}
```

**`for` is sugar for calling a step until `Finished`.** There is one
protocol and one lowering: `for x in e` is the call `next(e)` when some
`next` takes `e` (`e` is an **iterator**), and `next(iter(e))` otherwise
(`e` is a **source**, from which `iter` mints one). A `for` over *data*, such
as a list, an array or a string, is the same pair of calls, std's, which the
backends spell as their own native loop.

```salvo
for x in xs { ... }             // a list: walked in place
for c in "hi" { ... }           // its characters
for x in iter(xs) { ... }       // the iterator an `iter` hands back
for x in range(0, 10) { ... }   // one `range` mints
for x in bag { ... }            // a source: `iter(bag)`, then driven
for x in next(p) { ... }        // a step call, re-invoked each turn
```

An iterator is **advanced where it lives**: a named one keeps the position
the loop reached, so a function can take two elements and leave the rest.
Only a *temporary* subject (`for x in iter(xs)`) is consumed by the loop.

The rest of this page goes from the protocol's details to the ways of
writing an iterator: by hand, with an `iter fn`, and as a source.

## The protocol: `Next`, `Emitted` and `Finished`

`E` is what advancing performs, usually nothing: an obligation or a spread
that leaves it out means `[]`.

`Emitted` is a qualifier so the element keeps its own type, which is also
what keeps the end of a sequence of optionals distinguishable: `Emitted None
| Finished` has two arms where `None | None` would have one. `Finished` is a
fieldless struct because it has nothing to qualify. A `next` builds its
answers with [`emitted(v)`](../../lib/std/core/iterator.sv#fn-emitted) and
[`finished()`](../../lib/std/core/iterator.sv#fn-finished).

## Writing an iterator struct by hand

Declaring a `next` is what makes iteration writable: `zip`, `merge`,
anything reading two sources at once. The function is all it takes: `for
pair in z` is `next(z)`, so a struct some `next` takes is an iterator.

```salvo
struct Zip<A, B> : Next<self, (A, B)> canbe Mut {
    left: List<A>,
    right: List<B>,
    at: Int
}

fn next<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }
```

The `: Next<self, (A, B)>` clause is optional. It promises the `next`,
checked at the struct, where a misspelled one is caught (declaring the
clause without a matching `next` is an error naming the missing signature),
and it says what the iterator yields to a combinator that infers its
element type from it.

### A `next` that performs effects

A `next` that performs effects names them in the clause, as the group's
effect argument. std's stream lines
([`Lines`](../../lib/std/stream.sv#struct-lines)) are one:

```salvo
linear struct Lines : Next<self, Str, [Streams]> canbe Mut { ... }

fn next(p: Mut Lines) [Streams] -> Emitted Str | Finished => p: Mut { ... }
```

A `next` performing more than its clause names is an error at the struct.
Every combinator in std drives such an iterator (`map(lines, f)`,
`to_list(mapping(lines, f))`, `count(lines)`), and its caller supplies the
effects, as it does for a callback's. An `iter fn` with an effect list gets
that list in its struct's clause.

### Which `next` a `for` calls

The one the call `next(z)` would: the most specific of the `next`s that take
the iterator, wherever they come from, or an error when none is more
specific ([Overload resolution](Functions.md#overload-resolution)).
Everything that changes what a call means changes what the loop calls:

- an enclosing implicit `?next` that takes the iterator is the one called,
  as it is for a written `next(z)`
  ([Implicit parameters](Implicit-Parameters.md));
- a member `next` of an effect in the function's effect list competes with
  the functions ([Effects and handlers](Effects-and-Handlers.md));
- a `next` given another name with `rename fn` is no longer a `next`, for
  the loop as for a call.

A `next` counts only with its result shape, `Emitted T | Finished`: a
`next` returning anything else is an ordinary function. When nothing takes
the value, the loop says so in those terms:

```text
`Zip<Int, Str>` is not iterable: `for x in e` is `next(e)` when a `next` takes `e`,
and `next(iter(e))` otherwise, and no `next` or `iter` in scope takes `Zip<Int, Str>` …
```

### Linear iterators

An iterator struct that owns something declares itself `linear struct` and
names its death in its own file ([Linear types](Linear-Types.md)). The
`for` loop is **never** the discharge: a linear iterator is bound with
`let`, driven, and explicitly discharged after the loop (the ordinary
all-paths checking forces the `close` before every exit, `break` and early
`return` included). A **generic** function that owns an iterator which may
be linear (`<It canbe linear>`) takes its discharge as an ordinary consuming
callback:

```salvo
fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Next<It, Int>) -> Int => !it =>[end] !x {
    let sum = 0
    for n in it {
        sum = sum + n
    }
    end(it)                // the explicit terminal, on the one path out
    return sum
}
```

A linear caller passes the type's own discharger (`drain(handle, close)`); a
plain caller passes std's [`drop`](../../lib/std/core/seq.sv#fn-drop), the
consuming no-op.

### A step under another name

Only `next` is canonical: it is the one `for p in …` drives. A struct may
have other steps, and a `for` drives one by **naming the call**:

```salvo
fn skip(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }

for pair in skip(z) { ... }     // re-invokes `skip(z)` each turn
```

A `for` over a call whose result is `Emitted T | Finished` re-invokes the
call itself until `Finished`. Its arguments are evaluated on every turn, so
each has to be a place (a variable, a field path) or a literal (bind
anything else with `let` first), and the step must **keep** every argument,
since a consumed one could not be handed over twice. A step's *result* held
in a variable is a union, not a loop.

## Letting the compiler write the struct: `iter fn`

Writing the struct out is the right thing when its fields are not simply the
minter's parameters and a position. An **`iter fn`** is the same
hand-written `next` with the boilerplate removed:

```salvo
iter fn countdown(from: Int) -> Emitted Int | Finished {
    state Countdown {
        at: Int = from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

for n in countdown(3) { ... }   // 3, 2, 1
let p = countdown(3)            // a `Mut Countdown`: hold it and drive it yourself
```

The declaration is the **minter**: its name and parameters are the function
a caller writes, and its body is the step. What the compiler writes from it
is the iterator struct (every parameter plus the `state` fields) and the
`next` over it. Both are ordinary declarations, which is why everything that
works on a hand-written iterator works here.

- **The `state Countdown { ... }` block names the struct**, and the name is
  declared beside the fn, in its module or namespace. It is an ordinary
  struct there, so `Countdown` is a type a program writes (`fn
  countdown_from_ten() -> Mut Countdown`), and it collides with another type
  of the same name, as two structs would. Every `iter fn` has the block; it
  may be empty.
- **The block's fields are the iterator's own data**, declared exactly as a
  handler's state is: each field writes its type (`at: Int = 0`) or lets it
  be inferred from the initializer (`at = 0`), and each initializer is
  evaluated **once, when the iterator is minted**. It may read the
  parameters and call ordinary functions; it may not perform effects,
  because minting is not where a producer's work belongs. The block is
  declarations only: it is the iterator's shape, not code that runs.
- **The parameters are ordinary data, and read-only inside the body.** The
  iterator *borrows* them (a `proj` field, see
  [Deductions and ownership](Deductions-and-Ownership.md)), so nothing is
  copied at the mint, a parameter cannot be moved or mutated while an
  iterator over it lives, and writing through one is the same error as
  writing through any immutable value. Every parameter is held, whether the
  body reads it or not, so the struct's shape follows from the
  declaration's first line. An iterator that wants a snapshot writes one:
  `state Rows { rows: List<Int> = copy(c.rows) }`. A Copy scalar is simply
  stored.
- **Names do not repeat**: a `state` field may not be named like a
  parameter, and the body may not bind a name either already has, since it
  would shadow it.
- **Nothing suspends.** The body *is* the `next`: it returns on every turn,
  so there is no state machine, and effects are ordinary effects on an
  ordinary function.

What an `iter fn` may be:

- It may have any name and any number of parameters (named types; no
  variadics, no implicits yet), and must return `Emitted T | Finished`, or
  `Emitted (proj(xs) T) | Finished` when it emits borrowed elements of its
  parameter `xs`.
- An `iter fn` whose `state` would hold a linear value is refused: the
  struct would have to be linear and have a discharger, and the compiler
  writes neither. Write that iterator struct by hand.

## Sources: `Iter`

A type becomes a **source** (something `for` mints an iterator from) by
having an `iter` that takes it. Declaring the
[`Iter`](../../lib/std/core/iterator.sv#fns-iter) group as an obligation
states the pair and checks it at the type:

```salvo
fns Iter<C, It, T, [E]> => iter with next {
    fn iter(collection: C) -> Mut It
    fn next(iterator: Mut It) [E] -> Emitted T | Finished => iterator: Mut
}

struct Bag : Iter<self, BagIter, Int> {
    items: List<Int>
}

iter fn iter(bag: Bag) -> Emitted Int | Finished {
    state BagIter { at: Int = 0 }
    ...
}

for n in bag { ... }            // mints with `iter(bag)`, drives with its `next`
```

`It` is the iterator struct the implementation mints: one type across both
members, so the second member is `Next`'s `next` over it. The two groups
are one protocol stated from two sides. The obligation names it: `BagIter`
here is the struct the `iter fn`'s `state` block declares, and a fn `iter`
of your own returning a struct you wrote satisfies it the same way. `iter
with next` says the pair is filled together: a caller may override the
minter, never the step alone.

A combinator over any source spreads the group, writing the iterator type
as a generic of its own:

```salvo
fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let sum = 0
    for n in c { sum = sum + n }    // mints with `iter`, drives with `next`
    return sum
}
```

`total(bag)` learns `It` from the `iter` that fills the pair (`BagIter`) and
`total(bag, iter = other, next = other_next)` from the one written there.
Over an iterator rather than a source, the spread is `?Next<It, T>` with the
iterator as a parameter: `fn first<It>(it: Mut It, ?Next<It, Int>)`.
[Iteration](Iteration.md#iterating-anything-next) has more on writing
combinators.

A type may be both a source and an iterator struct (a `next` and an `iter`
take it, declared or not), and then `for x in it` has two answers, so it is
an error, as an operator with two answers is. The loop says which: `for x in
next(it)` drives it, and `for x in iter(it)` mints an iterator from it.

## std's iterators

std's containers are sources through their `iter` (`fn iter<T>(list:
List<T>) -> Mut ListIter<T>`, and the same for arrays, `Str`, `Set`, `Map`);
each answers a named struct with an ordinary `next`, so nothing about
container iteration is special-cased. The backends keep their native loop as
a fast path for a `for` straight over a list, an array or a string: their
spelling of `next(iter(xs))` when those are std's own `iter` and `next`. An
enclosing `?iter` or `?next` that takes the list is called instead.

std's [`range(start, end, step)`](../../lib/std/core/range.sv#fn-range)
mints a [`RangeIter`](../../lib/std/core/range.sv#struct-rangeiter), a
struct with a `next` written out by hand, and two overloads, `fn
range(start: Int, end: Int) -> Mut RangeIter` and `fn range(end: Int) -> Mut
RangeIter`, each delegate to it. std writes every iterator that way, and
names the struct an iterator-minting function answers after it, ending in
`Iter`: `ListIter` for a list's `iter`, `RangeIter`, `ListReversedIter`,
`MapEntriesIter`.

## Iterators over iterators: `mapping`, `taking`, …

std's lazy adaptors (`mapping`, `filtering`, `taking`, `taking_while`,
`skipping`, `skipping_while`) answer an iterator over another one. Each is
an ordinary iterator struct holding its source and the source's `next`, the
same thing you would write by hand, and nothing runs until it is driven.
Using them, and `detach`, which ends an adaptor and hands back its source,
is on the [Iteration](Iteration.md#iterators-over-iterators-mapping-taking-)
page and in [std.core.seq](../../lib/std/core/seq.sv).

## There is no iterator *type*

Every iterator struct is its own type, so two producers have unrelated
types: `RangeIter` and `Countdown` both emit `Int` and are still two types.
A position that has to hold either of two different producers is therefore
a union (`when` reads it like any other) or a re-wrap: drive the one you
have from an `iter fn` of your own and emit its elements. Nothing is boxed
behind your back, and nothing is dynamically dispatched: an element costs an
inlined call.

```salvo
// Two producers, two types.
let ys = if fast { counter(100) } else { primes(100) }   // Counter | Primes

when ys {
    is Counter { for n in ys { ... } }
    is Primes { for n in ys { ... } }
}
```
