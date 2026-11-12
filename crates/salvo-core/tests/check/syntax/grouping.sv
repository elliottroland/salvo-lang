// [expr-group] [lit-spelling] Parentheses and literal spellings are spelling
// only: a program written with redundant parentheses and separated or
// escaped literals checks as the same program without them.

struct Point canbe Mut {
    x: Int
}

fn first(p: Point) [] -> Int => p {
    return p.x
}

fn narrows(v: Int | Str) [] -> Int {
    if (v is Int) {
        type!(v: Int)
        return (v) + 1
    }
    if ((v is Str)) && (true) {
        type!(v: Str)
    }
    return 0
}

fn guards(v: Int | Str) [] -> Int {
    if !((v is Int)) {
        return 0
    }
    type!(v: Int)
    return ((v)) + 0
}

fn places(p: Mut Point) [] -> Int {
    let n = 1
    (n) = (2)
    (p).x = n
    (p.x) += 1
    return (first)(p) + (p).x + ((n))
}

fn literals() [] -> Int {
    let a = 1_000
    type!(a: Int)
    let _b = 2_000L
    type!(_b: Long)
    let _c = 0.500_5
    type!(_c: Double)
    let s = "a\$b \'c\'"
    let _ch = '\"'
    type!(_ch: Char)
    return a + (s.size()) + (-(a))
}
