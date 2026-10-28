// [kt-fn-mangling] overload dispatch is the checker's, not Kotlin's
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// The same program under kotlinc: before the rule it ran until the stack
// ran out, so the assertion that matters is that it terminates with the
// right output.
//--- overload_delegation

fn twice(p: Mut ListNext<Int>, f: (Int) -> Int) -> Mut List<Int> => p: Mut, f {
    return map(p, f)
}

fn twice(xs: List<Int>, f: (Int) -> Int) -> Mut List<Int> => f, !xs {
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
//= stdout
//= v=2
//= v=4
//= v=6
