# Pull iterators — the I1b prototype

Hand-written target-language code for the iterator rework decided
2026-09-07. See PROGRESS.md, "Roadmap: iterators — Salvo-level pull
iterators", for the decisions and the phases; this directory is the
*evidence* that phase I1's design works, kept because the findings recorded
there are only as good as the code they came from.

| file | what it is |
|---|---|
| `lines.sv` | the Salvo source, written in the **planned** language — it does not compile today (`Once Iter<T>` does, but an effectful iterator fn and the injected close do not) |
| `lines.rs` | what the Rust emitter would produce for it |
| `lines.kt` | what the Kotlin emitter would produce for it |

## Running them

```bash
rustc --edition 2021 lines.rs -o lines_rs && ./lines_rs
kotlinc lines.kt -d classes && kotlin -cp classes salvo.LinesKt
```

Both print, byte for byte:

```
opening data.txt
line -- data.txt --
line alpha
line beta
closing data.txt
done
```

## What the program is chosen to prove

It is deliberately the awkward case rather than a tidy one:

- the producer **performs effects** (`Console` *and* `FileSystem`) while the
  consumer drives it, so the interleaving is observable in the output;
- it **holds a resource**, released by a `defer`;
- the deferred block **itself performs effects** — it prints *and* closes,
  which is why a Rust `Drop` impl could never have been the mechanism (it
  takes no parameters);
- there are **two distinct resume points** (the header `yield`, then the
  `yield` inside `while true`), so the machine has real states rather than
  one loop;
- the producer is **unbounded** and terminates only because the consumer
  `break`s — the path the injected `close` exists for.

The `closing data.txt` line landing before `done` is the whole point: the
consumer stopped early, and the producer's `defer` still ran.

## The two findings worth keeping

**`close` is idempotent, and that collapses the injection.** Guard the
unwind path on the per-site `defer` flags and *one* call after the loop
covers `break` and exhaustion alike, because both land there. Verified by
raising the break threshold so the loop drains instead: exactly one
`closing` line either way, identical on both backends. Only `return` and
`throw` out of the loop body need their own splice, which is machinery
`defer` already has.

**Effects thread in, they are not captured.** `next(&mut self, fs: &mut dyn
FileSystem, console: &mut dyn Console)` holds no handler, so there is no
lifetime and no `'static` bound — which is exactly what `[iter-effect-free]`
existed to avoid. Kotlin's `iterator { … }` builder could only ever have
captured them, which is why one uniform lowering costs the JVM side nothing.

Note what is absent from `lines.rs`: no `Pin`, `Future`, `Waker`,
`Box<dyn Iterator>`, `Rc`, or `async`. The state machine is a struct with a
`u32` state, the body's locals as fields, and one `bool` per `defer` site.

## What it does not cover

One `defer` site and one loop. The interaction still to distrust is a body
where `defer`, `when`, labelled loops and a `Throw` transfer all have to be
resumable at once — see the gnarly prototype below, which took most of that
on.

# The gnarly prototype (I3)

`gnarly.sv` is the body that decides whether the general lowering is
tractable: everything resumable at once. `gnarly.rs` and `gnarly.kt` are the
hand-written state machines, and `gnarly_oracle.rs` is what says whether they
are right.

| file | what it is |
|---|---|
| `gnarly.sv` | the Salvo source, in the planned language |
| `gnarly_oracle.rs` | the **oracle**: the same body in *push* style, with `defer` as Rust scope guards |
| `gnarly.rs` | the state machine, Rust |
| `gnarly.kt` | the state machine, Kotlin |

```bash
rustc --edition 2021 gnarly_oracle.rs -o oracle && ./oracle
rustc --edition 2021 gnarly.rs -o gnarly_rs && ./gnarly_rs
kotlinc gnarly.kt -d classes && kotlin -cp classes salvo.GnarlyKt
```

All three print:

```
open
got 2
got 0
row 0 end
got 12
got 100
row 1 end
close
done
```

## Why there is an oracle

A hand-written state machine is only as trustworthy as the thing that decides
the expected answer, and hand-tracing when a *suspended* body's deferred
blocks run is exactly the reasoning most likely to be wrong. So the oracle
writes the same body in the one style that needs no bookkeeping at all —
`yield` becomes a callback — and expresses each `defer` as a `Drop` impl,
which makes **Rust's own scope discipline** the authority on when deferred
code runs and in what order. Salvo's `[defer]` rule (end of the enclosing
block, latest-registered first, on every exit path) is precisely Rust's drop
order for locals.

The two machines were then required to match it, on both exit paths: the
consumer breaking after four elements, and the consumer draining (which is
the only path that reaches the `yield` after the loop and the normal end of
the body).

## What the body exercises

- a `defer` at fn-block level, released on every exit path;
- a `defer` **inside a loop body**, registered anew each iteration, reading a
  per-iteration local, discharged at the end of that iteration;
- a nested loop over **another pass**, which has to stay alive across the
  outer body's suspensions;
