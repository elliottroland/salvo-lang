# Iterators: how `for` works

Iteration is ordinary Salvo, not a built-in protocol. An **iterator** is a value
that holds a position in a sequence, and it is advanced by a `next` returning
either an element or the end:

```
provenance qualifier Emitted<T> of T
struct Finished {}

fns Next<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}
```

`Emitted` is a qualifier so the element keeps its own type, which is also what
keeps the end of a sequence of optionals distinguishable: `Emitted None |
Finished` has two arms where `None | None` would have one. `Finished` is a
fieldless struct because it has nothing to qualify.

**`for` is sugar for calling a step until `Finished`.** There is one protocol
and one lowering: `for x in e` is the call `next(e)` when some `next` takes
`e` — `e` is an **iterator** — and `next(iter(e))` otherwise — `e` is a
**source**, from which `iter` mints one. A `for` over *data* — a list, an array,
a string — is the same pair of calls, std's, which the backends spell as their
own native loop.

```
for x in xs { ... }             // a list: walked in place
for c in "hi" { ... }           // its characters
for x in iter(xs) { ... }       // the iterator an `iter` hands back
for x in range(0, 10) { ... }   // one an `iter fn` mints
for x in bag { ... }            // a source: `iter(bag)`, then driven
for x in next(p) { ... }        // a step call, re-invoked each turn
```

An iterator is **advanced where it lives**: a named one keeps the position the
loop reached, so a function can take two elements and leave the rest. Only a
*temporary* subject (`for x in iter(xs)`) is consumed by the loop.

## Writing an iterator struct by hand

Declaring a `next` is what makes iteration writable — `zip`, `merge`, anything
reading two sources at once. The function is all it takes: `for pair in z` is
`next(z)`, so a struct some `next` takes is an iterator.

```
struct Zip<A, B> : Next<self, (A, B)> canbe Mut {
    left: List<A>,
    right: List<B>,
    at: Int
}

fn next<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }
```

The `: Next<self, (A, B)>` clause is optional. It promises the `next`, checked
at the struct, where a misspelled one is caught (declaring the clause without
a matching `next` is an error naming the missing signature), and it says what
the iterator yields to a combinator that infers its element type from it.

### Which `next` a `for` calls

The one the call `next(z)` would: the most specific of the `next`s that take
the iterator, wherever they come from, or an error when none is more specific.
Everything that changes what a call means changes what the loop calls:

- an enclosing implicit `?next` that takes the iterator is the one called, as
  it is for a written `next(z)` ([Implicit parameters](Implicit-Parameters.md));
- a member `next` of an effect in the function's effect list competes with the
  functions ([Effects and handlers](Effects-and-Handlers.md));
- a `next` given another name with `rename fn` is no longer a `next`, for the
  loop as for a call.

A `next` counts only with its result shape, `Emitted T | Finished`: a `next`
returning anything else is an ordinary function. When nothing takes the value,
the loop says so in those terms:

```
`Zip<Int, Str>` is not iterable: `for x in e` is `next(e)` when a `next` takes `e`,
and `next(iter(e))` otherwise, and no `next` or `iter` in scope takes `Zip<Int, Str>` …
```

An iterator struct that owns something declares itself `linear struct` and
names its death in its own file ([Linear types](Linear-Types.md)) — and the
`for` loop is **never** the discharge: a linear iterator is bound with `let`,
driven, and explicitly discharged after the loop (the ordinary all-paths
checking forces the `close` before every exit, `break` and early `return`
included). A **generic** function that owns an iterator which may be linear
(`<It canbe linear>`) takes its discharge as an ordinary consuming callback:

```
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
plain caller passes std's `drop`, the consuming no-op.

### A step under another name

Only `next` is canonical — the one `for p in …` drives. A struct may have
other steps, and a `for` drives one by **naming the call**:

```
fn skip(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }

for pair in skip(z) { ... }     // re-invokes `skip(z)` each turn
```

A `for` over a call whose result is `Emitted T | Finished` re-invokes the call
itself until `Finished`. Its arguments are evaluated on every turn, so each has
to be a place (a variable, a field path) or a literal — bind anything else with
`let` first — and the step must **keep** every argument, since a consumed one
could not be handed over twice. A step's *result* held in a variable is a
union, not a loop.

## Letting the compiler write the struct: `iter fn`

Writing the struct out is the right thing when its fields are not simply the
minter's parameters and a position. An **`iter fn`** is the same hand-written
`next` with the boilerplate removed:

```
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

The declaration is the **minter**: its name and parameters are the function a
caller writes, and its body is the step. What the compiler writes from it is
the iterator struct — every parameter plus the `state` fields — and the `next`
over it; both are ordinary declarations, which is why everything that works on
a hand-written iterator works here.

- **The `state Countdown { ... }` block names the struct**, and the name is
  declared beside the fn, in its module or namespace: it is an ordinary struct
  there, so `Countdown` is a type a program writes (`fn countdown_from_ten() ->
  Mut Countdown`), and it collides with another type of the same name, as two
  structs would. Every `iter fn` has the block; it may be empty.
