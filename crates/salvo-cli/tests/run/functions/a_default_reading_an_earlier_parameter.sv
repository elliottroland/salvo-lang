// [fn-param-default] [deduce-same-call] A default that reads an earlier
// parameter binds that parameter once, ahead of the call — to a fresh local,
// so a `name = value` argument mentioning a program variable of the same name
// reads the program's (ROADMAP 0n: `f(1, c = a)` printed 111), and every
// argument up to the last one read, so they still run left to right (only
// the read ones were bound, and `g(tick(1), tick(2))` ticked 2 first). A
// handler's construction follows the same rule.
//
// Compile-and-run data ([test-run-data]).

//--- a_named_value_reads_the_program_and_arguments_run_in_order
fn f(a: Int, b: Int = a, c: Int = 0) -> Int {
    return a * 100 + b * 10 + c
}

fn tick(n: Int) [Console] -> Int {
    println("tick ${n}")
    return n
}

fn g(x: Int, y: Int, z: Int = y) -> Int {
    return x + y + z
}

effect Spanned {
    fn width() -> Int
}

handler Span(lo: Int, hi: Int, top: Int = hi + 10) of Spanned {
    fn width() -> Int {
        return top - lo
    }
}

fn width_of() [Spanned] -> Int {
    return width()
}

fn main() [use] {
    use StdConsole()
    let a = 7
    println("${f(1, c = a)}")
    println("${g(tick(1), tick(2))}")
    use Span(tick(3), tick(4))
    println("${width_of()}")
}
//= stdout
//= 117
//= tick 1
//= tick 2
//= 5
//= tick 3
//= tick 4
//= 11
