// effect member fns with their own generics
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- member_generics
// BACKENDS: kotlin

effect Stash {
    fn pick<T>(a: T, b: T) -> T => !a, !b
}

handler FirstStash of Stash {
    fn pick<T>(a: T, b: T) -> T
    => !a, !b {
        return a
    }
}

fn main() [use] -> None {
    use StdConsole
    use FirstStash
    let x = pick(7, 2)
    let s = pick("l", "r")
    println("${x} ${s}")
}
//= stdout
//= 7 l
