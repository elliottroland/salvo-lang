// Time [time-types] [time-timer]: deadlines and the runtime module's wheel.

//--- a_deadline_lowers_to_the_runtime_modules_wheel
// [time-timer] [kt-time] A deadline is the runtime module's, mirroring the
// Rust backend: `after_nanos` from the default timer, the runtime module's
// lazily bound wheel, and no timer machinery in `SalvoSched`.
// (`TIME_TIMER` in codegen_tests.rs.)
//~ emits(std/time.kt): afterNanos(
//~ emits(std/runtime/timers.kt): val __module_use0: Deadlines by kotlin.lazy {
// The host scheduler keeps no timers; hosttime.kt is emitted.
//~ !emits(scheduler.kt): fun after(
//~ !emits(scheduler.kt): timers
//~ emits(hosttime.kt):
import std.time

fn main() [use, spawn] -> None {
    use StdConsole()
    use DefaultTicker()
    let timers = spawn DefaultTimer() on pool(1)
    let started = tick()
    let first = waitfor fired: Reply<Fired> {
        timers.after(millis(50), fired)
    }
    let waited = between(started, first.at)
    println("waited enough: ${to_millis(waited) >= 50}")
    let second = waitfor again: Reply<Fired> {
        timers.after(millis(10), again)
    }
    println("in order: ${second.at > first.at}")
}
