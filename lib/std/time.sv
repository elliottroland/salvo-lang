// Time: spans of time, points on two clocks, the effects that read the
// clocks, and a timer that fires after a delay.
//
// The module is not part of `std.core`; one line brings all of it:
//
// ```
// import std.time
// ```
//
// **Three types.** A [Duration] is a span of time ("1.5 seconds"). An
// [Instant] is a point on the **wall clock**, the clock that says what day
// and second it is. A [Tick] is a point on the **monotonic clock**, which
// only ever moves forward and is what intervals are measured with. Each is
// a struct holding one `Long` of nanoseconds, so values are plain data:
// compared with `==` and `<`, used as map keys, passed in messages.
//
// ```
// let budget = millis(1500)
// let longer = budget + seconds(2)                       // 3500ms
// let late = longer > budget                             // true
// let stamp = epoch_milli(1700000000000L)
// let next_minute = stamp + minutes(1)
// println("${longer}, ${to_epoch_second(next_minute)}")  // 3500ms, 1700000060
// ```
//
// The arithmetic is named functions, and the ones named for an operator are
// that operator: `plus`, `minus` and `times` are `+`, `-` and `*`, so
// `d1 + d2`, `at - d` and `d * 3L` all work. Interpolating a `Duration`
// prints it with the largest unit that divides it exactly (`1500ms`, `2s`).
//
// **Why two kinds of point.** The wall clock can jump: time
// synchronization steps it and slows it down, and a machine that sleeps
// wakes with its wall clock advanced while its monotonic clock stood still.
// A timeout measured against the wall clock could therefore expire early or
// never. The monotonic clock never jumps, but its starting point is
// arbitrary, so a `Tick` on its own cannot say what time it is. Measure with
// ticks; record and report with instants. Mixing them is a type error:
// [between] takes two points of one clock, and an `Instant` cannot be
// compared with a `Tick`.
//
// **Reading a clock is an effect.** A function whose answer depends on when
// it is called has a dependency, and Salvo puts dependencies in signatures:
// [Ticker] reads the monotonic clock and [Clock] the wall clock.
// [DefaultTicker] and [DefaultClock] are the machine's; a test binds a
// handler of its own to decide what the clock says.
//
// ```
// fn timed(budget: Duration) [Ticker, Console] -> None {
//     let started = tick()
//     println("working")
//     let took = elapsed(started)
//     if took > budget {
//         println("slow: ${took}")
//     }
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use DefaultTicker()
//     timed(millis(100))
// }
// ```
//
// Better still, where a time can be passed as a value, pass it: a function
// that takes its times as parameters reads no clock, declares no effect and
// is tested by calling it.
//
// **Waiting is a message.** A handler runs to completion and cannot block in
// the middle, so "wait two seconds" means "send me a message in two
// seconds": [Timer] is an actor effect whose `after` takes the reply to send.
// [DefaultTimer] is the machine's timer, and [ManualTime] is a timer whose
// time moves only when a test says so.
//
// **What is not here.** Dates, time zones and formatting. A calendar date
// is a view of an [Instant] in a zone, with its own arithmetic ("one month
// later" is not a fixed number of nanoseconds), and is not part of this
// module. An `Instant` covers the years 1678 to 2262, the right window for
// machine events.
// @compilerrules [mod-import-module, col-equality, time-types]

import std.runtime.now_nanos
import std.runtime.timers.after_nanos

// A span of time, in nanoseconds: what a [Timer] waits, what [between]
// answers, and what moves a point along its clock.
//
// Durations are **signed**, so [between] with its arguments the other way
// round answers a negative span rather than failing. A `Long` of
// nanoseconds spans about ±292 years. Make one with [nanos], [micros],
// [millis], [seconds], [minutes] or [hours], and read it back with
// [to_nanos], [to_millis] and the like.
//
// ```
// let d = seconds(2) - millis(500)
// println("${d} = ${to_millis(d)} ms")                   // 1500ms = 1500 ms
// let negative = abs(millis(1) - millis(3))              // 2ms
// ```
// @compilerrules [time-types]
export struct Duration : Ordered<self> by auto, Hashed<self> by auto {
    // The span in nanoseconds; negative for a span that runs backwards.
    nanos: Long
}

