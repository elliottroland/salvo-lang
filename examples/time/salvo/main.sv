// Time: two timelines, deadlines as messages, and how a test decides what
// "now" means.
//
// One `import time` brings the whole module — the types, the two clock effects,
// the timer, and the fake. Nothing here is built into the language: `Duration`
// is a struct, `Ticker` is an effect, `Timer` is an actor effect, and
// `ManualTime` is ordinary Salvo you could have written yourself.
import time

// What this program is chosen to show, in the order it needs it:
//
//   1. spans, and why there are two kinds of point
//   2. reading a clock is a capability — so a test can replace it
//   3. time as data: the posture that needs no clock at all
//   4. a deadline is a message, and the fire carries the time it came due
//   5. virtual time: `ManualTime`'s two faces, settle then advance
//   6. one virtual time, for code that genuinely measures

// ===== 3. time as data =====
//
// The decision takes its times as *parameters*. It reads no clock, so it
// declares no effect, so a test needs no fake for it at all — it is called with
// numbers and checked like arithmetic. This is the posture the standard library
// is built to encourage: stamp time at the edge of the system and let the stamp
// travel.
fn verdict(started: Tick, at: Tick, budget: Duration) [] -> Str {
    let took = between(started, at)
    if took > budget {
        return "late by ${minus(took, budget)}"
    }
    return "in time, ${minus(budget, took)} to spare"
}

// ===== 2. reading a clock is a capability =====
//
// A function whose answer depends on *when* it was called has a dependency, and
// Salvo's dependencies live in signatures: `[Ticker]` is that dependency, and
// `elapsed(since)` is `between(since, tick())`.
fn overdue(started: Tick, budget: Duration) [Ticker] -> Bool {
    return elapsed(started) > budget
}

// A scripted clock: a handler of your own whose readings come from its
// constructor. No timer, no actors, no virtual time — the cheapest determinism
// there is, and enough for code that only measures locally.
handler SteppingTicker(step: Duration) of Ticker {
    at: Long = 0

    fn tick() -> Tick {
        at = at + step.nanos
        return Tick {nanos: at}
    }
}

// ===== 4. a deadline is a message =====
//
// `Timer` is an *actor* effect, and it has to be: a handler runs to completion,
// so it cannot block mid-body, so "in two seconds" can only mean "park a
// continuation and resume me". `after(wait, done)` therefore takes the
// continuation and answers nothing.
//
// This session actor is written in the section-3 posture: the caller stamps the
// start, the fire carries the tick it came due at, and so the actor reads no
// clock anywhere — which is why it needs no `Ticker` in its dependency list.
actor effect Session {
    send fn open(started: Tick, budget: Duration, out: Reply<Str>)
        => !started, !budget, !out
    send fn expire(started: Tick, budget: Duration, out: Reply<Str>, f: Fired)
        => !started, !budget, !out, !f
}

handler Sessions() [Timer] of Session {
    mailbox { capacity: 8 }

    send fn open(started: Tick, budget: Duration, out: Reply<Str>) {
        // Wait a whole second longer than the budget, so the verdict is late.
        after(plus(budget, seconds(1)), replyto expire(started, budget, out))
    }

    send fn expire(started: Tick, budget: Duration, out: Reply<Str>, f: Fired) {
        // `f.at` is the tick the deadline came due at. Time arrived as data, so
        // the pure function from section 3 decides.
        send(out, verdict(started, f.at, budget))
    }
}

// ===== 6. one virtual time =====
//
// Some code genuinely must read the clock *mid-activation*: how long its own
// work took is not something a caller could have stamped for it. Under a
// scripted ticker that reading has nothing to do with the timer's virtual time,
// and a test that measures across a deadline would compare two unrelated
// clocks.
//
// This handler makes them one clock: a reading is a deadline of zero, so the
// answer is the timer's own virtual `now`. `waitfor` occupies the frame until
// the fire arrives — no declaration anywhere: occupancy is inferred, the wait
// serves its pool while it waits, and the deadlock graph prices the cycles.
//
// The timer arrives as a plain `Addr<Timer>` constructor parameter rather than
// as a handler dependency, because a handler with dependencies of its own cannot
// be *constructed* in a spawn's `use` clause: there is no scope on the child to
// resolve them from, so an addr is what crosses.
handler TestTicker(timer: Addr<Timer>) of Ticker {
    fn tick() -> Tick {
        let fired = waitfor answer: Reply<Fired> {
            timer.after(nanos(0), answer)
        }
        return fired.at
    }
}

