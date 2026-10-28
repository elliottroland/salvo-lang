// `is` on union-typed struct-field subjects
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- field_is

struct Holder {
    result: Ok Int | Err Str
}

fn main() [use] -> None {
    use StdConsole
    let h = Holder {result: ok(1)}
    if h.result is Ok Int r {
        println("ok ${r}")
    }
    if h.result is Ok {
        println("plain ${h.result}")
    }
    let h2 = Holder {result: err("bad")}
    if h2.result is Err Str e {
        println("err ${e}")
    }
}
//= stdout
//= ok 1
//= plain 1
//= err bad
