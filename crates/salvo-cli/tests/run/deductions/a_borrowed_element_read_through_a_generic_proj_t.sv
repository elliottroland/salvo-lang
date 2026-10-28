// [rs-proj-flat] a borrowed element read through a generic `proj T`
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- borrowed_element_reads

struct Bag {
    names: List<proj Str>
}

// A `proj` element list in a struct field.
fn show(b: Bag) [use] -> None {
    use StdConsole
    for n in iter(b.names) {
        println("field for ${n}")
    }
    let g = get(b.names, 0) ?: unreachable!()
    println("field get ${g}")
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    let vs = filter(iter(ws), s -> size(s) > 1)
    // The repro (ROADMAP 0z).
    for v in vs {
        println("list for ${v}")
    }
    let h = get(vs, 0) ?: unreachable!()
    println("get ${h}")
    for v in iter(vs) {
        println("iter for ${v}")
    }
    // Its neighbours.
    let l = last(vs) ?: unreachable!()
    println("last ${l}")
    if vs is NonEmpty {
        println("first ${first(vs)}")
    }
    let ms = map(iter(vs), s -> size(s))
    println("map ${size(ms)} ${get(ms, 1) ?: 0}")
    let fs = filter(iter(vs), s -> size(s) > 2)
    for f in fs {
        println("filter ${f}")
    }
    println("count ${count(iter(vs))}")
    let at = find_first(vs, s -> size(s) > 2) ?: unreachable!()
    println("find_first ${at}")
    println("reduce ${reduce(iter(vs), 0, (a, s) -> a + size(s))}")
    let t = to_list(iter(vs))
    for x in iter(t) {
        println("to_list ${x}")
    }
    let m = to_map(vs, v -> (copy(v), v))
    let mv = get(m, "def") ?: unreachable!()
    println("map get ${mv}")
    show(Bag {names: vs})
    // Nested: a `List<proj List<Str>>`.
    let ls = List(List("p", "q"), List("r"))
    let big = filter(iter(ls), x -> size(x) > 0)
    let fst = get(big, 0) ?: unreachable!()
    println("nested get ${size(fst)}")
    for b in iter(big) {
        println("nested for ${size(b)}")
    }
}
//= stdout
//= list for ab
//= list for def
//= get ab
//= iter for ab
//= iter for def
//= last def
//= first ab
//= map 2 3
//= filter def
//= count 2
//= find_first 1
//= reduce 5
//= to_list ab
//= to_list def
//= map get def
//= field for ab
//= field for def
//= field get ab
//= nested get 2
//= nested for 2
//= nested for 1
