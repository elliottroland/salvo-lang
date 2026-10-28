// The wire format: a struct's and a protocol's codecs, and the protocol hash
// [net-wire-format] [rs-wire].

//--- a_protocol_with_a_wire_form_gets_a_codec_and_a_hash
// [net-wire-format] [rs-wire] Every scalar, `Str`, `Bytes`, an optional, a list of
// structs, a three-armed union with a `None` arm — encoded and decoded, and
// the bytes printed as hex. **The hex is the parity assertion**: the Kotlin
// backend runs the same program and must print the same bytes, since a
// Kotlin node and a Rust node share a group. A malformed input decodes to
// `None`. The trailing `Ledger` protocol has a wire form, so its message
// codec and `__PROTO_Ledger` are generated.
import std.net
import std.codec

struct Point { x: Int, y: Long }
struct Tag { name: Str }
//~ emits(main.rs): impl crate::wire::__Wire for Pin {
// The union codec.
//~ emits(unions/union2.rs): impl<T1: crate::wire::__Wire, T2: crate::wire::__Wire> crate::wire::__Wire for Union2<T1, T2>
struct Pin { at: Point, label: Str?, tags: List<Tag>, kind: Circle | Square | None, raw: Bytes, ok: Bool }
struct Circle { r: Double }
struct Square { side: Float }

fn main() [use] {
    use StdConsole()
    let p = Pin {
        at: Point { x: -2, y: 5000000000 },
        label: "home",
        tags: List(Tag { name: "a" }, Tag { name: "bé" }),
        kind: Square { side: 1.5 },
        raw: to_bytes("hi"),
        ok: true
    }
    //~ emits(main.rs): crate::wire::salvo_encode(&
    let bytes = encode(p)
    println("${size(bytes)} bytes: ${to_hex(bytes)}")
    //~ emits(main.rs): crate::wire::salvo_decode::<crate::Pin>(&
    let back = decode<Pin>(bytes)
    if back is None {
        println("decode failed")
        return
    }
    println("${back.at.x} ${back.at.y} ${back.label ?: "-"} ${size(back.tags)} ${back.ok}")
    let kind = back.kind
    when kind {
        is Circle { println("circle") }
        is Square { println("square ${kind.side}") }
        is None { println("no kind") }
    }
    let bad = decode<Point>(to_bytes("x"))
    println("bad is none: ${bad is None}")
    println("int: ${to_hex(encode(258))}  str: ${to_hex(encode("é"))}  opt: ${to_hex(encode(none_int()))}")
}

fn none_int() -> Int? { return None }

//~ emits(main.rs): impl crate::wire::__Wire for __Msg_Ledger {
//~ emits(main.rs): pub const __PROTO_Ledger: &str = "
actor effect Ledger {
    send fn record(entry: Str, amount: Long) => !entry, !amount
    send fn close()
}
