// [col-bounds] every index answers rather than failing
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- bounds

fn main() [use] {
    use StdConsole()

    // [col-bounds] A total exchange, and the one positional write a list of
    // obligations can have: nothing enters, nothing leaves, nothing is dropped.
    let xs: Mut List<Str> = Mut List("a", "b", "c")
    println("swapped ${detach(swap(xs, 0, 2))} ${to_str(xs)}")
    println("in place ${detach(swap(xs, 1, 1))} ${to_str(xs)}")
    println("past the end ${detach(swap(xs, 0, 3))} ${to_str(xs)}")
    println("negative ${detach(swap(xs, -1, 0))} ${to_str(xs)}")

    // Reads answer an optional, at either end, for a literal as for a variable.
    if get(xs, -1) is None {
        println("read before the start is absent")
    }
    if get(xs, 9) is None {
        println("read past the end is absent")
    }
    if char_at("hi", -1) is None {
        println("a character before the start is absent")
    }

    // A positional *write* answers whether it wrote, which is the same report
    // for the same reason: a write that quietly did nothing has no symptom at
    // the call. Out of range the value is the caller's and stays untouched.
    let text = Mut Str("hello")
    println("wrote ${set(text, 0, 'H')} ${text}")
    println("write past the end ${set(text, 9, 'X')} ${text}")
    let buf = mut_bytes(bytes_of(to_byte(1), to_byte(2)))
    println("wrote a byte ${set(buf, 1, to_byte(9))} ${to_hex(buf)}")
    println("byte past the end ${set(buf, 2, to_byte(9))} ${to_hex(buf)}")

    // The heap's own move, in the spelling it uses [fn-dot].
    let heap: Mut List<Int> = Mut List(5, 9, 7)
    ignore(heap.swap(0, 2))
    println("sifted ${to_str(heap)}")
}
//= stdout
//= swapped true [c, b, a]
//= in place true [c, b, a]
//= past the end false [c, b, a]
//= negative false [c, b, a]
//= read before the start is absent
//= read past the end is absent
//= a character before the start is absent
//= wrote true Hello
//= write past the end false Hello
//= wrote a byte true 0109
//= byte past the end false 0109
//= sifted [7, 9, 5]
