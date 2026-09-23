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
