// precedence-aware binary rendering
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- precedence

fn main() [use] -> None {
    use StdConsole
    let a = 10
    let b = 3
    let c = 2
    let grouped = (a - b) * c
    let nested = a - (b - c)
    let neg = -(a + b)
    let logical = (a < b || b > c) && a > c
    println("${grouped} ${nested} ${neg} ${logical}")
}
//= stdout
//= 14 9 -13 true
