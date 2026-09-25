# Testing

A test is a declaration:

```
test "an empty heap pops nothing" {
    let heap = empty_heap<Int>()
    expect(pop(heap) is None, "popping an empty heap answers None")
}
```

Named by a string, because a test name is prose and there is nothing to call it by. It takes no parameters, declares no effects, returns nothing, and cannot be exported — a test is run, never referenced. `salvo test` finds them, runs them, and prints what happened:

```
test heap :: an empty heap pops nothing ... ok (2 ms)
test heap :: pops come out in order ... FAILED
    expected 9, got 5

5 tests: 4 passed, 1 failed
```

## Where tests live

**In a companion file, always.** The tests of module `heap` live in `heap.test.sv` beside `heap.sv`, and a `test` block written in a production source file is an error naming the companion. `.test` is the one dot a file name may carry; the file is module `heap.test`, the **test annex** of module `heap`.

The annex is the module's whitebox: it sees every declaration of `heap`, private ones included, exactly as another file of the module would. Nothing sees the annex. That is not a rule to remember but a consequence of two facts — no production file imports it, and `salvo compile` and `salvo run` do not load `.test.sv` files at all. There is nothing to strip from a production build, and no way for shipped code to come to depend on a test.

An annex may declare whatever its tests need: helper functions, structs, qualifiers of its own. They are invisible to the module it tests, so a test vocabulary never leaks into the shipped surface.

An annex with no module to be the annex of is an error: `heap.test.sv` needs a `heap.sv` beside it.

## What a test may do

A test body is an ordinary block with an entry point's powers. It may `use` handlers, which is how a test supplies fakes:

```
test "a missing file reports its path" {
    use MemFs()
    let outcome = try {
        read_to_str("/nothing")
    }
    expect(outcome is Thrown, "reading a missing file fails")
}
```

Everything else about a body is the language as it is everywhere else — narrowing, linearity, deductions, effects. A linear value a test opens must still be closed on every path, and a test that leaks one does not compile.

## How a test fails

`std.test` is **implicitly available in every annex**, which is why no test file imports anything to assert:

- `expect(condition, label)` — the general assertion. The label says what was expected.
- `expect_eq(actual, expected)` — for a type with an `eq` and a `to_str`, which is what the report prints.

An assertion that does not hold *throws*: assertions declare `[Throw<Failure>]`, and the harness reads the outcome off a `try` [throw]. Two consequences follow from that, and they are the whole model:

**A test stops at its first failing assertion**, because that is what `throw` does. Several independent facts are several tests.

**An assertion vocabulary of your own is an ordinary function.** A helper that asserts declares `[Throw<Failure>]` and composes with the built-in ones; there is nothing to register and no framework to extend:

```
// In an annex, beside the tests that use it.
fn expect_sorted(list: List<Int>) [Throw<Failure>] -> None => list {
    let i = 1
    while i < list.size() {
        expect(list.get(i - 1)! <= list.get(i)!, "sorted at ${i}")
        i += 1
    }
}
```

`Failure` is a small struct carrying the message, so a failure can be built and inspected like any other value.

## Running them

```
salvo test --src ./my_project                    # everything, on the rust backend
salvo test --backend kotlin --src ./my_project    # the same tests, the same report
salvo test --src . "empty"                        # only tests whose id contains "empty"
salvo test --src . --list                         # enumerate, run nothing
```

A test's **id** is the module a program would import, then the name as written: `heap :: an empty heap pops nothing`. The filter is a plain substring of that id, so one word selects a module, a test, or a family of tests. The command's exit code is what a build reads: nonzero when anything failed.

**A test that fails an assertion is a failed test, not a dead run.** An assertion traps, and a trap would ordinarily end the program — so the generated harness catches it and reports it like any other failure, with the trap's own message:

```
test calc :: first passes ... ok (0 ms)
test calc :: this one traps ... FAILED
    salvo: n should exceed 100, was 6 at calc.test:7:5
test calc :: and the run goes on ... ok (0 ms)

3 tests: 2 passed, 1 failed
```