// A point on the **wall clock**, as nanoseconds since the Unix epoch
// (1970-01-01T00:00:00Z). [Clock]'s `now` answers one.
//
// An `Instant` is what a program records, logs and compares against a time
// it was given, such as an expiry date. It is the wrong thing to measure an
// interval with, because the wall clock can be adjusted between two
// readings; measure with [Tick].
//
// ```
// let created = epoch_second(1700000000L)
// let expires = created + hours(24)
// println("${to_epoch_milli(expires)}")                  // 1700086400000
// ```
// @compilerrules [time-types]
export struct Instant : Ordered<self> by auto, Hashed<self> by auto {
    // Nanoseconds since the Unix epoch; negative before 1970.
    nanos: Long
}

// A point on the **monotonic clock**, the clock intervals and deadlines are
// measured on. [Ticker]'s `tick` answers one.
//
// The monotonic clock's starting point is arbitrary (the start of the
// process, a boot, any moment) and nothing reveals it, so a `Tick` on its
// own says nothing about *when*. What it is for is intervals: two ticks
// subtract into a [Duration] that no clock adjustment can distort, which is
// why deadlines and [Fired] carry ticks. To learn the wall time a tick
// corresponds to, ask a [Clock] with its `to_instant`, which is an estimate.
//
// ```
// let start = Tick { nanos: 0L }
// let stop = start + millis(250)
// println("${between(start, stop)}")                     // 250ms
// ```
// @compilerrules [time-types]
export struct Tick : Ordered<self> by auto, Hashed<self> by auto {
    // Nanoseconds from the clock's arbitrary starting point.
    nanos: Long
}

// ---------------------------------------------------------------- spans ----

// A span of [n] nanoseconds.
export fn nanos(n: Long) [] -> Duration {
    return Duration {nanos: n}
}

// A span of [n] microseconds.
export fn micros(n: Long) [] -> Duration {
    return Duration {nanos: n * 1000L}
}

// A span of [n] milliseconds.
export fn millis(n: Long) [] -> Duration {
    return Duration {nanos: n * 1000000L}
}

// A span of [n] seconds.
export fn seconds(n: Long) [] -> Duration {
    return Duration {nanos: n * 1000000000L}
}

// A span of [n] minutes.
export fn minutes(n: Long) [] -> Duration {
    return Duration {nanos: n * 60000000000L}
}

// A span of [n] hours.
export fn hours(n: Long) [] -> Duration {
    return Duration {nanos: n * 3600000000000L}
}

// [d] in nanoseconds. This is exact: nanoseconds are what a span holds.
export fn to_nanos(d: Duration) [] -> Long => d {
    return d.nanos
}

// [d] in whole microseconds, truncated toward zero.
export fn to_micros(d: Duration) [] -> Long => d {
    return d.nanos / 1000L
}

// [d] in whole milliseconds, truncated toward zero.
//
// ```
// let ms = to_millis(micros(2999))                       // 2
// ```
export fn to_millis(d: Duration) [] -> Long => d {
    return d.nanos / 1000000L
}

// [d] in whole seconds, truncated toward zero.
export fn to_seconds(d: Duration) [] -> Long => d {
    return d.nanos / 1000000000L
}

// The sum of two spans: what `d1 + d2` means.
// @compilerrules [op-arith]
export fn plus(d1: Duration, d2: Duration) [] -> Duration => d1, d2 {
    return Duration {nanos: d1.nanos + d2.nanos}
}

// [d2] taken off [d1], which is negative when [d2] is the longer: what
// `d1 - d2` means.
// @compilerrules [op-arith]
export fn minus(d1: Duration, d2: Duration) [] -> Duration => d1, d2 {
    return Duration {nanos: d1.nanos - d2.nanos}
}

// [d] repeated [n] times: what `d * n` means.
// @compilerrules [op-arith]
export fn times(d: Duration, n: Long) [] -> Duration => d {
    return Duration {nanos: d.nanos * n}
}

// [d] with its sign removed.
export fn abs(d: Duration) [] -> Duration => !d {
    if d.nanos < 0 {
        return Duration {nanos: 0L - d.nanos}
    }
    return d
}

