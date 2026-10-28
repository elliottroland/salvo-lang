// [rs-repoint] a re-pointed view: one Rust variable per live range
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- re_pointed_views

// A local view re-pointed at a parameter and returned (ROADMAP 0v).
fn pick(ws: List<Str>, keep: (Str) -> Bool) [] -> Mut List<proj Str> holds proj(ws) => ws, keep {
    let local = List("x", "y")
    let out = filter(local, keep)
    count(out)
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
    let big = filter(ns, n -> n > 2)
    println("ints kept ${size(big)}")
    big = filter(List(10, 1), n -> n > 2)
    println("ints then ${size(big)}")
}

// The value an arm or a loop stores flows out of it: one variable.
fn flows(ws: List<Str>, c: Bool) [Console] -> None => ws {
    let out = filter(ws, s -> size(s) > 5)
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
//= stdout
//= pick 2
//= yielded 2
//= either 2 0
//= early 2
//= early kept 1 of the local
//= late 0
//= total kept 1
//= turn kept 2
//= turn kept 1
//= turn kept 2
//= after the loop: 2
//= mine kept 1
//= ws kept 2, mine has 3
//= a starts 0
//= a then 2
//= b starts 3
//= b then 0
//= ints kept 2
//= ints then 1
//= flows if 3
//= flows break 2
//= flows continue 1
//= flows while 3
//= flows if 0
//= flows break 2
//= flows continue 1
//= flows while 3
