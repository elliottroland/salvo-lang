// [fn-variadic] mixing plain arguments with a `...spread`
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- mixed_spread

fn join_all(sep: Str, ...parts: Str[]) [Console] -> None => sep, parts {
    let out = Mut Str(...parts)
    println("${sep} ${out}")
}

fn main() [use] {
    use StdConsole()
    // A constructor consumes its tail, a spread's array with it: each
    // constructor gets an array of its own.
    let rest = array_of(2, 3)
    let all = List(1, ...rest)
    println("List ${to_str(all)}")

    // A function that keeps its tail leaves the spread usable.
    println("kept ${join_size(...all_parts())}")

    let s = Set(9, ...array_of(2, 3))
    println("Set ${to_str(s)}")
    let sorted = SortedSet(9, ...array_of(2, 3))
    println("SortedSet ${to_str(sorted)}")

    let pairs = array_of(("b", 2))
    let m = Map(("a", 1), ...pairs)
    println("Map ${to_str(m)}")

    let words = array_of("b", "c")
    let joined = Mut Str("a", ...words)
    println("mut_str ${joined}")
    join_all("ordinary", "x", ...words)

    // A lone spread still forwards the whole collection — into a purely
    // variadic constructor. `List` is not one any more [col-of-nonempty]:
    // its element shape takes a *first*, which a spread may not supply.
    let lone = Set(...array_of(2, 3))
    println("lone ${to_str(lone)}")

    let parts = array_of("p", "q")
    println("${join_size(...parts)} ${join_size(...parts)}")
}

fn all_parts() -> Str[] {
    return array_of("x", "y")
}

fn join_size(...parts: Str[]) -> Int => parts {
    return size(parts)
}
//= stdout
//= List [1, 2, 3]
//= kept 2
//= Set {9, 2, 3}
//= SortedSet {2, 3, 9}
//= Map {a: 1, b: 2}
//= mut_str abc
//= ordinary xbc
//= lone {2, 3}
//= 2 2
