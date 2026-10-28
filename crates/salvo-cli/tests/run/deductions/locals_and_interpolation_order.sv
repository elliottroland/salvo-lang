// reassigned locals and interpolation order
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run tests.

// [rs-borrow-locals] [rs-opt-borrow] [proj-local] A local bound from a kept
// `Str?` parameter and later reassigned owns its value from the start: Rust
// stored it as the parameter's `Option<&String>`, and the assignment of an
// owned `Option<String>` was E0308 (ROADMAP §2d item 11). A local whose type
// is a projection (`let k = get(xs, 0)`) is a borrow for its whole life;
// with `copy` at the declaration (the remedy the checker names, user
// decision 2026-10-20) it owns, and runs.
//--- a_reassigned_local_owns_for_its_whole_life
fn up(s: Str) [] -> Str? {
    if size(s) > 1 {
        return substr(s, 1, size(s))
    }
    return None
}

fn walk(key: Str?) [] -> Int => key {
    let k = key
    let n = 0
    while k !is None {
        let here = copy(k)
        n = n + 1
        k = up(here)
    }
    return n
}

fn walk_list(xs: List<Str>) [] -> Int => xs {
    let k = copy(get(xs, 0))
    let n = 0
    while k !is None {
        let here = copy(k)
        n = n + 1
        k = up(here)
    }
    return n
}

fn main() [use] -> None {
    use StdConsole()
    println("${walk("abcd")} ${walk_list(List("xyz"))}")
}
//= stdout
//= 4 3

// [rs-interp-sequence] An interpolation whose earlier part reads a place
// through a call and whose later part mutates it runs in source order
// (rustc used to refuse it: E0502).
//--- an_interpolation_reads_before_a_later_part_mutates
fn main() [use] {
    use StdConsole()
    let d = Mut List(1, 2, 3)
    let i = 1
    if i is Idx(d) {
        println("${get(d, i)} ${replace(d, i, 0)}")
    }
    println("${d}")
}
//= stdout
//= 2 2
//= [1, 0, 3]
