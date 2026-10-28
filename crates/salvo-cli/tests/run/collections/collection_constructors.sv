// collection constructors [fn-type-named]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- collection_literals

struct Point {
    x: Int,
    y: Int
}

fn takes_set(s: Set<Int>) -> Int => s {
    return size(s)
}

fn main() [use] -> None {
    use StdConsole()
    let xs = List(1, 2, 3)
    println("list: ${to_str(xs)}")
    let mxs: Mut List<Int> = Mut List(4, 5)
    add(mxs, 6)
    println("mut list: ${to_str(mxs)}")
    let s: Set<Str> = Set("b", "a", "b")
    println("set: ${to_str(s)}")
    let m: Map<Str, Int> = Map("one": 1, "two": 2)
    println("map: ${to_str(m)}")
    let ms: Mut Set<Int> = Mut Set<Int>()
    add(ms, 9)
    println("empty then filled: ${to_str(ms)}")
    let mm: Map<Str, Int> = Map<Str, Int>()
    println("empty map: ${to_str(mm)} via param: ${takes_set(Set())}")
    let p: Point = {x: 1, y: 2}
    println("struct literal still: ${p.x},${p.y}")
    let arr = array_of(7, 8)
    println("array: ${size(arr)} ${arr[0]}")
}
//= stdout
//= list: [1, 2, 3]
//= mut list: [4, 5, 6]
//= set: {b, a}
//= map: {one: 1, two: 2}
//= empty then filled: {9}
//= empty map: {} via param: 0
//= struct literal still: 1,2
//= array: 2 7
