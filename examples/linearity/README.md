# linearity

A `linear struct` is a value the compiler will not let you forget: every one
has to be consumed on every path, exactly once. This program walks the five
things that follow from that, in the order they bite.

Run it:

```bash
cargo run -- run --backend rust   --src examples/linearity/salvo
cargo run -- run --backend kotlin --src examples/linearity/salvo
```

## What to look for

**1 — where the obligation comes from.** `linear` on the struct is the entire
declaration; nothing else in the program opts in. A **discharger** is any
function in the same file that consumes one (`=> !ticket`), and inside it the
obligation still has to end — `discard` is the terminal, and it is legal only
there. That restriction is what keeps the escape hatch from being a hole.

**2 — the obligation moves.** After a consuming call the variable is gone:
using it again is an error naming it, which is exactly the double-free this
type exists to prevent.

**3 — a keeping call borrows it instead.** The deduction clause is the whole
difference: `=> ticket` keeps, `=> !ticket` consumes. A keeping function reads
the value freely and discharges nothing, because the obligation never left the
caller — so `describe` can be called twice and the ticket is still owed
afterwards.

**4 — the obligation belongs to the value.** Reading a field neither
discharges nor damages it. What a linear value may *not* do is hide inside a
composite — tuple component, union arm, list element — because the obligation
cannot yet be tracked through one; each is refused where it is written. A
linear value therefore lives in a local, a parameter or a return value, which
is enough for the resources it is for.

**5 — a generic that carries one.** A plain `<T>` may not be instantiated
with a linear type: nothing in the body would honor the obligation, so the
call is refused. `<T canbe linear>` is the opt-in, and the consuming callback
is `once` — it may be called at most once, which is what a callback that
discharges an obligation has to be.

## Where the errors are

The comments in `salvo/main.sv` name the diagnostics rather than showing them,
because the program has to compile. Each was checked by writing it and reading
what came back — the double use, the plain-`<T>` instantiation, the tuple
component and the variadic list element.

## Companion example

[`throw-and-release/`](../throw-and-release/) shows the same machinery doing
the job it exists for: releasing a resource on a path that leaves early,
including before a call that may throw.
