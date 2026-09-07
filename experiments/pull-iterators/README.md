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
resumable at once — phase I3 should open by prototyping that shape
(`rangeIncl` with a `defer` inside a nested `for`) before the general
lowering is written.