// The code under test: it naps, and reports how long the nap took by its own
// reading of the clock. Under `TestTicker` that answer is exact.
actor effect Sleeper {
    send fn nap(wait: Duration, out: Reply<Str>) => !wait, !out
    send fn woke(started: Tick, out: Reply<Str>, f: Fired) => !started, !out, !f
}

handler Napping() [Timer, Ticker] of Sleeper {
    mailbox { capacity: 8 }

    send fn nap(wait: Duration, out: Reply<Str>) {
        after(wait, replyto woke(tick(), out))
    }

    send fn woke(started: Tick, out: Reply<Str>, f: Fired) {
        // Two readings of one clock, taken a virtual nap apart.
        send(out, "napped ${elapsed(started)}")
    }
}

fn main() [use, spawn] -> None {
    use StdOutConsole()

    // ===== 1. spans, and two kinds of point =====
    //
    // A `Duration` is a span, in nanoseconds, and the constructors name their
    // unit. Interpolating one prints the largest unit that divides it exactly.
    let budget = millis(1500)
    println("budget ${budget}, doubled ${times(budget, 2)}, in millis ${to_millis(budget)}")

    // The two point types are kept apart on purpose. An `Instant` is a wall-clock
    // reading — what day and second it is — and the wall clock jumps: NTP steps
    // and slews it, a suspend moves it while the monotonic clock stops. A `Tick`
    // is a monotonic reading, which never jumps but has no epoch, so it cannot
    // say what day it is. Measure with ticks; record and report with instants.
    // Mixing them is a type error rather than a wrong answer.
    let stamp = epoch_milli(1700000000000)
    println("stamp ${to_epoch_second(stamp)}s, a minute later ${to_epoch_second(plus(stamp, minutes(1)))}s")

    // The machine's own clocks are handlers like any other. A real reading cannot
    // promise a number, so what is asserted here is a property of it.
    use DefaultClock()
    use DefaultTicker()
    println("wall clock is set: ${to_epoch_second(now()) > 1600000000}")

    // ===== 2. a scripted clock =====
    //
    // `SteppingTicker` shadows `DefaultTicker` for the rest of this scope, so
    // `overdue` — which reads the clock through `[Ticker]` — now gets the
    // readings this test chose: half a second per call.
    use SteppingTicker(millis(500))
    let started = tick()
    println("overdue after one more read: ${overdue(started, budget)}")
    println("overdue after three: ${overdue(started, budget)} ${overdue(started, budget)} ${overdue(started, budget)}")

    // ===== 3. the same decision, with no clock at all =====
    //
    // No handler, no effect, no fake: two ticks and a span, called like
    // arithmetic.
    println(verdict(Tick {nanos: 0}, Tick {nanos: 1000000000}, budget))
    println(verdict(Tick {nanos: 0}, Tick {nanos: 2000000000}, budget))

    // ===== 4 and 5. virtual time =====
    //
    // `ManualTime` is the fake, and it wears two faces: `Timer` for the code
    // under test and `TimerCtl` for the test. A spawn answers one addr per face,
    // so least authority falls out of the types — `sessions` below is handed the
    // timer and *cannot* reach `advance`.
    let p = pool(1)
    let (timer, ctl) = spawn ManualTime() on p
    let sessions = spawn Sessions() use timer on p

    // `advance` is a message like any other, so it races the `after` the code
    // under test is about to register. `on_idle` is the fix: it fires when
    // everything sent has settled, so the advance cannot arrive early. Time then
    // moves 2.5s, past the 1.5s budget's 2.5s deadline, and fires it.
    let outcome = waitfor answer: Reply<Str> {
        sessions.open(Tick {nanos: 0}, budget, answer)
        waitfor settled: Reply<Idle> {
            on_idle(p, settled)
        }
        ctl.advance(millis(2500))
    }
    println("session: ${outcome}")

    // ===== 6. one virtual time, for code that measures =====
    //
    // `Napping` reads the clock itself, so its `Ticker` is `TestTicker` over the
    // same `ManualTime` — one virtual clock behind both the deadline and the
    // reading. `TestTicker` waits inside `Napping`'s activations, and nothing
    // declares that anywhere: a wait serves the pool it runs on, so no special
    // placement is owed. The `on thread()` here is a *choice* — a thread of
    // its own, which the `on` clause consumes (`thread()` answers a
    // `Dedicated Pool`, and linearity gives it exactly one occupant).
    let sleeper = spawn Napping() use timer, TestTicker(timer) on thread()
    let napped = waitfor answer: Reply<Str> {
        sleeper.nap(seconds(2), answer)
        waitfor settled: Reply<Idle> {
            on_idle(p, settled)
        }
        ctl.advance(seconds(2))
    }
    println(napped)
}
