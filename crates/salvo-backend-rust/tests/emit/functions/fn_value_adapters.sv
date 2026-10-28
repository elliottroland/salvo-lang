// Fn values adapted to the slot they fill on the Rust backend: a retagged
// borrow peeled, a convention changed, a kept parameter reading a borrow.

//--- forwarded_fn_values
// [rs-proj-arm] [rs-fn-param-convention] [backend-never-wrong] ROADMAP 0y:
// a fn *value* passed to a slot whose element generic the call retags to a
// borrow (`filter`, `map`, `reduce` over a list iterator: `T = &String`, so
// the slot is `&mut dyn FnMut(&&String)`) is adapted as a lambda or a named
// fn already was: a closure peels the extra reference
// (`&mut |__a0: &&String| keep(*__a0)`). The repro (a parameter) and its
// neighbours: forwarded two levels down, a local bound from a lambda or a
// named fn, a field read (also into a plain generic slot, whose call-through
// closure was typed by the uninstantiated `&T`), a forwarded implicit, a
// generic body's own parameter, and a fn value a call answers (an
// `Arc<dyn Fn>`: returned wrapped, bound once and called through when passed,
// directly or from a local). `kotlinc_compiles_and_runs_forwarded_fn_values`
// runs the same program.
struct Picker {
    keep: (Str) -> Bool
}

fn long(s: Str) -> Bool {
    return size(s) > 1
}

// The repro (ROADMAP 0y): a fn parameter forwarded to `filter` over an
// iterator, whose element the call retags to a borrow.
fn longs(ws: List<Str>, keep: (Str) -> Bool) [] -> Int => ws, keep {
    let it = iter(ws)
    // A parameter (and a forwarded implicit) peels the retagged borrow.
    //~ emits: &mut |__a0: &&String| keep(*__a0)
    return size(filter(it, keep))
}

// Received as a parameter and forwarded two levels down.
fn longs2(ws: List<Str>, keep: (Str) -> Bool) [] -> Int => ws, keep {
    // Into a slot of the value's own convention, still passed as it is.
    //~ emits: return crate::longs(ws, &mut *keep);
    return longs(ws, keep)
}

// A lambda stored in a local first.
fn local_lambda(ws: List<Str>) [] -> Int => ws {
    let k: (Str) -> Bool = s -> size(s) > 1
    let it = iter(ws)
    // A local closure, a field, a generic body's parameter.
    //~ emits: &mut |__a0: &&String| k(*__a0)
    return size(filter(it, k))
}

// A named fn stored in a local first.
fn local_named(ws: List<Str>) [] -> Int => ws {
    let k: (Str) -> Bool = long
    let it = iter(ws)
    return size(filter(it, k))
}

// A field read, into a retagged slot and into a plain generic one.
fn field(ws: List<Str>, p: Picker) [] -> Int => ws, p {
    let it = iter(ws)
    //~ emits: &mut |__a0: &&String| (p.keep)(*__a0)
    return size(filter(it, p.keep))
}
fn field_any(ws: List<Str>, p: Picker) [] -> Bool => ws, p {
    // A plain generic slot types the call-through closure as instantiated.
    //~ emits: crate::std_core_list::any::<String>(ws, &mut |__a0: &String| (p.keep)(__a0))
    return any(ws, p.keep)
}

// An implicit forwarded.
fn longs_i(ws: List<Str>, ?keep: (x: Str) -> Bool) [] -> Int => ws {
    let it = iter(ws)
    return size(filter(it, keep))
}

// `map` and `reduce` over an iterator.
fn lens(ws: List<Str>, f: (Str) -> Int) [] -> Int => ws, f {
    let it = iter(ws)
    let ns = map(it, f)
    return reduce(ns, 0, (a, n) -> a + n)
}
fn total(ws: List<Str>, f: (Int, Str) -> Int) [] -> Int => ws, f {
    let it = iter(ws)
    // A convention change and a retag in one adapter.
    //~ emits: &mut |__a0: &i32, __a1: &&String| f((*__a0).clone(), *__a1)
    return reduce(it, 0, f)
}

// A generic body forwarding its own fn parameter.
fn glongs<T>(xs: List<T>, keep: (T) -> Bool) [] -> Int => xs, keep {
    //~ emits: &mut |__a0: &&T| keep(*__a0)
    let it = iter(xs)
    return size(filter(it, keep))
}

