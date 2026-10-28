// [fn-kept-borrow] a kept owned parameter reads a borrowed argument
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- kept_fns_over_views

fn long(s: Str) -> Bool {
    return size(s) > 1
}

fn longer(s: Str) -> Bool {
    return size(s) > 2
}

fn len(s: Str) -> Int {
    return size(s)
}

fn add_len(a: Int, s: Str) -> Int {
    return a + size(s)
}

// A parameter forwarded into a view-element slot.
fn keep_count(vs: List<proj Str>, keep: (Str) -> Bool) [] -> Int => vs, keep {
    return count(vs, keep)
}

// A slot whose parameter is written `proj`.
fn count_views(vs: List<proj Str>, keep: (x: proj Str) -> Bool) [] -> Int => vs, keep {
    return reduce(vs, 0, (a, v) -> if keep(v) { a + 1 } else { a })
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("def", "c", "ab")
    let vs = filter(iter(ws), s -> size(s) > 1)
    // Each combinator over the view list, with a named kept fn.
    println("any ${any(vs, longer)} ${any(vs, s -> size(s) > 5)}")
    println("all ${all(vs, long)} ${all(vs, longer)}")
    println("count ${count(vs, longer)}")
    let at = find_first(vs, longer) ?: unreachable!()
    println("find_first ${at}")
    let ns = map(vs, len)
    println("map ${size(ns)} ${get(ns, 0) ?: 0} ${get(ns, 1) ?: 0}")
    let fs = filter(vs, longer)
    for f in fs {
        println("filter ${f}")
    }
    println("reduce ${reduce(vs, 0, add_len)}")
    // By a parameter, into a written `proj` slot, and through a local.
    println("param ${keep_count(vs, longer)} ${keep_count(vs, long)}")
    println("slot ${count_views(vs, longer)}")
    let k: (proj Str) -> Bool = longer
    println("local ${k(get(vs, 1) ?: unreachable!())} ${count_views(vs, k)}")
    // Resolved implicits over the view type: `hash`, `eq`, `cmp`, `to_str`.
    let m = to_map(vs, v -> (v, size(v)))
    let n = get(m, "def") ?: unreachable!()
    println("map ${n} ${size(m)} ${contains_key(m, "ab")}")
    let st = to_set(vs)
    println("set ${size(st)} ${contains(st, "ab")} ${contains(st, "zz")}")
    println("list ${contains(vs, "ab")} ${eq(vs, vs)} ${cmp(vs, vs)}")
    println("to_str ${to_str(vs)} ${to_str(sort(vs))}")
}
//= stdout
//= any true false
//= all true false
//= count 1
//= find_first 0
//= map 2 3 2
//= filter def
//= reduce 5
//= param 1 2
//= slot 1
//= local false 1
//= map 3 2 true
//= set 2 true false
//= list true true 0
//= to_str [def, ab] [ab, def]