- **The block's fields are the iterator's own data**, declared exactly as
  a handler's state is: each field writes its type (`at: Int = 0`) or lets it be
  inferred from the initializer (`at = 0`), and each initializer is evaluated
  **once, when the
  iterator is minted**. It may read the parameters and call ordinary functions;
  it may not perform effects, because minting is not where a producer's work
  belongs. The block is declarations only — it is the iterator's shape, not
  code that runs.
- **The parameters are ordinary data, and read-only inside the body.** The
  iterator *borrows* them (a `proj` field, see
  [Deductions](Deductions-and-Ownership.md)), so nothing is copied at the mint,
  a parameter cannot be moved or mutated while an iterator over it lives, and
  writing through one is the same error as writing through any immutable
  value. Every parameter is held, whether the body reads it or not, so the
  struct's shape follows from the declaration's first line. An iterator that
  wants a snapshot writes one: `state Rows { rows: List<Int> = copy(c.rows) }`.
  A Copy scalar is simply stored.
- **Names do not repeat**: a `state` field may not be named like a parameter,
  and the body may not bind a name either already has — it would shadow it.
- **Nothing suspends.** The body *is* the `next`: it returns on every turn, so
  there is no state machine, and effects are ordinary effects on an ordinary
  function.
- An `iter fn` may have any name and any number of parameters (named types;
  no variadics, no implicits yet), and must return `Emitted T | Finished` — or
  `Emitted (proj(xs) T) | Finished` when it emits borrowed elements of its
  parameter `xs`.
- An `iter fn` whose `state` would hold a linear value is refused: the struct
  would have to be linear and have a discharger, and the compiler writes
  neither. Write that iterator struct by hand.

## Sources: `Iter`

A type becomes a **source** — something `for` mints an iterator from — by
having an `iter` that takes it; declaring the `Iter` group as an obligation
states the pair and checks it at the type:

```
fns Iter<C, It, T> => iter with next {
    fn iter(collection: C) -> Mut It
    fn next(iterator: Mut It) -> Emitted T | Finished => iterator: Mut
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
members, so the second member is `Next`'s `next` over it — the two groups are
one protocol stated from two sides. The obligation names it: `BagIter` here is
the struct the `iter fn`'s `state` block declares, and a fn `iter` of your own
returning a struct you wrote satisfies it the same way. `iter with next` says
the pair is filled together: a caller may override the minter, never the step
alone.

A combinator over any source spreads the group, writing the iterator type as
a generic of its own:

```
fn total<C, It>(c: C, ?Iter<C, It, Int>) -> Int => c {
    let sum = 0
    for n in c { sum = sum + n }    // mints with `iter`, drives with `next`
    return sum
}
```

`total(bag)` learns `It` from the `iter` that fills the pair — `BagIter` — and
`total(bag, iter = other, next = other_next)` from the one written there.
Over an iterator rather than a source, the spread is `?Next<It, T>` with the
iterator as a parameter: `fn first<It>(it: Mut It, ?Next<It, Int>)`.

A type declares `: Iter<self, It, T>` **or** `: Next<self, T>`, never both: it is
a source or an iterator struct, and declaring both is refused. A type that has
both a `next` and an `iter` without declaring either is driven by its `next`:
a value some `next` takes is already a position in a sequence.

std's containers are sources through their `iter` (`fn iter<T>(list: List<T>)
-> Mut ListNext<T>`, and the same for arrays, `Str`, `Set`, `Map`); each
answers a named struct with an ordinary `next`, so nothing about container
iteration is special-cased. The backends keep their native loop as a fast path
for a `for` straight over a list, an array or a string — their spelling of
`next(iter(xs))` when those are std's own `iter` and `next`. An enclosing
`?iter` or `?next` that takes the list is called instead.

std's `range` is an `iter fn range(start, end, step)` whose state is `Range`,
and two overloads, `fn range(start: Int, end: Int) -> Mut Range` and `fn
range(end: Int) -> Mut Range`, each delegating to it.

## Iterators over iterators: `mapping`, `taking`, …

A function whose name is a present participle answers an iterator over another one: `mapping(it, f)`, `filtering(it, keep)`, `taking(it, n)`, `taking_while(it, keep)`, `skipping(it, n)` and `skipping_while(it, skip)`. Nothing runs until the result is driven, and each step pulls one element from the source, so they work on a source that never ends. `map` and `filter` are their eager counterparts: they drive the source at once and answer a list.

```
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

`to_list(it)` drives an iterator to its end and answers its elements as a list. Each adaptor is an ordinary iterator struct holding its source and the source's `next`, the same thing you would write by hand.

## There is no iterator *type*

Every iterator struct is its own type, so two producers have unrelated types:
`Range` and `Countdown` both emit `Int` and are still two types. A position that has to hold either of two different producers is
therefore a union — `when` reads it like any other — or a re-wrap: drive the
one you have from an `iter fn` of your own and emit its elements. Nothing is
boxed behind your back, and nothing is dynamically dispatched: an element
costs an inlined call.

```
// Two producers, two types.
let ys = if fast { counter(100) } else { primes(100) }   // Counter | Primes

when ys {
    is Counter { for n in ys { ... } }
    is Primes { for n in ys { ... } }
}
```
