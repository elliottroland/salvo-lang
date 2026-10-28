// FILE: bag.sv
// [struct-opaque] `opaque struct`: an ordinary struct in its declaring module
// and that module's `*.test.sv` annex; elsewhere the type is usable but its
// fields are not — no literal, field read or write, spread or destructuring —
// and a comptime fn stamped there sees kind `opaque`.

export opaque struct Bag {
    items: List<Int>,
    total: Int
}

export fn bag() [] -> Bag {
    return Bag { items: List<Int>(), total: 0 }
}

export fn total(b: Bag) [] -> Int => b {
    return b.total
}

// FILE: bag.test.sv
// The module's test annex counts as the module.
fn peek() [] -> Int {
    let b = bag()
    return b.total
}

// FILE: main.sv
// Outside the module, every way into the fields is refused, naming the type
// as opaque and pointing at its functions; the type itself and its functions
// are fine.
import bag.Bag
import bag.bag
import bag.total

fn fine() [] -> Int {
    let b: Bag = bag()
    return total(b)
}

fn read(b: Bag) [] -> Int => b {
    //~ error[struct-opaque:outside-module]
    return b.total
}

fn split(b: Bag) [] -> Int => b {
    //~ error[struct-opaque:outside-module]
    let {total} = b
    return total
}

fn build() [] -> Bag {
    //~ error[struct-opaque:outside-module]
    return Bag { items: List<Int>(), total: 1 }
}

// A comptime fn stamped outside the module sees kind `opaque`, so `by auto`
// there cannot walk the fields.
//~ error[fn-by:opaque-target]
fn same(a: Bag, b: Bag) [] -> Bool by auto
