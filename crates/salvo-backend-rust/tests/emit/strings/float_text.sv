// A float's text goes through Salvo's own rule [interp-float].

//--- a_float_renders_by_salvos_rule
// [interp-float] A float's text is **Salvo's rule, which is Kotlin's** (user
// decision 2026-09-25): the shortest digits that round-trip, always a decimal
// point, and scientific notation outside `[10^-3, 10^7)`. Rust's `Display`
// writes the number in full and drops the `.0`, so the same value printed
// differently on the two backends (`2` against `2.0`,
// `100000000000000000000` against `1.0E20`) — a [backend-parity] break in
// program *output*, which is why the expected text below is the assertion.
//
// The table walks both ends of the plain window (`1234567.0` plain,
// `1.2345678E7` scientific), the signs, zero, `NaN` and both infinities, a
// struct rendered field-wise [interp-struct], a list, an explicit `to_str`, a
// one-element list, and a map *value* — every path where a float becomes text
// (a nested list waits on recursive implicit resolution, ROADMAP 6).
struct Point : ToStr<self> by auto {
    x: Double,
    ratio: Float
}

fn main() [use] -> None {
    use StdConsole()
    // A float goes through the helper, not `Display`.
    //~ emits: crate::strings::salvo_f64_text
    println("${2.0} ${8.25} ${0.0 - 2.0} ${0.0} ${0.001} ${0.0001}")
    println("${1234567.0} ${12345678.0} ${100000000000000000000.0} ${0.00000000000001}")
    let zero = 0.0
    let one = 1.0
    println("${zero / zero} ${one / zero} ${0.0 - one / zero}")
    // …and a `Float` through its own.
    //~ emits: crate::strings::salvo_f32_text
    println("${Point { x: 2.0, ratio: 3.0 }}")
    let xs: List<Double> = List(2.0, 0.5)
    println("${xs} ${to_str(xs)} ${List(4.0)}")
    let m: Map<Str, Double> = Map("a": 2.0)
    println("${m}")
}
