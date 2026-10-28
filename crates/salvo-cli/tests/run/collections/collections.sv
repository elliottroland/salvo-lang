// collections
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- collection_iteration

fn main() [use] -> None {
    use StdConsole()
    let s = Set("b", "a", "c")
    for e in iter(s) {
        println("elem: ${e}")
    }
    let xs = to_list(s)
    println("as list: ${to_str(xs)}")
    let n: Int = reduce(iter(s), 0, (acc, e) -> acc + size(e))
    println("total length: ${n}")

    let m = Map(("one", 1), ("two", 2), ("three", 3))
    for k in iter(m) {
        let v = get(m, k)
        if v is Int {
            println("${k} -> ${v}")
        }
    }
    println("keys: ${to_str(keys(m))}")
    let total: Int = reduce(iter(m), 0, (acc, k) -> acc + size(k))
    println("key length total: ${total}")

    let empty: Set<Int> = Set()
    for e in iter(empty) {
        println("unreachable ${e}")
    }
    println("done")
}
//= stdout
//= elem: b
//= elem: a
//= elem: c
//= as list: [b, a, c]
//= total length: 3
//= one -> 1
//= two -> 2
//= three -> 3
//= keys: [one, two, three]
//= key length total: 11
//= done

//--- collections

fn main() [use] -> None {
    use StdConsole()

    let s: Mut Set<Str> = Mut Set("b", "a", "c")
    println("set: ${to_str(s)} size: ${size(s)}")
    let added_new = add(s, "d")
    let added_dup = add(s, "a")
    println("add new: ${added_new} dup: ${added_dup} now: ${to_str(s)}")
    let has_a = contains(s, "a")
    let gone = remove(s, "a")
    let gone_again = remove(s, "a")
    println("contains: ${has_a} remove: ${gone} again: ${gone_again} left: ${to_str(s)}")
    let dedup = Set(1, 2, 1, 3)
    println("dedup: ${to_str(dedup)} size: ${size(dedup)}")

    let m: Mut Map<Str, Int> = Mut Map(("one", 1), ("two", 2))
    put(m, "three", 3)
    println("map: ${to_str(m)}")
    put(m, "one", 111)
    println("overwrite keeps position: ${to_str(m)}")
    let two = get(m, "two")
    if two is Int {
        println("get: ${two}")
    }
    let absent = get(m, "nope")
    let is_absent = absent is None
    let taken = remove(m, "two")
    if taken is Int {
        println("absent: ${is_absent} removed: ${taken} left: ${to_str(m)}")
    }
    let has_one = contains_key(m, "one")
    println("contains_key: ${has_one} size: ${size(m)}")
    let dup_keys = Map(("x", 1), ("y", 2), ("x", 9))
    println("dup keys: ${to_str(dup_keys)}")
    let ints: Mut Map<Int, Str> = Mut Map()
    put(ints, 7, "seven")
    put(ints, 3, "three")
    println("int keys: ${to_str(ints)}")
}
//= stdout
//= set: {b, a, c} size: 3
//= add new: true dup: false now: {b, a, c, d}
//= contains: true remove: true again: false left: {b, c, d}
//= dedup: {1, 2, 3} size: 3
//= map: {one: 1, two: 2, three: 3}
//= overwrite keeps position: {one: 111, two: 2, three: 3}
//= get: 2
//= absent: true removed: 2 left: {one: 111, three: 3}
//= contains_key: true size: 2
//= dup keys: {x: 9, y: 2}
//= int keys: {7: seven, 3: three}
