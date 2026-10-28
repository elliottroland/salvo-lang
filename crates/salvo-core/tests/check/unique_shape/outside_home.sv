// [op-unique-shape] A function of a unique shape on a type is declared in the
// type's own file (user decision O1-D, 2026-10-27; OPERATOR_OVERLOAD.md
// "D's details"): the class is `eq` (2 explicit parameters, `-> Bool`), `cmp`
// (2, `-> Int`), `plus`, `minus`, `times`, `divide`, `pow` (2), `to_str` (1),
// `iter` (1) and `next` (1, `-> Emitted T | Finished`); "on `T`" is the first
// explicit parameter's base type, qualifiers stripped; a second one anywhere
// else is an error at its declaration, used or not.
//
// Every row of the class on a struct (`pt.sv`: its own file and its `next`'s
// pass struct), declared in another file: each is refused, naming `pt.sv`
// and the move as the remedy. Its own file accepting the same rows is
// `own_file.sv`.
// FILE: pt.sv
export struct Pt { x: Int }

export struct PtNext canbe Mut { at: Int }

// FILE: other.sv
import pt

//~ error[op-unique-shape:outside-home]
export fn eq(a: Pt, b: Pt) [] -> Bool => a, b { return true }
//~ error[op-unique-shape:outside-home]
export fn cmp(a: Pt, b: Pt) [] -> Int => a, b { return 0 }
//~ error[op-unique-shape:outside-home]
export fn plus(a: Pt, b: Pt) [] -> Pt => a, b { return Pt { x: 0 } }
//~ error[op-unique-shape:outside-home]
export fn minus(a: Pt, b: Pt) [] -> Pt => a, b { return Pt { x: 0 } }
//~ error[op-unique-shape:outside-home]
export fn times(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
//~ error[op-unique-shape:outside-home]
export fn divide(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
//~ error[op-unique-shape:outside-home]
export fn pow(a: Pt, n: Int) [] -> Pt => a, n { return Pt { x: 0 } }
//~ error[op-unique-shape:outside-home]
export fn to_str(p: Pt) [] -> Str => p { return "pt" }
//~ error[op-unique-shape:outside-home]
export fn iter(p: Pt) [] -> Mut PtNext => p { return Mut PtNext { at: 0 } }
//~ error[op-unique-shape:outside-home]
export fn next(p: Mut PtNext) [] -> Emitted Int | Finished => p: Mut { return finished() }
