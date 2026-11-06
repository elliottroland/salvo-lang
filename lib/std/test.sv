// Assertions for tests: what a `test` block uses to say what it expects, and
// to check that code fails where it is meant to.
//
// A test is a `test "name" { … }` block in a module's **test annex**, the
// file `<name>.test.sv` beside `<name>.sv`. The annex is part of the module,
// so its tests see the module's private declarations, and nothing outside it
// sees the annex. `salvo test` finds every test, runs each one, and prints a
// line per test with its verdict. Every annex can use this module without
// importing it.
//
// ```no-check
// // halve.test.sv, the annex of halve.sv, which declares
// //     fn halve(n: Int) -> Int
// // and asserts that n is even.
//
// test "halving an even number" {
//     expect_eq(halve(10), 5)
//     expect(halve(0) == 0, "half of zero is zero")
// }
//
// test "halving an odd number traps" {
//     expect_trap_with(() -> { halve(7) }, "got 7", "halving 7")
// }
// ```
//
// **An assertion throws.** [expect] and the others declare
// `[Throw<Failure>]`: one that does not hold throws a [Failure], and the
// harness `salvo test` generates runs each test inside its own `try` and
// reports the message it caught. A test body may throw a `Failure` itself
// (`throw(Failure { message: "…" })`) to fail for a reason of its own.
//
// **A test stops at its first failure**, because that is what a throw does.
// Several independent facts are several tests, so that one failure does not
// hide another.
//
// **Your own assertions are ordinary functions.** A helper that declares
// `[Throw<Failure>]` can call these and be called from a test; there is
// nothing to register. Write it in the annex, beside the tests that use it,
// and it stays out of the module's public surface:
//
// ```
// fn expect_sorted(xs: List<Int>) [Throw<Failure>] -> None => xs {
//     let i = 1
//     while i < size(xs) {
//         expect((get(xs, i - 1) ?: unreachable!()) <= (get(xs, i) ?: unreachable!()), "sorted at ${i}")
//         i = i + 1
//     }
// }
//
// test "sorting sorts" {
//     expect_sorted(sort(List(3, 1, 2)))
// }
// ```
//
// **Close what you open before you assert.** Nothing linear may be live
// across a call that may throw, since the code that would close it does not
// run when the call throws. A test that opens a stream reads what it needs,
// closes the stream, and then asserts on what it read.
//
// **A trap is a failure too, not the end of the run.** A failed `assert!`,
// an `unreachable!()` that was reached or any other trap inside a test is
// caught by the harness and reported as that test's failure, with the trap's
// own message; the tests after it still run. [expect_trap],
// [expect_trap_with] and [trap_of] use the same catch to test that code
// traps. The catch exists only in tests: production code cannot catch a
// trap.
//
// A `test actor` block, a test that spawns actors, runs on a virtual runtime
// with one thread and a clock that jumps; see [std.test.actor].
// @compilerrules [test-decl, test-fail, test-implicit-import, throw, throw-try, test-run]
// @compilerknown

import std.throw
import std.runtime.trap_boundary
import std.runtime.trap_boundary_once

// `Failure` is a struct rather than a bare `Str` so that it can grow fields
// (a property test's seed, a shrink count) without changing the channel it
// travels on (user decision 2026-09-23).

// Why a test failed: the message the report prints under the test's line.
//
// ```
// let f = Failure { message: "expected 3, got 2" }
// println("${f}")                                      // expected 3, got 2
// ```
// @compilerrules [test-fail]
// @compilerknown
export struct Failure {
    // What went wrong, ideally on one line: the report indents it under the
    // test's own line.
    message: Str
}

// The failure's message, which is how `${failure}` renders.
// @compilerrules [interp-to-str]
export fn to_str(failure: Failure) [] -> Str => failure {
    return failure.message
}

// Fails the enclosing test unless a condition holds.
//
// The general assertion: everything else in this module could be written
// with it. The failure's message is `expectation failed: ` followed by
// [label], so write the label as the fact you expected ("the list is
// empty"), not as the check ("size is 0").
//
// ```
// test "a new list is empty" {
//     let xs = Mut List<Int>()
//     expect(size(xs) == 0, "a new list is empty")
// }
// ```
//
// @param condition  what must be `true` for the test to go on
// @param label      the expected fact, reported if it does not hold
// @throws           a [Failure] when [condition] is `false`
// @compilerrules [test-fail]
export fn expect(condition: Bool, label: Str) [Throw<Failure>] -> None => label {
    if !condition {
        throw(Failure { message: "expectation failed: ${label}" })
    }
}

// Fails the enclosing test unless two values are equal, reporting both.
//
// The failure's message is `expected <expected>, got <actual>`, so the order
// of the arguments matters: the value under test comes first.
//
// Equality and rendering are found at the call site, as the type's own `eq`
// and `to_str`. A type that declares both can be compared this way; for one
// that does not, the call does not compile, and [expect] with a label of your
// own is the way to test it.
//
// ```
// test "words are counted" {
//     expect_eq(size(split("a b c", " ")), 3)
// }
// ```
//
// @param actual    the value under test
// @param expected  the value it should be
// @throws          a [Failure] naming both values when they differ
// @compilerrules [test-fail, implicit-group, cmp-groups, interp-to-str]
export fn expect_eq<T>(actual: T, expected: T, ?Eq<T>, ?ToStr<T>) [Throw<Failure>] -> None
=> actual, expected {
    if !eq(actual, expected) {
        throw(Failure {
            message: "expected ${to_str(expected)}, got ${to_str(actual)}"
        })
    }
}

