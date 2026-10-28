// [type-bytes] [type-byte] `Bytes` — std's byte buffer — as the checker sees
// it: a type that opts into `Mut`, so it behaves like `Str` rather than like a
// container of `Byte`.
//
// What is worth pinning here is the *rules*, not the lowerings (the backends'
// e2e cases cover those): a buffer is written only through `Mut`, dropping
// `Mut` reaches the read surface, a `Byte` is not a number to the operators,
// and a buffer is not a hash key — the last one deliberately, since the
// Kotlin buffer is a mutable object and a key that can change under a map is
// a bug nobody sees.

// [type-bytes] The read surface takes a `Bytes`, and a `Mut Bytes` reaches it
// by dropping its `Mut` — the `Str`/`Mut Str` rule, on the buffer
// [str-drop-mut].
fn read_surface() [] -> None {
    let buf = mut_bytes()
    add(buf, to_byte(1))
    let _n = size(buf)
    let _hex = to_hex(buf)
    let _first = get(buf, 0)
}

// [type-bytes] [type-canbe-mut] Writing needs the `Mut`: a plain `Bytes` is
// read-only, which is what makes `copy` and the fs payloads safe to hand out.
fn write_plain_add() [] -> None {
    let data = bytes_of(to_byte(1))
    //~ error[call-resolve:no-overload]
    add(data, to_byte(2))
}

fn write_plain_clear() [] -> None {
    let data = bytes_of(to_byte(1))
    //~ error[call-resolve:no-overload]
    clear(data)
}

// [type-bytes] The text bridge is strict, so it answers an optional; the
// element reads are optional too, since an index may be outside the buffer
// [type-nullable].
fn optional_reads() [] -> None {
    let data = to_bytes("hi")
    let text = str_of_bytes(data)
    let _n = 0
    if text is Str {
        _n = 1
    }
    let _part = slice(data, 0, 1) ?: unreachable!()
    let _b = get(data, 0) ?: unreachable!()
}

// `Str?` is not a `Str`: the surface says so rather than coercing.
fn optional_not_narrowed() [] -> None {
    let data = to_bytes("hi")
    //~ error[var-let-infer:annotation-mismatch]
    let _text: Str = str_of_bytes(data)
}

// [type-byte] [op-arith] A `Byte` read out of a buffer is an octet, not a
// number: arithmetic goes through `to_int`.
fn byte_arithmetic() [] -> None {
    let data = bytes_of(to_byte(1))
    let b = get(data, 0) ?: unreachable!()
    //~ error[call-resolve:no-overload]
    let _sum = b + b
}

fn byte_through_int() [] -> None {
    let data = bytes_of(to_byte(1))
    let b = get(data, 0) ?: unreachable!()
    let _sum = to_int(b) + 1
}

// [col-hashed-ordered] A buffer is **not** a hash key, deliberately: the
// Kotlin `Bytes` is one mutable object, so a key could change under the map
// it is filed in. Refused where it is written, as every unhashable key is.
fn bytes_key() [] -> None {
    //~ error[col-key-eligible:key]
    let m: Mut Map<Bytes, Int> = Mut Map()
    put(m, bytes_of(to_byte(1)), 1)
}
