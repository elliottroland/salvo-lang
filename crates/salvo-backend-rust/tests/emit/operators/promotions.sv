// Operator typing: widening, literal adoption, conversions.

//--- promotions_cast_and_literals_adopt
// [op-arith] [op-promote] [type-lit-adopt] [op-convert] The same source and
// expected stdout as the Kotlin backend's case [backend-parity]: `Long`
// arithmetic with widened `Int` operands, adopted literals, mixed-width
// ordering, and the explicit conversions. Values are chosen fractional
// where floats print, since whole-double rendering differs between the
// backends today (open defect).
// [op-promote] [type-lit-adopt] Rust has no mixed-width operators, so widened
// operands cast (`as i64`), and adopted literals render at their checked
// type (`1i64`).
fn main() [use] -> None {
    use StdConsole
    // The adopted literal is suffixed.
    //~ emits: x: i64 = 1i64;
    let x: Long = 1
    let n = 5
    // The widened operand is cast as a whole.
    //~ emits: i64::wrapping_add(x, ((i32::wrapping_mul(n, 2i32)) as i64))
    let y = x + n * 2
    let big = 4000000000L
    let scaled = big * 2 + n
    let d: Double = 3
    let g = d + to_double(n) + 0.25
    let f: Float = 0.5
    let h = f * 1.5f
    // The ordering operand is cast.
    //~ emits: (((n) as i64) < x)
    let cmp = n < x
    // [op-convert] The operand is parenthesized inside the cast: `as` binds
    // tighter than unary minus, so `-1 as u8` would be `-(1 as u8)` — which
    // rustc refuses for `to_byte(-1)` and would have meant the wrong thing
    // for a saturating target [type-byte].
    //~ emits: ((big) as i32)
    let back = to_int(big)
    let neg: Long = -7
    println("${y} ${scaled} ${g} ${h} ${cmp} ${back} ${neg}")
}
