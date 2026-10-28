// tuple element access [expr-tuple-index]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- tuple_index

fn main() [use] -> None {
    use StdConsole
    let t: (Int, Str, Bool) = (1, "two", true)
    println("${t.0} ${t.1} ${t.2}")
    let nested: (Int, (Str, Int)) = (7, ("in", 9))
    println("${nested.1.0} ${nested.1.1}")
    let maybe: (Str?, Int) = ("here", 5)
    if maybe.0 is Str {
        println("some ${maybe.0} ${maybe.1 + 1}")
    } else {
        println("none")
    }
}
//= stdout
//= 1 two true
//= in 9
//= some here 6