// Runs a body inside the harness's trap catch, and answers what went wrong:
// the message the body answered, the message of a trap that ended it, or
// `None` when it answered `None`.
//
// This is the catch the generated harness wraps each plain test in, so that
// a trap fails one test rather than ending the run. A test seldom needs it:
// [trap_of] and [expect_trap] are the forms for a body that answers nothing.
//
// The body is a plain fn value, so it does not inherit the caller's effects;
// one that needs a handler registers it itself (`() -> { use StdConsole() …
// }`). It answers rather than prints, so the verdict is printed by the
// caller.
//
// ```
// let verdict = trapped_by(() -> {
//     if 1 + 1 == 2 {
//         return None
//     }
//     return "arithmetic is broken"
// })
// println("${verdict ?: "passed"}")                    // passed
// ```
//
// @param body  answers `None` on success or a message on failure
// @returns     that answer, or the trap's message if [body] trapped
// @compilerrules [test-recover, assert-trap, test-implicit-import]
export fn trapped_by(body: () -> Str?) [] -> Str? => body {
    // Only test files have this module, so production code still cannot
    // catch a trap: the stance A-2 took when it rejected an interceptable
    // abort (user decision 2026-09-23). The catch is the runtime's fault
    // boundary (`runtime.trap_boundary`): an exception handler on the JVM,
    // `catch_unwind` on Rust; the message is the trap's own text. Catching in
    // the harness makes a death local to the test that caused it, which is
    // what running tests in parallel will need.
    return trap_boundary(body)
}

// [trapped_by] for a body called once, which may consume what it captured:
// how a property test runs a property over the parameters it drew.
//
// @param body  answers `None` on success or a message on failure
// @returns     that answer, or the trap's message if [body] trapped
// @compilerrules [test-recover, test-property]
export fn trapped_by_once(body: once () -> Str?) [] -> Str? => !body {
    return trap_boundary_once(body)
}

// The message of the trap a body ended with, or `None` when it completed.
//
// For a test about a failure the program is not meant to survive: a failed
// `assert!`, a subscript out of range, an `unreachable!()` that was reached.
// The message is the trap's own text, which for an `assert!` includes its
// message and the Salvo module, line and column it failed at.
//
// ```
// let trap = trap_of(() -> { assert!(1 > 2, "one is not more than two") })
// println("${trap ?: "no trap"}")    // salvo: one is not more than two at …
// let none = trap_of(() -> {})
// println("${none is None}")         // true
// ```
//
// @param body  the code that may trap; it registers any handler it needs
// @compilerrules [test-trap-expect, test-recover]
export fn trap_of(body: () -> None) [] -> Str? => body {
    return trapped_by(() -> {
        body()
        return None
    })
}

// Fails the enclosing test unless a body traps.
//
// The failure's message is `expected a trap: ` followed by [label], so write
// the label as what should fail ("popping an empty heap"): the report then
// reads as a statement about the program.
//
// ```
// test "an empty list has no first element" {
//     let xs = Mut List<Int>()
//     expect_trap(() -> { let x = first(xs) ?: unreachable!("empty") }, "the first of an empty list")
// }
// ```
//
// @param body   the code that should trap
// @param label  what should fail, reported if [body] completes
// @throws       a [Failure] when [body] completes
// @compilerrules [test-trap-expect]
export fn expect_trap(body: () -> None, label: Str) [Throw<Failure>] -> None
=> body, label {
    if trap_of(body) is None {
        throw(Failure { message: "expected a trap: ${label}" })
    }
}

// Fails the enclosing test unless a body traps with a message containing a
// given text.
//
// [expect_trap] accepts any trap; this one pins which failure the test
// meant, so a body that traps for another reason fails the test.
//
// ```
// test "the assertion names the number" {
//     expect_trap_with(() -> { assert!(7 % 2 == 0, "got 7") }, "got 7", "an odd number")
// }
// ```
//
// @param body    the code that should trap
// @param needle  text the trap's message must contain
// @param label   what should fail, reported if [body] does not trap
// @throws        a [Failure] when [body] completes, or when it traps with a
//                message that does not contain [needle]
// @compilerrules [test-trap-expect]
export fn expect_trap_with(
    body: () -> None,
    needle: Str,
    label: Str
) [Throw<Failure>] -> None => body, needle, label {
    let trap = trap_of(body)
    when trap {
        is None {
            throw(Failure { message: "expected a trap: ${label}" })
        }
        is Str {
            if !contains(trap, needle) {
                throw(Failure {
                    message: "expected a trap containing `${needle}`, got `${trap}`: ${label}"
                })
            }
        }
    }
}
