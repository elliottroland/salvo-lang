# Mutable handles and aliasing

Salvo has no references, but it does have **handles**: a value that names
storage someone else owns, and through which that storage can be changed.
An element of a list, a field of a struct, a value a function lends back —
each can be mutated in place without copying it out and putting it back.

What makes this interesting is that two handles might name the *same*
storage. Rust answers that question with a rule — a mutable borrow excludes
all others — and pays for it by refusing programs that are fine. Salvo
answers it three ways instead, in order of how much you have to say:

1. say nothing, and the compiler keeps one handle at a time;
2. **prove** two handles are different, and use both;
3. **name the container** two handles share, and let the callee cope with
   their being the same element.

This page is those three, plus what each costs on the two backends.

## Mutable elements

Element mutability belongs to the **element type**, not to the container
handle. The two are different permissions:

* `Mut List<T>` — a mutable *container*: it can be reshaped (`add`,
  `remove_at`, `set`, `swap`).
* `List<Mut T>` — a container of mutable *elements*: its shape is fixed, but
  each element can be changed in place.

Two accessors reach an element, and the choice between them says what you
mean to do with it: **`get` reads, `at` handles.**

```
struct Counter canbe Mut {
    n: Int
}

fn bump(c: Mut Counter) -> None => c: Mut {
    c.n = c.n + 1
}

fn main() [use] {
    use StdOutConsole()
    let counters: List<Mut Counter> = list_of(Mut Counter {n: 1}, Mut Counter {n: 2})

    bump(at(counters, 0)!)             // a handle, used where it is minted

    let second = at(counters, 1)!      // …or bound and used across statements
    second.n = second.n + 10
    bump(second)

    println("${get(counters, 0)!.n} ${get(counters, 1)!.n}")   // 2 13
}
```

`at(counters, i)` answers `ref(counters) Mut Counter?`. `ref(counters)` is a
qualifier naming the container the value is a position in, and the `Mut`
on top of it is the permission to write through it; together they make the
value a **handle** [ref-handle]. `get(counters, i)` answers
`proj(counters) Mut Counter?`, a read-only *projection*: its `Mut` is only
part of the element type, so it fits no `Mut` position. Passing a `get`
result to `bump` is an error that names `at` as the remedy, and assigning
through one is refused as a write to a read-only projection. `copy` is
still how you get a value of your own.

Like `get`, `at` has a **total** overload: with an index proven in range
(`i is Idx(counters)`), `at(counters, i)` answers `ref(counters) Mut Counter`
itself — no `None` arm, nothing to `!`.

```
if i is Idx(counters) {
    bump(at(counters, i))
}
```

Maps work the same way: `at(m, key)` answers `ref(m) Mut V?`, and a handle
chains through a map exactly as through a list:

```
let m: Mut Map<Str, Mut List<Mut Item>> = mut_map_of()
put(m, "a", mut_list_of(Mut Item {tag: "x"}, Mut Item {tag: "y"}))
let it = at(at(m, "a")!, 1)!           // the list at "a", then its element 1
it.tag = "z"
```

A handle, or a `get` result, can be **read** wherever a value can, with no
`copy` — the position only reads it. That holds for optionals too: a
parameter `s: Str?` that the function only reads takes `get(xs, 0)` or an
optional handle as it is, and its `None` stays `None` [proj-opt-slot]:

```
fn maybe_print(c: Counter?) [Console] {
    if c is Counter {
        println("${c.n}")
    }
}

maybe_print(get(counters, 0))      // a read-only projection
maybe_print(at(counters, 5))       // an optional handle; prints nothing
let mine = copy(get(counters, 1))  // `Counter?`, a value of your own
```

Handles that are only **read** impose nothing, so any number coexist:

```
let a = at(counters, 0)!
let b = at(counters, 1)!
println("${a.n} ${b.n}")          // two live handles, both read: fine
```

A **write** through one is a mutation of the container, so anything else
derived from it is invalidated — the acting handle itself survives, since
its storage did not move.

## Lending a handle from your own function

A function can hand a handle back. It says so on its return type, naming
the container the result is a handle into:

```
struct Entity canbe Mut {
    hp: Int
}

// Searches, and lends back what it found.
fn wounded(es: List<Mut Entity>) -> (ref(es) Mut Entity)? {
    for e in es {
        if e.hp < 10 {
            return e
        }
    }
    return None
}

fn heal(e: Mut Entity) -> None => e: Mut {
    e.hp = e.hp + 10
}

fn main() [use] {
    use StdOutConsole()
    let squad: List<Mut Entity> = list_of(Mut Entity {hp: 50}, Mut Entity {hp: 3})
    heal(wounded(squad)!)                  // mutate through the lent handle
    println("${get(squad, 0)!.hp} ${get(squad, 1)!.hp}")   // 50 13
}
```

