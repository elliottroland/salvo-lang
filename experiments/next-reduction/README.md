# The reduction to `next` — the R0 prototype

Hand-written target-language code for the iterator reduction decided
2026-09-08. See PROGRESS.md, "Roadmap: iterators — **the reduction to
`next`**", for the decisions and the six phases; this directory is the
*evidence* that R0's target shape works, kept because the findings recorded
there are only as good as the code they came from.

| file | what it is |
|---|---|
| `passes.sv` | the Salvo source, in the **planned** language — it does not compile today (obligation groups, a `yield` fn naming its pass struct, `for` closing a pass) |
| `passes.rs` | what the Rust emitter would produce for it |
| `passes.kt` | what the Kotlin emitter would produce for it |
| `expected.txt` | the stdout both print, byte for byte |

**Superseded spelling, same machine (2026-09-08, after this prototype ran).**
`passes.sv` writes the `yield` fn as
`fn chatty(limit: Int) [Console] -> Chatty : Yield<Int>` — a return type that
*names* the generated struct. The user replaced that surface with the
**origin-struct model** (PROGRESS.md, "Unknown 1: the origin-struct model"):
the author declares an origin struct with the `: Yield<T>` clause and writes
`yield fn next(origin) -> T`; the state struct is hidden and never nameable.
The *generated code* this prototype validates is unchanged by that — `Chatty`
in `passes.rs`/`passes.kt` simply becomes an anonymous machine capturing the
origin's fields, with the same states, the same flags and the same `close` —
so the findings below stand. Only `passes.sv`'s spelling of section 2 (and the
`map_a`/`map_b` signatures) is out of date.

## Running them

```bash
rustc --edition 2021 passes.rs -o ../../tmp/next/passes_rs && ../../tmp/next/passes_rs
kotlinc passes.kt -d ../../tmp/next/classes && kotlin -cp ../../tmp/next/classes salvo.PassesKt
```

Verified with `diff`: identical to `expected.txt` on both sides. `kotlinc`
says **nothing at all**; `rustc` says nothing under
`#![allow(dead_code, non_snake_case)]`, both of which are in the emitter's own
header (`next__2` is how a Rust overload is disambiguated).

## What the program is chosen to prove

Six shapes in one program, because the interactions are the point:

1. **a hand-written pass** (`Countdown` + `next`) driven by `for`, **beside a
   `for` over a `List`** — the pass drives `next`, the list keeps its native
   loop;
2. **a `yield`-generated pass** with a `defer` and a `Console` effect,
   **abandoned** after two elements, so the release path is observable;
3. **a composed pass**, rendering **(A)**: the obligation group as a *generic
   bound*;
4. the same, rendering **(B)**: the group's member as an *implicit parameter*
   — over an **effectful** source, which is what (A) cannot express;
5. **a linear pass** (`Lines` + `close`) driven to **exhaustion**;
6. the same, **abandoned** early.

`close` landing exactly once in 2, 4, 5 and 6 — on both the drained and the
abandoned path — is the part worth distrusting, which is why all four are in
the same stdout.

## What is *absent*, which is the point of the reduction

