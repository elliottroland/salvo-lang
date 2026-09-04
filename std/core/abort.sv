// [abort] Non-resumption: leaving an enclosing `try` block with a message
// instead of a value. `abort` is an ordinary effect operation — a function
// that may perform it says so in its effect list, `[Abort<Str>]` — but it
// has no handler: the delimiter is the compiler-intrinsic `try` block
// [try], and `use` of a handler for `Abort` is an error.
//
// Own module rather than part of `core.basic` so a program that never
// aborts emits no code for it [mod-used-only].

// [abort] Ends the enclosing `try` block with [message]. Returns `Nothing`
// (the bottom type), which is what keeps the frames in between silent: a
// function that may abort declares `[Abort<M>]` and returns its *own*
// type, never an outcome union — the union appears in exactly one place,
// the `try` that delimits it.
//
// The message is *moved* into the outcome (`[]` deductions), like the
// value passed to `err`.
effect Abort<M> {
    fn abort(message: M) -> [] Nothing
}

// [try] The aborted arm of a `try` outcome: `try { ... }` evaluates to
// `Ok T | Aborted M`, so the arms are discriminated with `is`/`when`
// exactly like a result union's. Erased in generated code
// [qual-erasure]: the union wrapper carries the arm.
//
// Deliberately *forgeable*: the qualifier carries no authority, so the
// constructor below produces a value in the aborted arm without
// transferring control. The authority to abort is `[Abort<M>]`
// availability alone.
qualifier Aborted<M> of M

// [qual-ctor-fn] Tags a value as the aborted arm of an outcome union
// without aborting anything — for code that produces an outcome by hand
// (a stub, a test double, a value read back from storage).
fn aborted<M canbe Linear>(message: M) [] -> [] M as Aborted {
    return message
}
