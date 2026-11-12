// What runs a property test: the driver that hands out each run's
// randomness, keeps the simplest failure found, and shrinks it.
//
// A `test property(runs=200) "…"(xs: List<Int>) { … }` block becomes a
// function of its parameters and a function that draws them and calls it;
// the harness `salvo test` generates loops over this module's driver,
// [PropertyRun], asking it for each run's [Rng] and telling it how the run
// went. A test file has no reason to call these itself, except [check], the
// same machinery for a property written inside an ordinary test:
//
// ```no-check
// test "reversing twice gives the list back" {
//     check(100, (xs: List<Int>) -> {
//         expect_eq(reverse(reverse(xs)), xs)
//     })
// }
// ```
//
// **How a failure shrinks.** A run's randomness is a list of draws, and a
// generator maps smaller draws to simpler values ([std.test.random]). Once a
// run fails, the driver replays the property over smaller lists of draws —
// shorter, with chunks removed or zeroed, with single draws halved — and
// keeps each that still fails, until nothing smaller does or 1,000 replays
// have been spent. The values a replay makes are made by the generators, so
// they keep every claim their types make.
//
// **The final replay** runs the simplest failure once more and records its
// parameters for the report. A failure that does not happen again is
// reported as flaky: the property reached something that is not decided by
// its draws.
// @compilerrules [test-property, prop-rng]

import std.runtime
import std.runtime.timers
import std.test
import std.test.random
import std.throw

// The state of one property test's runs: the live runs, then the shrinking
// of the first failure, then the final replay.
// @compilerrules [test-property]
export struct PropertyRun canbe Mut {
    runs: Int
    seed: Long
    // Live runs that passed.
    done: Int
    // 0 live, 1 shrinking, 2 the final replay, 3 over.
    phase: Int
    // The simplest failing draws so far, and that run's message.
    best: List<Long>
    message: Str
    // Which live run failed first.
    found_at: Int
    // The replays still to try, simpler than `best`.
    queue: Mut List<List<Long>>
    // What the replay in flight replays.
    current: List<Long>
    shrinks: Int
    steps: Int
    // The final replay's parameters and outcome.
    shown: List<Str>
    last: Str?
}

// A fresh driver for [runs] runs from [seed].
// @compilerrules [test-property]
export fn property_run(runs: Int, seed: Long) [] -> Mut PropertyRun {
    return Mut PropertyRun {
        runs: runs,
        seed: seed,
        done: 0,
        phase: 0,
        best: List<Long>(),
        message: "",
        found_at: 0,
        queue: Mut List<List<Long>>(),
        current: List<Long>(),
        shrinks: 0,
        steps: 0,
        shown: List<Str>(),
        last: None
    }
}

// Whether there is another run to make, readying it: a live run, a
// shrinking replay, or the final replay.
// @compilerrules [test-property]
export fn next_attempt(run: Mut PropertyRun) [] -> Bool => run: Mut {
    if run.phase == 0 {
        return run.done < run.runs
    }
    if run.phase == 1 {
        if run.steps < 1000 && size(run.queue) > 0 {
            run.current = remove_first(run.queue) ?: List<Long>()
            run.steps = run.steps + 1
            return true
        }
        run.phase = 2
        run.current = copy(run.best)
        return true
    }
    return run.phase == 2
}

// The randomness of the run [next_attempt] readied: a live source seeded
// for this run, or the draws a replay replays.
// @compilerrules [test-property]
export fn attempt_rng(run: PropertyRun) [] -> Mut Rng => run {
    if run.phase == 0 {
        let seeder = live_rng(run.seed + to_long(run.done))
        return live_rng(long(seeder))
    }
    return replay_rng(copy(run.current))
}

// Whether this run's parameters are wanted for the report: on the final
// replay.
// @compilerrules [test-property]
export fn wants_shown(run: PropertyRun) [] -> Bool => run {
    return run.phase == 2
}

// Records how the run [next_attempt] readied went: its failure, or `None`;
// the draws it made; and, on the final replay, its parameters as text.
// @compilerrules [test-property]
export fn record(run: Mut PropertyRun, failure: Str?, draws: List<Long>, shown: List<Str>) [] -> None
=> run: Mut, !failure, !draws, !shown {
    if run.phase == 0 {
        if failure is Str {
            run.phase = 1
            run.best = trimmed(draws)
            run.message = failure
            run.found_at = run.done + 1
            refill(run)
        } else {
            run.done = run.done + 1
        }
        return
    }
    if run.phase == 1 {
        if failure is Str {
            let t = trimmed(draws)
            if simpler(t, run.best) {
                run.best = t
                run.message = failure
                run.shrinks = run.shrinks + 1
                refill(run)
            }
        }
        return
    }
    if run.phase == 2 {
        run.last = failure
        run.shown = shown
        run.phase = 3
    }
}

