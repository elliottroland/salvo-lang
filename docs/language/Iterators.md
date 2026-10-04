# Iterators: how `for` works

Iteration is ordinary Salvo, not a built-in protocol. An **iterator** is a value
that holds a position in a sequence, and it is advanced by a `next` returning
either an element or the end:

```
provenance qualifier Emitted<T> of T
struct Finished {}

params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}
```

`Emitted` is a qualifier so the element keeps its own type, which is also what
keeps the end of a sequence of optionals distinguishable: `Emitted None |
Finished` has two arms where `None | None` would have one. `Finished` is a
fieldless struct because it has nothing to qualify.

**`for` is sugar for calling a step until `Finished`.** There is one protocol
and one lowering. A `for` over *data* — a list, an array, a string — is walked
natively by the backend; anything else is an iterator, driven by its `next`, or
a source, from which an `iter` mints one.

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
reading two sources at once. The method alone does not make a struct an
iterator: the tie is stated as an obligation, `: Yield<self, T>`, and checked
at the struct (declaring it without a matching `next` is an error naming the
missing signature). `for` reads the declaration — it does not scan overloads
for a `next` and guess:

```
struct Zip<A, B> : Yield<self, (A, B)> canbe Mut {
    left: List<A>,
    right: List<B>,
    at: Int
}

fn next<A, B>(z: Mut Zip<A, B>) -> Emitted (A, B) | Finished => z: Mut { ... }
```

Leave the clause off `Zip` and the `for` reports it:

```
`Zip<(A, B)>` is not iterable … (`Zip` has a matching `next` — declare
`: Yield<self, (A, B)>` on it to make it an iterator struct)
```

An iterator struct that owns something declares itself `linear struct` and
names its death in its own file ([Linear types](Linear-Types.md)) — and the
`for` loop is **never** the discharge: a linear iterator is bound with `let`,
driven, and explicitly discharged after the loop (the ordinary all-paths
checking forces the `close` before every exit, `break` and early `return`
included). A **generic** function that owns an iterator which may be linear
(`<It canbe linear>`) takes its discharge as an ordinary consuming callback:

