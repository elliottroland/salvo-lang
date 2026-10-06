// The keyed-collection benchmarks (ROADMAP §0j step 7): each workload timed on
// the monotonic clock and printed as milliseconds, with a checksum so nothing
// is optimised away. Run with `tools/bench-collections.sh`, which builds both
// backends optimised. Not part of the test suite: the output is timing.
//
//   set-int    10^6 adds of Int, then 2·10^6 lookups (half hits)
//   map-str    10^5 puts of Str keys, then 10^6 gets
//   remove     10^6 adds, 9·10^5 removes, then a full walk
//   sorted     2·10^4 adds to a SortedSet<Int>, then 10^6 lookups
import time

fn ms_since(start: Long) [] -> Long {
    return (monotonic_nanos() - start) / 1000000
}

fn set_int() [] -> Int {
    let s: Mut Set<Int> = mut_set_of()
    let i = 0
    while i < 1000000 {
        let _new = add(s, i * 7)
        i = i + 1
    }
    let hits = 0
    let j = 0
    while j < 2000000 {
        if contains(s, j * 7 / 2) {
            hits = hits + 1
        }
        j = j + 1
    }
    return hits
}

fn map_str() [] -> Int {
    let m: Mut Map<Str, Int> = mut_map_of()
    let i = 0
    while i < 100000 {
        put(m, "key-${i}", copy(i))
        i = i + 1
    }
    let total = 0
    let j = 0
    while j < 1000000 {
        let v = get(m, "key-${j % 120000}")
        if v is Int n {
            total = total + n
        }
        j = j + 1
    }
    return total
}

fn remove_walk() [] -> Int {
    let s: Mut Set<Int> = mut_set_of()
    let i = 0
    while i < 1000000 {
        let _new = add(s, copy(i))
        i = i + 1
    }
    let j = 0
    while j < 900000 {
        let _gone = remove(s, j)
        j = j + 1
    }
    let total = 0
    for x in s {
        total = total + x
    }
    return total
}

fn sorted() [] -> Int {
    let s: Mut SortedSet<Int> = mut_sorted_set_of()
    let i = 0
    while i < 20000 {
        let _new = add(s, (i * 7919) % 20011)
        i = i + 1
    }
    let hits = 0
    let j = 0
    while j < 1000000 {
        if contains(s, j % 30000) {
            hits = hits + 1
        }
        j = j + 1
    }
    return hits
}

fn main() [use] {
    use StdOutConsole()
    let t1 = monotonic_nanos()
    let a = set_int()
    println("set-int ${ms_since(t1)} ms (${a})")
    let t2 = monotonic_nanos()
    let b = map_str()
    println("map-str ${ms_since(t2)} ms (${b})")
    let t3 = monotonic_nanos()
    let c = remove_walk()
    println("remove ${ms_since(t3)} ms (${c})")
    let t4 = monotonic_nanos()
    let d = sorted()
    println("sorted ${ms_since(t4)} ms (${d})")
}
