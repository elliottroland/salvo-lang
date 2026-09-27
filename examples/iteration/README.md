# Iteration, in every form

One program, seven ways of walking a sequence — because in Salvo they are all
the same rule. An **iterator struct** is an ordinary struct that declares
`: Yield<self, T>` and has a `next`; `for` is sugar for calling that `next`
until it answers `Finished`. There is no iterator type, no `Iterable` interface
and no protocol built into the compiler.

Run it:

```bash
cargo run -- run --backend rust   --src examples/iteration/salvo
cargo run -- run --backend kotlin --src examples/iteration/salvo
```

## What it shows, section by section

1. **A container, driven natively.** `for` straight over a `List`, an array or
   a `Str`. No iterator is allocated and the container is not consumed — the
   backends keep their own loop for this shape, which is why it is the cheapest
   and the most common.
2. **An iterator struct of your own, written out.** `Countdown` holds the
   position as a field and its `next` is an ordinary function — and it is also
   what `iter(xs)` hands back. Because an iterator is a *value*, `take` drives
   it partway and `main` carries it on from where the `break` left it. `skip` is
   a second step over the same struct: only `next` is canonical, so a `for`
   drives `skip` by naming the call — `for n in skip(q)` re-invokes it each
   turn. Write a struct out like this when it needs a **name**: to store it in
   a field, to hand it to a function, to zip two of them.
2b. **The same thing with the struct generated: `iter fn`.** `halving(start)`
   is the minter — any name, any parameters — the `state { … }` block is the
   iterator's own fields, evaluated once when it is minted, and the body is its
   `next`. The compiler writes the struct; its type is spelled `iter Int`, and
   `halving_from_ten() -> iter Int` shows the spelling as a return pattern
   filled from the body. Nothing suspends, so there is no state machine in the
   output.
2c. **A source: `Iter`.** `Bag` declares `: Iter<self, Int>` and an `iter fn
   iter`, so `for n in bag` mints with `iter(bag)` and drives what it answers.
3. **An effectful iterator, and an unbounded one.** A `next` is an ordinary
   function, so effects are ordinary effects: `fibs` declares `[Console]` and
   the `for` that drives it supplies the handler once per turn. `naturals` is
   unbounded — nothing runs until a consumer pulls — so the `break` is what ends
   it.
4. **Combinators of your own.** `sum_of<It>(it: Mut It, ?Yield<It, Int>)` takes
   an iterator; `total<C>(c: C, ?Iter<C, Int>)` takes a *source* — a `Bag`, a
   `List`, anything with an `iter` — over a hidden iterator type; `first(it:
   iter Int)` takes "any iterator of `Int`" as a hidden generic. Each spread is
   a bundle of implicit parameters, not a bound and not a trait, and each is
   also the declaration `for` reads inside the body.
5. **The sequence functions.** `map`/`filter`/`reduce` are eager and take an
   *iterator*, so a container is iterated by writing `iter(xs)`. A `List` has a
   fast path under the same name, which is why `map(xs, f)` still reads well.
   An `iter fn`'s iterator fits the same functions.
6. **Mapping into a collection you provide.** `map_to` maps into a destination
   you hand it and gives it back, reaching it through an `add` the call site
   resolves — so the destination need not be a `List`. Nothing in std is lazy:
   the lazy pair was removed 2026-09-10 and laziness is reconsidered after
   concurrency (see ROADMAP.md).
7. **Ranges.** std's `range` is an `iter fn` with three parameters and two
   overloads returning `iter Int` that delegate to it.

## What to look for in the generated code

- **The native loops are native.** `rust/main.rs` has `for n in xs`, `for mut c
  in "salvo".to_string().chars()` and `for n in &arr`; `kotlin/main.kt` has
  Kotlin `for` loops. Nothing is boxed and nothing is allocated for them.
- **A hand-written iterator stays a plain struct and a plain function.**
  `pub struct Countdown` plus `next__…`, and the loop over it is
  `while let Union2::U1(mut n) = next__…(p)` — advancing the iterator *where it
  lives*, which is what lets the caller keep driving it. The step call
  `for n in skip(q)` is `while let Union2::U1(mut n) = skip(&mut q)`: the call
  as written, re-invoked.
- **An `iter fn` generates exactly what you would have written by hand.**
  `pub struct __Iter_halving_Int { pub start: i32, pub at: i32 }`, a
  `halving(start)` minting one, and the author's body as an ordinary `next`.
  The struct's name is not writable in Salvo, which is what keeps "an iterator
  struct you must name is written by hand" true; `iter Int` is how it prints.
- **A pattern-typed fn returns the concrete struct.** `halving_from_ten()`
  is `-> __Iter_halving_Int` in both outputs — the `iter Int` was filled from
  the body.
- **Effects are parameters, not capture.** `next__…(&mut console, …)` on the
  effectful iterator and `next__…(…)` on the pure one; the effect list is on
  the `iter fn`, and driving is what performs it.
- **A generic combinator receives its `next` as a function value.**
  `sum_of::<__Iter_halving_Int>(&mut hp, &mut |__i0| next__…(__i0))` — the
  implicit resolved at the call site. `total::<Bag, __Iter_iter_Bag>(&bag,
  &mut |__i0| iter__…(__i0), &mut |__i0| next__…(__i0))` shows the `?Iter`
  pair with its hidden generic filled.
- **Nothing of the iterator machinery survives that is not used.** No factory
  type, no `Iterable` representation, no per-effect-set trait.

## Known wart

`kotlinc` emits `unchecked cast of 'Any?' to 'T'` warnings for std's generic
combinators (`core/seq.kt`). A generic union arm read has to go through `Any?`
on the JVM, so the cast is in code the author never wrote; it is recorded in
COMPLETED.md rather than silenced.