// The text form of [d]: a whole number and the largest unit that divides
// the span exactly, one of `s`, `ms`, `us` and `ns`. Zero is `0s`, and a
// negative span starts with `-`. This is what interpolating a `Duration`
// prints.
//
// The number is never fractional, so the text loses nothing and reads the
// same on every backend. Seconds is the largest unit used: two minutes is
// `120s`, not `2m`.
//
// ```
// println("${seconds(2)} ${millis(1500)} ${micros(250)} ${nanos(37L)}")   // 2s 1500ms 250us 37ns
// println("${minutes(2)} ${millis(0) - millis(5)}")      // 120s -5ms
// ```
export fn to_str(d: Duration) [] -> Str => d {
    // Minutes and hours invite a composite reading (`1h30m`), which is a
    // formatting decision this function is the wrong size for, and which
    // belongs with a calendar layer.
    if d.nanos < 0 {
        let positive = Duration {nanos: 0L - d.nanos}
        return "-${to_str(positive)}"
    }
    if d.nanos == 0 {
        return "0s"
    }
    if d.nanos % 1000000000 == 0 {
        return "${d.nanos / 1000000000L}s"
    }
    if d.nanos % 1000000 == 0 {
        return "${d.nanos / 1000000L}ms"
    }
    if d.nanos % 1000 == 0 {
        return "${d.nanos / 1000L}us"
    }
    return "${d.nanos}ns"
}

// --------------------------------------------------------------- points ----

// The wall-clock point [n] nanoseconds after the Unix epoch.
export fn epoch_nano(n: Long) [] -> Instant {
    return Instant {nanos: n}
}

// The wall-clock point [n] milliseconds after the Unix epoch. Most systems
// hand out times in epoch milliseconds, so this is the usual way in from
// outside the program.
//
// ```
// let at = epoch_milli(1700000000000L)
// println("${to_epoch_second(at)}")                      // 1700000000
// ```
export fn epoch_milli(n: Long) [] -> Instant {
    return Instant {nanos: n * 1000000L}
}

// The wall-clock point [n] seconds after the Unix epoch.
export fn epoch_second(n: Long) [] -> Instant {
    return Instant {nanos: n * 1000000000L}
}

// [at] as nanoseconds since the Unix epoch.
export fn to_epoch_nano(at: Instant) [] -> Long => at {
    return at.nanos
}

// [at] as whole milliseconds since the Unix epoch, truncated toward zero:
// the number to hand to a system that speaks epoch milliseconds.
export fn to_epoch_milli(at: Instant) [] -> Long => at {
    return at.nanos / 1000000L
}

// [at] as whole seconds since the Unix epoch, truncated toward zero.
export fn to_epoch_second(at: Instant) [] -> Long => at {
    return at.nanos / 1000000000L
}

// The span from [start] to [end] on the wall clock, negative when [end] is
// the earlier. The argument order is the direction of the answer.
//
// ```
// let a = epoch_second(100L)
// let b = epoch_second(160L)
// println("${between(a, b)} ${between(b, a)}")           // 60s -60s
// ```
export fn between(start: Instant, end: Instant) [] -> Duration => start, end {
    return Duration {nanos: end.nanos - start.nanos}
}

// The span from [start] to [end] on the monotonic clock, negative when
// [end] is the earlier. This is the reliable way to measure an interval,
// because no clock adjustment can distort it; [elapsed] measures up to now.
export fn between(start: Tick, end: Tick) [] -> Duration => start, end {
    return Duration {nanos: end.nanos - start.nanos}
}

// The wall-clock point [d] after [at]: what `at + d` means.
// @compilerrules [op-arith]
export fn plus(at: Instant, d: Duration) [] -> Instant => at, d {
    return Instant {nanos: at.nanos + d.nanos}
}

// The wall-clock point [d] before [at]: what `at - d` means.
// @compilerrules [op-arith]
export fn minus(at: Instant, d: Duration) [] -> Instant => at, d {
    return Instant {nanos: at.nanos - d.nanos}
}

// The monotonic point [d] after [at]: what `at + d` means, and how a
// deadline is computed.
// @compilerrules [op-arith]
export fn plus(at: Tick, d: Duration) [] -> Tick => at, d {
    return Tick {nanos: at.nanos + d.nanos}
}

// The monotonic point [d] before [at]: what `at - d` means.
// @compilerrules [op-arith]
export fn minus(at: Tick, d: Duration) [] -> Tick => at, d {
    return Tick {nanos: at.nanos - d.nanos}
}

// --------------------------------------------------------------- clocks ----

