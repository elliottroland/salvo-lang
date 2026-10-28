// [rs-proj-arm] a forwarded fn value into a retagged slot
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- forwarded_fn_values

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
    return size(filter(it, keep))
}

// Received as a parameter and forwarded two levels down.
fn longs2(ws: List<Str>, keep: (Str) -> Bool) [] -> Int => ws, keep {
    return longs(ws, keep)
}

// A lambda stored in a local first.
fn local_lambda(ws: List<Str>) [] -> Int => ws {
    let k: (Str) -> Bool = s -> size(s) > 1
    let it = iter(ws)
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
    return size(filter(it, p.keep))
}
fn field_any(ws: List<Str>, p: Picker) [] -> Bool => ws, p {
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
    return reduce(it, 0, f)
}

// A generic body forwarding its own fn parameter.
fn glongs<T>(xs: List<T>, keep: (T) -> Bool) [] -> Int => xs, keep {
    let it = iter(xs)
    return size(filter(it, keep))
}

// A fn value a call answers: passed directly, and through a local.
fn picker() [] -> (Str) -> Bool {
    return long
}
fn from_call(ws: List<Str>) [] -> Int => ws {
    let it = iter(ws)
    return size(filter(it, picker()))
}
fn from_call_local(ws: List<Str>) [] -> Int => ws {
    let k = picker()
    let it = iter(ws)
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
//= stdout
//= param lambda 1
//= param named 2
//= two levels 2
//= local lambda 2
//= local named 2
//= field 2
//= field any true
//= implicit 2
//= map 6
//= reduce 6
//= generic 2
//= call 2
//= call local 2
//= call any true true false
