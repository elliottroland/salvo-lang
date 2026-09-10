# Iteration, in every form

One program, six ways of walking a sequence — because in Salvo they are all the
same rule. A **pass** is an ordinary struct that declares `: Yield<self, T>`
and has a `next`; `for` is sugar for calling that `next` until it answers
`Finished`. There is no iterator type, no `Iterable` interface and no protocol
built into the compiler.

Run it:

```bash
cargo run -- run --backend rust   --src examples/iteration/salvo
cargo run -- run --backend kotlin --src examples/iteration/salvo
```

## What it shows, section by section

1. **A container, driven natively.** `for` straight over a `List`, an array or
   a `Str`. No pass is allocated and the container is not consumed — the
   backends keep their own loop for this shape, which is why it is the cheapest
   and the most common.
2. **A pass of your own, written out.** `Countdown` holds the position as a
   field and its `next` is an ordinary function. This is the form `yield` cannot
   express — two sources at once, explicit state — and it is also what
   `iter(xs)` hands back. Because a pass is a *value*, `take` drives it partway
   and `main` carries it on from where the `break` left it. Write it out like
   this when the pass needs a **name**: to store it in a field, to hand it to a
   function, to zip two of them.
2b. **The same pass with the struct generated: `pass fn`.** `Halving` is
   ordinary data; the `state { … }` block is the pass's own fields, evaluated
   once when the pass is minted; and the compiler writes the pass struct and the
   `iter` that mints one. Nothing suspends — the body *is* the `next` — so there
   is no state machine in the output. The subject's fields are read-only inside
   the body, and `iter(h)` hands back a pass you can hold in a local.
3. **A generator.** `yield fn next(f: Fibs)` says the same thing as a body and
   leaves the state machine to the compiler. The subject is an **origin** —
   ordinary data you keep — and the machine is nameless, so nobody can hold
   one: every `for` builds a fresh machine, which is why driving the same
   origin twice starts over. The `defer` inside runs however the loop ends, and
   `Naturals` is unbounded, terminated by the consumer's `break`.
4. **A combinator of your own.** `sum_of<It>(it: Mut It, ?Yield<It, Int>)`. The
   `?Yield` spread is a bundle of implicit parameters, not a bound and not a
   trait: the call site fills it with whichever `next` fits. That same spread
   is the declaration `for` reads, so a generic pass is driven with an ordinary
   loop.
5. **The sequence functions.** `map`/`filter`/`reduce` are eager and take a
   *pass*, so a container is iterated by writing `iter(xs)`. A `List` has a
   fast path under the same name, which is why `map(xs, f)` still reads well.
   A generator's origin fits the same functions.
6. **Laziness, asked for by name.** `map_lazy`/`filter_lazy` hand back a
   composed pass — a struct holding the source, the callback and the source's
   `next` — so nothing runs until it is driven and an unbounded source is fine.
   `map_to` maps into a collection you provide, reached through an `add` the
   call site resolves, so the destination need not be a `List`.

## What to look for in the generated code

- **The native loops are native.** `rust/main.rs` has `for n in xs`, `for mut c
  in "salvo".to_string().chars()` and `for n in &arr`; `kotlin/main.kt` has
  Kotlin `for` loops. Nothing is boxed and nothing is allocated for them.
- **A hand-written pass stays a plain struct and a plain function.**
  `pub struct Countdown` plus `next__6`, and the loop over it is
  `while let Union2::U1(mut n) = next__6(p)` — advancing the pass *where it
  lives*, which is what lets the caller keep driving it.
- **A `pass fn` generates exactly what you would have written by hand.**
  `pub struct __Pass_Halving { pub __subject: Halving, pub at: i32 }`, an
  `iter__…` that copies the subject in (`__subject: h.clone()`, which is why
  the subject replays), and the author's body as an ordinary `next`. No state
  number, no `__advance`, no flags — the difference from the generator below is
  the whole point of the form. The struct's name is not writable in Salvo, which
  is what keeps "a pass you must name is written by hand" true.
- **A generator becomes a state machine nobody can name.** `__Pass_Fibs` with
  `__advance` and `__close`, minted at the loop (`__Pass_Fibs::new(fibs(6))`)
  and closed on every exit — including where a combinator abandons it, which is
  where the `defer`'s "3. closing" comes from.
- **Effects are parameters, not capture.** `__advance(&mut console)` on the
  effectful machine and `__advance()` on the pure one; the effect list is on
  the `yield fn`, and driving is what performs it.
- **A generic combinator receives its `next` as a function value.**
  `sum_of::<__Pass_Fibs>(&mut __mint1, &mut |__p| …)` — the implicit resolved
  at the call site, which is why an effectful `next` composes at all (its
  handlers are parameters of the value's type, where a fixed trait method would
  have nowhere to put them).
- **Nothing of the pass machinery survives that is not used.** No factory type,
  no `Iterable` representation, no per-effect-set trait.

## Known wart

`kotlinc` emits `unchecked cast of 'Any?' to 'T'` warnings for std's generic
combinators (`core/seq.kt`). A generic union arm read has to go through `Any?`
on the JVM, so the cast is in code the author never wrote; it is recorded in
COMPLETED.md rather than silenced.
