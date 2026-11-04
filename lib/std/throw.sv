// Throwing: leaving an enclosing `try` block with a message instead of a
// value, through any number of calls in between.
//
// A function that may throw says so in its effect list, `[Throw<Str>]`, and
// keeps its own return type, so the functions between the `throw` and the
// `try` need no result plumbing. The `try` block is built into the language;
// it evaluates to an outcome union, `Ok T | Thrown M`, which is told apart
// with `is` or `when` like any other union.
//
// ```
// fn parse(line: Str) [Throw<Str>] -> Int => line {
//     if size(line) == 0 {
//         throw("empty line")                          // does not return
//     }
//     return size(line)
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     let outcome = try {
//         parse("") * 2
//     }
//     when outcome {
//         is Ok { println("doubled: ${outcome}") }
//         is Thrown { println("could not parse: ${outcome}") }   // printed
//     }
// }
// ```
//
// **Where a throw lands.** In the innermost enclosing `try`; there are no
// labelled throws. A function that calls one that throws either declares
// `[Throw<M>]` too, passing the throw on, or wraps the call in a `try`.
// `main` cannot declare `[Throw<M>]`, since there is no caller to receive
// the throw, and a `try` whose body cannot throw is an error.
//
// **No handler.** `Throw` is an effect, but it is not provided with `use`:
// the `try` block is what makes it available, and `use` of a handler for it
// is an error.
//
// **Linear values.** Nothing linear (a value that must be used exactly
// once, such as an open file) may be live across a call that may throw,
// since the code that would discharge it does not run on the throw path.
// Release it before the call, or move it into the call.
//
// For failures a caller should handle at the call, return an
// `Ok T | Err E` result instead ([std.core.result]). Not part of
// `std.core`, so it is imported: `import std.throw`.
// @compilerrules [throw, throw-try, mod-import-module]
//
// @compilerknown

// The effect a function declares to be allowed to throw messages of type
// `M`: `[Throw<Str>]`. It is made available by an enclosing `try` block,
// never by `use`.
// @compilerrules [throw]
// @compilerknown
export effect Throw<M> {
    // Ends the enclosing `try` block, whose outcome becomes
    // `Thrown message`. It does not return: its type is `Never`, so nothing
    // after it runs. The message is moved into the outcome.
    fn throw(message: M) -> Never => !message
}

// The thrown arm of a `try` outcome: `try { … }` evaluates to
// `Ok T | Thrown M`, where `M` is the union of the message types its body
// may throw.
//
// A thrown message can also be made by hand with [thrown], without
// throwing: the authority to throw is having `[Throw<M>]` available, not
// the tag. At run time the tag is erased; the union records the arm.
// @compilerrules [throw-try, qual-erasure]
// @compilerknown
export provenance qualifier Thrown<M> for M

// Tags a value as the thrown arm of an outcome union without throwing
// anything, for code that produces an outcome by hand: a stub, a test
// double, a value read back from storage.
//
// ```
// fn failed() [] -> Ok Int | Thrown Str {
//     return thrown("not today")
// }
// ```
// @compilerrules [qual-ctor-fn]
export fn thrown<M canbe linear>(message: M) [] -> +Thrown M => !message {
    return message
}
