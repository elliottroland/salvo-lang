// std's byte buffer [type-bytes]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- bytes

fn main() [use] -> None {
    use StdConsole()
    // The two constructors: a fixed buffer, and one under construction.
    let fixed = bytes_of(to_byte(0), to_byte(255), to_byte(200))
    println("fixed: ${fixed} / ${to_hex(fixed)} / ${size(fixed)}")
    let buf = mut_bytes()
    buf.add(to_byte(104))
    buf.append(to_bytes("é"))
    println("built: ${buf} / ${to_hex(buf)}")
    // A `Mut Bytes` reaches the read surface by dropping its `Mut`.
    let text = str_of_bytes(buf)
    when text {
        is Str { println("text: ${text}") }
        is None { println("text: invalid") }
    }
    // Invalid UTF-8 decodes to nothing, strictly, on both backends.
    let broken = mut_bytes()
    broken.add(to_byte(200))
    println("broken: ${str_of_bytes(broken) is None}")
    // Reading: element, slice, search — each optional where it can miss.
    println("at 1: ${to_int(get(fixed, 1) ?: unreachable!())} / oob: ${get(fixed, 9) is None}")
    println("slice: ${slice(fixed, 1, 3) ?: unreachable!()} / bad: ${slice(fixed, 1, 9) is None}")
    println("index: ${index_of(fixed, to_byte(200)) ?: unreachable!()} / ${index_of(fixed, to_byte(7)) is None}")
    // Writing in place, and the clear that makes a buffer reusable.
    let scratch = mut_bytes(fixed)
    scratch.set(0, to_byte(1))
    scratch.set(9, to_byte(2))
    println("set: ${scratch}")
    clear(scratch)
    println("cleared: ${scratch} / ${size(scratch)}")
    // `==` is structural, and `copy` really copies.
    let a = bytes_of(to_byte(1), to_byte(2))
    let b = bytes_of(to_byte(1), to_byte(2))
    println("equal: ${a == b} / ${a == fixed}")
    let dup = copy(a)
    let grow = mut_bytes(dup)
    grow.add(to_byte(3))
    println("copy independent: ${a} vs ${grow}")
    // A `for` over a buffer, and the combinators over the same pass.
    let total = 0
    for byte in fixed {
        total = total + to_int(byte)
    }
    println("sum: ${total}")
    let doubled = map(iter(a), each -> to_int(each) * 2)
    println("mapped: ${doubled}")
}
//= stdout
//= fixed: [0, 255, 200] / 00ffc8 / 3
//= built: [104, 195, 169] / 68c3a9
//= text: hé
//= broken: true
//= at 1: 255 / oob: true
//= slice: [255, 200] / bad: true
//= index: 2 / true
//= set: [1, 255, 200]
//= cleared: [] / 0
//= equal: true / false
//= copy independent: [1, 2] vs [1, 2, 3]
//= sum: 455
//= mapped: [2, 4]