// A fn value a call answers: passed directly, and through a local.
// A fn value a call answers: returned as an `Arc`, passed through it.
//~ emits: return std::sync::Arc::new(move |__a0: &String| crate::long(__a0));
fn picker() [] -> (Str) -> Bool {
    return long
}
fn from_call(ws: List<Str>) [] -> Int => ws {
    let it = iter(ws)
    //~ emits: &mut { let __f = crate::picker(); move |__a0: &&String| __f(*__a0) }
    return size(filter(it, picker()))
}
fn from_call_local(ws: List<Str>) [] -> Int => ws {
    let k = picker()
    let it = iter(ws)
    //~ emits: &mut |__a0: &&String| (k)(*__a0)
    return size(filter(it, k))
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    println("param lambda ${longs(ws, s -> size(s) > 2)}")
    println("param named ${longs(ws, long)}")
    println("two levels ${longs2(ws, s -> size(s) > 1)}")
    println("local lambda ${local_lambda(ws)}")
    println("local named ${local_named(ws)}")
    println("field ${field(ws, Picker { keep: long })}")
    println("field any ${field_any(ws, Picker { keep: s -> size(s) > 2 })}")
    println("implicit ${longs_i(ws, keep = long)}")
    println("map ${lens(ws, s -> size(s))}")
    println("reduce ${total(ws, (a, s) -> a + size(s))}")
    println("generic ${glongs(ws, long)}")
    println("call ${from_call(ws)}")
    println("call local ${from_call_local(ws)}")
    let k = picker()
    println("call any ${any(ws, k)} ${any(ws, picker())} ${k("x")}")
}

//--- kept_fns_over_views
// [fn-kept-borrow] [rs-proj-arm] A written fn value whose parameter is an
// owned `Str` it keeps fills a slot whose parameter is `proj Str` (user
// decision 2026-10-14, ROADMAP §7 option (b)). Rust adapts the convention:
// a named fn's forwarding closure peels the element's extra reference
// (`|__a0: &&String| { let __a0 = *__a0; crate::longer(__a0) }`), a
// forwarded parameter goes through `fn_adapter` (`keep(*__a0)`), a slot
// written `proj` passes the borrow as it is (`|__a0: &String|
// crate::longer(__a0)`), and an owned key at a slot the call instantiates as
// a borrow is lent twice (`&&String::from("def")`). The resolved implicits
// over the view type (`hash`, `eq`, `cmp`, `to_str` of `Str`) take the same
// rule. `kotlinc_compiles_and_runs_kept_fns_over_views` runs the same program.
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
    //~ emits: count::<&String>(vs, &mut |__a0: &&String| keep(*__a0))
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
    //~ emits: any::<&String>(&vs, &mut |__a0: &&String| { let __a0 = *__a0; crate::longer(__a0) })
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
    //~ emits: reduce__List_A_Fn::<&String, i32>(&vs, 0i32, &mut |__a0: &i32, __a1: &&String| { let __a1 = *__a1; crate::add_len(*__a0, __a1) })
    println("reduce ${reduce(vs, 0, add_len)}")
    // By a parameter, into a written `proj` slot, and through a local.
    println("param ${keep_count(vs, longer)} ${keep_count(vs, long)}")
    //~ emits: crate::count_views(&vs, &mut |__a0: &String| crate::longer(__a0))
    println("slot ${count_views(vs, longer)}")
    //~ emits: let mut k = |__a0: &String| crate::longer(__a0);
    let k: (proj Str) -> Bool = longer
    println("local ${k(get(vs, 1) ?: unreachable!())} ${count_views(vs, k)}")
    // Resolved implicits over the view type: `hash`, `eq`, `cmp`, `to_str`.
    let m = to_map(vs, v -> (v, size(v)))
    //~ emits: get_platform(&m, &&String::from("def")
    let n = get(m, "def") ?: unreachable!()
    println("map ${n} ${size(m)} ${contains_key(m, "ab")}")
    let st = to_set(vs)
    println("set ${size(st)} ${contains(st, "ab")} ${contains(st, "zz")}")
    println("list ${contains(vs, "ab")} ${eq(vs, vs)} ${cmp(vs, vs)}")
    println("to_str ${to_str(vs)} ${to_str(sort(vs))}")
}
