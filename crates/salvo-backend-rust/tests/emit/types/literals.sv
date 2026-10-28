// Literals render at their Rust types.

//--- numeric_literal_suffixes_emit_rust_types
// [type-lit-numeric] [type-basic] Literal suffixes emit explicit Rust types:
// `5L` -> `5i64`, `2.5f` -> `2.5f32`; unsuffixed literals stay bare.
fn main() [use] -> None {
    use StdConsole
    //~ emits: = 5i64
    let big: Long = 5L
    //~ emits: = 2.5f32
    let ratio: Float = 2.5f
    //~ emits: = 1.5
    let d: Double = 1.5
    println("${big} ${ratio} ${d}")
}
