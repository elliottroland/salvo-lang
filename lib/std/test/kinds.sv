// The test kinds std ships: `plain` (a test with no kind written), `actor`
// and `property`.
//
// A test kind is a `comptime fn` whose first parameter is a `Test`. It is
// stamped at a `test` block that names it, `test property(runs=200) "…"
// (xs: List<Int>) { … }`: the fn's other parameters are the kind's arguments,
// the compiler makes the block's body into a fn of its own, `[test.body]`,
// that the kind runs, and what the kind answers is the test's [Verdict].
//
// A kind lists the powers its body may use as its own effect list (`[use,
// spawn]` lets the body spawn), says which runtime it needs with a `runtime`
// argument (`"threaded"` or `"virtual"`; the runner writes one program per
// runtime), and may take any other arguments, constants with defaults. A test
// can override one with `--arg name=value` on the command line, and the
// manifest's `[test.args]` supplies one the test does not write.
//
// To write your own, copy `plain` below. What a template may read of the
// test: `test.name`, `test.id`, `[test.body]` (the body, called with the
// test's parameters), `[test.config]` (a `TestConfig`), and `[for p in
// test.params]` over its parameters: `p.name`, `[p.type]`, `[p]` for the
// local, `p.type.slots` for a parameter type's qualifier slots.
//
// ```no-check
// // A test that runs its body twice, to see that it can.
// comptime fn twice(test: Test) [use] -> Verdict {
//     [test.body]()
//     return plain_verdict(try { [test.body]() })
// }
// ```
// @compilerrules [test-kind]

import std.throw
import std.test.actor
import std.test.property
import std.test.random

// A test with no kind written: its body, in the threaded runtime, a failed
// expectation or a trap its failure.
// @compilerrules [test-kind, test-recover]
comptime fn plain(test: Test, runtime: Str = "threaded") [use] -> Verdict {
    let failure = trapped_by@std.test(() -> {
        let outcome = try {
            [test.body]()
        }
        return when outcome {
            is Ok {
                None
            }
            is Thrown {
                let why: Failure = outcome
                copy@basic(why.message)
            }
        }
    })
    if failure is Str {
        return Failed { message: failure, detail: List<Str>() }
    }
    return Passed {}
}

// A test that may spawn actors, on the virtual runtime: one thread, a clock
// the scheduler moves when nothing can run, randomness from `seed`. A trap in
// its own frame ends the process, which the runner recovers from in a fresh
// one (an actor test cannot be wrapped in a trap catch, since its body spawns
// and a lambda cannot).
// @compilerrules [test-kind, test-actor]
comptime fn actor(test: Test, seed: Long = 0, runtime: Str = "virtual") [use, spawn] -> Verdict {
    begin_actor_test@std.test.actor(seed)
    let outcome = try {
        [test.body]()
    }
    return when outcome {
        is Ok {
            Passed {}
        }
        is Thrown {
            let why: Failure = outcome
            Failed { message: copy@basic(why.message), detail: List<Str>() }
        }
    }
}

// A property test: its body run over `runs` draws of its parameters, each
// from its type's generator, shrunk to the simplest failure. `seed` pins the
// run; without one, it is derived from the run's seed and the test's id, and
// the report's `re-run:` line names it.
// @compilerrules [test-kind, test-property]
comptime fn property(test: Test, runs: Int = 100, seed: Long? = None, runtime: Str = "virtual") [use] -> Verdict {
    let pinned = seed ?: derive_seed@std.test.property([test.config], test.id)
    let run = property_run@std.test.property(runs, pinned)
    while next_attempt@std.test.property(run) {
        let rng = attempt_rng@std.test.property(run)
        begin_property_attempt@std.test.property(long@std.test.random(rng))
        [for p in test.params] {
            let [p]: [p.type] = draw_with@std.test.random<[p.type]>(rng, gen@[site]([...for s in p.type.slots] { [s] }))
        }
        let shown: Mut List<Str> = Mut List<Str>()
        if wants_shown@std.test.property(run) {
            [for p in test.params] {
                add@std.core.list(shown, shown_line@std.test.property(p.name, to_str@[site]([p])))
            }
        }
        let failure = trapped_by_once@std.test(() -> {
            let outcome = try {
                [test.body]([...for p in test.params] { [p] })
            }
            return when outcome {
                is Ok {
                    None
                }
                is Thrown {
                    let why: Failure = outcome
                    copy@basic(why.message)
                }
            }
        })
        record@std.test.property(run, failure, draws_of@std.test.random(rng), shown)
    }
    return report_of@std.test.property(run, pinned, test.id)
}
