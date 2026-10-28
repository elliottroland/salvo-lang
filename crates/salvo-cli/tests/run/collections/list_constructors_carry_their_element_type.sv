// list constructors carry their element type
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- list_element_types

export fn main() [use] -> None {
    use StdConsole
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    add(xs, 2)
    let ys = Mut List<Str>()
    add(ys, "a")
    println("${size(xs)} ${size(ys)}")
}
//= stdout
//= 2 1
