// [kt-variadic] a user-declared variadic, every element type
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- user_variadics

fn count_ints(label: Str, ...ns: Int[]) [Console] -> Int => label, ns {
    let seen = Mut List(0)
    for n in iter(ns) {
        add(seen, copy(n))
    }
    println("${label} ${size(ns)}")
    return size(seen)
}

fn join_strs(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let joined = Mut Str(...parts)
    println("${sep} ${joined}")
}

fn main() [use] {
    use StdConsole()

    // A variadic of a *primitive* element type: plain, spread, mixed, empty.
    let a = count_ints("plain", 1, 2, 3)
    let rest = array_of(2, 3)
    let b = count_ints("spread", ...rest)
    let c = count_ints("mixed", 1, ...rest)
    let d = count_ints("empty")
    println("counts ${a} ${b} ${c} ${d}")

    // The spread source survives the calls: a variadic position is not
    // tracked, so the callee's owned parameter is built by cloning.
    println("reusable ${size(rest)}")

    // A reference element type, which always worked.
    join_strs("refs", "a", "b")
    let words = array_of("y", "z")
    join_strs("refs", "x", ...words)
}
//= stdout
//= plain 3
//= spread 2
//= mixed 3
//= empty 0
//= counts 4 3 4 1
//= reusable 2
//= refs ab
//= refs xyz

// [rs-read-mode] [rs-narrow-mut] The parity half of the Rust backend's
// `an_intrinsic_argument_takes_the_intrinsics_own_mode`: Kotlin needs no
// change for it (`?:` yields the storage, and a list is a reference), so this
// case is what says the two backends agree. Until 2026-09-23 they did not —
// `add(xs!, 3)` appended to a clone on Rust and printed `xs 1`.
//--- reads_of_an_optional_local

fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    if contains(maybe ?: unreachable!(), "ell") { println("found") }
    println("upper ${to_upper(maybe)}")
    let xs: Mut List<Int>? = Mut List(1)
    add(xs ?: unreachable!(), 3)
    println("xs ${size(xs)}")
}
//= stdout
//= found
//= upper HELLO
//= xs 2
