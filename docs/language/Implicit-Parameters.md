# Implicit parameters

Some things a function needs are not really *arguments*: how to compare two
`T`s, how to copy one, how to advance a pass. They are properties of the
type, and repeating them at every call is noise. Salvo passes them as
**implicit parameters** — written with `?`, resolved by the compiler from
what is visible at the call.

The same mechanism reaches further than function signatures. A struct can
*require* a set of functions, and a **qualifier** can *carry* one, so a value
remembers which function it was built with. This page is the three, in that
order, and the standard library's heap as the worked example that uses all
of them.

## A function's own implicits

A parameter written with `?` is one the caller does not have to pass:

```
fn sort<T>(list: List<T>, ?cmp: (T, T) -> Int) -> List<T> => list { ... }
```

At the call site the compiler fills `cmp` by looking for a function *named*
`cmp` whose type fits `(T, T) -> Int` with `T` as this call binds it. So
declaring the default for a type is just declaring a function:

```
fn cmp(a: Str, b: Str) -> Int { ... }

sort(names)                    // cmp resolved
sort(names, cmp = descending)  // or supply your own, by name
```

Nothing ties `cmp` to `Str`. There is no declaration saying "`Str` has an
ordering" — the overload whose parameters accept `Str` *is* the ordering, and
a different one is a lambda or a function reference away. Overriding is by
the parameter's own name, and the value can be either:

```
sort(names, cmp = (a: Str, b: Str) -> size(a) - size(b))
```

### Bundling them: `params`

A set of related functions is declared once and spread with `?`:

```
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn total<T>(xs: List<T>, ?Field<T>) -> T => xs {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

total(list_of(1, 2, 3))                          // 6
total(list_of(2, 3, 4), add = times, zero = one) // 24 — override one or both
```

The spread has **no name of its own**, deliberately: its members become
implicit parameters in their own right, so they are called unqualified inside
the body and overridden by their own names outside it. A `params` group is
never a value — it exists only to keep a signature short.

Overriding "one or both" is the usual freedom: a group is a convenience for the
declaration, not a contract the caller must fill wholesale. Some members are only
meaningful together, though, and a clause says which:

```
params Hashed<T> => eq with hash {
    fn hash(value: T) -> Long
    fn eq(a: T, b: T) -> Bool
}
```

`eq with hash` means the two are **one decision**: a call writes both, forwards
both, or leaves both to resolution, and mixing those is an error that names the
fix. A hash and an equality have to agree — equal values must hash equally — and
a container that buckets by the hash would never consult an equality somebody
else chose, so half a pair is either invisible or broken.

`with` is symmetric (agreement has no direction) and chains: `a with b with c`
makes the three one class. A function may add relations of its own —
`fn f<T>(?Hashed<T>, ?at: …) => at with hash` — but never drop one it inherited
by spreading a group; declaring the members individually (`?hash: …, ?eq: …`) is
how a signature takes them unrelated.

`core` declares the ones everything else builds on: `Ordered<T>` (`cmp`),
`Eq<T>` (`eq`), `Hashed<T>` (`hash` and `eq` together), `Yield<It, T>`
(`next`), `Locate<C, L, T>` (`at`). A type joins any of them by declaring the
function — there is nothing to register.

### Details worth knowing

- **An implicit parameter's type must be a function type.** What fills it is
  resolved as a function of that name.
- **They come last.** A normal parameter written after one could not be
  passed positionally.
- **Generic code passes its own implicits on.** Inside `total`, `T` is
  opaque, so a call to another function needing `?Field<T>` is filled from
  *this* function's implicits — matched by name and type, whatever grouping
  either side used. A generic function that declares none cannot call one
  that needs one: there is nothing to resolve and nothing to forward, and the
  error says which to add. This is the same colouring effects have, for the
  same reason.
- **Resolution is local.** Which default a call gets depends on what is
  visible where the call is written, exactly as with `use` and handlers. Two
  matching declarations make the call ambiguous, which is an error naming the
  override as the remedy.
- **An implicitly resolved function is effect-free.** A fn type without an
  effect list means "performs nothing", and an effectful function does not
  fit there — so resolution can never quietly add an effect to a caller.
- **Effect members have them too.** A member is an ordinary signature, so
  `fn show(v: T, ?fmt: (T) -> Str) -> Str` works: the call site resolves
  `fmt`, and every handler implementing `show` receives it. Handler
  *constructors* do not — their instance is built by `use`, which resolves
  nothing — and neither do lambdas, whose types have no room to declare one.
- **A mismatch that types cannot show is explained.** What a call does to
  each argument is part of whether a function fits, but not part of how a
  type prints, so a function that *consumes* an argument where the position
  keeps it is reported in words: which argument, which direction, and the two
  ways to fix it.
