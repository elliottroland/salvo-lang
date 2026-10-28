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
    let rest = array_of(2, 3)
    let all = List(1, ...rest)
    println("List ${to_str(all)}")

    // The spread source is still usable: a variadic position is not tracked,
    // so the store clones rather than moves.
    println("reusable ${size(rest)}")

    let s = Set(9, ...rest)
    println("Set ${to_str(s)}")
    let sorted = SortedSet(9, ...rest)
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
    let lone = Set(...rest)
    println("lone ${to_str(lone)}")
}
//= stdout
//= List [1, 2, 3]
//= reusable 2
//= Set {9, 2, 3}
//= SortedSet {2, 3, 9}
//= Map {a: 1, b: 2}
//= mut_str abc
//= ordinary xbc
//= lone {2, 3}
