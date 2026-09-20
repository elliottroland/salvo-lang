# time

Salvo's time surface is a std module rather than a language feature: one
`import time` brings three types, two clock effects, a timer, and a fake. What
the module is *designed around* is a posture — **prefer time as data** — and
this program is arranged to show why, and what to do in the cases where data is
not enough.

```bash
cargo run -- run --backend rust   --src examples/time/salvo
cargo run -- run --backend kotlin --src examples/time/salvo
```

## What to look for

**1 — a span, and two kinds of point.** `Duration` is a span in nanoseconds and
the constructors name their unit (`millis(1500)`); interpolating one prints the
largest unit that divides it exactly, so `1500ms` and `3s` are the same
function's output. The two *point* types are kept apart deliberately. An
`Instant` is a wall-clock reading — what day and second it is — and the wall
clock jumps: NTP steps and slews it, and a suspend advances it while the
monotonic clock stops. A `Tick` is a monotonic reading, which never jumps but
has no epoch, so it cannot say what day it is. Measure with ticks, record and
report with instants; mixing them is a type error rather than a plausible wrong
answer, which is the whole reason there are two structs and not one with a
qualifier.

**2 — reading a clock is a capability.** `overdue(started, budget) [Ticker]`
carries its dependency in its signature, because a function whose answer depends
on *when* it was called has one. That is what makes it replaceable:
`SteppingTicker` is a handler of your own whose readings come from its
constructor, `use` shadows the machine's `DefaultTicker` with it, and the three
`overdue` calls then walk a scripted clock forward half a second at a time —
`false false true`, the same three answers on every run and on both backends.
For code that only measures locally, this is the whole testing story: no timer,
no actors, no virtual time.

**3 — the same decision with no clock at all.** `verdict(started, at, budget)`
takes its times as parameters. It reads no clock, so it declares no effect, so
there is nothing to fake: the test calls it with two ticks and checks the string,
like arithmetic. **This is the posture the module is built to encourage** —
stamp time at the edge of the system and let the stamp travel in the message —
and it is worth stating what it buys, since it is a design stance rather than a
mechanism. A function that reads an ambient clock mid-body is a function whose
answer depends on when the scheduler happened to run it; in an actor that is a
race with your own mailbox. Passing the time in removes the dependency instead
of mocking it.

**4 — a deadline is a message.** `Timer` is an *actor* effect, and it could not
be anything else: a handler runs to completion and cannot block mid-body, so
"in two seconds" can only mean "park a continuation and resume me".
`after(wait, done)` therefore takes the continuation and answers nothing, and
`Sessions` parks one with `replyto expire(...)`. Notice that `Sessions` declares
only `[Timer]` — no `Ticker`, no clock reading anywhere: the caller stamped the
start, the fire carries the tick it came due at (`f.at`), and section 3's pure
function decides. That is section 3's posture applied to an actor, and it is
what most code that "needs the time" actually needs.

**5 — virtual time, in pure Salvo.** `ManualTime` is the fake, and it is
ordinary Salvo you could have written: a list of deadlines, a list of reply
tokens, and time that moves only when told. It wears **two faces** —
`handler ManualTime() of Timer, TimerCtl` — so the spawn answers one addr per
face and least authority falls out of the types: `sessions` is handed the
`Timer` and *cannot* reach `advance`. The two lines before the advance are the
ones to copy:

```
sessions.open(Tick {nanos: 0}, budget, answer)
waitfor settled: Reply<Idle> { on_idle(p, settled) }
ctl.advance(millis(2500))
```

`advance` is a message like any other, so without the middle line it races the
`after` that the code under test has not registered yet. `on_idle` fires when
everything sent has settled, which turns "advance at the right moment" into
"advance once nothing is in flight". A two-and-a-half second deadline is then
observed in microseconds, and the program prints the same thing every time.

**6 — one virtual time, for code that genuinely measures.** Some readings cannot
be passed in: how long a handler's own work took is not something its caller
could have stamped. `Napping` reads `tick()` inside its own activation, so under
a *scripted* ticker it would be comparing an unrelated clock against the timer's
virtual time. `TestTicker` makes them one clock, in six lines:

```
handler TestTicker(timer: Addr<Timer>) [waitfor] of Ticker {
    fn tick() -> Tick {
        let fired = waitfor answer: Reply<Fired> {
            timer.after(nanos(0), answer)
        }
        return fired.at
    }
}
```

A reading is a deadline of zero, so the answer *is* the timer's virtual now, and
`napped 2s` is exact rather than approximately right. Two things make this
writable, and both are visible in the code. `waitfor` occupies the thread until
the fire arrives, which is a capability — `[waitfor]` — that propagates: because
`Napping` binds `TestTicker`, `Napping` carries it too, and the compiler then
requires the actor to run on a thread of its own. `thread()` answers a
`Dedicated Pool` and the `on` clause **consumes** it, so a thread cannot be
handed to two occupants; a wait can therefore only ever occupy its own. And the
timer reaches `TestTicker` as a plain `Addr<Timer>` *constructor parameter*
rather than as a handler dependency, because a handler with dependencies of its
own cannot be **constructed** in a spawn's `with` clause — there is no scope on
the child to resolve them from, so an addr is what crosses.

## The order to reach for these

1. **Pass the time in** (section 3). No effect, no fake, no scheduler.
2. **Script the readings** (section 2) when the code must read a clock but
   nothing else in the test depends on time.
3. **`ManualTime`** (sections 4–5) when deadlines are part of the behaviour
   under test — with `on_idle` before every advance.
4. **`ManualTime` plus a `TestTicker`** (section 6) when a measurement has to
   agree with a deadline. It costs a dedicated thread per waiting actor, which is
   why it is last.

Production is the same program with the machine's handlers bound instead:
`DefaultTicker`, `DefaultClock`, and `spawn DefaultTimer() on pool(1)`. Neither
the code under test nor its signatures change — which is the point of every
clock here being an effect.
