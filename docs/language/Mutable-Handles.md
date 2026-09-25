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
3. **declare** that two may be the same, and let the callee cope.

This page is those three, plus what each costs on the two backends.

## Mutable elements

Element mutability belongs to the **element type**, not to the container
handle. The two are different permissions:

* `Mut List<T>` — a mutable *container*: it can be reshaped (`add`,
  `remove_at`, `set`, `swap`).
* `List<Mut T>` — a container of mutable *elements*: its shape is fixed, but
  each element can be changed in place.

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

    bump(get(counters, 0)!)            // a handle, used where it is minted

    let second = get(counters, 1)!     // …or bound and used across statements
    second.n = second.n + 10
    bump(second)

    println("${get(counters, 0)!.n} ${get(counters, 1)!.n}")   // 2 13
}
```

`get` is the ordinary accessor: at `T = Mut Counter` it answers
`(proj(counters) Mut Counter)?` — a *projection* that carries `Mut`, which
is what makes it a handle rather than a reading. A projection of a
non-`Mut` element stays read-only, and `copy` is still how you get a value
of your own.

Handles that are only **read** impose nothing, so any number coexist:

```
let a = get(counters, 0)!
let b = get(counters, 1)!
println("${a.n} ${b.n}")          // two live handles, both read: fine
```

A **write** through one is a mutation of the container, so anything else
derived from it is invalidated — the acting handle itself survives, since
its storage did not move.

## Lending a handle from your own function

A function can hand a handle back. It says so on its return type, naming
what the result borrows:

```
struct Entity canbe Mut {
    hp: Int
}

// Searches, and lends back what it found.
fn wounded(es: List<Mut Entity>) -> (proj(es) Mut Entity)? {
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

The handle may be used where it is minted, bound across statements, or —
as here — **found by a loop**, where the position is never written down.

That shape is worth pausing on, because it is the one Rust cannot express:
a function that searches a collection and returns a mutable reference to
what it found conflicts with the borrow it needs to keep searching. Salvo
accepts it, and the Rust backend renders it by handing back the *position*
rather than a reference — see [What the backends do](#what-the-backends-do).

A generic function can lend too, if the caller supplies the accessor. That
is what `params Locate` is for — the bundle pattern, applied to positions:

```
// core.list
params Locate<C, L, T> {
    fn at(c: C, l: L) -> proj(c) Mut T?
}

// Generic over what a *position* is; the caller fills `at`.
fn heal_at<L>(c: List<Mut Entity>, l: L, ?Locate<List<Mut Entity>, L, Entity>) -> None {
    heal(at(c, l)!)
}

heal_at(squad, 1)                  // `at` resolves to the list accessor
```

## One handle at a time, by default

Say nothing and the compiler keeps a single live handle per container. A
second one is refused, because a computed index cannot be told from
another computed index:

```
let a = get(squad, i)!
let b = get(squad, j)!             // error: `a` is a live mutable handle into `squad`
```

The same rule reaches call arguments: two handles into one container in one
call is refused at the second argument, and the diagnostic names the two
ways forward — prove them apart, or mutate through one at a time.

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
if j is NotEq(i) {
    let a = get(squad, i)          // the total `get`: the `Idx` claim did the checking
    let d = get(squad, j)
    a.hp = a.hp + 1
    d.hp = d.hp - 1                // `a` is untouched by this
    attack(a, d)                   // …and one call may take both
}
```

The claim is a fact about the indices' *current values*: reassigning either
side takes it away, like any dependent claim.

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
    update2(squad, i, j, (a, d) -> { a.hp = a.hp - 1; d.hp = d.hp - 2 })
}
println("${get(squad, i).hp}")     // still the *total* read: `Idx` survived
```

An ordinary mutating call has no such promise: after one, an index claim is
gone and the total read stops resolving until you test again. That is the
conservative default, and `preserve Idx` on a signature is how a function
opts out of it.

## Declaring that two may be the same: `canbe`

Sometimes the aliasing is the point. A function that means to accept two
handles which might be one object says so in its deduction clause:

```
fn attack(a: Mut Entity, d: Mut Entity) -> None
=> a canbe d, a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
}
```

Now the caller needs no proof at all — self-attack included:

```
attack(get(squad, i)!, get(squad, j)!)     // i and j unknown: fine
attack(get(squad, i)!, get(squad, i)!)     // the same entity, twice: also fine
```

`canbe` is **symmetric** (`a canbe d` says it once) and
**non-transitive** (`a canbe b, b canbe c` does not relate `a` and `c` —
the relation is a graph, and the sentence says exactly what the rule
means). A `|` list on the right is a hub: `a canbe b|c` relates `a` to each
of them, not `b` to `c`. On the left it is plural-subject sugar:
`a|b|c canbe in es` is three entries.

The **anchored** form says a parameter may be an element of a named
container, which is how the n-way case stays linear in n:

```
fn shuffle(lib: Mut Library, track: Mut Track, other: Mut Track) -> None
=> track canbe in lib.tracks, other canbe in lib.tracks, lib: Mut {
    …
}
```

Two parameters anchored in the *same* path are maybe-elements of one
container, so the mutual aliasing falls out of the shared anchor rather
than needing a `canbe` between them.

Aliasability is **written, never inferred**: it changes what a caller may
pass, so it stays visible in the signature.

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
let boss = get(squad, i)!             let __h0 = (i) as usize;      // captured
boss.hp = boss.hp + 5                 squad[__h0].hp = squad[__h0].hp + 5;
let n = size(squad)                   let n = squad.len();          // legal: no live borrow
boss.hp = boss.hp + n                 squad[__h0].hp = squad[__h0].hp + n;
```

That is what buys the flexibility. A bound `&mut squad[i]` would forbid the
`size(squad)` in the middle — Salvo's rules allow it, because a *read* of
the container cannot invalidate a handle into it, and the position-based
rendering is what lets Rust agree. The same trick carries the shapes Rust
refuses outright:

* **A search that lends what it found** returns a position, so the
  borrow-that-must-outlive-the-loop never exists.
* **A handle across a closure or trait boundary** — `params Locate`'s `at`,
  an effect member that lends — travels as data, where a `&mut`-returning
  closure would tie the borrow to the closure.
* **Two handles that may be one** (`canbe`) render as *one* shared anchor
  plus two positions, so the aliasing is exact: both positions index the
  same storage, which is what Kotlin does natively.
* **Proven-disjoint handles** (`NotEq`) render as a single `split_at_mut`,
  the pattern a Rust programmer writes by hand.
* **A handle across a disjoint mutation** re-reads its path, which agrees
  with Kotlin precisely because the field it names was untouched.

The cost is honest: a position is re-indexed per use, where a `&mut` is
free, and the bounds check is paid unless a claim has removed it. Read
handles keep the zero-cost path — a read-only projection *is* `&T`.