// The monotonic clock, as an effect: the capability to measure how long
// something takes.
//
// Declare `[Ticker]` on a function that measures a latency, a timeout or an
// interval. [DefaultTicker] is the machine's clock; a test binds a handler
// of its own to make the readings whatever it wants. In an actor test, the
// machine's clock is the virtual one, which starts at zero.
//
// ```
// handler SteppingTicker(step: Duration) for Ticker {
//     state {
//         at: Long = 0L
//     }
//
//     fn tick() -> Tick {
//         at = at + step.nanos
//         return Tick {nanos: at}
//     }
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     use SteppingTicker(millis(500))
//     let started = tick()
//     println("${elapsed(started)}")                     // 500ms
// }
// ```
// @compilerrules [time-ticker]
export effect Ticker {
    // The monotonic clock's reading now. Two readings subtract into the time
    // that passed between them ([between]); one on its own means nothing.
    fn tick() -> Tick
}

// The wall clock, as an effect: what day and second it is, and the bridge
// between the wall clock and the monotonic one.
//
// Reading it is a capability for the same reason [Ticker] is, and binding a
// handler of your own is how a test decides what "now" means.
//
// The conversions `to_instant` and `to_tick` are **estimates**. Nothing
// reveals the monotonic clock's starting point, so the only way to relate
// the two clocks is to read both at nearly the same moment and keep the
// difference, and that difference drifts as the wall clock is adjusted.
// That is why they belong to a handler: [DefaultClock] measures the
// difference once, when it is made, so its conversions keep order (an
// earlier tick converts to an earlier instant), and a test's handler can
// make them exact.
//
// ```
// use DefaultClock()
// use DefaultTicker()
// let started = now()
// let set = to_epoch_second(started) > 1600000000L       // true on a machine whose clock is set
// let wall = to_instant(tick())                          // the wall time of this tick, estimated
// ```
// @compilerrules [time-clock]
export effect Clock {
    // The wall clock's reading now.
    fn now() -> Instant

    // The estimated wall-clock time of the monotonic reading [at]; see
    // [Clock] for why it is an estimate.
    //
    // Nothing exposes the monotonic clock's origin, so the only way to relate
    // the two timelines is to read both at nearly the same moment and keep
    // the difference. That difference then drifts: the wall clock is slewed
    // and stepped under it, and a suspend moves one clock and not the other.
    // A handler therefore owns the correlation, which is what makes the
    // conversion available at all, and exact in a test.
    //
    // The correlation is taken once, so the conversion is a fixed affine map
    // and preserves order: earlier ticks convert to earlier instants. A
    // handler that re-read the clocks per call would track adjustments better
    // and could answer out of order, which is the worse surprise.
    fn to_instant(at: Tick) -> Instant => at

    // The estimated monotonic reading of the wall-clock time [at], with the
    // same caveats as `to_instant`: how to wait for a deadline that arrived
    // as a wall-clock time.
    fn to_tick(at: Instant) -> Tick => at
}

// How long ago [since] was on the monotonic clock: `between(since,
// tick())`, the shape almost every measurement takes.
//
// ```
// use DefaultTicker()
// let started = tick()
// println("took ${elapsed(started)}")
// ```
// @compilerrules [time-ticker]
export fn elapsed(since: Tick) [Ticker] -> Duration => since {
    return between(since, tick())
}

// The machine's monotonic clock. In an actor test it reads the virtual
// clock the test's scheduler moves, which starts at zero.
//
// ```
// use DefaultTicker()
// let t = tick()
// ```
// @compilerrules [time-ticker]
export handler DefaultTicker() for Ticker {
    fn tick() -> Tick {
        return Tick {nanos: monotonic_nanos()}
    }
}

// Ordinary Salvo over the two readings rather than an intrinsic handler:
// the arithmetic is the same on both backends, so there is nothing for a
// backend to decide, and the drift model is readable where it is
// implemented.

// The machine's wall clock. It relates the two clocks once, when the
// handler is made, by reading both one after the other; `to_instant` and
// `to_tick` use that one measurement from then on, so they keep order but
// do not follow later adjustments of the wall clock. `now` always reads the
// clock live.
//
// ```
// use DefaultClock()
// let at = now()
// println("${to_epoch_milli(at)}")
// ```
// @compilerrules [time-clock]
export handler DefaultClock() for Clock {
    state {
        base_tick: Long = monotonic_nanos()
        base_epoch: Long = epoch_nanos()
    }

    fn now() -> Instant {
        return Instant {nanos: epoch_nanos()}
    }

    fn to_instant(at: Tick) -> Instant {
        return Instant {nanos: base_epoch + (at.nanos - base_tick)}
    }

    fn to_tick(at: Instant) -> Tick {
        return Tick {nanos: base_tick + (at.nanos - base_epoch)}
    }
}