// The report's lines when the property failed — the failure first, then
// where it was found and how far it shrank, then the parameters — and none
// when it held.
// @compilerrules [test-property]
export fn verdict(run: PropertyRun) [] -> List<Str> => run {
    let out: Mut List<Str> = Mut List<Str>()
    if run.found_at == 0 {
        return out
    }
    add(out, copy(run.last ?: run.message))
    let times = if run.shrinks == 1 {
        "once"
    } else {
        "${run.shrinks} times"
    }
    add(out, "falsified after ${run.found_at} of ${run.runs} runs, shrunk ${times}")
    for line in run.shown {
        add(out, "  ${line}")
    }
    return out
}

// Whether the property failed only once: its simplest failure passed when
// replayed, so something besides its draws decided it.
// @compilerrules [test-property]
export fn flaky(run: PropertyRun) [] -> Bool => run {
    return run.found_at > 0 && run.last is None
}

// A property's own seed: the run's, mixed with the test's id, so that
// running one test with a filter draws what the full run drew. FNV-1a over the
// id's bytes, then mixed with the run's seed as splitmix64 does, in wrapping
// `Long`s, so it is the same on every backend.
// @compilerrules [test-property, prop-rng]
export fn derive_seed(config: TestConfig, id: Str) [] -> Long => config, id {
    let h = -3750763034362895579L
    let bytes = to_bytes(id)
    let i = 0
    while i < size(bytes) {
        h = bit_xor(h, to_long(to_int(get(bytes, i) ?: to_byte(0)))) * 1099511628211L
        i = i + 1
    }
    return long(live_rng(h + run_seed(config)))
}

// One parameter's line in the report: `xs = [1, 2]`.
// @compilerrules [test-property]
export fn shown_line(name: Str, text: Str) [] -> Str => name, text {
    return "${name} = ${text}"
}

// What the property test answers once its runs and shrinking are done: the
// failure (or `Flaky`, when its simplest case passed on replay) with where it
// was found, its parameters and the line that runs it again, or `Passed`.
// @compilerrules [test-property, test-kind]
export fn report_of(run: PropertyRun, seed: Long, id: Str) [] -> Verdict => run, seed, id {
    let lines = verdict(run)
    if size(lines) == 0 {
        return Passed {}
    }
    let detail: Mut List<Str> = Mut List<Str>()
    let message = ""
    let first = true
    for line in lines {
        if first {
            first = false
            message = copy(line)
        } else {
            add(detail, copy(line))
        }
    }
    add(detail, "re-run: salvo test \"${id}\" --arg seed=${seed}")
    if flaky(run) {
        return Flaky { message: message, detail: detail }
    }
    return Failed { message: message, detail: detail }
}

// Starts a run on a fresh virtual runtime, seeded with [seed] (the run's
// first draw, so a replay replays the scheduling too).
// @compilerrules [test-property, test-actor]
export fn begin_property_attempt(seed: Long) [] -> None => !seed {
    enter_virtual(seed)
    reset_timers()
}

// [property] checked over [runs] values of its parameter's type, inside an
// ordinary test: the first failure is shrunk, and thrown as one `Failure`
// naming the simplest value that fails.
//
// The values come from the parameter type's generator, as a property
// test's do. Each run is called directly, not caught: a trap in [property]
// ends the test, unshrunk. Runs from a fixed seed, so the test is the same
// every time.
//
// @param gen  found at the call site: the parameter type's generator
// @param to_str  found at the call site: how the failing value is shown
// @compilerrules [test-property]
export fn check<T>(
    runs: Int,
    property: (value: T) [Throw<Failure>] -> None,
    ?gen: () -> Gen<T>,
    ?to_str: (value: T) -> Str
) [Throw<Failure>] -> None
=> property {
    let lines = checked(runs, property, gen(), to_str)
    if size(lines) > 0 {
        throw(Failure { message: join(lines, "; ") })
    }
}

