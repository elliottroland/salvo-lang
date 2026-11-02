// [op-arith] [type-lit-adopt] Unary `-` passes its expectation's numeric
// width to its operand and nothing else: in `let x: Long? = -1L` the
// negation is made optional, not the literal under it (ROADMAP 0al: the
// literal was wrapped too, and rustc refused `i64::wrapping_neg(Some(1))`).
//
// Compile-and-run data ([test-run-data]).

//--- the_negation_is_wrapped_once
fn main() [use] {
    use StdConsole()
    let x: Long? = -1L
    let z: Long? = -2
    let w: Double? = -1.5
    println("${x ?: 0L} ${z ?: 0L} ${w ?: 0.0}")
}
//= stdout
//= -1 -2 -1.5