// The monotonic clock's reading, in nanoseconds from an arbitrary starting
// point. It is what [DefaultTicker] and [DefaultClock] are built on; declare
// [Ticker] instead, so the dependency shows in your signature. In an actor
// test it reads the virtual clock the scheduler moves.
// @compilerrules [time-clock, test-actor]
export fn monotonic_nanos() [] -> Long {
    return now_nanos()
}

// The wall clock's reading, in nanoseconds since the Unix epoch. It is what
// [DefaultClock] is built on; declare [Clock] instead.
// @compilerrules [time-clock]
export platform fn epoch_nanos() [] -> Long

// ---------------------------------------------------------------- timer ----

// What a [Timer] sends when a wait is over.
//
// [at] is the monotonic reading the deadline came due at. It is a [Tick]
// rather than an [Instant] because a deadline that moved when the wall
// clock was adjusted would not be a deadline. It is the time the timer
// fired, not the time the reply is handled: the reply waits in the
// receiver's mailbox like any message, so a busy actor reads an `at`
// slightly older than its own `tick` would say.
// @compilerrules [time-timer]
export struct Fired {
    // When the deadline came due.
    at: Tick
}

// A timer, as an actor effect: the capability to have a message sent after
// a delay.
//
// A handler runs to completion and cannot block in the middle of its work,
// so a wait cannot be a call that returns later. `after` instead takes the
// reply to send when the time is up, and returns at once. Inside a handler,
// make the reply with `replyto` naming the message to receive; anywhere
// else, `waitfor` waits for it (and lets the pool run other work while it
// waits).
//
// ```
// fn main() [use, spawn] -> None {
//     use StdConsole()
//     let timer = spawn DefaultTimer() on pool(1)
//     let fired = waitfor answer: Reply<Fired> {
//         timer.after(millis(10), answer)
//     }
//     println("fired at tick ${fired.at.nanos}")
// }
// ```
//
// There is no cancellation: a timer nobody wants any more still fires, and
// the message it sends should find its work already done and do nothing.
// @compilerrules [time-timer, actor-kind]
export actor effect Timer {
    // Sends [done] once at least [wait] has passed. A [wait] of zero or less
    // fires as soon as the scheduler looks, which is still a later
    // activation and never inside the call to `after`.
    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done
}

// The machine's timer. One timer serves the whole program however many
// waits are outstanding, with one deadline structure and one thread, and it
// is spawned like any actor: `spawn DefaultTimer() on pool(1)`.
//
// Its mailbox holds requests that have not been registered yet, not the
// waits themselves: `after` hands the deadline over and returns, so many
// more than 64 waits can be outstanding. In an actor test the waits are on
// the virtual clock, so an hour's timer fires at once.
//
// ```
// fn main() [use, spawn] -> None {
//     use StdConsole()
//     let timer = spawn DefaultTimer() on pool(1)
//     use DefaultTicker()
//     let started = tick()
//     let fired = waitfor answer: Reply<Fired> {
//         timer.after(millis(20), answer)
//     }
//     println("${between(started, fired.at) >= millis(20)}")    // true: at least 20ms passed
// }
// ```
// @compilerrules [time-timer]
export handler DefaultTimer() for Ordered Timer {
    // The runtime's wheel (`runtime.after_nanos`), written in Salvo over a
    // parker, keeps the deadlines.
    mailbox { capacity: 64 }

    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done {
        after_nanos(wait.nanos, done)
    }
}

// ----------------------------------------------------------- manual time ----

// The control side of a [ManualTime]: the message that moves its virtual
// time forward.
//
// It is a separate effect from [Timer] so that holding one says nothing
// about holding the other. Spawning a [ManualTime] answers one address per
// effect: the code under test is handed the `Timer` and cannot move time,
// while the test keeps the `TimerCtl`.
// @compilerrules [time-manual]
export actor effect TimerCtl {
    // Moves virtual time forward by [by], firing every deadline it reaches,
    // in deadline order.
    send fn advance(by: Duration) => !by
}

