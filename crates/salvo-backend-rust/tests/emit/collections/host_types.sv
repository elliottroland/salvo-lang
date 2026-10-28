// The keyed collections are std's host types.

//--- std_host_types
// [platform-value-type] The keyed collections are std's host types
// (`lib/platform/std/core/{set,map,sorted}.rs`), so no runtime module is emitted
// for them (ROADMAP §0j step 7), and the emitted code names Rust's unordered
// `HashMap`/`HashSet` nowhere [col-insertion-order]. (That no
// `collections.rs` is emitted stays in `codegen_tests.rs`.)
fn main() [use] -> None {
    use StdConsole()

    //~ !emits(main.rs): HashSet<
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

    //~ !emits(main.rs): HashMap<
    // The map's host file is carried.
    //~ emits(platform/std/core/map.rs): pub struct Map<K, V> {
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