No `SalvoIter`, no `Box<dyn SalvoPass<T>>`, no `Rc<dyn Fn() -> …>` factory, no
`Iterable<T>` as a representation, no per-effect-set pass trait, no variance
adapter, and — on Kotlin — **no runtime file at all**. `SalvoPass<T>` existed
to turn "advance and report" into `hasNext`/`next` with one element of
lookahead; with `next` returning `Emitted T | Finished` the tagging does that
job, so the class is dead. Rust's remaining `'static` bounds sit only on
*stored callbacks*, not on passes: they came from the factory.

## The findings

### 1. Composition only breaks when the source's type is a *parameter*

A **non-generic** composed pass already works in today's language: a struct
with a `Mut Countdown` field, a `next` that calls `next(d.src)`, driven by
`for`. Compiled and run on the Rust backend (`n 6 / n 4 / n 2`). What is
refused is the generic form — `next(Mut P)` on an unbounded `P` is
`no matching overload for next(Mut P)`, exactly as [call-resolve] promises,
since with no bound nothing about a `P` is knowable.

So "how is a composed pass generated" reduces to one question: **how does
`next` become reachable through a type parameter?** There are two answers,
both in the running prototype.

### 2. (A) group-as-bound and (B) member-as-implicit are both viable, and they are not equivalent

**Decided (user, 2026-09-08): (B).** Both are kept in this prototype anyway —
it is the evidence the choice rests on, and (A)'s bound form is still wanted
for `canbe Linear` on type parameters.

**(A)** renders `params Yield<T>` as a trait/interface with one `impl` per
declaring type, and the bound `P: SalvoYield<T>` justifies the call. Static
dispatch, one monomorphization per source type, and no value ever has the
group as its type — [group-not-a-value] holds. `map_a` in the prototype.

**(B)** passes the source's `next` (and `close`) as *function values*, stored
in the composed pass. This is not new machinery: today's generated `map_lazy`
already stores its `?Iterable` member that way
(`iter: Rc<dyn Fn(It) -> SalvoIter<T>>`), on both backends. `map_b` in the
prototype.

Two differences decide it:

* **Effects.** (A)'s trait method has a fixed signature, and an effectful
  `next` takes its handlers as *parameters* — so `Chatty` (which performs
  `Console` per resume) **cannot implement `SalvoYield<Int>` at all**. That is
  the "one generated trait per effect set" problem [iter-effects] had,
  resurfacing at the bound. (B) has it for free: the stored `next`'s *type*
  carries the effect list, and "a fn inherits its fn-typed parameters'
  effects" [fn-effects] is already the rule. Section 4 of the prototype
  composes over `chatty`, and only (B) can.
* **Optional members.** A composed pass wants to close its source *if it has a
  close*. Under (A) that is a `P: Linear` bound, which a pass without a `close`
  cannot satisfy — so the same combinator cannot serve both. Under (B) the
  `?close` implicit is resolved at the *call site*, where whether the source
  has one is known.

### 3. A consuming `close` moves the idempotency, it does not remove it

`fn close(s: Self)` consumes, so on Rust it takes the value and **cannot be
called twice** — the "one call after the loop covers `break` and exhaustion
because the flags make it idempotent" reasoning from I1b/I4 is simply gone.
What survives is smaller and in a different place:

* the per-`defer`-site **flag guard inside** `close` is still needed, because
  the body's own exhaustion path may already have discharged the `defer`
  (section 4's inner `close` prints once, from the body, and the outer call is
  a no-op);
* a **composed** pass needs its source in an `Option`/nullable **slot**, not a
  plain field: the release path only ever holds `&mut self`, while closing the
  source consumes it. `MapB.src` is `Option<P>` / `P?` for that reason, and
  `take()`-ing it is what makes `__close_src` idempotent.

The second one was not predicted, and it is the same shape as I3's "a field
whose type has no zero has to be a slot" — arriving from ownership rather than
from initialization.

### 4. Unpredicted collision: `close`-implies-`Linear` would make most generated passes uncomposable

Two decisions meet badly:

* the proposed answer to abandonment — *a pass type that declares a `close`
  must be linear*;
* the interim rule — *storing a linear value in a composite is an error*.

A composed pass **stores its source**. Every `yield` fn whose body has a
`defer` gets a `close`. Together those say: a generated pass with a `defer`
cannot be mapped over. That is most of them.

The prototype resolves it by keeping the two independent — `Chatty` has a
`close` and is **not** `Linear`; `Lines` declares `: Linear` and its `close`
is the discharge — which is what lets section 4 run at all.

**Decided (user, 2026-09-08): that is the rule.** A `close` never implies
linearity; `: Linear` on the type is the only thing that does, and generic code
that may carry one opts in with `<T canbe Linear>`. Both halves stay explicit.
The accepted cost: a hand-written `while` driver may abandon a `close`-bearing
pass unchecked, since most generated passes are not linear. The accepted
casualty: mapping over a *linear* pass — a file's lines — stays unavailable
until composition or conditional linearity lands (recorded as roadmap L8).

### 5. Smaller things, all verified

* **`Mut` on a user struct is only reachable through a qualified struct
  literal** — `Mut Countdown { at: 3 }`. A plain literal cannot be coerced
  (`let a: Mut Box = Box { … }` is an error), and `for x in Mut P { … }` is a
  parse error, so a pass always arrives from a call or a local. Every pass
  constructor therefore writes `Mut`.
* **A fn-typed field is refused on Rust for *user* structs** ("the rust
  backend cannot store a function in a struct field") while *generated* pass
  structs store `Rc<dyn Fn…>` freely — and **Kotlin accepts it**, so this is a
  live backend divergence, not a shared rule: the same program builds on one
  target and not the other. So a hand-written composed pass holding a callback
  is not expressible, and (B) is available to generated passes only unless
  that refusal is lifted — which the generated code shows is possible, since
  `Rc<dyn Fn…>` is exactly what it would take.
* **Calling a fn-typed field needs a local first** (`let g = h.f` then
  `g(e)`): `h.f(e)` is dot-notation, i.e. `f(h, e)` [fn-dot].
* **A generic struct literal needs written type arguments**
  (`Mut Doubling<Countdown> { … }`); they are not inferred from the fields.
* **Rust overload mangling reaches `close` too** (`close`, `close__2`, …), so
  the `for` sugar needs the checker to hand over *two* resolved functions per
  driven subject, where `for_drivers` carries one today. Kotlin needs no
  mangling: `next(Chatty, Console)` and `next(Lines)` are ordinary overloads.

## What it does not cover

* A **`Throw` transfer** out of a suspended body — unchanged from the earlier
  prototypes, and unchanged in the reduction: a fallible pass yields a result.
* A **recursive** producer (the field would have the struct's own type, so it
  needs a `Box`) — established by the I3 prototype and untouched here.
* A **hand-written `while` driver** around `next`, which is the case with no
  sugar to inject a `close`, and therefore the case linearity has to catch. It
  is a checker question, not an emission one, so it belongs to R4.
* Composition over a **linear** source, which finding 4 says is refused.
