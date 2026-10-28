// Views (`proj`) on the Rust backend: re-pointed views, borrowed elements
// read through a generic `proj T`, and a capture-rooted projection.

//--- re_pointed_views
// [rs-repoint] [rs-proj-generic] [fate-link] ROADMAP 0v: a local holding a
// view, re-pointed at another source, compiles and runs as on Kotlin
// (`kotlinc_compiles_and_runs_re_pointed_views` runs the same program). The
// repro (`pick`) and its neighbours: over an iterator, in both arms of an
// `if`, in an arm that returns, on every turn of a loop, not returned while
// the first source is written, the empty-then-borrowed and
// borrowed-then-empty orders, Copy elements, and the shapes whose stored
// value flows out of its block (one variable). Two causes, both Rust's: the
// `List` fast path of `filter` answered owned clones where its type is a
// view (`Vec<String>` against `Vec<&String>`), and one variable has one
// lifetime for every borrow ever stored in it (E0515).
// A local view re-pointed at a parameter and returned (ROADMAP 0v).
// [rs-proj-generic] The fast path's view borrows: no clones, and Copy
// elements are copied out at the call.
//~ emits(std/core/seq.rs): -> Vec<&'a T> {
//~ emits(platform/std/core/seq.rs): -> Vec<&'a T> {
fn pick(ws: List<Str>, keep: (Str) -> Bool) [] -> Mut List<proj Str> holds proj(ws) => ws, keep {
    let local = List("x", "y")
    let out = filter(local, keep)
    count(out)
    // [rs-repoint] A plain re-point in the declaring block shadows; arms
    // that all re-point share a fresh variable declared before them; an arm
    // that returns its re-point shadows inside it; a value that flows out of
    // its arm or loop stays an assignment.
    //~ emits:     let mut out: Vec<&String> = crate::std_core_seq::filter_platform::<String>(ws, &mut *keep);
    //~|     return out;
    out = filter(ws, keep)
    return out
}

fn count(xs: List<proj Str>) [] -> Int {
    return size(xs)
}

// The same over an iterator: the generic `filter` holds the iterator's source.
fn yielded(ws: List<Str>) [] -> Mut List<proj Str> holds proj(ws) => ws {
    let local = List("x", "yy")
    let li = iter(local)
    let out = filter(li, s -> size(s) > 1)
    count(out)
    let wi = iter(ws)
    out = filter(wi, s -> size(s) > 1)
    return out
}

// Re-pointed in both arms of an `if`, then returned.
fn either(a: List<Str>, b: List<Str>, first: Bool) [] -> Mut List<proj Str> holds proj(a, b) => a, b {
    let local = List("long", "longer")
    let out = filter(local, s -> size(s) > 2)
    count(out)
    //~ emits:     let mut out: Vec<&String>;
    //~|     if first {
    //~|         out = 
    if first {
        out = filter(a, s -> size(s) > 1)
    } else {
        out = filter(b, s -> size(s) > 1)
    }
    return out
}

// Re-pointed in one arm that returns; the old view is used after.
fn early(ws: List<Str>, quit: Bool) [Console] -> Mut List<proj Str> holds proj(ws) => ws {
    let local = List("aa", "b")
    let out = filter(local, s -> size(s) > 1)
    //~ emits:     if quit {
    //~|         let mut out: Vec<&String> = 
    if quit {
        out = filter(ws, s -> size(s) > 1)
        return out
    }
    println("early kept ${size(out)} of the local")
    return filter(ws, s -> size(s) > 5)
}

// Re-pointed on every turn of a loop at a local of the loop body, and in a
// second loop at the parameters, used after it.
fn turns(ws: List<Str>, more: List<Str>) [Console] -> None => ws, more {
    let total = List("t")
    let out = filter(total, s -> size(s) > 0)
    println("total kept ${size(out)}")
    for w in ws {
        let row = List(copy(w), "zz", "y")
        out = filter(row, s -> size(s) > 1)
        println("turn kept ${size(out)}")
    }
    let seen = filter(total, s -> size(s) > 0)
    for w in more {
        if size(w) > 1 {
            seen = filter(ws, s -> size(s) > 1)
        } else {
            seen = filter(more, s -> size(s) > 1)
        }
    }
    println("after the loop: ${size(seen)}")
}

// A re-pointed view that is not returned, its first source written after
// the re-point (Rust: the first borrow must have ended).
fn rewrite(ws: List<Str>) [Console] -> None => ws {
    let mine = Mut List("p", "qq")
    let out = filter(mine, s -> size(s) > 1)
    println("mine kept ${size(out)}")
    out = filter(ws, s -> size(s) > 1)
    add(mine, "rrr")
    println("ws kept ${size(out)}, mine has ${size(mine)}")
}

// Owned (an empty list of views) then borrowed, and borrowed then owned.
fn orders(ws: List<Str>) [Console] -> None => ws {
    let a: Mut List<proj Str> = Mut List()
    println("a starts ${size(a)}")
    a = filter(ws, s -> size(s) > 1)
    println("a then ${size(a)}")
    let b = filter(ws, s -> size(s) > 0)
    println("b starts ${size(b)}")
    b = Mut List()
    println("b then ${size(b)}")
}

