// the defect round of 2026-09-25, byte-identical to the Rust side
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_read_before_a_mutation_2

struct Box canbe Mut { n: Int, tag: Str }

fn bumped(b: Mut Box) -> Int => b: Mut {
    b.n = b.n + 1
    return b.n
}

fn label(s: Str, n: Int) -> Str => s, n {
    return "${s}/${n}"
}

fn main() [use] {
    use StdConsole()
    let b = Mut Box { n: 1, tag: "t" }
    println("1. ${b.n} ${bumped(b)}")
    println("2. ${label(b.tag, bumped(b))}")
    println("3. ${bumped(b)} ${b.n}")
}
//= stdout
//= 1. 1 2
//= 2. t/3
//= 3. 4 4
