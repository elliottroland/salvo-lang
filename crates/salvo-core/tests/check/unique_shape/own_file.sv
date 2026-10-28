// [op-unique-shape] Every row of the class on `Pt` (and `PtNext`), declared
// in the types' own file: accepted.
// FILE: pt.sv
export struct Pt { x: Int }

export struct PtNext canbe Mut { at: Int }

export fn eq(a: Pt, b: Pt) [] -> Bool => a, b { return true }
export fn cmp(a: Pt, b: Pt) [] -> Int => a, b { return 0 }
export fn plus(a: Pt, b: Pt) [] -> Pt => a, b { return Pt { x: 0 } }
export fn minus(a: Pt, b: Pt) [] -> Pt => a, b { return Pt { x: 0 } }
export fn times(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
export fn divide(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
export fn pow(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
export fn to_str(p: Pt) [] -> Str => p { return "pt" }
export fn iter(p: Pt) [] -> Mut PtNext => p { return Mut PtNext { at: 0 } }
export fn next(p: Mut PtNext) [] -> Emitted Int | Finished => p: Mut { return finished() }

// [op-unique-shape] Several overloads of one shape in the type's own file stay legal:
// uniqueness is across files, and inside the file the author controls the
// set.
// FILE: boxes.sv
export struct Box<T> { value: T }
export fn eq<T>(a: Box<T>, b: Box<T>, ?eq: (x: T, y: T) -> Bool) [] -> Bool => a, b { return eq(a.value, b.value) }
export fn eq(a: Box<Int>, b: Box<Int>) [] -> Bool => a, b { return a.value == b.value }
export fn to_str<T>(b: Box<T>, ?to_str: (x: T) -> Str) [] -> Str => b { return to_str(b.value) }
export fn to_str(b: Box<Str>) [] -> Str => b { return b.value }
