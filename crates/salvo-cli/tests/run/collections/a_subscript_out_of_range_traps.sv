// [col-index-trap] [assert-trap] An array subscript out of range — past the
// end, or negative — is Salvo's trap, reading the same on both backends,
// at the index; before 2026-10-30 it was the host's (Rust's `index out of
// bounds` panic, the JVM's `ArrayIndexOutOfBoundsException`). An index in
// range reads and writes as before, through a field and a nested array.
//
// Compile-and-run data ([test-run-data]).

//--- in_range_reads_and_writes
struct P canbe Mut {
    x: Int
}

fn main() [use] -> None {
    use StdConsole()
    let numbers = array_of(1, 2, 3)
    numbers[1] = 5
    let ps = array_of(Mut P { x: 1 })
    ps[0].x = 4
    let grid = array_of(array_of(1, 2))
    grid[0][1] = 7
    println("${numbers[1]} ${ps[0].x} ${grid[0][1]}")
}
//= stdout
//= 5 4 7

//--- past_the_end
fn main() [use] -> None {
    use StdConsole()
    let numbers = array_of(1, 2, 3)
    let i = 10
    println("before")
    println("value ${numbers[i]}")
}
//= trap index out of range at main:6:30
//= stdout
//= before

//--- a_negative_index_on_a_write
fn main() [use] -> None {
    use StdConsole()
    let numbers = array_of(1, 2, 3)
    let i = 0 - 1
    numbers[i] = 4
    println("never")
}
//= trap index out of range at main:5:13
