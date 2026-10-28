// [rs-opt-borrow] an owned optional local, read twice
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- an_intrinsic_argument_takes_the_intrinsics_own_mode

fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    if contains(maybe ?: unreachable!(), "ell") { println("found") }
    let other: Str? = "hello"
    println("upper ${to_upper(other ?: unreachable!())}")
    let xs: Mut List<Int>? = Mut List(1)
    add(xs ?: unreachable!(), 3)
    println("xs ${size(xs)}")
}
//= stdout
//= found
//= upper HELLO
//= xs 2

//--- a_narrowed_read_borrows_unless_the_position_owns

fn len_of(s: Str) -> Int => s { return size(s) }
fn shout(s: Str) -> Str => !s { return s }

struct Box { label: Str? }

fn main() [use] {
    use StdConsole()
    let name: Str? = "hello"
    if name is Str {
        println("size ${size(name)}")
        println("kept ${len_of(name)}")
        let held: Str = name
        println("held ${held}")
        println("owned ${shout(name)}")
    }
    let b = Box { label: "boxed" }
    if b.label is Str {
        println("field ${len_of(b.label)}")
    }
    let n: Int? = 4
    if n is Int {
        println("copy ${n + n}")
    }
}
//= stdout
//= size 5
//= kept 5
//= held hello
//= owned hello
//= field 5
//= copy 8

//--- an_owned_optional_local_is_read_through_a_borrow

fn len_of(s: Str) -> Int => s {
    return size(s)
}

fn shout(s: Str) -> Str => !s {
    return s
}

fn main() [use] {
    use StdConsole()
    let maybe: Str? = "hello"
    println("once ${size(maybe ?: "")}")
    println("twice ${size(maybe ?: "")}")
    let kept: Str? = "hello"
    println("kept ${len_of(kept ?: unreachable!())}")
    let owned: Str? = "hello"
    println("owned ${shout(owned ?: unreachable!())}")
    let n: Int? = 3
    println("copy ${(n ?: 0) + (n ?: 0)}")
}
//= stdout
//= once 5
//= twice 5
//= kept 5
//= owned hello
//= copy 6
