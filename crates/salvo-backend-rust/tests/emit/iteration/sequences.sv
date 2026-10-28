// [rs-seq] [seq-iterator] [implicit-group] The sequence functions on the Rust backend.

//--- sequence_functions_lower_to_helpers
// The same source and stdout as the Kotlin backend's
// `kotlinc_compiles_and_runs_sequences`: `map`/`filter`/`reduce` over a `List`
// (the intrinsic fast path), and over the *passes* an array, a `Str`, an origin
// and a struct of the program's own hand out — `iter` written at each use site
// [seq-iterator].
//
// [rs-seq] The `List` fast paths are std functions (`filter` a platform fn),
// whose parameter types are what give a callback its expected type.
// [implicit-intrinsic] The generic overload's implicit `iter` arrives as an
// adapter closure whose body is the *intrinsic's own lowering*: emitting
// `iter(__i0)` would name the generated `iter` **module** (E0423).
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
    //~ emits: map__List_Fn::<i32, i32>(&xs, &mut |mut n| -> i32 {
    //~|         i32::wrapping_mul(*n, 2i32)
    let doubled = map(xs, n -> n * 2)
    //~ emits: reduce__List_A_Fn::<i32, i32>(&xs, 0i32, &mut |mut a, mut b| -> i32 {
    //~|         i32::wrapping_add(*a, *b)
    let sum = reduce(xs, 0, (a, b) -> a + b)
    //~ emits: filter_platform::<i32>(&xs, &mut |mut n| -> bool {
    //~|         (*n > 2i32)
    let big = filter(xs, n -> n > 2)
    println("list: ${size(doubled)} ${sum} ${size(big)}")
    // A named fn as the callback wraps in an adapter: a fn item's own
    // convention is by value, and `map` hands it `&T` [fn-contract].
    //~ emits: (&xs, &mut |__a0: &i32| crate::double(*__a0))
    let named = map(xs, double)
    println("named: ${size(named)}")
    let arr = array_of(10, 20, 30)
    let arr_sum = reduce(iter(copy(arr)), 0, (a, b) -> a + b)
    let arr_mapped = map(iter(arr), n -> n + 1)
    println("array: ${arr_sum} ${size(arr_mapped)}")
    let hello = "hello"
    let letters = filter(iter(hello), c -> c == 'l')
    println("chars: ${size(letters)}")
    // [iter-fn] A pass subject reaches the generic overload through its `iter`,
    // and the `next` the checker resolved arrives as the implicit argument.
    //~ emits: NaturalsIter2
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
