// [rs-fn-param-convention] generic fn-typed parameters
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// The same program under rustc: the conventions have to agree for every
// argument form and both `Copy`-ness cases, and the output is what the
// Kotlin backend asserts for the same source.
//--- a_generic_higher_order_fn

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
    for v in map(iter(ns), (n: Int) -> n * 3) { println("ann=${v}") }
    let ws = List("hi")
    for w in map(iter(copy(ws)), (s: Str) -> shout(s)) { println("str=${w}") }
    for w in map(iter(copy(ws)), shout) { println("named=${w}") }
    for n in map(iter(ws), s -> size(s)) { println("size=${n}") }
    println("applied=${apply(2, (n: Int) -> n + 1)}")
}
//= stdout
//= bare=2
//= bare=4
//= ann=3
//= ann=6
//= str=hi!
//= named=hi!
//= size=2
//= applied=3
