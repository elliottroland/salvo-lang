// The sequence functions [implicit-group] [seq-iterator].

//--- sequence_functions_lower_to_collection_operations
// [kt-seq] [seq-iterator] A `fns` group emits nothing, and the `List` fast
// paths are std functions (`filter` a platform fn over Kotlin's own); the generic body is a
// plain generic function whose implicit parameter arrives as a trailing
// argument — an *adapter lambda* when the resolved `iter` is an intrinsic,
// since an intrinsic has no Kotlin name to reference [implicit-intrinsic].
// `map`/`filter`/`reduce` over a `List` (the intrinsic fast path), and over
// the *passes* an array, a `Str`, an origin and a struct of the program's own
// hand out — `iter` written at each use site [seq-iterator]. Shared with the
// Rust backend's `rustc_compiles_and_runs_sequences` (`SEQ_DEMO` in
// codegen_tests.rs, which keeps the every-file check that a `fns` group emits
// nothing).
struct Bag {
    items: List<Int>
}

fn iter(bag: Bag) -> Mut ListNext<Int> => bag {
    return iter(bag.items)
}

fn double(n: Int) -> Int {
    return n * 2
}

struct Naturals {
    from: Int
}

fn naturals(from: Int) -> Naturals {
    return Naturals {from: from}
}

iter fn iter(n: Naturals) -> Emitted Int | Finished {
    state NaturalsIter2 {
        at = n.from
    }
    if at >= n.from + 4 {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn main() [use] {
    use StdConsole()
    let xs = List(1, 2, 3, 4)
    // Lambdas are anonymous fns [kt-ir].
    //~ emits: fun(n: Int): Int {
    //~|         return (n * 2)
    let doubled = map(xs, n -> n * 2)
    //~ emits: return (a + b)
    let sum = reduce(xs, 0, (a, b) -> a + b)
    //~ emits: filterPlatform(xs, fun(n: Int): Boolean {
    let big = filter(xs, n -> n > 2)
    println("list: ${size(doubled)} ${sum} ${size(big)}")
    let named = map(xs, double)
    println("named: ${size(named)}")
    let arr = array_of(10, 20, 30)
    let arr_sum = reduce(iter(copy(arr)), 0, (a, b) -> a + b)
    // The generic overload for a pass subject gets the resolved `next` as
    // its implicit argument.
    //~ emits: salvo.std.core.seq.map__It_Fn(salvo.std.core.array.iter(arr), fun(n: Int): Int {
    //~ emits: { __a0 -> salvo.std.core.array.next(__a0) })
    let arr_mapped = map(iter(arr), n -> n + 1)
    println("array: ${arr_sum} ${size(arr_mapped)}")
    let hello = "hello"
    let letters = filter(iter(hello), c -> c == 'l')
    println("chars: ${size(letters)}")
    let lazy_sum = reduce(iter(naturals(1)), 0, (a, b) -> a + b)
    let tripled = map(xs, n -> n * 3)
    let chained = filter(iter(tripled), n -> n > 6)
    println("iter: ${lazy_sum} ${size(chained)}")
    let names = List("ann", "bob", "carol")
    let lens = map(names, n -> size(n))
    let long = filter(names, n -> size(n) > 3)
    println("names: ${size(lens)} ${size(long)}")
    let bag = Bag {items: List(5, 6)}
    println("bag: ${reduce(iter(bag), 0, (a, b) -> a + b)}")
    // [iter-mint] `for` over a *container of one's own*: the loop calls its
    // `iter` once and drives the pass that answers. Until 2026-09-09 the
    // checker recorded that mint and neither emitter made the call, so this
    // shape emitted code the target compiler rejected.
    // (a second bag, because this demo's `iter` *moves* its container — the
    // one above was consumed by the `reduce`.)
    let more = Bag {items: List(5, 6)}
    for n in more {
        println("bag element ${n}")
    }
}
