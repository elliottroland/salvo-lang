// L7b: once fn types [fn-once]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- once_fns

fn run_once(f: once () [Console] -> None) {
    f()
}

fn consume_list(v: List<Int>) [Console] -> None => !v {
    println("consumed ${size(v)} items")
}

fn main() [use] -> None {
    use StdConsole
    let xs = List(1, 2, 3)
    let g = () -> { consume_list(xs) }
    run_once(g)
    let n = 7
    let plain = () -> { println("plain ${n}") }
    run_once(plain)
    println("done")
}
//= stdout
//= consumed 3 items
//= plain 7
//= done