The body returns `e`, which the loop produced as a projection of `es`. The
declared `ref(es)` return is what turns it into a handle: a function
declared `-> ref(c) Mut T` that returns a projection of `c` is **minting**
one. That is all std's own `at` is — `return get(list, index)` under a
`-> ref(list) Mut T?` signature.

The handle may be used where it is minted, bound across statements, or —
as here — **found by a loop**, where the position is never written down.

That shape is worth pausing on. Rust can write the *function* — with
`iter_mut` and NLL, a search returning `Option<&mut Entity>` compiles — but
it cannot then use the result the way the program above does: a live `&mut`
into `squad` forbids even a **read** of `squad`, so `size(squad)` or a
second lookup between two writes through the handle is E0502. Salvo accepts
that, and the Rust backend renders the handle by handing back the *path* to the
element rather than a reference — see
[What the backends do](#what-the-backends-do).

A generic function can lend too, if the caller supplies the accessor. That
is what `params Ref` is for [col-locate] — the bundle pattern, applied to
positions:

```
// core.list
params Ref<C, L, T> {
    fn at(c: C, l: L) -> ref(c) Mut T?
}

// Generic over what a *position* is; the caller fills `at`.
fn heal_at<L>(c: List<Mut Entity>, l: L, ?Ref<List<Mut Entity>, L, Entity>) -> None {
    heal(at(c, l)!)
}

heal_at(squad, 1)                  // `at` resolves to the list accessor
```

The container may be generic too. A handle into a `c: C` is still a
position in `c`, whatever `C` turns out to be, so a function generic over
the container takes handles into it and passes them on, and a function
that mints through `?Ref` may itself be a mint — **delegating** to the
accessor the caller supplied:

```
struct Player canbe Mut { goals: Int }

fn bump<C>(c: C, a: ref(c) Mut Player) -> None {
    a.goals = a.goals + 1
}

fn borrow_via<C, L>(c: C, l: L, ?Ref<C, L, Player>) [] -> ref(c) Mut Player? => c, l {
    return at(c, l)
}

bump(squad, at(squad, 0)!)         // C is List<Mut Player>
bump(squad, borrow_via(squad, 1)!)  // the caller's `at` fills `?Ref`
```

Effect members can lend the same way: a member declared `-> ref(c) Mut T`
is a mint each handler implements, and a handle bound from it behaves like
any other. A `platform handler` cannot implement such a member (host code
has no way to answer a path into the caller's container), and an `actor
effect`'s members cannot return one.

## One handle at a time, by default

Say nothing and the compiler keeps a single *usable* handle per container
once one of them writes. Two handles from computed indices may name the
same element, so a write through one invalidates the other:

```
let a = at(squad, i)!
let b = at(squad, j)!
a.hp = a.hp + 1
b.hp = b.hp + 1                    // error: `squad` was mutated (through `a`) after `b` was bound
```

The same rule reaches call arguments: two handles into one container in one
call is refused at the second argument, and the diagnostic names the two
ways forward — prove them apart, or mutate through one at a time.

```
duel(at(squad, i)!, at(squad, j)!)     // error at the second argument
```

## Proving two handles apart: `NotSame`

`core.ref` declares a qualifier for exactly this. It is a claim about the
**handles**, not about how they were found: `d is NotSame(a)` says that `d`
names a different element from `a` [ref-notsame].

```
export qualifier NotSame<T>(a: T) of T {
    fn qualifies(b: T, a: T) -> Bool {
        return !same(a, b)
    }
}
```

`same` is an identity comparison, which Salvo has no syntax for, so each
backend answers it: Kotlin by reference (`===`), Rust by address. It is
exact for a handle, since `Mut` never applies to a scalar. Two handles of
the same element test false.

Test it and both handles live at once, a write through one leaves the
other standing, and one call may take both:

```
fn duel(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}

let a = at(squad, i)!
let d = at(squad, j)!
if d is NotSame(a) {
    a.hp = a.hp + 1
    d.hp = d.hp - 1                // `a` is untouched by this
    duel(a, d)                     // …and one call may take both
}
```

The test runs at run time, and the claim it leaves is about the two
handles' identity [elem-distinct]:

* a write **through** either handle keeps it, since the element changed but
  is still the same element;
* **rebinding** either handle strips it, like any dependent claim;
* a handle minted inside a statement (`duel(at(squad, i)!, at(squad, j)!)`)
  has no name to prove anything about, so **bind the handles first**.

Because the claim is about handles, it holds whatever the container's `at`
does: an index into a list, a key into a map, or a search through your own
struct.

For the one-handle shape std wraps the handle up, so a caller writes
neither the handle nor the claim:

```
update(squad, i, hero -> { hero.hp = hero.hp - 3 })
```

`update` takes an `i: Idx(squad)` and **preserves `Idx`**, so a sequence of
them stays total. An in-place write moves no boundary, so the indices you
proved remain valid:

```
update(squad, i, hero -> { hero.hp = hero.hp - 3 })
update(squad, j, hero -> { hero.hp = hero.hp + 1 })
println("${get(squad, i).hp}")     // still the *total* read: `Idx` survived
```

An ordinary mutating call has no such promise: after one, an index claim is
gone and the total read stops resolving until you test again. That is the
conservative default, and `preserve Idx` on a signature is how a function
opts out of it.

## Handles that share a container: `ref(c)` parameters

Sometimes the aliasing is the point. A function that means to accept two
handles which might be one element says so by typing them as handles into
a container it also takes [ref-anchor]:

```
fn strike(c: List<Mut Entity>, a: ref(c) Mut Entity, d: ref(c) Mut Entity) -> None {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}
```

`a` and `d` are both positions in `c`, so they may be the same position,
and the caller needs no proof at all — self-strike included:

```
strike(squad, at(squad, i)!, at(squad, j)!)     // i and j unknown: fine
strike(squad, at(squad, i)!, at(squad, i)!)     // the same entity, twice: also fine
```

The rules are few:

* `ref(c)` must name **another parameter** of the same function; anything
  else is an error at the declaration.
* Any number of parameters may name one container. Each one says
  `ref(c)` once — there is no relation to write per pair.
* The caller must pass handles **minted from that very container**. A
  handle into a different list, an owned value, or a read-only `get`
  result is refused, and the diagnostic names `at`.

The container may be a **field path** at the call, which is how handles
travel *beside the container they came from*:

```
struct Library canbe Mut {
    name: Str,
    tracks: List<Mut Track>
}

fn shuffle(tracks: List<Mut Track>, track: ref(tracks) Mut Track, other: ref(tracks) Mut Track) -> None {
    track.plays = track.plays + 1
    other.plays = other.plays - 1
}

shuffle(lib.tracks, at(lib.tracks, i)!, at(lib.tracks, j)!)
```

Whether two handles may alias is **written, never inferred**: it changes
what a caller may pass, so it stays visible in the signature.

## Which field a call touches

A handle into a struct raises the same question one level up: if a function
mutates a struct, what happens to a value read out of one of its fields?
Say nothing and the answer is conservative — everything derived from the
struct falls. A clause entry can be finer:

```
struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn damage(e: Mut Entity, n: Int) -> None => e.hp: Mut, n {
    e.hp = e.hp - n
}

let rings = e.rings
damage(e, 5)
println("${size(rings)}")          // fine: `damage` touched `.hp`, not `.rings`
```

Three readings, and the difference is *storage identity*:

| Entry | Means | A handle to that field |
|---|---|---|
| `=> e: Mut` | mutated somewhere in `e` | falls |
| `=> e.rings: Mut` | the field's *contents* change | **survives** |
| `=> !e.rings` | the field is **replaced** | falls |

So a call that only adds to `e.rings` leaves a handle to that list usable —
it is still the same list — while one that swaps the list for another does
not. A value read *out* of the contents (an element) falls either way,
because an element may be gone.

Written entries are checked against the body: a function claiming
`=> e.hp: Mut` that also assigns `e.rings` is an error naming the widening
remedy. Where a function writes no clause, the field set is **inferred**
from its body, so ordinary code gets the precision without saying anything
— conservatively, so a body that replaces a field, or hands the whole value
to another mutator, narrows nothing.

This is also what makes a **qualifier about a field** useful:

```
if h.tags is NonEmpty {
    bump(h)                        // touches h.n only
    println("${first(h.tags)}")    // the claim still holds here
}
```

## What the backends do

Kotlin needs none of this: objects are references, aliasing is native, and
a handle is the element itself. Everything above is about keeping Rust
honest while still accepting the programs — and the two backends print the
same thing, which is the property all of it exists to protect.

On the Rust side a mutable handle is not a `&mut`. It is a **storage
path** from its container: the field steps, list indices and map slots
that lead from the container to the element [rs-path]. The path is
computed once, at the mint, by running the mint's body, and every use walks
it again.

```
// Salvo                              // Rust
let boss = at(squad, i)!              let __h1: usize = at__List_Int(&squad, i).expect(…);
boss.hp = boss.hp + 5                 squad[__h1].hp = squad[__h1].hp + 5;
let n = size(squad)                   let n = squad.len();          // legal: no live borrow
boss.hp = boss.hp + n                 squad[__h1].hp = squad[__h1].hp + n;
```

A mint renders as its path function under its own name, and nothing else:
std's `at` overloads become `at__List_Int` and `at__List_IdxInt`, which
answer a position, and there is no `&mut`-returning version beside them.
Since the path is read off the types and the mint's body, it exists for any
container's `at`, and what you can do with a handle never depends on how
`at` is written. A container needs no `at` of its own for the structs a
path passes through:

```
struct A canbe Mut { b: B }
struct B canbe Mut { c: List<Mut Str> }

fn at(a: A, index: Int) -> ref(a) Mut Str? {
    return a.b.c.get(index)           // answers `Option<usize>` on Rust
}

let s = at(a, 1)!                     // used as `a.b.c[__h1]`
```

When there is one way from container to element, the path is just its
positions: `squad[__h1]` above, `a.b.c[__h1]` here. When an accessor can
answer from different fields, the path is a generated enum with one variant
per way, and each use walks it with a `match`. A league whose `at` reaches
a player either through a team in a map or on the bench:

```
struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

fn at(l: League, team: Str, n: Int) [] -> ref(l) Mut Player? => l, team, n {
    if n < 0 {
        return at(l.bench, -n - 1)          // .bench[i]
    }
    let t = at(l.teams, team)               // .teams{slot}
    if t is None {
        return None
    }
    return at(t.roster, n)                  // .teams{slot}.roster[i]
}

let star = at(league, "red", 1)!
let sub = at(league, "", -1)!
if sub is NotSame(star) {
    score(star, sub)
}
```

On Rust `at` answers `Option<__Path_League__Player>`, an enum of
`V0(usize, usize)` (a map slot and a roster index) and `V1(usize)` (a bench
index), and `star.goals` reads
`match __h1 { V0(s, i) => &league.teams[s].roster[i].goals, V1(i) => &league.bench[i].goals }`.
The program prints the same on both backends.

That is what buys the flexibility. A bound `&mut squad[i]` would forbid the
`size(squad)` in the middle. Salvo's rules allow it, because a *read* of
the container cannot invalidate a handle into it, and the path-based
rendering is what lets Rust agree. The same trick carries every other shape
a live `&mut` could not survive:

* **A search that lends what it found** returns a path, so there is no
  borrow to keep alive past the loop, and the caller may go on reading the
  container it searched.
* **A handle across a closure or trait boundary** (`params Ref`'s `at`, an
  effect member that lends) travels as data, so the borrow is created on
  the far side of the boundary instead of crossing it.
* **Handles that share a container** (`ref(c)` parameters) render as the
  container, borrowed once, plus a path per handle:
  `pub fn strike(c: &mut Vec<Entity>, __c1: usize, __c2: usize)`, indexing
  `c[__c1]` and `c[__c2]`. When the two coincide they index the same
  storage, which is what Kotlin does natively. Rust cannot express this
  with references at all: `&mut` has no way to say "these may coincide"
  (E0499). At the call the paths are computed first, so the container's
  `&mut` is the only borrow: `strike(&mut squad, __c1, __c2)`, or
  `shuffle(&mut lib.tracks, …)` for a field path.
* **Proven-distinct handles** (`NotSame`) split where their paths diverge:
  `salvo_pair_mut` (a `split_at_mut`) at a list position, `Map::pair_mut`
  at a map slot, and plain disjoint borrows at different fields. For two
  handles into one list that is the pattern a Rust programmer writes by
  hand, and the one rustc's own E0499 suggests:
  `if __h5 != __h6 { let (a, d) = salvo_pair_mut(&mut squad[..], __h5, __h6)…; duel(a, d) } else { panic!(…) }`.
  The `NotSame` test itself compares the two materialized references by
  address.
* **A handle across a disjoint mutation** walks its path again, which
  agrees with Kotlin precisely because the field it names was untouched.
  Rust splits borrows by field *within* a function, never across a call,
  so the hand translation is E0502.

A container that is a type parameter (`ref(c)` with `c: C`) has no path
yet, and the Rust backend refuses it with an error naming it.

The cost is honest: a path is walked per use, where a `&mut` is free, and
the bounds check is paid unless a claim has removed it. Reads keep the
zero-cost path: a `get` result is a read-only projection, and a read-only
projection *is* `&T`.
