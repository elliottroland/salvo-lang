# Throwing

## Overview

Most languages report a failure in one of two ways. An **exception** (Java,
Kotlin, Python) leaves the function at once and unwinds to whoever catches
it, so the code in between stays clean, but usually nothing in a signature
says it can happen (Java's checked exceptions are the rare case that does).
A **result value**
(Rust's `Result`, Salvo's `Ok T | Err E`) is returned like any other value,
so the signature says it all, but every caller has to unwrap or forward it.

Salvo's `throw` takes the control flow of an exception and the honesty of a
result. It is an effect ([Effects and handlers](Effects-and-Handlers.md)), so
a function that may throw says so in its effect list, and keeps its own
return type:

```salvo
import std.throw

fn parse(line: Str) [Throw<Str>] -> Int {
    if size(line) == 0 {
        throw("empty line")        // does not return
    }
    return size(line)
}
```

`parse` returns `Int`, not an outcome union: no `Result` plumbing, no
unwrapping at each call. A function that calls `parse` either declares
`[Throw<Str>]` too, passing the throw on, or *delimits* it with `try`, which
turns the throw back into a value:

```salvo
let outcome = try {
    let n = parse(line)
    n * 2
}
// outcome: Ok Int | Thrown Str

when outcome {
    is Ok {
        println("length doubled: ${outcome}")
    }
    is Thrown {
        println("could not parse: ${outcome}")
    }
}
```

The outcome is an ordinary union, so `is`, `when` and exhaustiveness work
exactly as they do on `Ok Int | Err Str`.

## The `Throw` effect

An effect handler normally *resumes*: the operation runs and control comes
back to the caller. `throw` is the other option: it does not come back. It
is declared in the standard library as an ordinary effect, in a module of its
own ([`std.throw`](../../lib/std/throw.sv)) that you import:

```salvo
effect Throw<M> {
    fn throw(message: M) -> Never => !message
}
```

`import std.throw` brings [`Throw`](../../lib/std/throw.sv#effect-throw),
`try`'s [`Thrown`](../../lib/std/throw.sv#qualifier-thrown) arm and
[`thrown`](../../lib/std/throw.sv#fn-thrown), so a program that never throws
links none of it. A test annex is the exception: the harness, not the
author, puts `[Throw<Failure>]` on a test body, so a test file behaves as if
it had written the import (see [Testing](Testing.md)).

`throw` returns `Never`, the bottom type, so the code after it never runs,
which is what keeps the frames in between silent. The message is moved into
the outcome.

`Throw` has **no handlers**: `handler X for Throw<Str>` is an error. What
provides the effect is an enclosing `try`, or a caller that declares
`[Throw<M>]` in turn. A throw's message type must fit where it lands: a
`Str` throw inside a function declaring `[Throw<Int>]` is an error naming
both.

## Delimiting with `try`

`try { … }` is a **compiler intrinsic** rather than an effect: there is no
`Try` handler to register, just as there is nothing to register for loops or
`if`. It is an expression of type `Ok T | Thrown M`, where `T` is the body's
value type and `M` the message type.

Both arms are qualified. `Ok` is the same tag
[`std.core.result`](../../lib/std/core/result.sv#qualifier-ok) uses; `Err` is
deliberately *not* reused, because a throw is not an error value.

## Rules and caveats

- **`M` is the union of the message types the body can throw with.** A body
  that throws with a `Str` in one place and an `Int` in another yields
  `Ok T | Thrown (Str | Int)`, the same way an `if` with two branch types
  yields their union. With one message type it stays bare.
- **A throw lands in the innermost `try`.** There are no labelled throws; a
  nested delimiter takes its own body's throws and lets an outer one pass
  through.
- **A `return` inside a `try` returns from the enclosing function**: the
  delimiter catches throws, not returns.
- **A `try` whose body cannot throw is an error.** Nothing can produce the
  `Thrown` arm, so the `try` is dead scaffolding; the diagnostic says to drop
  it.
- **`main` cannot declare `[Throw<M>]`**: there is no caller to receive the
  throw, so the delimiter has to be inside.
- **`Thrown M` is forgeable, deliberately.** The qualifier carries no
  authority: the `throw` module's `thrown(message)` constructor produces a
  value in the thrown arm without transferring control. The authority to
  throw is `[Throw<M>]` availability alone.

## Linear values and throws

**Nothing linear may be live across a call that may throw.** The code after
the call does not run on the throw path, so the obligation would be owed on a
path with no code left to discharge it (see [Linear types](Linear-Types.md)).
Release the value before the call, or move it onward so the obligation
travels with it.

Salvo once had a third option, `defer`: a block spliced in at every exit,
including the throw path. It was removed because linearity is what *checks*
the obligation, so a construct that discharged it out of sight was a partial
solution to a problem already solved.

## How the backends implement it

Both backends implement throwing without colouring any function the author
did not annotate. Rust returns `ControlFlow<M, T>` from a function that
declares `[Throw<M>]` (a throw is a plain `return`, propagation is `?`), and
Kotlin throws a generated signal the innermost `try` catches.

## Every error, not the first: `Errs`

`throw` stops at the first failure, and so does an `Ok T | Err E` result.
When the user should see every problem at once (each bad flag, each invalid
row), [`std.core.result`](../../lib/std/core/result.sv) has an accumulating
outcome, `Ok T | Errs NonEmpty List<E>`
([`Errs`](../../lib/std/core/result.sv#qualifier-errs)), and combinators that
keep collecting after a failure:

```salvo
fn port(s: Str) [] -> Ok Int | Errs NonEmpty List<Str> => s {
    let n = parse_int(s)
    if n is None {
        return errs("not a port: ${s}")
    }
    return Ok(n)
}

map2(port("x"), name(""), (p: Int, n: Str) -> "${n}:${p}")
// Errs ["not a port: x", "empty name"]
```

[`errs(e)`](../../lib/std/core/result.sv#fn-errs) is the failure with one
error, [`both(a, b)`](../../lib/std/core/result.sv#fn-both) pairs two values,
and [`map2(a, b, f)`](../../lib/std/core/result.sv#fn-map2) combines them;
when both sides failed, the result holds `a`'s errors and then `b`'s.
[Command-line tools](Command-Line.md#errors) use this outcome to report every
bad argument at once.
