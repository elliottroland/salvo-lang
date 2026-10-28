// [op-unique-shape] D-1: a `.test.sv` annex is a different file — it may
// declare shapes only for the types it declares itself.
// FILE: pt.sv
export struct Pt { x: Int }

// FILE: pt.test.sv
//~ error[op-unique-shape:outside-home]
fn to_str(p: Pt) [] -> Str => p { return "pt" }
struct Probe { n: Int }
fn to_str(p: Probe) [] -> Str => p { return "probe" }
