// [fmt-comments] [doc-comment] [deduce-syntax] A comment the printer has to
// move never lands directly above a declaration, where it would read back as
// its docs: one written inside a declaration's header goes to the next slot,
// and the docs stay the declaration's. Above a deduction clause on a line of
// its own, it keeps its line; among the generics, it goes after the member,
// with a blank line before the next, which keeps no docs.

fn f(x: Int) -> Int
// inside the header
=> x {
    return x
}
// the docs of g
fn g() {}

effect E {
    fn f(x: Int) -> Int
    // inside the header
    => x
    fn g() -> None
}

effect F {
    fn f<A,
        // inside the header
        B>(x: A) -> B
    fn g() -> None
}
