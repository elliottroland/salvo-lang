# Throwing

## Throwing: leaving early with a message

Handlers so far always *resume*: an effect operation runs and control comes back. `throw` is the other option — it does not come back. It is declared in the core library as an ordinary effect:

```
effect Throw<M> {
    fn throw(message: M) -> Never => !message
}
```

A function that may throw says so in its effect list, and keeps its own return type:

```
fn parse(line: Str) [Throw<Str>] -> Int {
    if size(line) == 0 {
        throw("empty line")        // does not return
    }
    return size(line)
}
```

`throw` returns `Never`, the bottom type, so the code after it never runs — which is what keeps the frames in between silent. `parse` returns `Int`, not an outcome union: no `Result` plumbing, no unwrapping at each call. A function that calls `parse` either declares `[Throw<Str>]` too, passing the throw on, or delimits it.

The delimiter is `try`, a **compiler intrinsic** rather than an effect — there is no `Try` handler to register, just as there is nothing to register for loops or `if`. It evaluates to an outcome:

```
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

Both arms are qualified, and the outcome is an ordinary union, so `is`, `when` and exhaustiveness work exactly as they do on `Ok Int | Err Str`. `Ok` is the same tag `core.result` uses; `Err` is deliberately *not* reused, because a throw is not an error value.

Details worth knowing:

- **`M` is the union of the message types the body can throw with.** A body that throws with a `Str` in one place and an `Int` in another yields `Ok T | Thrown (Str | Int)`, the same way an `if` with two branch types yields their union. With one message type it stays bare.
- **A throw lands in the innermost `try`.** There are no labelled throws; a nested delimiter takes its own body's throws and lets an outer one pass through.
- **A `try` whose body cannot throw is an error.** Never can produce the `Thrown` arm, so the `try` is dead scaffolding; the diagnostic says to drop it.
- **`main` cannot declare `[Throw<M>]`**: there is no caller to receive the throw, so the delimiter has to be inside.
- **`Thrown M` is forgeable, deliberately.** The qualifier carries no authority — `core.throw`'s `thrown(message)` constructor produces a value in the thrown arm without transferring control. The authority to throw is `[Throw<M>]` availability alone.
- **Nothing linear may be live across a call that may throw**: the code after the call does not run on the throw path, so the obligation would be owed on a path with no code left to discharge it. Release before the call, or move the value onward so the obligation travels with it. (`defer` used to be the third option — a block spliced at every exit, including the throw path — and it was removed 2026-09-10: linearity is what *checks* the obligation, so the construct that discharged it out of sight was the partial solution to a problem already solved.)

Both backends implement this without colouring any function the author did not annotate: Rust returns `ControlFlow<M, T>` from a function that declares `[Throw<M>]` (a throw is a plain `return`, propagation is `?`), and Kotlin throws a generated signal the innermost `try` catches.
