// [diag-structured] Errors from different files of one program index the
// right file; resolution errors flow into the checker's errors structured.
// (The rendering — `file:line:col` and the caret — is `diag_tests.rs`.)

// FILE: a.sv
export fn fine() -> Int {
    return 1
}

// FILE: b.sv
//~ error[mod-import:unresolved]
import nope.thing
