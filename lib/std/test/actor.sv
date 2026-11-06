// The runtime a `test actor` block runs on: one thread, a clock that moves
// only when nothing else can, and randomness from a seed, so a test about
// actors and timers runs the same way every time and takes no real time.
//
// A plain `test` that spawns actors runs on the real, threaded runtime, as a
// program does. A `test actor` block runs on the **virtual runtime** instead,
// which the harness `salvo test` generates enters before each such test by
// calling [begin_actor_test]. On it:
//
// - **One thread runs everything.** A pool starts no threads of its own; the
//   test's frame runs every actor's work while it waits for an answer.
// - **The clock starts at zero and jumps.** When every actor is waiting and
//   a deadline is pending, the clock moves to the earliest deadline and fires
//   it, so an hour-long timer fires at once and the clock then reads exactly
//   one hour.
// - **Randomness comes from a seed**, written `test actor(seed=7) "…"` and
//   0 when not given, so a run repeats exactly.
// - **Each test starts fresh**: no actors, no queued work and no pending
//   deadline left over from an earlier test.
// - **A wait nothing can answer is a deadlock**, reported by name, since on
//   one thread nobody else could answer it.
//
// ```
// // In a test annex.
// import std.time
//
// test actor "an hour passes at once" {
//     let timer = spawn DefaultTimer() on pool(1)
//     use DefaultTicker()
//     let started = tick()                             // 0 ns
//     let fired = waitfor answer: Reply<Fired> {
//         timer.after(hours(1), answer)
//     }
//     expect_eq(between(started, fired.at).nanos, hours(1).nanos)
// }
// ```
//
// **No host threads.** Work arriving from a thread of the host's would
// arrive off the virtual clock, so an actor test may not open one: a
// platform handler that listens or reads on a thread of its own (the host's
// network transport, a host stream's `receive`) stops the test with a report.
// Use the in-memory fakes (`MemTransport`, `MemFs`) instead.
//
// **A trap in the test's own frame ends the run.** A plain test is run
// inside the harness's trap catch; an actor test is not, since its body
// spawns and the catch takes a fn value, which cannot. The runner reports
// the test as having died and runs the remaining tests in a fresh process.
// A trap inside an actor is that actor's death, as anywhere.
// @compilerrules [test-actor, test-recover]

import std.runtime
import std.runtime.timers

// Puts the program on a fresh virtual runtime for the next actor test: a
// new scheduler with no actors and no pending deadline, the clock at zero,
// and randomness drawn from [seed].
//
// The generated harness calls this before each `test actor` block; a
// program has no reason to.
//
// @param seed  the seed the test declared, or 0
// @compilerrules [test-actor]
export fn begin_actor_test(seed: Long) [] -> None => !seed {
    enter_virtual(seed)
    reset_timers()
}
