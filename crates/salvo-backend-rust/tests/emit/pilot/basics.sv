// The emit runner's own pilot.

//--- own_code
fn twice(n: Int) -> Int {
    //~ emits: pub fn twice(mut n: i32) -> i32 {
    //~|     return i32::wrapping_mul(n, 2i32);
    return n * 2
}

fn main() [use] -> None {
    use StdConsole()
    //~ emits(main.rs): crate::twice(21i32)
    //~ !emits: twice_twice
    println("${twice(21)}")
}

//--- a_refusal
// A trait object cannot dispatch a member with generics of its own; the
// run data runs this program on Kotlin (`effects/`).
//~ refused[effect-member-generics:no-dynamic-dispatch]
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
    println("${x}")
}
