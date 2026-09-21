# Qualifiers: claims, inference, state and provenance

A qualifier is a *claim* about a value — `NonEmpty List<Int>`, `Celsius Int`,
`Authenticated Request`. Every one of them erases: none exists in the generated
Rust or Kotlin. What they decide is which overload a call reaches and which
calls are legal at all, so this example makes the compiler's reasoning
observable by printing which overload ran.

Run it:

```bash
cargo run -- run --backend rust   --src examples/qualifiers/salvo
cargo run -- run --backend kotlin --src examples/qualifiers/salvo
```

## What it shows, section by section

1. **A state qualifier with a predicate.** `NonEmpty` declares `qualifies`,
   which is the body an `is` check runs. `head(list: NonEmpty List<Int>)` needs
   the claim, so it can return an `Int` rather than an `Int?` and has nothing to
   check.
2. **Where a claim comes from — three ways.** A **refinement** (`refn add(list:
   Mut List<T>, elem: T) -> [list: +NonEmpty]`) is the qualifier saying what
   someone else's function does to its claim: `add` has never heard of
   `NonEmpty`, but appending cannot leave a list empty, and the qualifier is the
   party that knows it. An **`is` check** establishes the claim at run time,
   which is how a list from elsewhere earns it. A **constructor function**
   (`fn celsius(degrees: Int) -> [] Int as Celsius`) asserts one outright, which
   is the only way to get a constructive qualifier — one with no predicate to
   test. `is ^Celsius` **lifts** the claim: the same arm test, after which the
   subject reads with the claim *removed*, which is how the less specific
   overload is reached on purpose.
3. **What survives a call.** A deduction list is exhaustive: `[list: Mut]` says
   `Mut` is the only claim left afterwards, so `compact` drops `NonEmpty` and
   `head(xs)` stops compiling until an `add` re-establishes it. `sum` writes no
   deduction list at all — the compiler infers that it keeps the list and
   mutates nothing, so a caller's claims survive the call.
4. **State versus provenance.** `provenance qualifier Authenticated of Request`
   is a claim about *where the handle came from*, so it is exempt from the rule
   that a mutating call invalidates unlisted claims — a request does not stop
   having been authenticated because a handler wrote to it. `Fresh` is an
   ordinary state qualifier about the same struct, and one `touch(…)` takes it
   away. The output line is the proof: after the mutation, `session` still
   reaches the `Authenticated` overload while `fresh` falls back to the plain
   one.

## What to look for in the generated code

- **The claims are gone.** `rust/main.rs` has no `NonEmpty`, `Celsius`,
  `Authenticated` or `Fresh` anywhere except in *mangled function names* —
  `describe__Celsius`, `handle__Authenticated`, `handle__Fresh` — which exist
  because two Salvo overloads that differ only by a qualifier are identical
  after erasure and need distinct target names.
- **The selection is baked in at compile time.** The two `println` calls in
  section 4 differ: the first is `handle__Authenticated(&session)` /
  `handle__Fresh(&fresh)`, the second `handle__Authenticated(&session)` /
  `handle(&fresh)`. The mutating call in between changed *which function is
  called*, with no runtime test anywhere.
- **A predicate is emitted, a refinement is not.** `NonEmpty_qualifies` exists
  and is called exactly once, at the `is` check. The refinement that let `add`
  supply the claim emits nothing at all: it is trusted, like a constructor
  function, so there is no check to pay for.
- **A constructor function is the identity.** `celsius` returns its argument;
  the claim it adds lives only in the checker.
