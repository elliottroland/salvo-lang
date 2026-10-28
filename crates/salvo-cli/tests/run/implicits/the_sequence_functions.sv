// [rs-seq] [seq-iterator] [implicit-group] the sequence functions
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- sequences

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
    let doubled = map(xs, n -> n * 2)
    let sum = reduce(xs, 0, (a, b) -> a + b)
    let big = filter(xs, n -> n > 2)
    println("list: ${size(doubled)} ${sum} ${size(big)}")
    let named = map(xs, double)
    println("named: ${size(named)}")
    let arr = array_of(10, 20, 30)
    let arr_sum = reduce(iter(copy(arr)), 0, (a, b) -> a + b)
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
//= stdout
//= list: 4 10 2
//= named: 4
//= array: 60 3
//= chars: 2
//= iter: 10 2
//= names: 3 1
//= bag: 11
//= bag element 5
//= bag element 6
