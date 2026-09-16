# actors

An **actor** is an effect handler bound asynchronously. That sentence is the
whole design: `actor effect Counter { send fn bump(n: Int) }` declares a
protocol, `handler Counting() of Counter` implements it exactly as a
synchronous handler does, and `spawn Counting() capacity 8 on workers` brings
it to life with a mailbox instead of binding it inline. Nothing about the
handler changes; the *binding* is what makes it an actor.

Run it:

```bash
cargo run -- run --backend rust   --src examples/actors/salvo
cargo run -- run --backend kotlin --src examples/actors/salvo
```

## What to look for

**1 — the smallest actor.** `send fn` means the call enqueues and answers
nothing, so a payload always crosses a thread boundary and is always consumed
(`=> !n`). State is the actor's alone and members run one at a time to
completion, which is why `sum = sum + n` needs no lock: the scheduler's
serialization *is* the mutual exclusion. `main` reads the total through
`waitfor`, its own bridge into the surface — it mints a token, hands it over,
and blocks the real thread until the answer arrives.

**2 — the reply token is linear.** `Reply<Int>` is a one-shot answer channel,
and `total` must send to it exactly once on every path or the program does not
compile. That is the guarantee everything else rests on: "answered exactly
once" is a *static* fact here, where the effects literature settles for a
dynamic check.

**3 — an answer that needs another actor.** `Bookkeeping` cannot answer
`report` itself — the number lives in the counter — so it asks and **parks a
continuation**: `replyto reported(label, out)` mints a token aimed at its own
`reported` member, carrying the caller's token as a capture. The activation
then *ends*. When the counter answers, `reported` runs as its own later
activation and fulfils the original token. `main` is not in the loop, and no
thread is blocked anywhere.

Read the handler's dependency too: `handler Bookkeeping() [Counter]` declares
what it needs exactly as a synchronous handler does, and the spawn site
supplies it — `use counter` passes an addr here, and a construction
(`use Counting()`) would have been the same act. The child cannot tell whether
its `Counter` is another actor or a local handler.

**4 — a queue of obligations.** A `Reply<Str>` is linear, so
`Mut List<Reply<Str>>` is linear too: the queue owes, and `drain` is its
terminal. `Desking` is what an actor that cannot answer yet looks like —
`ticket` parks a token, `serve` takes one out with `remove_first` (a move, and
the `None` arm owes nothing), and `close_up` answers everyone still waiting.
Note the last line of that member: an activation may take state out, but it may
not *return* with a hole in it, so the drained queue is replaced. The
obligations belong to the actor until it ends.

The ordering in the output is not luck. Four sends reach one actor, which
serves them in arrival order, one at a time — so the second waiter is
answered by the shutdown drain and the first by `serve`, in that order, with no
synchronisation anywhere in the program.

**5 — death, and watching for it.** Death is a **faulted activation** and
nothing else: there is no `kill`, and a Salvo-level `throw` cannot cross a
member boundary, so an actor in a program with no faults and no platform
handlers cannot die at all. `watch(fragile, gone)` is the entire monitor
surface — one function and an `Exit` struct — because a death notification is
itself an answer, so the reply machinery already carries it. The token is
consumed by the registration, which means forgetting to handle a watch is the
ordinary leak diagnostic rather than a silently dropped registration.

Two things the program shows about the corpse: `Exit.reason` is the *host's*
text (a panic message on Rust, an exception's on Kotlin), so the example prints
only that there was one — it is the single value on this surface that is not
identical across backends; and the send *after* the death is a silent no-op,
which is why holding a stale addr is safe.

**6 — the same handler, bound synchronously.** `use Counting()` runs those
same members inline on `main`'s thread: no mailbox, no scheduler, no addr, and
`bump`/`total` resolve to it unqualified. `waitfor` still works, because the
token is answered *before* `total(out)` returns. The inline total is 9 and the
spawned one is 5 — two bindings, two states, one handler.

## What the generated code looks like

Worth opening `rust/main.rs` and `kotlin/main.kt` side by side:

- **A message type per protocol** (`__Msg_Counter`), one variant per `send fn`,
  owning its payload. It belongs to the *effect*, not the handler, because a
  sender holds an addr and knows only the protocol.
- **An actor body per handler** (`__Actor_Counting`) wrapping the handler
  instance, whose `handle` downcasts a message and calls the member the variant
  names. A handler's state *is* the actor's state.
- **A continuation type** (`__Cont_Ledger`) beside the message type, carrying
  each parked member's captures *minus* the trailing answer, plus a `__parked`
  table on the handler keyed by slot. That pair is section 3's mechanism.
- **A forwarding stub** (`__Stub_Counter`) for the dependency that arrived as
  an addr: it implements the effect by sending, which is how the child stays
  ignorant of what backs its capability.
- **`scheduler.rs` / `scheduler.kt`**, the one shipped runtime file, mounted
  only into a program that spawns. It is a library, not a runtime baked into
  the emitted code: bounded arrival-order queues, run-to-completion
  activations, reply capacity reserved at park time, a per-activation catch that
  turns a fault into a death, and the idle-with-parked-gates report.

The Rust program also prints a panic message on **stderr** when section 5's
actor faults — Rust's default hook does that before the scheduler's catch sees
it, where Kotlin's catch is silent. `expected.txt` is stdout, and stdout is
identical.

## Companion examples

[`effects/`](../effects/) is the synchronous half of this surface: the same
declarations, `use` instead of `spawn`, and interception — which composes
across a spawn boundary too, since a handler may depend on the effect it
implements. [`linearity/`](../linearity/) explains the obligation machinery the
reply token and the queue in section 4 rest on.
