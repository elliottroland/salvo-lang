// [fn-overload-at] [fn-rename] The caller's two overrides on the Rust backend.

//--- scope_selectors_and_renames_are_erased
// The same source and stdout as the Kotlin backend's
// `kotlinc_compiles_and_runs_overload_overrides`: a module-level `size` that
// shadows std's, `@module` to reach past it (and past a *local* of the same
// name), and a `rename` giving one of two unrankable overloads a name of its
// own. Both mechanisms are compile-time only, so what the pair asserts is
// that the two targets agree about which declaration each call meant.
//
// [fn-overload-at] [rs-shadowed-call] Neither override survives into the
// output — with one Rust-specific consequence: a **local of the same name**
// shadows the
// function in Rust's value namespace (E0618), where Kotlin keeps functions
// and properties apart, so such a call is spelled as a path.
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
//~ !emits: label_small
// The renamed overload is called by its declaration's mangled name.
//~ emits: crate::label__SmallInt(n)
rename fn label_small = label(n: Small Int)

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    // Two `size`s fit and neither is more specific, so each call names its
    // place [fn-overload-ambiguous].
    println(size@main(xs))
    //~ !emits: @
    // `@core.list` is std's `size`, a platform fn's wrapper called by its
    // module's path.
    //~ emits: crate::std_core_list::size_platform::<i32>(&xs)
    println("core: ${size@core.list(xs)}")
    println("dot: ${xs.size@core.list()}")
    let n = 4
    if n is Even && n is Small {
        println("${label(n)} ${label_small(n)}")
    }
    // A local shadows every function of that name — `@` is the way out.
    let describe = "a local"
    println(describe)
    // The shadowed call goes through the crate path.
    //~ emits: crate::describe(7i32)
    println(describe@main(7))
}

fn describe(n: Int) -> Str {
    return "fn ${n}"
}
