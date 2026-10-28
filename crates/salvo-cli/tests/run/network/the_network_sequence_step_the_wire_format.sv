// the network sequence, step ②: the wire format
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- the_wire_format
import std.net
import std.codec

struct Point { x: Int, y: Long }
struct Tag { name: Str }
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
    let bytes = encode(p)
    println("${size(bytes)} bytes: ${to_hex(bytes)}")
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

actor effect Ledger {
    send fn record(entry: Str, amount: Long) => !entry, !amount
    send fn close()
}
//= stdout
//= 50 bytes: fffffffe000000012a05f2000100000004686f6d650000000200000001610000000362c3a901013fc0000000000002686901
//= -2 5000000000 home 2 true
//= square 1.5
//= bad is none: true
//= int: 00000102  str: 00000002c3a9  opt: 00
