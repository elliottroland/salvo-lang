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

Maps work the same way: `at(m, key)` answers `ref(m) Mut V?`, and a handle
chains through a map exactly as through a list:

```
let m: Mut Map<Str, Mut List<Mut Item>> = mut_map_of()
put(m, "a", mut_list_of(Mut Item {tag: "x"}, Mut Item {tag: "y"}))
let it = at(at(m, "a")!, 1)!           // the list at "a", then its element 1
it.tag = "z"
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
that, and the Rust backend renders the handle by handing back the *position*
rather than a reference — see
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

## Proving two handles apart: `NotEq`

`core.list` declares a qualifier for exactly this:

```
export qualifier NotEq(i: Int) of Int with Idx {
    fn qualifies(j: Int, i: Int) -> Bool {
        return j != i
    }
}
```

Test it and the two handles are known to name different elements, so both
live at once and a write through one leaves the other standing:

```
fn duel(a: Mut Entity, d: Mut Entity) -> None => a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}

if j is NotEq(i) {
    let a = at(squad, i)!
    let d = at(squad, j)!
    a.hp = a.hp + 1
    d.hp = d.hp - 1                // `a` is untouched by this
    duel(a, d)                     // …and one call may take both
}
```

The claim is a fact about the indices' *current values*: reassigning either
side takes it away, like any dependent claim [elem-distinct].

For the common shapes std wraps the proof up, so a caller writes neither
the handles nor the claim:

```
update(squad, i, hero -> { hero.hp = hero.hp - 3 })

update2(squad, i, j, (a, d) -> {            // `j: NotEq(i)` is on the signature
    a.hp = a.hp - 1
    d.hp = d.hp - 2
})
```

Both **preserve `Idx`**, so a sequence of them stays total — an in-place
write moves no boundary, so the indices you proved remain valid:

```
update(squad, i, hero -> { hero.hp = hero.hp - 3 })
if j is NotEq(i) {
    update2(squad, i, j, (a, d) -> {
        a.hp = a.hp - 1
        d.hp = d.hp - 2
    })
}
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

On the Rust side a mutable handle is not a `&mut`. It is a **position**: the
container plus an index or a field path, re-materialized at each use.

```
// Salvo                              // Rust
let boss = at(squad, i)!              let __h1: usize = at__loc(&squad, i).expect(…);
boss.hp = boss.hp + 5                 squad[__h1].hp = squad[__h1].hp + 5;
let n = size(squad)                   let n = squad.len();          // legal: no live borrow
boss.hp = boss.hp + n                 squad[__h1].hp = squad[__h1].hp + n;
```

That is what buys the flexibility. A bound `&mut squad[i]` would forbid the
`size(squad)` in the middle — Salvo's rules allow it, because a *read* of
the container cannot invalidate a handle into it, and the position-based
rendering is what lets Rust agree. The same trick carries every other shape
a live `&mut` could not survive:

* **A search that lends what it found** returns a position, so there is no
  borrow to keep alive past the loop — and the caller may go on reading the
  container it searched.
* **A handle across a closure or trait boundary** — `params Ref`'s `at`,
  an effect member that lends — travels as data, so the borrow is created on
  the far side of the boundary instead of crossing it.
* **Handles that share a container** (`ref(c)` parameters) render as the
  container, borrowed once, plus a position per handle —
  `pub fn strike(c: &mut Vec<Entity>, __c1: usize, __c2: usize)`, indexing
  `c[__c1]` and `c[__c2]`. When the two coincide they index the same
  storage, which is what Kotlin does natively. Rust cannot express this
  with references at all: `&mut` has no way to say "these may coincide"
  (E0499). At the call the positions are computed first, so the
  container's `&mut` is the only borrow:
  `strike(&mut squad, __c1, __c2)`, or `shuffle(&mut lib.tracks, …)` for a
  field path.
* **Proven-disjoint handles** (`NotEq`) render as a single `split_at_mut`,
  the pattern a Rust programmer writes by hand — and the one rustc's own
  E0499 suggests.
* **A handle across a disjoint mutation** re-reads its path, which agrees
  with Kotlin precisely because the field it names was untouched. Rust
  splits borrows by field *within* a function, never across a call, so the
  hand translation is E0502.

The cost is honest: a position is re-indexed per use, where a `&mut` is
free, and the bounds check is paid unless a claim has removed it. Reads
keep the zero-cost path — a `get` result is a read-only projection, and a
read-only projection *is* `&T`.