Only the harness can do this: the catch is `std.test`'s own, so it exists in test files and nowhere else — production code still cannot catch a trap. A death the harness *cannot* catch (a process killed outright) is still handled, by naming the test that was running and re-running the rest.

**A test can also be about a trap.** The same catch is available to you:

```
test "halving an odd number traps" {
    expect_trap(() -> { halve(3) }, "halving an odd number")
}

test "the trap says which number" {
    expect_trap_with(() -> { halve(7) }, "got 7", "halving an odd number")
}

test "the message is available" {
    let trap = trap_of(() -> { halve(5) })
    expect(trap is Str, "a trap message came back")
}
```

The body is a plain fn value, so it does not inherit the test's effects — a body that needs one registers it itself (`() -> { use StdOutConsole(); … }`).

The runner works by writing a Salvo program. It synthesizes a module that calls each test inside its own `try`, compiles it with the rest of the sources exactly as `salvo run` would, and renders what it prints. So the two backends run the same tests the same way, and the report is identical on both — a test suite is not a place where a target language should show through.

# Assertions

Four ways exist to deal with a fact that might not hold, and they are in order of
preference — the ladder is the point of this section, and assertions are its
bottom rung:

1. **Prove it.** A qualifier carries the fact in the type, established by
   construction (`list_of(1, 2, 3)` is `NonEmpty`) or by a refinement, so nothing
   checks it twice and nothing can fail.
2. **Require it.** A linear type makes the caller *act* rather than merely know:
   forgetting is a compile error, not a run-time one.
3. **Handle it.** `?:` picks a fallback, `is` narrows, `when` covers the cases,
   `throw`/`try` carries a failure to a delimiter. Use these when the absence is
   a *case* rather than a bug.
4. **Assert it.** Last, and only where the fact is true but unprovable here.

An assertion says something the checker cannot prove, and fails if it is wrong.
Three forms, each carrying a `!` because a bang in Salvo marks a place that can
fail:

```
let head = first(xs)!                    // asserts presence
assert!(n > 0, "n must be positive, was ${n}")
unreachable!("an Int is negative, zero or positive")
```

**`expr!` asserts presence** and answers the value without its `None` arms. It
needs an operand that *can* be absent: `3!` is an error, because it states
something false. Where a value can legitimately be missing, `?:` and `is None`
are the answers — the diagnostic says so.

**`assert!(cond)` asserts a condition**, with an optional message, and it also
**narrows**:

```
fn describe(value: Int | Str) [Console] -> None {
    assert!(value is Str, "expected a string, got ${value}")
    println("len ${size(value)}")       // `value` is a `Str` here
}
```

That is what makes it more than a check: the same `is` test that would narrow
inside an `if` narrows for the rest of the scope, so an assertion buys a fact the
type system then carries. `assert!(xs is NonEmpty)` makes `first(xs)` answer an
element.

**`unreachable!()` asserts that a path is not taken.** Its type is `Never`, so it
stands wherever a value is expected and ends the path, exactly as `throw` and
`return` do:

```
return when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    n > 0 { "positive" }
    else { unreachable!("an Int compares one way or the other") }
}
```

The two named forms look like function calls and are not: the message is built
*only when the assertion fails*, the condition's narrowing reaches the enclosing
scope, and neither name can be shadowed. `assert` and `unreachable` stay ordinary
identifiers — only `assert!(` and `unreachable!(` are the forms.

**A failed assertion reports in Salvo's words.** The message names the Salvo
module, line and column, and reads identically on both backends:

```
salvo: n must be positive, was 7 at main:5:5
salvo: value is absent at core.list:153:12
```

The mechanism underneath is each target's own trap — a panic on Rust, an
`AssertionError` on the JVM — because neither program is meant to continue.
**Assertions are always on.** There is no build mode that removes them; a check
you cannot rely on is not worth writing.