- **What the implicit resolves to can determine the call's type arguments.**
  Resolution runs *between* the arguments, not after them, so a variable that
  appears only in the implicit's type is still inferred. A `?Yield<It, T>`
  spread goes one step further: `T` is read off `It`'s own declaration (see
  [Passes](Passes.md)), so the element type of a combinator is never written.

## A struct that requires them

A struct can state that its type parameter must come with a bundle, using a
`:` clause between the generics and `canbe`. The obligation is then the
*type's*, not each function's:

```
// core.array — the pass an array is walked by. `: Yield<self, proj T>` is the
// obligation: this struct comes with a `next`, and `self` names the struct.
export struct ArrayYield<T> : Yield<self, proj T> canbe Mut {
    items: proj (T[]),
    at: Int
}
```

The obligation now travels with the *type*: a combinator taking an
`ArrayYield` writes no `?Yield` of its own, and `for` over one needs nothing
either. Without the clause the same struct is just a struct — the `for` loop
says so, and names the clause as the remedy.

## A qualifier that carries one

`sort(list, cmp = …)` picks an ordering per call, which is right for an
algorithm that does its work and hands the result back. A structure that
*stays* ordered — a heap, a ranked list — cannot work that way: building it
under one ordering and reading it under another is a corrupt structure, and
"which ordering" is a property of the value, not of each call.

So a qualifier can carry the function as a **slot**, fixed where the value is
constructed. Type arguments come first, slots after:

```
qualifier Heap<T>(?Ordered<T>) of List<T> with NonEmpty
```

A use site fills the slot with a **function identity** — a bare name
(`Heap<Person>(min_by_age)`), a canonical (`Heap<Person>(cmp@Person)`) — or
with the signature's own binder (`Heap<T>(?cmp)`), which is how a generic
function says "whatever this value carries".

Three consequences, and they are the point:

- **`Heap(min_by_age)` and `Heap(max_by_age)` are different types.** Passing
  one where the other is expected is an ordinary type error naming both.
- **All the `?cmp` in one signature are one binding.** A
  `merge(a: Heap<T>(?cmp) Mut List<T>, b: Heap<T>(?cmp) Mut List<T>)` accepts
  two heaps only if they carry the same ordering. Two *different* orderings
  need two names, which is what an alias is for: `b: Heap<T>(?cmp: cmp2)`
  fills the same slot under the name `cmp2`. A slot a signature never
  mentions is not constrained at all, so a function that does not care writes
  the claim bare (`Heap List<T>`) and accepts any of them. In that `merge`,
  `x < y` would be ambiguous between `cmp` and `cmp2`, so it is an error
  naming both and the body calls the one it means.
- **A function bound into a type must be named, top-level and capture-free.**
  A lambda has no identity a type could carry, and the error says so instead
  of losing the claim quietly. Everything a type can *print* it can carry.

Dropping the claim is safe rather than silently wrong: the operations demand
it, and a plain value never gets it back by subtyping — you lose access,
never correctness.

## Worked example: `std`'s heap

`import heap` brings a binary heap that is **not a container**: it is a claim
on an ordinary `List<T>`, arranged and kept by the ordering it was built
with. Every piece of this page appears in it.

The claim, with its slot:

```
export qualifier Heap<T>(?Ordered<T>) of List<T> with NonEmpty
```

Construction takes the ordering as an ordinary implicit, and the return type
**publishes what the call resolved** — `+Heap<T>(?cmp)` establishes the claim:

```
export fn heap_of<T>(?Ordered<T>) -> +Heap<T>(?cmp) Mut List<T> {
    return mut_list_of()
}

export fn heap_of<T>(first: T, ...elems: T[], ?Ordered<T>)
-> Heap<T>(?cmp) NonEmpty Mut List<T> {
    let list = mut_list_of(first, ...elems)
    heapify(list)
    return list
}
```

Turning an arbitrary list into a heap says the same thing in a deduction
clause, because the claim is established rather than kept:

```
export fn heapify<T>(list: Mut List<T>, ?Ordered<T>)
=> list: +Heap<T>(?cmp) Mut {
    if list is NonEmpty {
        heapify(list)              // routes to the NonEmpty overload
    }
}
```

An operation on an existing heap *captures* the ordering from its argument's
type rather than stating it — inside the body `cmp` is an ordinary implicit,
and it is the one this heap was built with:

```
export fn push<T>(heap: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
=> heap: +Heap<T>(?cmp) Mut, !elem {
    ...
}
```

Using it:

```
import heap

fn main() [use] {
    use StdOutConsole()
    let h = heap_of(5, 1, 4)           // Heap<Int>(cmp) NonEmpty Mut List<Int>
    push(h, 0)
    println("${peek(h)}")              // 0 — under the ordering it was built with
}
```

Two heaps built under different orderings will not mix, a function that needs
the ordering never asks for it, and none of it costs anything at run time:
qualifiers are erased, and the carried identity resolves to a direct call.