// Ints: the view of Copy scalars is a list of its own.
fn ints() [Console] -> None {
    let ns = List(1, 5, 9)
    //~ emits: ).into_iter().copied().collect::<Vec<_>>();
    let big = filter(ns, n -> n > 2)
    println("ints kept ${size(big)}")
    big = filter(List(10, 1), n -> n > 2)
    println("ints then ${size(big)}")
}

// The value an arm or a loop stores flows out of it: one variable.
fn flows(ws: List<Str>, c: Bool) [Console] -> None => ws {
    let out = filter(ws, s -> size(s) > 5)
    //~ emits:     if c {
    //~|         out = 
    if c {
        out = filter(ws, s -> size(s) > 0)
    }
    println("flows if ${size(out)}")
    for w in ws {
        out = filter(ws, s -> size(s) > 1)
        if size(w) > 0 {
            break
        }
    }
    println("flows break ${size(out)}")
    let n = 0
    while n < 2 {
        n = n + 1
        if n == 1 {
            out = filter(ws, s -> size(s) > 2)
            continue
        }
        println("flows continue ${size(out)}")
        out = filter(ws, s -> size(s) > 0)
    }
    println("flows while ${size(out)}")
}

fn main() [use] -> None {
    use StdConsole
    let ws = List("ab", "c", "def")
    println("pick ${size(pick(ws, s -> size(s) > 1))}")
    println("yielded ${size(yielded(ws))}")
    let a = List("aa", "bb", "c")
    let b = List("z")
    println("either ${size(either(a, b, true))} ${size(either(a, b, false))}")
    println("early ${size(early(ws, true))}")
    println("late ${size(early(ws, false))}")
    turns(ws, List("q", "rs"))
    rewrite(ws)
    orders(ws)
    ints()
    flows(ws, true)
    flows(ws, false)
}

//--- borrowed_element_reads
// [rs-proj-flat] [backend-never-wrong] ROADMAP 0z: a std fn whose result is
// a generic `proj T` (rendered `&T`), called on a list of borrows
// (`T = &String`), answers `&&String` where Salvo's type is `proj Str`
// (`proj proj Str` is `proj Str`); the outer reference is copied out — on
// `get`/`last` (`.copied()`), `first` (`*`), and a pass's `next`, in a
// `for` loop and in every `?Next` adapter (`U1(*__v)`). The call's
// instantiation is read off its arguments where the IR states no type
// arguments (a `for` loop's `next`, a forwarding closure's call).
// `kotlinc_compiles_and_runs_borrowed_element_reads` runs the same program.
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
    // A `for` loop's `next`, whose instantiation the arguments state.
    //~ emits: = (match crate::std_core_list::next__ListNext(&mut __pass_6) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) });
    for v in vs {
        println("list for ${v}")
    }
    // `get` and `last` copy the outer reference out of the optional.
    //~ emits: let mut __pick_3: Option<&String> = crate::std_core_list::get_platform(&vs, 0i32).copied();
    let h = get(vs, 0) ?: unreachable!()
    println("get ${h}")
    for v in iter(vs) {
        println("iter for ${v}")
    }
    // Its neighbours.
    //~ emits: crate::std_core_list::last::<&String>(&vs).copied()
    let l = last(vs) ?: unreachable!()
    println("last ${l}")
    if vs is NonEmpty {
        //~ emits: (*crate::std_core_list::first::<&String>(&vs))
        println("first ${first(vs)}")
    }
    // A `?Next` adapter.
    //~ emits: &mut |__a0: &mut crate::std_core_list::ListNext<'_, &String>| (match crate::std_core_list::next__ListNext(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v),
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
    //~ emits: let mut __pick_25: Option<&Vec<String>> = crate::std_core_list::get_platform(&big, 0i32).copied();
    let fst = get(big, 0) ?: unreachable!()
    println("nested get ${size(fst)}")
    for b in iter(big) {
        println("nested for ${size(b)}")
    }
}

//--- a_capture_rooted_projection
// A callback picking elements out of a *captured* list — the projection
// is rooted in the capture, which no fn type can name, so the closure
// itself carries the link [fate-lambda-view]. The emitted Rust must hold the
// borrow (`Vec<&String>`, no clone in the lambda) and deref the retagged
// index in the cast (`(*i) as usize` — E0606 without it; both were open
// defects, closed 2026-09-12).
fn main() [use] {
    use StdConsole()
    let all = List("a", "b", "c", "d")
    let indices = List(1, 3)
    // The lambda returns the borrow itself: no clone, and the Copy index
    // is deref'd for the cast — which goes through `i64`, so a negative
    // literal index stays an *answer* rather than rustc's E0600 [col-bounds].
    //~ emits: &mut |mut i| -> &String {
    //~ emits: let mut __pick_3: Option<&String> = crate::std_core_list::get_platform::<String>(&all, *i);
    //~ !emits: __picked_4.clone()
    let picked = map(indices, i -> (get(all, i) ?: unreachable!()))
    for w in picked {
        println(w)
    }
    let p = iter(indices)
    let doubled = map(p, i -> i + i)
    for n in doubled {
        println("${n}")
    }
}