- `continue` inside the inner loop;
- two yields per outer iteration, and a third after the loop — four resume
  points in all;
- effects performed by the producer *and* by a deferred block;
- a consumer that stops early, so the release path runs while the body is
  suspended mid-nest.

## What it established

- **The lowering is mechanical**: numbered resume points, the body's locals
  as fields, and a flat `loop { match state }` dispatch. Nothing about the
  nesting needed a special case — an inner loop is just more states.
- **`continue` is free**: it is staying in the same state.
- **A yield in the middle of a loop body is free**: the state *after* the
  yield is "the statements after it", and the back edge is a state
  transition.
- **A nested pass becomes a field**, which is also the reason a *recursive*
  producer needs a `Box`: the field would have the struct's own type.
- **One slot per `defer` site is enough, and now we know why**: a loop-body
  `defer` is discharged before the back edge, so it cannot outlive its
  iteration and no stack is needed. What the flag needs alongside it is the
  local the deferred block *reads* — which is already a field, since every
  local is.
- **The release path is unchanged from I1b**: flags, latest-first,
  idempotent, so a single `close` after the loop covers `break` and
  exhaustion alike.

## Still not prototyped

A **`Throw` transfer out of a suspended body**. The shape is probably
`next` returning `ControlFlow<M, Emitted T | Finished>` with the pending
defers running on the `Break` path, but that changes the protocol's result
type and how a `for` drives it, so it wants its own prototype before it is
designed.

# The effectful prototype (I4)

`effectful.sv` is the program I4's emission half had to produce: a producer that
**performs effects while the consumer drives it**. The language part landed
2026-09-07 ([iter-effects]: the claim is written on the producer's type), and the
emission half landed the same day, built against these files — so `effectful.sv`
now compiles and runs on both backends, and is a test on each side
(`rustc_compiles_and_runs_an_effectful_producer`,
`kotlinc_compiles_and_runs_an_effectful_producer`: one source, one expected
stdout, which is the parity claim).

| file | what it is |
|---|---|
| `effectful.sv` | the Salvo source — compiles and runs today, on both backends |
| `effectful.rs` | the state machine plus the per-effect-set runtime, Rust — the shape the emitter was aimed at |
| `effectful.kt` | the same, Kotlin |

The emitters' output differs from the hand-written files only in naming and
scaffolding: `SalvoPureAsConsole` where the sketch wrote `PureAsConsole`,
`__advance`/`__close` as inherent methods with the interface forwarding to them,
and — Kotlin — the `close` in a `finally` rather than a statement after the
loop, which is how `defer` is lowered there anyway.

```bash
rustc --edition 2021 effectful.rs -o ../../tmp/eff/eff_rs && ../../tmp/eff/eff_rs
kotlinc effectful.kt -d ../../tmp/eff/classes && kotlin -cp ../../tmp/eff/classes salvo.EffectfulKt
```

Both print, byte for byte (verified with `diff`), and neither compiler says
anything at all:

```
open
make 0
got 0
make 1
got 1
close
open
make 0
make 1
close
sum 1
plain 6
```

## What the program is chosen to prove

* the producer's work **interleaves** with the consumer's, so `open`/`make n`
  land between the `got n` lines rather than before them;
* a `defer` inside the producer that **itself performs an effect**, which is
  what makes the injected `close` observable for the first time: the first loop
  `break`s after two elements and `close` still prints;
* `close` is **idempotent**, so one call after the loop covers the abandoned
  path and the drained one — the drained `total(chatty(2))` prints `close`
  exactly once, from the body's own exit;
* a fn that **inherits** the claim from a producer parameter (`total` declares
  no effects of its own, and takes the handler as a leading parameter);
* the claim is on the **return type**, so `chatty(3)` itself needs no handler —
  only driving the pass does;
* and a **factory** stays replayable: two calls, two passes, each from the
  beginning.

## The three shapes it fixes

1. **A trait/interface per effect set.** A claiming pass cannot be a
   `std::iter::Iterator` or a Kotlin `Iterator<T>` — its `advance` takes a
   handler — so the emitter generates one per effect set a program's producer
   types mention, the way it already generates one `UnionN` per arity. The pure
   path is untouched.
2. **The factory stays a factory**: `Console Iter<T>` is
   `Rc<dyn Fn() -> Box<dyn SalvoPassConsole<T>>>` on Rust and a
   `fun interface … { fun mint(): … }` on Kotlin. The boxing costs nothing new —
   the pure path already boxes.
3. **Variance needs an adapter.** A *pure* producer used where a claiming one is
   expected has a different representation, so the compiler wraps it at that
   boundary (an `advance` that ignores the handler). This is the finding worth
   having early: it is invisible in the type rules, which say only that fewer
   effects fit where more are expected.

Kotlin's `advance` returns a Boolean and leaves the element in `current()`
rather than returning `T?`, for the reason the whole protocol is tagged: a `T?`
return could not tell "no more" from "the element is null".
