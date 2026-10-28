// The runner's own pilot: one program per outcome it can assert.
fn main() [use] -> None {
    use StdConsole()
    println("hello")
    println("")
    println("  indented")
}
//= stdout
//= hello
//=
//=   indented

//--- arguments
// ARGS: alpha beta
import std.args

fn main() [use] -> None {
    use StdConsole()
    use HostArgs()
    for arg in args() {
        println(arg)
    }
}
//= stdout
//= alpha
//= beta

//--- exit-code
import std.process

fn main() [use] -> None {
    use StdConsole()
    use HostProcess()
    println("leaving")
    exit(3)
}
//= exit 3
//= stdout
//= leaving

//--- trap
fn first(xs: List<Int>) -> Int => xs {
    return xs.get(0) ?: unreachable!()
}

fn main() [use] -> None {
    use StdConsole()
    println("before")
    println("${first(List<Int>())}")
}
//= trap unreachable at main:2:25
//= stdout
//= before

//--- modules
// FILE: main.sv
import shapes

fn main() [use] -> None {
    use StdConsole()
    println("${area(Square {side: 3})}")
}
// FILE: shapes.sv
export struct Square {
    side: Int
}

export fn area(s: Square) -> Int => s {
    return s.side * s.side
}
//= stdout
//= 9
