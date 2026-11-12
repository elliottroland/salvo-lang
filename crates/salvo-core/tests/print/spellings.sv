// [ast-print] [expr-group] [lit-spelling] Parentheses and literals as
// written: a formatter keeps the parentheses the author wrote, adds only the
// ones precedence needs, and writes each literal in its own spelling. Parsed
// only (`print_tests.rs`), never checked.

fn groups(a: Bool, b: Bool, c: Bool, x: Int, p: Point, xs: List<Int>) -> Int {
    let y = (x)
    let z = ((x))
    let o = (a && b) || c
    let q = a && (b || c)
    let r = (((a && b)) || (c))
    let s = (x + 1) * (2 - x) + (x * 2)
    let n = -(x) + -(-x) + - -x + !(a) + !!b
    if (a) {
        return (y + 1) * 2
    } elif ((b)) {
        return 0
    }
    while (x > 0) {
        x -= (1)
    }
    for e in (xs) {
        print((e))
    }
    when (x) {
        is Int {}
    }
    if p == (Point { x: 1 }) {
        print((Point { x: 2 }))
    }
    if (p == Point { x: 1 }) {}
    let f = (v) -> (v + 1)
    let g = (v) -> (Point { x: v })
    let h = (v: (Int | Str)?) -> v
    let t = ((1, 2))
    let u = (t).0 + (xs)[0] + (p).x + (p.x) + (f)(1)
    let m = (p.f)(1) + ((p.f))(1)
    let i = f((x: -Ok), (x))
    let j = "${(x + 1)} and ${x}"
    return (y)
}

fn types(a: (Int | Str), b: ((Int)), c: (Idx(xs) Int)?, d: ((Int) -> Int)?, e: ((A | B))[]) -> (Int) {
    let f: List<(Int)> = List()
    let g: Ok (A | B) = a
    let h: Ok ((A | B)) = a
    let k: (Mut List<Int>) = List()
    return 0
}

fn literals() -> None {
    let a = 1_000
    let b = 1_000_000L
    let c = 0.500_5
    let d = 1.50
    let e = 2.0f
    let f = 007
    let g = "a\$b, \'q\', tab\tend"
    let h = "\$${a} costs ${b}"
    let i = '\"'
    let j = '"'
    let k = '\''
    let l = '\\'
    let m = '\0'
    let n = "plain"
    let o = 12
}

comptime fn grouped<T is Struct>(t: T) -> Str {
    [if (T.fields is None) || ((1 + 2) * 3 > 4)] {
        return "a"
    }
    return "b"
}
