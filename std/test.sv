// [test-decl] [test-fail] The standard test module: what a `test` block has
// to hand, and the only surface `salvo test` adds to the language.
//
// **Implicitly available in every test annex** [test-implicit-import] — a
// `<name>.test.sv` file behaves as if it wrote the whole-module import, so no
// test ever spells one — and available nowhere else: this module reaches a
// program only through its tests.
//
// Failure travels on the non-resumption channel the language already has
// [throw] [try]: an assertion declares `[Throw<Failure>]`, a failing one
// throws, and the generated harness reads the outcome off an ordinary `try`
// [test-run] (user decision 2026-09-23 — no second non-resumption channel,
// and no `Test` effect to intercept). Two consequences worth knowing while
// writing tests:
//
//   * **An assertion vocabulary of your own is an ordinary function.** A
//     helper that asserts declares `[Throw<Failure>]` and composes with
//     these; there is nothing to register.
//   * **A test stops at its first failure**, because that is what `throw`
//     does. Several independent facts are several tests.

// [test-fail] Why a test failed: the message the report prints under it.
//
// A struct rather than a bare `Str` so the failure can grow fields — a
// property test's seed, a shrink count — without changing the channel it
// travels on (user decision 2026-09-23).
export struct Failure {
    // One line, ideally: the report indents it under the test's own line.
    message: Str
}

// [interp-to-str] So `${failure}` renders, in the harness and in a test.
export fn to_str(failure: Failure) [] -> Str => failure {
    return failure.message
}

// [test-fail] Fails the enclosing test unless [condition] holds, reporting
// [label] as what was expected.
//
// The general assertion: everything else is a convenience over it.
export fn expect(condition: Bool, label: Str) [Throw<Failure>] -> None => label {
    if !condition {
        throw(Failure { message: "expectation failed: ${label}" })
    }
}

// [test-fail] Fails the enclosing test unless [actual] equals [expected],
// reporting both.
//
// Equality and rendering are **capabilities**, not built-ins, so both arrive
// as implicit parameters [implicit-group]: `?Eq<T>` is the `eq` that decides
// [cmp-groups], `?ToStr<T>` the `to_str` that renders [interp-to-str]. A type
// that declares the two is testable this way; one that declares neither is
// tested with `expect` and a message of its own, which is what the resolution
// error at the call site says (user decision 2026-09-23).
export fn expect_eq<T>(actual: T, expected: T, ?Eq<T>, ?ToStr<T>) [Throw<Failure>] -> None
=> actual, expected {
    if !eq(actual, expected) {
        throw(Failure {
            message: "expected ${to_str(expected)}, got ${to_str(actual)}"
        })
    }
}

// [test-recover] Runs [body] and answers what went wrong: the message [body]
// itself answered (a failed `expect`, which throws and is read off a `try`), or
// the **trap** that ended it — an assertion that failed [assert-trap], a
// subscript out of range, any of the failures a program is not meant to continue
// past. `None` means the test passed.
//
// [body] answers rather than prints, and performs no effects, so nothing has to
// be threaded into the catch: the verdict is printed by the caller.
//
// This is how the generated harness makes a failing `assert!` *that test's*
// failure rather than the end of the run (user decision 2026-09-23): the catch
// happens in the harness, so a death is local to the test that caused it —
// which is what running tests in parallel will need, since a restart cannot be
// coordinated across concurrent tests.
//
// It lives here, in the test module, and therefore **only exists in test
// files** [test-implicit-import]: production code still cannot catch a trap, the
// stance A-2 took when it rejected an interceptable abort. Each backend lowers
// it to its own catch — an exception handler on the JVM, `catch_unwind` on Rust
// — and the message is the trap's own text [assert-trap].
export intrinsic fn trapped_by(body: () -> Str?) [] -> Str? => body

// [test-trap-expect] The trap a [body] produced, or `None` when it completed:
// `trapped_by` with the shape a test wants, since a test's body answers nothing.
//
// The three functions below are the vocabulary for a test about a **failure the
// program is not meant to survive** — a failed `assert!`, a subscript out of
// range, an `unreachable!` that turned out to be reachable. They exist because
// the harness can catch one [test-recover]; before that, a test about a trap
// could only be written by not writing it.
export fn trap_of(body: () -> None) [] -> Str? => body {
    return trapped_by(() -> {
        body()
        return None
    })
}

// [test-trap-expect] Fails the enclosing test unless [body] traps.
//
// The [label] says what was expected to fail, so the report reads as a
// statement about the program rather than about the test: "expected a trap:
// popping an empty heap".
export fn expect_trap(body: () -> None, label: Str) [Throw<Failure>] -> None
=> body, label {
    if trap_of(body) is None {
        throw(Failure { message: "expected a trap: ${label}" })
    }
}

// [test-trap-expect] Fails the enclosing test unless [body] traps with a message
// containing [needle] — which is how a test pins *which* failure it meant,
// rather than accepting any trap at all.
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
