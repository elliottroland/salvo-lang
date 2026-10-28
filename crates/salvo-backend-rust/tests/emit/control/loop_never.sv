// A non-escaping loop as `Never`.

//--- an_infinite_loop_as_never
// [loop-never] A non-escaping `while true` is `Never`: a `-> T` fn whose
// tail is one needs no return, and it lowers to a bare `loop {}` (no exit
// test) that rustc types as `!`. The loop is guarded so the program ends.
fn pick(n: Int) -> Int {
    if n > 0 { return n }
    // A bare `loop {` is emitted.
    //~ emits: loop {
    while true {
    }
}

fn main() [use] {
    use StdConsole()
    println("${pick(7)}")
}
