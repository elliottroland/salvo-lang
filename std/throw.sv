// [throw] Non-resumption: leaving an enclosing `try` block with a message
// instead of a value. `throw` is an ordinary effect operation — a function
// that may perform it says so in its effect list, `[Throw<Str>]` — but it
// has no handler: the delimiter is the compiler-intrinsic `try` block
// [try], and `use` of a handler for `Throw` is an error.
//
// Its own module, **outside `core`** (2026-09-26): a program that never throws
// never names `Throw`, `Thrown` or `throw`, so it neither imports this module
// nor emits code for it. `import throw` is the one line a program that does
// need it writes [mod-import-module].

// [throw] Ends the enclosing `try` block with [message]. Returns `Never`
// (the bottom type), which is what keeps the frames in between silent: a
// function that may throw declares `[Throw<M>]` and returns its *own*
// type, never an outcome union — the union appears in exactly one place,
// the `try` that delimits it.
//
// The message is *moved* into the outcome (the body returns it, so the move
// is inferred), like the value passed to `err`.
export effect Throw<M> {
    fn throw(message: M) -> Never => !message
}

// [try] The thrown arm of a `try` outcome: `try { ... }` evaluates to
// `Ok T | Thrown M`, so the arms are discriminated with `is`/`when`
// exactly like a result union's. Erased in generated code
// [qual-erasure]: the union wrapper carries the arm.
//
// Deliberately *forgeable*: the qualifier carries no authority, so the
// constructor below produces a value in the thrown arm without
// transferring control. The authority to throw is `[Throw<M>]`
// availability alone.
export provenance qualifier Thrown<M> of M

// [qual-ctor-fn] Tags a value as the thrown arm of an outcome union
// without throwing anything — for code that produces an outcome by hand
// (a stub, a test double, a value read back from storage).
export fn thrown<M canbe linear>(message: M) [] -> +Thrown M {
    return message
}
