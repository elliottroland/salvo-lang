// Generic fn-typed parameters [rs-fn-param-convention].

//--- a_lambda_binds_a_generic_fn_parameter_by_reference
// A generic higher-order fn, in every argument form: an un-annotated
// lambda, an *annotated* one, and a named fn — over a `Copy` element type
// (`Int`) and a non-`Copy` one (`Str`).
//
// [rs-fn-param-convention] [fn-contract] The declaration of a generic
// fn-typed parameter borrows — a type variable is never known to be
// `Copy` — so a lambda passed into that position has to bind its
// parameters the same way. Deciding from the lambda's *own* annotation
// instead disagreed exactly where the two types differ: `(n: Int) -> …`
// rendered `|n: i32|` against `FnMut(&T)` and rustc rejected the call
// with `E0631`, while the un-annotated form compiled — so the bug was
// invisible until a lambda was annotated.
//
// The other convention — a callback the callee *keeps*, which arrives owned
// and `'static` because it is called after the call returns — is asserted
// where it belongs, on the hand-written composed pass
// (`a_composed_pass_stores_its_source_and_callback` [rs-fn-field]). std has no
// such function since the lazy pair was removed 2026-09-10.

// The plain generic higher-order fn: borrowed `FnMut`.
//~ emits(main.rs): f: &mut dyn FnMut(&'value T) -> U
fn apply<T, U>(value: T, f: (T) -> U) -> U {
    return f(value)
}

fn shout(word: Str) -> Str {
    return "${word}!"
}

fn main() [use] {
    use StdConsole()
    let ns = List(1, 2)
    for v in map(iter(copy(ns)), n -> n * 2) { println("bare=${v}") }
    // The lambda's *inner* convention follows the declaration: its
    // parameter arrives borrowed, whatever the annotation says.
    //~ emits(main.rs): &mut |mut n| -> i32 {
    //~|         i32::wrapping_mul(*n, 3i32)
    for v in map(iter(ns), (n: Int) -> n * 3) { println("ann=${v}") }
    let ws = List("hi")
    // [iter-yield-proj] The element is a borrow, so the parameter is one
    // reference deeper than the annotation, and is peeled once.
    //~ emits(main.rs): &mut |mut s| -> String {
    //~|         let s = *s;
    //~|         crate::shout(s)
    for w in map(iter(copy(ws)), (s: Str) -> shout(s)) { println("str=${w}") }
    for w in map(iter(copy(ws)), shout) { println("named=${w}") }
    for n in map(iter(ws), s -> size(s)) { println("size=${n}") }
    println("applied=${apply(2, (n: Int) -> n + 1)}")
}
