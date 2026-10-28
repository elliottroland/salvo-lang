// Value platform types [platform-value-type]: what a program names of the
// host's types, and the host files std ships for them.

//--- bytes_is_a_value_platform_type
// [platform-value-type] `Bytes` is a value platform type: the program names
// the host's types — `Bytes`, and `MutBytes` for the mutable kind, by
// convention — and calls the platform wrappers; std's host file aliases
// both to the shipped `salvo.SalvoBytes`.
// [type-bytes] [type-byte] The whole `Bytes` surface in one program
// (`BYTES_PROGRAM` in codegen_tests.rs).
// std's host file for core.bytes travels with the program, and the buffer
// class still ships.
//~ emits(platform/std/core/bytes.kt): typealias MutBytes
//~ emits(platform/std/core/bytes.kt): fun copy(data: Bytes)
//~ emits(bytes.kt): class SalvoBytes
fn main() [use] -> None {
    use StdConsole()
    // The two constructors: a fixed buffer, and one under construction.
    let fixed = bytes_of(to_byte(0), to_byte(255), to_byte(200))
    println("fixed: ${fixed} / ${to_hex(fixed)} / ${size(fixed)}")
    // The host's mutable kind, named by convention.
    //~ emits: salvo.platform.std.core.bytes.MutBytes
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