// A timer whose time moves only when a test says so: the test-side
// counterpart of [DefaultTimer], written in ordinary Salvo with no threads
// and no real waiting.
//
// Virtual time starts at zero and moves only through `advance` (a message
// of [TimerCtl]), so a test of code that waits two seconds runs in
// microseconds and reports the same thing every time. An advance fires the
// deadlines it reaches in deadline order, each carrying the virtual [Tick]
// it was set for, and virtual time stands at each deadline while it fires,
// so a chain of timers sees the times it would see in production. A wait of
// zero or less fires at once, without an advance.
//
// ```
// fn main() [use, spawn] -> None {
//     use StdConsole()
//     let p = pool(1)
//     let (timer, ctl) = spawn ManualTime() on p
//     let fired = waitfor answer: Reply<Fired> {
//         timer.after(seconds(2), answer)
//         waitfor settled: Reply<Idle> {
//             on_idle(p, settled)
//         }
//         ctl.advance(seconds(5))
//     }
//     println("${fired.at.nanos}")                       // 2000000000
// }
// ```
//
// **Settle, then advance.** `advance` is a message like any other, so it
// races the `after` calls of the code under test. Wait for the pool to go
// idle with `on_idle` first, as above, so that every wait has been
// registered before time moves.
//
// **A clock that agrees with the timer.** When the code under test reads a
// clock as well as setting deadlines, the two must agree. A [Ticker] whose
// reading is a wait of zero on this same timer makes them one clock:
//
// ```
// handler TestTicker(timer: eff[Timer]) for Ticker {
//     fn tick() -> Tick {
//         let fired = waitfor answer: Reply<Fired> {
//             timer.after(nanos(0L), answer)
//         }
//         return fired.at
//     }
// }
// ```
//
// std does not ship it, because which effect a test fakes ([Ticker],
// [Clock] or both) is the test's business. Prefer passing time as a value
// over reading it at all; `examples/time/` works through all of this.
// @compilerrules [time-manual, time-coupling]
export handler ManualTime() for Ordered Timer, Ordered TimerCtl {
    mailbox { capacity: 64 }

    // Virtual monotonic nanoseconds. A real [Tick]'s origin is arbitrary, so
    // zero is as honest a start as any.
    state {
        now: Long = 0L

        // The pending deadlines and the tokens that answer them, index-aligned:
        // entry `i` of [deadlines] is when entry `i` of [pending] comes due.
        //
        // Two lists rather than one list of pairs, because a list holding
        // obligations cannot be *read* positionally — `get` would alias a linear
        // element, so only `remove_at` reaches one. Keeping the
        // deadlines in a plain list beside it is what makes "which is earliest" a
        // question this handler can ask at all.
        // @compilerrules [linear-container]
        deadlines: Mut List<Long> = Mut List()
        pending: Mut List<Reply<Fired>> = Mut List()
    }

    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done {
        // [time-coupling] A deadline that is *already* due fires here rather
        // than waiting for an [advance], which is what the real timer does —
        // "as soon as the scheduler looks" — and is what makes
        // `after(nanos(0), done)` a reading of virtual time rather than a park
        // that never ends. A test clock is written on exactly that.
        if wait.nanos <= 0 {
            send(done, Fired {at: Tick {nanos: now}})
        } else {
            add(deadlines, now + wait.nanos)
            add(pending, done)
        }
    }

    send fn advance(by: Duration) => !by {
        let target = now + by.nanos
        while earliest_due(deadlines, target) is Int at {
            let deadline = copy(get(deadlines, at) ?: unreachable!())
            remove_at(deadlines, at)
            // Virtual time stands *at* the deadline while it fires, so a
            // continuation that sets another timer measures from there.
            now = copy(deadline)
            if remove_at(pending, at) is Reply<Fired> token {
                send(token, Fired {at: Tick {nanos: deadline}})
            }
        }
        now = target
    }
}

// The index of the earliest deadline at or before [target], or
// `None` when none is due — [ManualTime]'s ordering, factored out because it is
// the only part of it with a loop.
//
// A linear search: a fake holds a handful of deadlines, and the simple honest
// thing beats a heap nobody will profile.
// @compilerrules [time-manual]
fn earliest_due(deadlines: List<Long>, target: Long) [] -> Int? => deadlines, target {
    let best = -1
    let best_at = 0L
    let i = 0
    while i < size(deadlines) {
        let at = copy(get(deadlines, i) ?: unreachable!())
        if at <= target && (best < 0 || at < best_at) {
            best = copy(i)
            best_at = copy(at)
        }
        i = i + 1
    }
    if best < 0 {
        return None
    }
    return best
}
