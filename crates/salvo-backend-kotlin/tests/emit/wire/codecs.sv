// The wire format [net-wire-format] [kt-wire]: codecs and protocol hashes.

//--- a_protocol_with_a_wire_form_gets_a_codec_and_a_hash
// The Rust backend's `WIRE_DEMO`, verbatim, with the same expected hex — the
// byte-for-byte parity assertion for the wire [net-wire-format] [kt-wire].
import std.net
import std.codec

struct Point { x: Int, y: Long }
struct Tag { name: Str }
//~ emits: object __Codec_Pin : salvo.WireCodec<Pin> {
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
    //~ emits: salvo.salvoEncode(
    let bytes = encode(p)
    println("${size(bytes)} bytes: ${to_hex(bytes)}")
    //~ emits: salvo.salvoDecode(
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

//~ emits: object __Codec___Msg_Ledger : salvo.WireCodec<__Msg_Ledger> {
//~ emits: const val __PROTO_Ledger: String = "
actor effect Ledger {
    send fn record(entry: Str, amount: Long) => !entry, !amount
    send fn close()
}
