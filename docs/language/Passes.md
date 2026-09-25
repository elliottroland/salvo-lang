# Passes: how `for` works

Iteration is ordinary Salvo, not a built-in protocol. A **pass** is a value
that holds a position in a sequence, and a pass is advanced by a `next`
returning either an element or the end:

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

**`for` is sugar for calling `next` until `Finished`.** There is one protocol
and one lowering. A `for` over *data* — a list, an array, a string — is walked
natively by the backend; anything else is a pass, driven by its `next`.

```
for x in xs { ... }             // a list: walked in place
for c in "hi" { ... }           // its characters
for x in iter(xs) { ... }       // the pass an `iter` hands back
for pair in zip(names, ages) { ... }   // a pass of your own
```

A pass is **consumed by driving it**: the position it holds has moved, so a
second `for` over the same value is the ordinary consumed-use error. Data is
not — a `for` over a list leaves the list alone.

### Writing a pass by hand

Declaring a `next` is what makes iteration writable
— `zip`, `merge`, anything reading two sources at once. The method alone does
not make a struct a pass: the tie is stated as an obligation, `: Yield<self,
T>`, and checked at the struct (declaring it without a matching `next` is an
error naming the missing signature). `for` reads the declaration — it does not
scan overloads for a `next` and guess:

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
`: Yield<self, (A, B)>` on it to make it a pass)
```

A pass that owns something declares itself `linear struct` and names its
death in its own file ([Linear types](Linear-Types.md)) — and the `for` loop is
**never** the discharge. Any *named* pass is advanced **where it lives**: the
position the loop reaches is what the owner sees next, so a function can take
two elements and leave the rest — and a linear pass is bound with `let`,
driven, and explicitly discharged after the loop (the ordinary all-paths
checking forces the `close` before every exit, `break` and early `return`
included).

```
fn take(p: Mut Slice<Int>, count: Int) -> Int => p: Mut { ... }   // keeps it

let p = slice(list_of(1, 2, 3, 4))
let first = take(p, 2)     // 1 + 2
let rest = take(p, 9)      // 3 + 4 — the same pass, carried on
```

Only a *temporary* subject (`for x in iter(xs)`) is consumed by the loop — and
a linear temporary is refused there, since the loop discharges nothing: bind
it first. A **generic** function that owns a pass which may be linear
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
plain caller passes std's `drop`, the consuming no-op. Nothing is implicit:
leave the `end(it)` off and the obligation reports the leak at the exit.

### Letting the compiler write the struct: `iter fn`

Writing the pass out is the right thing when it needs a *name* — to store it, to
hand it to a function, to zip two of them. Most passes need none of that: they
are only ever driven by a `for`. A **`iter fn`** is the same hand-written `next`
with the boilerplate removed:

```
struct Countdown {
    from: Int
}

iter fn next(c: Countdown) -> Emitted Int | Finished {
    state {
        at: Int = c.from
    }
    if at <= 0 {
        return finished()
    }
    at = at - 1
    return emitted(at + 1)
}

let c = Countdown {from: 3}
for n in c { ... }        // 3, 2, 1
for n in c { ... }        // again: driving copied the subject
let p = iter(c)           // or hold the pass and drive it yourself
```

One declaration makes `Countdown` iterable. What the compiler writes from it is
the pass struct — the subject plus the `state` fields — and the `iter` that mints
one; both are ordinary declarations, which is why everything that works on a
hand-written pass works here.

- **The `state { ... }` block is the pass's own data**, declared exactly as a
  struct's fields are, and each initializer is evaluated **once, when the pass is
  minted**. It may read the subject and call ordinary functions; it may not
  perform effects, because minting is not where a producer's work belongs. The
  block is declarations only — it is the pass's shape, not code that runs.
- **The subject is ordinary data, and read-only inside the body.** The pass
  *borrows* it (a `proj` field, see "Projections"), so nothing is copied at the
  mint, the subject cannot be moved or mutated while a pass over it lives, and
  writing through it is the same error as writing through any immutable value.
  A pass that wants a snapshot writes one: `state { rows: List<Int> = copy(c.rows) }`.
- **The pass has no name.** No variable may be annotated with it, no field may
  store it — a pass you need to name is the written-out form above. `iter(c)`
  still hands one to you, and inference carries it, so holding one in a local
  and driving it in stages works.
- **Nothing suspends.** The body *is* the `next`: it returns on every turn, so
  there is no state machine, and effects are ordinary effects on an ordinary
  function.
- An `iter fn` must be called `next`, must take exactly one parameter, and must
  return `Emitted T | Finished` — or `Emitted (proj(c) T) | Finished` when
  it emits borrowed elements of its subject `c` — it is the obligation's member,
  so the subject needs no `: Yield<self, T>` clause of its own.

### What a container hands you

A container is iterated through its `iter`, which builds a fresh pass:

```
fn iter<T>(list: List<T>) [] -> Mut ListYield<T>
fn iter<T>(array: T[]) [] -> Mut ArrayYield<T>
fn iter(str: Str) [] -> Mut StrYield
```

Each is an ordinary struct with an ordinary `next` — `ListYield` *borrows* the
list (`items: proj List<T>`) and keeps an index — so nothing about container
iteration is special-cased in the language, and nothing is copied to walk a
container. The backends keep their native loop as a fast path for a `for`
straight over a list, an array or a string, which is why *that* form neither
allocates a pass nor consumes the container.

A type of your own becomes iterable by declaring any one of three things: a
`iter fn next` (the compiler writes the pass *and* the `iter`), an `iter` that
hands back a pass (so combinators reach it), or its own `: Yield<self, T>` plus
`next` (so it *is* a pass). `for x in bag` works as soon as `iter(bag)` does —
the loop calls it once and drives what it answers.

### There is no iterator *type*

Every pass is its own struct, so two producers have unrelated types. A position that has to hold either of two
different producers is therefore a union — `when` reads it like any other — or
a re-wrap: drive the one you have from an `iter fn` of your own and emit its
elements. Nothing is boxed behind your back, and nothing is dynamically
dispatched: an element costs an inlined call.

```
// Two producers, two types.
let ys = if fast { counter(100) } else { primes(100) }   // Counter | Primes

when ys {
    is Counter { for n in ys { ... } }
    is Primes { for n in ys { ... } }
}
```
