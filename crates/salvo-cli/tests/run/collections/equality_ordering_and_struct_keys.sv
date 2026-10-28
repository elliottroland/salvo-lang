// equality, ordering and struct keys [col-equality]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- equality_and_ordering

struct Point : Ordered<self> by auto, Hashed<self> by auto {
    x: Int,
    y: Int
}

struct Version : Ordered<self> by auto, Hashed<self> by auto {
    parts: List<Int>,
    label: (Str, Int)
}

// Equality only — and a float field, which is fine for `eq` and not for a key.
struct Measure : Eq<self> by auto {
    value: Double
}

fn main() [use] -> None {
    use StdConsole()
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 1, y: 3 }
    let r = Point { x: 1, y: 2 }
    let eq = p == r
    let ne = p != q
    let lt = p < q
    let ge = q >= p
    println("eq: ${eq} ne: ${ne} lt: ${lt} ge: ${ge}")

    let seen: Mut Set<Point> = Mut Set<Point>()
    add(seen, Point { x: 1, y: 2 })
    let dup = add(seen, Point { x: 1, y: 2 })
    println("struct key: size ${size(seen)} dup ${dup}")

    let by: Mut Map<Point, Str> = Mut Map<Point, Str>()
    put(by, Point { x: 9, y: 9 }, "nine")
    let hit = get(by, Point { x: 9, y: 9 })
    if hit is Str {
        println("struct lookup: ${hit}")
    }

    let a = Version { parts: List(1, 2, 0), label: ("a", 1) }
    let b = Version { parts: List(1, 3), label: ("a", 1) }
    let c = Version { parts: List(1, 2), label: ("a", 1) }
    let ab = a < b
    let ca = c < a
    println("list order: ${ab} prefix: ${ca}")

    let zero = 0.0
    let m1 = Measure { value: zero / zero }
    let m2 = Measure { value: zero / zero }
    let nan_eq = m1 == m2
    println("struct nan equality: ${nan_eq}")
}
//= stdout
//= eq: true ne: true lt: true ge: true
//= struct key: size 1 dup false
//= struct lookup: nine
//= list order: true prefix: true
//= struct nan equality: false
