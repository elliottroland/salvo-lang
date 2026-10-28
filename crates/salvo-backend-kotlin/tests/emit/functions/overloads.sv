// Overloads [kt-fn-mangling]: dispatch is the checker's, not Kotlin's.

//--- every_emitted_overload_gets_its_own_kotlin_name
// [kt-fn-mangling] [fn-overload] Every emitted overload of a name gets a
// unique Kotlin name, so Kotlin never re-resolves a call the checker
// already resolved. Sharing the name let Kotlin apply *its* lattice:
// `iter(list)` lowers to the identity, so the `List` overload's
// `twice(xs.iter(), f)` emitted `twice(xs, f)`, whose most specific
// Kotlin candidate is the `List` overload itself — an infinite recursion,
// with no diagnostic anywhere [backend-never-wrong].
// Two overloads the *checker* tells apart by Salvo types — a pass and the
// `List` it walks — where the `List` overload **delegates** to the pass one.
// (`OVERLOAD_DELEGATION` in codegen_tests.rs.)
//~ emits: fun twice__ListNext_Fn(p: salvo.std.core.list.ListNext<Int>
fn twice(p: Mut ListNext<Int>, f: (Int) -> Int) -> Mut List<Int> => p: Mut, f {
    return map(p, f)
}

//~ emits: fun twice__List_Fn(xs: List<Int>
fn twice(xs: List<Int>, f: (Int) -> Int) -> Mut List<Int> => f, !xs {
    // The delegation calls the *other* overload, by its own name.
    //~ emits: return salvo.main.twice__ListNext_Fn(
    return twice(iter(xs), f)
}

fn double(n: Int) -> Int {
    return n * 2
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    for v in xs.twice(double) {
        println("v=${v}")
    }
}

//--- scope_selectors_and_renames_are_erased
// [fn-overload-at] [fn-rename] Neither override survives into the output:
// the calls emit the ordinary (mangled) names of the declarations the
// checker resolved.
// A program that overrides overload resolution both ways: a module-level
// `size` that shadows std's, `@module` to reach past it (and past a local of
// the same name), and a `rename` that gives one of two unrankable overloads
// a name of its own. Both are **compile-time only** — the checker records
// which declaration each call means and mangling keeps the target from
// re-resolving anything [kt-fn-mangling] — so the point of the pair is that
// the two backends produce the same answers from the same source.
// (`OVERLOAD_OVERRIDE_DEMO` in codegen_tests.rs.)
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
// No `@` and no renamed name reaches Kotlin.
//~ !emits: @
//~ !emits: label_small
rename fn label_small = label(n: Small Int)

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3)
    // Two `size`s fit and neither is more specific, so each call names its
    // place [fn-overload-ambiguous].
    println(size@main(xs))
    // `size@core.list(xs)` is std's `size` — the mangled one, since this
    // program declares its own.
    //~ emits: "core: ${salvo.std.core.list.sizePlatform(xs)}"
    println("core: ${size@core.list(xs)}")
    println("dot: ${xs.size@core.list()}")
    let n = 4
    if n is Even && n is Small {
        // The renamed overload is called by its declaration's mangled name —
        // both `label`s are mangled, since they are overloads of one name
        // [kt-fn-mangling].
        //~ emits: ${salvo.main.label__EvenInt(n)} ${salvo.main.label__SmallInt(n)}
        println("${label(n)} ${label_small(n)}")
    }
    // A local shadows every function of that name — `@` is the way out.
    let describe = "a local"
    println(describe)
    // A call reaching past a local of the same name names the declaration
    // by its package [kt-decl-call-qualified], as every declared call does:
    // a fn-typed local of the name would otherwise take it.
    //~ emits: , salvo.main.describe(7))
    println(describe@main(7))
}

fn describe(n: Int) -> Str {
    return "fn ${n}"
}
