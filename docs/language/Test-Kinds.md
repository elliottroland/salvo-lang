# Writing Test Kinds

## Overview

A test's **kind** says how its body is run: on which runtime, how many times,
with what set up, and what counts as a pass. You have met three of them in
[Testing](Testing.md): a plain `test "name"`, `test actor`, and `test
property`. None of them is built into the compiler. Each is a
[compile-time function](Compile-Time-Functions.md) written in Salvo, in
[`std.test.kinds`](../../lib/std/test/kinds.sv), and you can write more.

A kind is a `comptime fn` whose first parameter is a `Test`. The `test`
declaration that names it is the call:

```salvo
// Runs the body `times` times, and fails with the number of the run that failed.
comptime fn repeated(test: Test, times: Int = 2, runtime: Str = "threaded") [use] -> Verdict {
    let i = 0
    while i < times {
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
            return Failed { message: "${test.name}, run ${i + 1}: ${failure}", detail: List<Str>() }
        }
        i = i + 1
    }
    return Passed {}
}

test repeated(times=3) "bump adds one" {
    expect_eq(bump(1), 2)
}
```

The compiler turns the block's body into a function of its own, stamps the
kind at the block, and the stamped function is what `salvo test` runs. Where
the kind writes `[test.body]()`, the body is called.

Compared with a test framework in another language, a kind is the part that
JUnit calls a runner or an extension, and Rust spells as a custom `#[test]`
harness. The difference is that it is checked like the rest of your program,
and it runs on both backends.

## What a kind is

**The first parameter is the test.** It is a `Test`, a compile-time value that
describes the block the kind is stamped at. Its parts are read inside square
brackets or as plain projections:

| Written | Meaning |
|---|---|
| `[test.body]` | the body, as a function to call with the test's parameters |
| `[test.config]` | the run's `TestConfig`, which carries the run's seed and the `--arg` and manifest values |
| `test.name`, `test.id` | the test's name, and its full id (`module :: name`), as strings |
| `[for p in test.params] { … }` | once for each parameter the test declares |
| `p.name` | the parameter's name, as a string |
| `[p.type]` | the parameter's type |
| `[p]` | a local named like the parameter |
| `[for s in p.type.slots] { [s] }` | the arguments of the parameter type's qualifier, such as `xs` in `Idx(xs) Int` |

The test's parameters are given by the kind, not found by the caller, so an
implicit parameter in a `test` declaration is an error.

**The other parameters are the kind's arguments.** They are typed and may have
defaults. A `test` block passes them by name: `test repeated(times=3) "name"`.
An argument is a compile-time constant; a value that is not one is an error.

**The effect list is what the body may use.** The body can do whatever the
kind's list says, plus throwing a `Failure`. A kind that lists `[use, spawn]`
lets its body spawn actors, as `actor` does. One that lists `[use]` does not.

**The result is a `Verdict`.** A kind answers with one of:

- `Passed {}`;
- `Failed { message, detail }`, where `message` is the failure and `detail`
  is a list of extra lines the runner prints under it (a property test puts
  the counterexample there);
- `Flaky`, a failure that did not happen again when replayed. It counts as a
  failure.

**The `runtime` argument picks the runtime.** It is `"threaded"` or
`"virtual"`, and it is read when the program is built. `salvo test` writes one
program for each runtime, because a process cannot leave the threaded one once
its workers have started. A kind that does not mention `runtime` is threaded.
The [actor tests](Testing.md#actor-tests) section lists what the virtual
runtime does.

## Setting a kind's arguments

An argument's value comes from the first of these that has one:

1. `--arg NAME=VALUE` on the command line, for every test whose kind takes
   `NAME`;
2. the value written at the `test` block;
3. the manifest's `[test.args]` table;
4. the default in the kind's signature.

```toml
[test.args]
runs = 500
```

```bash
salvo test --arg times=5
salvo test "bump adds one" --arg times=5
```

A kind reads `--arg` and manifest values through the argument's name, so the
same name means the same thing to every kind that takes it. `run_seed` is the
one name the runner itself reads: `--arg run_seed=7` fixes the run's seed.

## Writing the template

A kind's body is a template. It is written once and stamped at each block, so
a few things differ from ordinary code.

**Qualify the names the template uses.** A name in a template is looked up at
the block that uses it, which may be in another module with other names in
scope. `trapped_by@std.test` and `copy@basic` name the module a function comes
from, as the kinds in `std.test.kinds` do. Functions from the *test's* module,
such as a generator the user wrote, are reached with `@[site]`:
`gen@[site]()`.

**Catch the body's failures yourself.** The body ends at its first failing
assertion by throwing a `Failure`. A kind that must carry on, or must report
something of its own, calls the body inside `try`, and a trap (a failed
`assert!`) inside `trapped_by@std.test`, as `plain` does. A kind that lists
`spawn` cannot do the second, because a lambda cannot spawn; it takes only the
`try`, and a trap in its own frame ends the process, which the runner recovers
from in a fresh one.

**A kind is a function of the block, not a loop over the file.** To run
something once per parameter, use `[for p in test.params]`. The
[`property`](../../lib/std/test/kinds.sv) kind is the worked example: it draws
each parameter from its type's generator, calls `[test.body]` with the draws,
and shrinks a failure.

```salvo
no-check
[for p in test.params] {
    let [p]: [p.type] = draw_with@std.test.random<[p.type]>(rng, gen@[site]())
}
[test.body]([for p in test.params] { [p] })
```

The `[for p in test.params] { [p] }` in the call is a head in argument
position: it adds one argument for each parameter, as a list is spliced into
a struct literal.

## Where a kind can live

**In the annex that uses it.** A kind written in `counter.test.sv` can be used
by the tests of `counter.test.sv`. Sharing a kind between modules, with a
common test-support module the annexes import, is not built yet.

**In std, for the kinds std ships.** `plain`, `actor` and `property` live in
[`std.test.kinds`](../../lib/std/test/kinds.sv), which every annex imports
without writing it.

## What does not work yet

- A kind cannot be shared between annexes (above).
- Locals in a template are not renamed apart from a test's parameters. A
  template that declares a local called like one of the test's parameters
  shadows it.
- A `while` loop written inside a `try` block, in a body that answers
  nothing, does not compile on the Rust backend (the emitted loop variable
  is typed `()` and assigned `None`). Put the `try` inside the loop, as the
  example does.