```
fn drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>) -> Int => !it =>[end] !x {
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

Writing the struct out is the right thing when it needs a *name* — to store it,
to hand it to a function, to zip two of them. Most iterators need none of that:
they are only ever driven by a `for`. An **`iter fn`** is the same hand-written
`next` with the boilerplate removed:

```
iter fn countdown(from: Int) -> Emitted Int | Finished {
    state {
        at: Int = from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

for n in countdown(3) { ... }   // 3, 2, 1
let p = countdown(3)            // or hold the iterator and drive it yourself
```

The declaration is the **minter**: its name and parameters are the function a
caller writes, and its body is the step. What the compiler writes from it is
the iterator struct — the parameters it needs plus the `state` fields — and
the `next` over it; both are ordinary declarations, which is why everything
that works on a hand-written iterator works here.

- **The `state { ... }` block is the iterator's own data**, declared exactly as
  a struct's fields are, and each initializer is evaluated **once, when the
  iterator is minted**. It may read the parameters and call ordinary functions;
  it may not perform effects, because minting is not where a producer's work
  belongs. The block is declarations only — it is the iterator's shape, not
  code that runs.
- **The parameters are ordinary data, and read-only inside the body.** The
  iterator *borrows* them (a `proj` field, see
  [Deductions](Deductions-and-Ownership.md)), so nothing is copied at the mint,
  a parameter cannot be moved or mutated while an iterator over it lives, and
  writing through one is the same error as writing through any immutable
  value. An iterator that wants a snapshot writes one:
  `state { rows: List<Int> = copy(c.rows) }`. A Copy scalar is simply stored.
- **The struct has no name.** Its type is spelled `iter T` — "an iterator of
  `T`" — wherever a spelling is needed (below). An iterator you must *name* is
  the written-out form above.
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
declaring the `Iter` group as an obligation:

```
params Iter<C, T> => iter with next {
    fn iter(collection: C) -> iter T
    fn next(iterator: iter T) -> Emitted T | Finished => iterator: Mut
}

struct Bag : Iter<self, Int> {
    items: List<Int>
}

iter fn iter(bag: Bag) -> Emitted Int | Finished { ... }

for n in bag { ... }            // mints with `iter(bag)`, drives with its `next`
```

`iter T` in the group is the placeholder for the iterator struct the
implementation mints: one type across both members, so the second member is
`Yield`'s `next` over it — the two groups are one protocol stated from two
sides. The obligation is satisfied by an `iter fn iter` over the type (the
compiler writes the struct and both members) or by a fn `iter` of your own
returning a named iterator struct. `iter with next` says the pair is filled
together: a caller may override the minter, never the step alone.

A type declares `: Iter<self, T>` **or** `: Yield<self, T>`, never both: it is
a source or an iterator struct, and `for x in s` would otherwise have two
answers.

std's containers are sources through their `iter` (`fn iter<T>(list: List<T>)
-> Mut ListYield<T>`, and the same for arrays, `Str`, `Set`, `Map`); each
answers a named struct with an ordinary `next`, so nothing about container
iteration is special-cased. The backends keep their native loop as a fast path
for a `for` straight over a list, an array or a string.

## `iter T`: the name of an anonymous iterator

An `iter fn`'s struct has no name a program can write, so the language gives it
one spelling: **`iter T`**, "a mutable type declaring `: Yield<self, T>`". It
means one thing everywhere, read in whichever way the position allows — a
*placeholder* in a group, a *hidden generic* in a signature, a *pattern* where
there is a value to fill it from, and the *concrete anonymous struct* an `iter
fn` mints. `Mut` is implied (an iterator is only ever advanced in place), so
`Mut iter T` is refused.

Every use lowers to something you could write explicitly. The table is the
specification; the sugar is a shorter spelling of the right-hand column.

| `iter T` written as | what it stands for |
|---|---|
| **Group member** — `params Iter<C, T> { fn iter(c: C) -> iter T; fn next(it: iter T) -> … }` | the iterator struct as a group parameter: `params Iter<C, It, T> { fn iter(c: C) -> Mut It; fn next(it: Mut It) -> … }`. One `iter T` per group: both members name the same struct. |
| **Obligation** — `struct Bag : Iter<self, Int>` | `struct Bag : Iter<self, BagIter, Int>` plus `struct BagIter : Yield<self, Int> canbe Mut`, its `next`, and `fn iter(bag: Bag) -> Mut BagIter` — or one `iter fn iter(bag: Bag)`. |
| **Spread** — `fn total<C>(c: C, ?Iter<C, Int>)` | `fn total<C, It>(c: C, ?iter: (c: C) -> Mut It, ?Yield<It, Int>)`: a hidden generic for the struct, and the pair as implicits. Inside, `for x in c` mints with `iter` and drives with `next`, and `iter(c)` is a `Mut It` a `?Yield` combinator accepts. |
| **Parameter** — `fn first(it: iter Int)` | `fn first<It>(it: Mut It, ?Yield<It, Int>)`: "any iterator struct emitting `Int`", a fresh hidden generic **per occurrence** — `chain(a: iter T, b: iter T)` takes two different structs. Two parameters that must be the *same* struct write `<It>` themselves. |
| **Type of an `iter fn` call** — `let p = countdown(3)` | `p` holds the concrete anonymous struct, which prints as `iter Int`; it has the obligation and the `next`, so `for x in p`, `next(p)` and `map(p, f)` all work. |
| **`let` annotation** — `let p: iter Int = countdown(3)` | a pattern: asserts "an iterator of `Int`", and `p` keeps the value's own concrete type. A named iterator struct fits it too; nothing is widened. |
| **`state` field** — `state { inner: iter Int = countdown(3) }` | a pattern, filled from the initializer — a concrete source names its struct (`inner: Mut ListYield<Int>`), a generic one uses the spread's `<It>`. |
| **Return type of a fn with a body** — `fn evens(bag: Bag) -> iter Int { return iter(bag) }` | a pattern filled from the body's `return`s: the fn returns whichever concrete struct its body mints, and callers see that type. Every path must mint the *same* struct — two anonymous ones are two types — so `if c { range(3) } else { evens(bag) }` is an error naming both; fold the case into one `iter fn`, or name the struct. |
| **Linear** | no sugar. A struct owning a resource is written out — `linear struct Lines : Yield<self, Str> canbe Mut { s: InStream, … }` with its `next`, minter and `close` — and generic code that may receive one takes the discharger as a callback (`drain<It canbe linear>(it: Mut It, end: (x: It) -> None, ?Yield<It, Int>)`). Linear *elements* are fine: `iter T` with `T canbe linear` needs no discharger, since each element leaves on its turn. |

Refused, with the replacement named:

| position | why | write instead |
|---|---|---|
| return type of a bodiless fn (`intrinsic`) | no body to fill the pattern from | the struct's name |
| an effect member's parameter or return | implemented per handler, one interface type needed | the struct's name |
| a function *type* (a lambda, a fn-typed parameter) | the value's iterator struct is its own to choose | `<It>` on the enclosing declaration |
| a struct or handler field | storing "some iterator struct" is boxing | the struct's name, or `<It>` on the struct |
| `Mut iter T` | `Mut` is implied | `iter T` |

std's `range` is the pattern in one file: an `iter fn range(start, end, step)`
and two overloads, `fn range(start: Int, end: Int) -> iter Int` and `fn
range(end: Int) -> iter Int`, each delegating to it.

## Iterators over iterators: `mapping`, `taking`, …

A function whose name is a present participle answers an iterator over another one: `mapping(it, f)`, `filtering(it, keep)`, `taking(it, n)`, `taking_while(it, keep)`, `skipping(it, n)` and `skipping_while(it, skip)`. Nothing runs until the result is driven, and each step pulls one element from the source, so they work on a source that never ends. `map` and `filter` are their eager counterparts: they drive the source at once and answer a list.

```
iter fn naturals(n: Int) -> Emitted Int | Finished {
    state {
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

Every iterator struct is its own type, so two producers have unrelated types,
and `iter Int` from `range` and from `countdown` are two types that print
alike. A position that has to hold either of two different producers is
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
