// What the lexer refuses: one program per error, since a parse error stops
// a program.

//--- tuple_index_rejects_numeric_suffixes
// [expr-tuple-index] Numeric suffixes are not tuple indices.
fn f(t: (Int, Int)) -> None {
    //~ error[expr-tuple-index:long-suffix]
    let _x = t.0L
}

//--- unterminated_string
fn f() {
    //~ error[type-str:unterminated-string]
    let _s = "oops
}

//--- unterminated_nested_string
// [interp-nested-str] Unterminated is still unterminated: the fragment's
// literal has to close, and the diagnostic is the interpolation's.
fn f() -> Str {
    //~ error[type-str:unterminated-string]
    //~ error[type-str:unterminated-interpolation]
    //~ error[syntax-expected:token]
    return "q ${size("a)} z"
}
