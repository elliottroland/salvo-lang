# Time

Time is a std module rather than a language feature, and it is not part of `core`, so it is imported. The surface is a dozen names, so one line brings all of it:

```
import time
```

**Three types, one representation.** A `Duration` is a span, an `Instant` is a point on the wall clock, and a `Tick` is a point on the monotonic clock. Each is an ordinary struct holding a single `nanos: Long`, which is what makes every value canonical: a two-field `{secs, nanos}` form would let the same span be written two ways, and struct equality is structural, so the two would compare unequal.

**Why there are two kinds of point, and not one.** The wall clock jumps — NTP steps and slews it, and a suspend advances it while the monotonic clock stops — so a deadline measured against it would move under the program. The monotonic clock never jumps, but its origin is arbitrary and no API reveals it, so a `Tick` on its own cannot say what day it is. Keeping them apart makes the mistake a *type* error rather than a plausible wrong answer: `between` takes two points of one timeline, and comparing an `Instant` with a `Tick` is refused where it is written. Measure with ticks; record and report with instants.

The arithmetic is named functions, because operators are numeric-only: `millis(1500)` and its siblings build a span, `to_millis` and its siblings read one back, `plus`/`minus`/`times`/`abs` compute, and `between(start, end)` answers the span from one point to another — signed, so the argument order *is* the direction of the answer. Interpolating a `Duration` prints the largest unit that divides it exactly (`1500ms`, `120s`, `37ns`).

**Reading a clock is a capability.** A function whose answer depends on when it was called has a dependency, and Salvo's dependencies live in signatures:

```
effect Ticker { fn tick() -> Tick }               // the monotonic clock
effect Clock  { fn now() -> Instant  … }           // the wall clock
```

`DefaultTicker` and `DefaultClock` are the machine's. `Clock` also carries the bridge between the timelines — `to_instant(at)` and `to_tick(at)` — and those are *members* rather than free functions because the answer is an estimate: nothing exposes the monotonic origin, so relating the two means reading both clocks at nearly the same moment and keeping the difference, which then drifts. A handler owns that correlation, which is what makes the conversion available at all, and exact in a test.

**A deadline is a message.** Sleeping is an `actor effect`, and it could not be anything else: a handler runs to completion and cannot block mid-body, so "in two seconds" can only mean "park a continuation and resume me".

```
struct Fired { at: Tick }

actor effect Timer {
    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done
}
```

`after` takes the continuation and answers nothing; the caller mints one with `replyto` inside a handler, or `waitfor` anywhere that may occupy its thread. The fire carries a `Tick`, because a deadline that moved when the wall clock was adjusted would not be a deadline. `DefaultTimer` is spawned like any actor (`spawn DefaultTimer() on pool(1)`) and keeps one deadline structure and one thread for the whole program, however many deadlines are outstanding. There is no cancellation: a timer nobody wants any more fires into a continuation that finds its work already done.

## Time in a test

Because every clock is an effect, a test replaces it — and `ManualTime` is the replacement std ships, in pure Salvo: virtual time starts at zero and moves only when told, so a program that would wait two seconds runs in microseconds and prints the same thing every time. It wears two faces, so a spawn answers one addr per face and least authority falls out of the types — the code under test is handed the `Timer` and *cannot* reach `advance`:

```
let (timer, ctl) = spawn ManualTime() on p
let sessions = spawn Sessions() with timer on p

sessions.open(order, answer)
waitfor settled: Reply<Idle> { on_idle(p, settled) }   // let it register first
ctl.advance(millis(2500))
```

The middle line is not optional in spirit: `advance` is a message like any other, so without it the advance races the `after` the code under test has not registered yet. Quiescence is the sequencing tool, which is what `on_idle` is for.

**The posture the module is built around is to pass time rather than read it.** A `Fired` carries the `at` it came due at; a request can be stamped where it enters the system. A function that takes its times as parameters declares no effect, needs no handler, and is tested by being called:

```
fn verdict(started: Tick, at: Tick, budget: Duration) [] -> Str { … }
```

That is a stance rather than a mechanism, and the reason for it is not only testability: a function that reads an ambient clock in the middle of its body has an answer that depends on when the scheduler ran it, which inside an actor is a race with its own mailbox. Passing the time in removes the dependency instead of faking it.

Where a reading genuinely cannot be passed in — how long a handler's *own* work took is not something its caller could have stamped — the reading and the deadlines must agree, and they are made to agree by writing a clock over the timer the test advances. A reading is a deadline of zero:

```
handler TestTicker(timer: Addr<Timer>) of Ticker {
    fn tick() -> Tick {
        let fired = waitfor answer: Reply<Fired> { timer.after(nanos(0), answer) }
        return fired.at
    }
}
```

One virtual clock is then behind both, so a measurement taken across a two-second virtual nap is exactly two seconds. Two existing rules shape this handler and are worth reading off it. The timer arrives as a **value** — an `Addr<Timer>` constructor parameter — because a handler with dependencies of its own cannot be *constructed* in a spawn's `with` clause: there is no scope on the child to resolve them from. And the wait is declared nowhere: occupancy is inferred, the wait serves its pool while it waits, and the deadlock graph prices any cycle it could close. The round trip per reading is why this is the posture of last resort rather than the default.
