// [fn-overload-at] [fn-rename] the caller's two overrides
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- overload_overrides

qualifier Even of Int {
    fn qualifies(n: Int) -> Bool { return n % 2 == 0 }
}

qualifier Small of Int {
    fn qualifies(n: Int) -> Bool { return n < 10 }
}

fn size<T>(list: List<T>) -> Str => list {
    return "mine"
}

fn label(n: Even Int) -> Str => n {
    return "even"
}

fn label(n: Small Int) -> Str => n {
    return "small"
}

// The two `label`s are unrankable — one qualifier each, and the *kind* of
// qualifier never ranks — so one of them takes a name of its own.
rename fn label_small = label(n: Small Int)

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    // Two `size`s fit and neither is more specific, so each call names its
    // place [fn-overload-ambiguous].
    println(size@main(xs))
    println("core: ${size@core.list(xs)}")
    println("dot: ${xs.size@core.list()}")
    let n = 4
    if n is Even && n is Small {
        println("${label(n)} ${label_small(n)}")
    }
    // A local shadows every function of that name — `@` is the way out.
    let describe = "a local"
    println(describe)
    println(describe@main(7))
}

fn describe(n: Int) -> Str {
    return "fn ${n}"
}
//= stdout
//= mine
//= core: 3
//= dot: 3
//= even small
//= a local
//= fn 7
