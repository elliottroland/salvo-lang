// [fn-kept-borrow] A fn value at a slot whose parameter is a borrow.

//--- a_consuming_fn_does_not_read_a_borrow
// [fn-kept-borrow] Only a *kept* parameter reads a borrow: a fn consuming
// its parameter (`=> !s`) is refused at a view-element slot, by an overload,
// an annotated local and an override, with the reason and the remedies; and
// a `Mut` difference stays invariant ([fn-erased-variance]): neither a
// mutating `grow(xs: Mut List<Str>)` nor a reading `peek(xs: List<Str>)`
// fills a slot whose parameter is a `proj Mut` list. (From the Rust
// backend's codegen tests: these are the checker's errors.)
fn eat(s: Str) -> Bool => !s {
    return size(s) > 1
}

fn same(a: Str, b: Str) -> Bool => !a, b {
    return a == b
}

fn grow(xs: Mut List<Str>) -> Bool => xs: Mut {
    add(xs, "x")
    return true
}

fn peek(xs: List<Str>) -> Bool => xs {
    return size(xs) > 0
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    let vs = filter(iter(ws), s -> size(s) > 1)
    //~ error[call-resolve:no-overload-why]
    println("any ${any(vs, eat)}")
    //~ error[var-let-infer:annotation-mismatch]
    //~ warning[var-unused:never-used]
    let k: (proj Str) -> Bool = eat
    //~ error[implicit-override:does-not-fit]
    println("has ${contains(vs, "ab", eq = same)}")
    let ls = List(Mut List("p"), Mut List("q"))
    let big = filter(iter(ls), x -> size(x) > 0)
    //~ error[call-resolve:no-overload]
    println("grow ${any(big, grow)}")
    //~ error[call-resolve:no-overload]
    println("peek ${any(big, peek)}")
}
