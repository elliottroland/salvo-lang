// The emit runner's own pilot.

//--- own_code
fn twice(n: Int) -> Int {
    //~ emits: fun twice(n: Int): Int
    return n * 2
}

fn main() [use] -> None {
    use StdConsole()
    //~ emits(main.kt): twice(21)
    //~ !emits: twice_twice
    println("${twice(21)}")
}
