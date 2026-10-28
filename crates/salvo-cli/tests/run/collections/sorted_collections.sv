// sorted collections [col-sorted]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- codepoint_string_order

fn main() [use] -> None {
    use StdConsole()
    let bmp = "Ａ"
    let s = SortedSet("😀", "Ａ")
    let first = min(s)
    if first is Str {
        let is_bmp = first == bmp
        println("smallest is the BMP char: ${is_bmp}")
    }
}
//= stdout
//= smallest is the BMP char: true

//--- sorted_collections

fn main() [use] -> None {
    use StdConsole()
    let s: Mut SortedSet<Str> = Mut SortedSet("pear", "apple", "fig")
    println("set: ${to_str(s)}")
    let added = add(s, "banana")
    let dup = add(s, "apple")
    println("add ${added} dup ${dup} now ${to_str(s)}")
    // [col-nonempty] `add` established `NonEmpty`, so `min`/`max` resolve to
    // the overloads that answer with an element rather than an optional —
    // no test needed here any more.
    let lo = min(s)
    let hi = max(s)
    println("min ${lo} max ${hi}")
    let had = remove(s, "fig")
    println("removed ${had} left ${to_str(s)} size ${size(s)}")
    let ns = SortedSet(10, 2, 33, 4)
    println("ints ${to_str(ns)} list ${to_str(to_list(ns))}")

    let m: Mut SortedMap<Str, Int> = Mut SortedMap(("two", 2), ("one", 1))
    put(m, "three", 3)
    println("map ${to_str(m)}")
    let v = get(m, "two")
    if v is Int {
        println("get ${v}")
    }
    // Likewise: `put` established the claim on the map.
    let fk = first_key(m)
    let lk = last_key(m)
    println("first ${fk} last ${lk}")
    let taken = remove(m, "one")
    if taken is Int {
        println("took ${taken} left ${to_str(m)}")
    }
    let has3 = contains_key(m, "three")
    println("keys ${to_str(keys(m))} has ${has3}")
    for e in iter(ns) {
        println("e ${e}")
    }
    for k in iter(m) {
        println("k ${k}")
    }
}
//= stdout
//= set: {apple, fig, pear}
//= add true dup false now {apple, banana, fig, pear}
//= min apple max pear
//= removed true left {apple, banana, pear} size 3
//= ints {2, 4, 10, 33} list [2, 4, 10, 33]
//= map {one: 1, three: 3, two: 2}
//= get 2
//= first one last two
//= took 1 left {three: 3, two: 2}
//= keys [three, two] has true
//= e 2
//= e 4
//= e 10
//= e 33
//= k three
//= k two