// [check]'s runs: the report's lines, none when the property held. Its own
// effect list is empty, since it catches every throw of [property] itself.
fn checked<T>(
    runs: Int,
    property: (value: T) [Throw<Failure>] -> None,
    g: Gen<T>,
    show: (value: T) -> Str
) [] -> List<Str>
=> property, !g {
    let run = property_run(runs, 0L)
    while next_attempt(run) {
        let rng = attempt_rng(run)
        let value = g.run(rng)
        let shown: Mut List<Str> = Mut List<Str>()
        if wants_shown(run) {
            add(shown, "value = ${show(value)}")
        }
        let outcome = try {
            property(value)
        }
        let failure: Str? = when outcome {
            is Ok {
                None
            }
            is Thrown {
                let why: Failure = outcome
                copy(why.message)
            }
        }
        record(run, failure, draws_of(rng), shown)
    }
    return verdict(run)
}

// [draws] without its trailing zeros, which a replay reads anyway.
fn trimmed(draws: List<Long>) [] -> List<Long> => draws {
    let n = size(draws)
    while n > 0 && (get(draws, n - 1) ?: 0L) == 0L {
        n = n - 1
    }
    return prefix(draws, n)
}

fn prefix(draws: List<Long>, n: Int) [] -> List<Long> => draws {
    let out: Mut List<Long> = Mut List<Long>()
    let i = 0
    while i < n {
        add(out, get(draws, i) ?: 0L)
        i = i + 1
    }
    return out
}

// Whether [a] is a simpler run than [b]: fewer draws, or as many and smaller
// at the first that differs.
fn simpler(a: List<Long>, b: List<Long>) [] -> Bool => a, b {
    if size(a) != size(b) {
        return size(a) < size(b)
    }
    let i = 0
    while i < size(a) {
        let x = bit_and(get(a, i) ?: 0L, 9223372036854775807L)
        let y = bit_and(get(b, i) ?: 0L, 9223372036854775807L)
        if x != y {
            return x < y
        }
        i = i + 1
    }
    return false
}

// Refills the replays to try from the simplest failure: the draws cut
// short, chunks of them removed, chunks zeroed, single draws halved.
fn refill(run: Mut PropertyRun) [] -> None => run: Mut {
    while size(run.queue) > 0 {
        let _old = remove_first(run.queue)
    }
    let b = copy(run.best)
    let n = size(b)
    if n > 1 {
        add(run.queue, prefix(b, n / 2))
    }
    if n > 0 {
        add(run.queue, prefix(b, n - 1))
    }
    for chunk in List(8, 4, 2, 1) {
        let i = 0
        while i + chunk <= n {
            add(run.queue, without(b, i, chunk))
            i = i + 1
        }
    }
    for chunk in List(8, 4, 2, 1) {
        let i = 0
        while i + chunk <= n {
            if !all_zero(b, i, chunk) {
                add(run.queue, zeroed(b, i, chunk))
            }
            i = i + 1
        }
    }
    let i = 0
    while i < n {
        let v = bit_and(get(b, i) ?: 0L, 9223372036854775807L)
        if v > 0L {
            add(run.queue, replaced(b, i, v / 2L))
            add(run.queue, replaced(b, i, v - 1L))
            // A generator reads a draw's low bits (a mask, a remainder), so
            // keeping fewer of them shrinks what it makes.
            for bits in List(32, 16, 8, 4, 2, 1) {
                let low = bit_and(v, bit_shl(1L, bits) - 1L)
                if low < v {
                    add(run.queue, replaced(b, i, low))
                    add(run.queue, replaced(b, i, low / 2L))
                }
            }
        }
        i = i + 1
    }
}

fn without(b: List<Long>, at: Int, count: Int) [] -> List<Long> => b {
    let out: Mut List<Long> = Mut List<Long>()
    let i = 0
    while i < size(b) {
        if i < at || i >= at + count {
            add(out, get(b, i) ?: 0L)
        }
        i = i + 1
    }
    return out
}

fn zeroed(b: List<Long>, at: Int, count: Int) [] -> List<Long> => b {
    let out: Mut List<Long> = Mut List<Long>()
    let i = 0
    while i < size(b) {
        if i >= at && i < at + count {
            add(out, 0L)
        } else {
            add(out, get(b, i) ?: 0L)
        }
        i = i + 1
    }
    return out
}

fn all_zero(b: List<Long>, at: Int, count: Int) [] -> Bool => b {
    let i = at
    while i < at + count {
        if (get(b, i) ?: 0L) != 0L {
            return false
        }
        i = i + 1
    }
    return true
}

fn replaced(b: List<Long>, at: Int, value: Long) [] -> List<Long> => b {
    let out: Mut List<Long> = Mut List<Long>()
    let i = 0
    while i < size(b) {
        if i == at {
            add(out, value)
        } else {
            add(out, get(b, i) ?: 0L)
        }
        i = i + 1
    }
    return out
}
