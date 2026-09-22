// Time: spans, the two timelines, and the effects that read them.
//
// Not part of `core`, so it is imported — and since the surface is a dozen
// names, one line brings all of it [mod-import-module]:
//
//     import time
//
// **Three types, one representation.** A [Duration] is a span, an [Instant] is
// a point on the wall clock, a [Tick] is a point on the monotonic clock, and
// each is a single `Long` of nanoseconds. One field is deliberate: struct
// equality is structural [col-equality] and fields are public, so a
// two-field `{secs, nanos}` form would make non-canonical values
// constructible — `{secs: 1, nanos: 0}` and `{secs: 0, nanos: 1000000000}`
// mean one span and would compare unequal. With nanoseconds in a `Long`,
// every value is canonical by construction, and both backends carry an
// `i64` / `Long` [time-types].
//
// **Why two point types.** The wall clock can jump — NTP steps and slews it,
// a suspend advances it while the monotonic clock stops — so a deadline
// measured against it would move under the program. The monotonic clock never
// jumps but has no epoch, so it cannot say what day it is. Fusing them is
// what Go's `time.Time` does, and the price is a documented rule about when
// the monotonic reading is silently dropped. Keeping them apart makes the
// mistake a type error instead: `between` takes two of *one* timeline, and
// comparing an `Instant` with a `Tick` is refused where it is written, since
// equality demands the same base type [col-equality].
//
// **What is not here.** Dates, zones and formatting: a calendar type
// (`DateTime`) is a *view* of an [Instant] in a zone, with its own arithmetic
// ("one month later" is not a number of nanoseconds), and it is a separate
// design. [Instant]'s range in nanoseconds is 1678–2262, which is the right
// window for machine events; historical dates belong to the calendar layer.
//
// Deadlines are monotonic, so [Timer] fires carry a [Tick]. Reading the wall
// clock is the [Clock] effect and reading the monotonic clock is [Ticker] —
// both of them capabilities, because a function that secretly reads a clock
// is a function whose answer depends on when you called it.

// A span of time, in nanoseconds, and the currency of every time API here:
// `after` takes one, `between` answers one, and both timelines add one.
//
// **Signed**, which is what makes `between` total: arguments in the wrong
// order answer a negative span rather than trapping or clamping at zero.
// A `Long` of nanoseconds spans ±292 years.
export struct Duration : auto Ordered<self>, auto Hashed<self> {
    nanos: Long
}

// A point on the **wall clock**, as nanoseconds since the Unix epoch
// (1970-01-01T00:00:00Z), and what [Clock] answers.
//
// It is what a program records, logs and compares against a deadline it was
// *given*; it is the wrong thing to measure an interval with, because the
// clock under it can be adjusted between two readings. Measure with [Tick].
export struct Instant : auto Ordered<self>, auto Hashed<self> {
    nanos: Long
}

