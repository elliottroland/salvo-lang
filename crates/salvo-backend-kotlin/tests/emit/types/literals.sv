// Literals: the Kotlin spelling of a numeric literal.

//--- numeric_literal_suffixes_emit_kotlin_suffixes
// [type-lit-numeric] Literal suffixes map onto Kotlin's: `5L` stays `5L`,
// `2.5f` stays `2.5f`, unsuffixed floats are Double (`1.5`).
fn main() [use] -> None {
    use StdConsole
    //~ emits: = 5L
    let big: Long = 5L
    //~ emits: = 2.5f
    let ratio: Float = 2.5f
    //~ emits: = 1.5
    let d: Double = 1.5
    println("${big} ${ratio} ${d}")
}
