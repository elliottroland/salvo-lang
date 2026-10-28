// Parse errors are reported where they are, and do not hide another file's
// diagnostics. Each program is its own: a parse error does not share one.

//--- parse_errors_are_reported
// Parse errors are reported with location; exit is nonzero.
// FILE: bad.sv
//~ error[syntax-expected:identifier]
fn broken( {

//--- parse_error_in_one_file_does_not_suppress_other_files
// A parse error in one file must not suppress checker diagnostics in
// other files: the broken file reports only its parse error, the clean
// file still gets type-checked.
// FILE: broken.sv
//~ error[syntax-expected:identifier]
fn broken( {
// FILE: typed.sv
fn wrong() -> Int {
    //~ error[var-let-infer:annotation-mismatch]
    let x: Int = "hello"
    return x
}