// A point on the **monotonic clock**, and what [Ticker] answers.
//
// The origin is arbitrary — a process start, a boot, an unspecified moment
// possibly in the future — and no API reveals it, so a `Tick` on its own says
// nothing about *when*. What it is for is intervals: two of them subtract into
// a [Duration] that no clock adjustment can distort, which is why deadlines
// and [Fired] payloads are expressed in ticks. To learn the wall time a tick
// happened at, ask a [Clock] with [to_instant] — an estimate, for the reasons
// documented there.
export struct Tick : auto Ordered<self>, auto Hashed<self> {
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

// [d] in whole nanoseconds, which is exact: nanoseconds are the
// representation.
export fn to_nanos(d: Duration) [] -> Long {
    return d.nanos
}

// [d] in whole microseconds, truncated toward zero.
export fn to_micros(d: Duration) [] -> Long {
    return d.nanos / 1000L
}

// [d] in whole milliseconds, truncated toward zero.
export fn to_millis(d: Duration) [] -> Long {
    return d.nanos / 1000000L
}

// [d] in whole seconds, truncated toward zero.
export fn to_seconds(d: Duration) [] -> Long {
    return d.nanos / 1000000000L
}

// The sum of two spans.
export fn plus(d1: Duration, d2: Duration) [] -> Duration {
    return Duration {nanos: d1.nanos + d2.nanos}
}

// [d2] taken off [d1], which may be negative.
export fn minus(d1: Duration, d2: Duration) [] -> Duration {
    return Duration {nanos: d1.nanos - d2.nanos}
}

// [d] repeated [n] times.
export fn times(d: Duration, n: Long) [] -> Duration {
    return Duration {nanos: d.nanos * n}
}

// [d] with its sign removed.
export fn abs(d: Duration) [] -> Duration {
    if d.nanos < 0 {
        return Duration {nanos: 0L - d.nanos}
    }
    return d
}

// The text form of [d]: an integer and the largest unit that divides it
// exactly — `2s`, `1500ms`, `250us`, `37ns`, `0s`. Deliberately never
// fractional, so the rendering loses nothing and does not depend on how a
// backend prints a floating-point number.
//
// Seconds is the largest unit it will use: two minutes reads `120s`, not `2m`.
// Minutes and hours invite a *composite* reading (`1h30m`), which is a
// formatting decision this function is the wrong size for — and which belongs
// with the calendar layer, where the units have calendars behind them.
export fn to_str(d: Duration) [] -> Str {
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

// The wall-clock point [n] milliseconds after the Unix epoch — the epoch
// number most systems hand out, so this is the usual way in from the outside
// world.
export fn epoch_milli(n: Long) [] -> Instant {
    return Instant {nanos: n * 1000000L}
}

// The wall-clock point [n] seconds after the Unix epoch.
export fn epoch_second(n: Long) [] -> Instant {
    return Instant {nanos: n * 1000000000L}
}

// [at] as nanoseconds since the Unix epoch.
export fn to_epoch_nano(at: Instant) [] -> Long {
    return at.nanos
}

// [at] as whole milliseconds since the Unix epoch, truncated toward zero —
// the number to hand to a system that speaks epoch millis.
export fn to_epoch_milli(at: Instant) [] -> Long {
    return at.nanos / 1000000L
}

// [at] as whole seconds since the Unix epoch, truncated toward zero.
export fn to_epoch_second(at: Instant) [] -> Long {
    return at.nanos / 1000000000L
}

// The span from [start] to [end] — negative when [end] is the earlier of the
// two, since a [Duration] is signed. Named for how it reads at the call site:
// the argument order *is* the direction of the answer.
export fn between(start: Instant, end: Instant) [] -> Duration {
    return Duration {nanos: end.nanos - start.nanos}
}

// The span from [start] to [end] on the monotonic clock — the honest way to
// measure an interval, because no clock adjustment can distort it.
export fn between(start: Tick, end: Tick) [] -> Duration {
    return Duration {nanos: end.nanos - start.nanos}
}

// [d] after [at].
export fn plus(at: Instant, d: Duration) [] -> Instant {
    return Instant {nanos: at.nanos + d.nanos}
}

// [d] before [at].
export fn minus(at: Instant, d: Duration) [] -> Instant {
    return Instant {nanos: at.nanos - d.nanos}
}

// [d] after [at].
export fn plus(at: Tick, d: Duration) [] -> Tick {
    return Tick {nanos: at.nanos + d.nanos}
}

// [d] before [at].
export fn minus(at: Tick, d: Duration) [] -> Tick {
    return Tick {nanos: at.nanos - d.nanos}
}

// --------------------------------------------------------------- clocks ----

// [time-ticker] The monotonic clock: a capability, because a function whose
// answer depends on *when* it was called has a dependency, and Salvo's
// dependencies live in signatures.
//
// This is the effect to bind for measuring — a latency, a timeout, an
// interval — and the one a test replaces to make those measurements
// deterministic.
export effect Ticker {
    // The monotonic clock's reading now. Two readings subtract into the time
    // that passed between them ([between]); one on its own means nothing.
    fn tick() -> Tick
}

// [time-clock] The wall clock: what day and second it is, as an [Instant],
// plus the bridge between the two timelines.
//
// Reading it is a capability for the same reason [Ticker] is, and replacing it
// is how a test decides what "now" means.
export effect Clock {
    // The wall clock's reading now.
    fn now() -> Instant

    // The wall-clock time [at] happened at — an **estimate**, and the reason
    // this is a member of an effect rather than a free function.
    //
    // Nothing exposes the monotonic clock's origin, so the only way to relate
    // the two timelines is to read both at nearly the same moment and keep
    // the difference. That difference then drifts: the wall clock is slewed
    // and stepped under it, and a suspend moves one clock and not the other.
    // A handler therefore owns the correlation, which is what makes the
    // conversion available at all — and exact in a test, where the handler is
    // told what it is.
    //
    // The correlation is taken once, so the conversion is a fixed affine map
    // and preserves order: earlier ticks convert to earlier instants. A
    // handler that re-read the clocks per call would track adjustments better
    // and could answer out of order, which is the worse surprise.
    fn to_instant(at: Tick) -> Instant => at

    // The monotonic reading that matches the wall-clock time [at], by the
    // same estimate and with the same caveats as [to_instant]. Useful for
    // "how long until this deadline I was handed", where the deadline arrived
    // as a wall-clock time and the waiting must be monotonic.
    fn to_tick(at: Instant) -> Tick => at
}

// [time-ticker] How long ago [since] was — `between(since, tick())`, which is
// the shape almost every measurement takes.
export fn elapsed(since: Tick) [local Ticker] -> Duration {
    return between(since, tick())
}

// [time-ticker] The machine's monotonic clock.
export handler DefaultTicker() of Ticker {
    fn tick() -> Tick {
        return Tick {nanos: monotonic_nanos()}
    }
}

// [time-clock] The machine's wall clock, with the correlation between the two
// timelines taken once — here, at construction, where the two readings are as
// close together as a pair of calls can be.
//
// Ordinary Salvo over the two readings rather than an intrinsic handler:
// the arithmetic is the same on both backends, so there is nothing for a
// backend to decide, and the drift model above is readable where it is
// implemented.
export handler DefaultClock() of Clock {
    base_tick: Long = monotonic_nanos()
    base_epoch: Long = epoch_nanos()

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

// [time-clock] The monotonic clock's reading, in nanoseconds from an
// arbitrary origin. The plumbing under [DefaultTicker] and [DefaultClock] —
// bind [Ticker] instead, so the dependency is visible in your signature.
export intrinsic fn monotonic_nanos() [] -> Long

// [time-clock] The wall clock's reading, in nanoseconds since the Unix epoch.
// The plumbing under [DefaultClock]; bind [Clock] instead.
export intrinsic fn epoch_nanos() [] -> Long

// ---------------------------------------------------------------- timer ----

// [time-timer] A deadline that has passed, and what [Timer]'s answer carries.
//
// [at] is the monotonic reading the deadline came due at — a [Tick] rather
// than an [Instant], because a deadline that moved when the wall clock was
// adjusted would not be a deadline. It is the time the timer *fired*, not the
// time the continuation runs: the answer is queued like every other, so a busy
// actor reads a slightly older `at` than its own [tick] would say.
export struct Fired {
    at: Tick
}

// [time-timer] Sleeping, as an effect — the capability to have something
// happen later.
//
// An **actor effect**, which is what run-to-completion leaves: a handler
// cannot block mid-body [actor-kind], so "wait for two seconds" can only mean
// "park a continuation and be resumed". `after` therefore takes the
// continuation rather than returning anything, and the caller mints it with
// `replyto` (in a handler) or `waitfor` (anywhere — a wait needs no
// declaration, and serves its pool while it waits).
//
// There is no cancellation: a timer nobody wants any more fires into a
// continuation that finds its work already done — one no-op activation, the
// same shape as losing a race. A cancel handle can be added if that ever
// measures.
export actor effect Timer {
    // Consume [done] once at least [wait] has passed. A `wait` of zero or less
    // fires as soon as the scheduler looks, which is still a later activation
    // and never a call inside `after`.
    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done
}

// [time-timer] The machine's timer: one deadline structure and one thread for
// the whole program, however many deadlines are outstanding.
//
// Spawned like any actor — `spawn DefaultTimer() on pool(1)` — and the
// mailbox bounds *registrations*, not deadlines: `after` hands the deadline to
// the runtime and returns, so the queue only ever holds requests that have not
// been registered yet.
export handler DefaultTimer() of Timer {
    mailbox { capacity: 64 }

    send fn after(wait: Duration, done: Reply<Fired>) => !wait, !done {
        fire_after(wait, done)
    }
}

// [time-timer] Hands a deadline to the runtime: [done] is consumed when
// [wait] has passed. [DefaultTimer]'s plumbing, and the one function here that
// is not ordinary Salvo — a deadline needs the scheduler, which is the
// backend's.
//
// Holding a `Reply<Fired>` is what it takes to call this, so it cannot
// manufacture time out of nothing; the surface to program against is [Timer],
// which a test can replace.
intrinsic fn fire_after(wait: Duration, done: Reply<Fired>) [] -> None => wait, !done

// ----------------------------------------------------------- manual time ----

// [time-manual] The administrative face of a fake clock: the protocol that
// *moves* virtual time, kept separate from [Timer] so that holding one says
// nothing about holding the other.
//
// That separation is the point of the two-face handler below. A spawn answers
// an addr per face, so the code under test is handed the [Timer] and cannot
// reach `advance`, while the test keeps the control addr — least authority
// falling out of the types rather than out of discipline.
export actor effect TimerCtl {
    // Move virtual time forward by [by], firing every deadline it passes, in
    // deadline order.
    send fn advance(by: Duration) => !by
}

// [time-manual] A timer that only moves when told to: `MemFs`'s answer applied
// to time, and ordinary Salvo — no runtime, no threads, no sleeping in a test.
//
// Virtual time starts at zero and advances only through [advance], so a test
// that would otherwise wait two seconds runs in microseconds and always reports
// the same thing. Deadlines fire in deadline order, each carrying the virtual
// [Tick] it was set for, and virtual `now` moves *to* each deadline as it fires
// — so a chain of timers sees the times it would see in production.
//
// One caveat, and it is the reason [on_idle] exists: `advance` is a message
// like any other, so it races the `after` registrations of the code under test.
// Sequence the test with a quiescence hook — "everything I sent has settled" —
// and then advance.
//
// [time-coupling] Where the code under test *reads* a clock as well as setting
// deadlines, the two must agree, and the way to make them agree is a handler of
// your own whose reading is a deadline of zero on this very timer:
//
//     handler TestTicker(timer: Addr<Timer>) of Ticker {
//         fn tick() -> Tick {
//             let fired = waitfor answer: Reply<Fired> {
//                 timer.after(nanos(0), answer)
//             }
//             return fired.at
//         }
//     }
//
// std does not ship it: it is six lines, and which effect a test fakes —
// [Ticker], [Clock] or both — is the test's business. Prefer passing time as
// data over reading it at all; see `examples/time/`.
export handler ManualTime() of Timer, TimerCtl {
    mailbox { capacity: 64 }

    // Virtual monotonic nanoseconds. A real [Tick]'s origin is arbitrary, so
    // zero is as honest a start as any.
    now: Long = 0L

    // The pending deadlines and the tokens that answer them, index-aligned:
    // entry `i` of [deadlines] is when entry `i` of [pending] comes due.
    //
    // Two lists rather than one list of pairs, because a list holding
    // obligations cannot be *read* positionally — `get` would alias a linear
    // element, so only `remove_at` reaches one [linear-container]. Keeping the
    // deadlines in a plain list beside it is what makes "which is earliest" a
    // question this handler can ask at all.
    deadlines: Mut List<Long> = mut_list_of()
    pending: Mut List<Reply<Fired>> = mut_list_of()

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
            let deadline = copy(get(deadlines, at)!)
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

// [time-manual] The index of the earliest deadline at or before [target], or
// `None` when none is due — [ManualTime]'s ordering, factored out because it is
// the only part of it with a loop.
//
// A linear search: a fake holds a handful of deadlines, and the simple honest
// thing beats a heap nobody will profile.
fn earliest_due(deadlines: List<Long>, target: Long) [] -> Int? => deadlines, target {
    let best = -1
    let best_at = 0L
    let i = 0
    while i < size(deadlines) {
        let at = copy(get(deadlines, i)!)
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
