// `Bytes`: a byte buffer, and the currency of every byte-shaped API in std.
//
// It exists because `List<Byte>` was the wrong shape for the job in two ways.
// One is representation: a list of octets boxes every element on the JVM
// (`UByte` is a value class with no box cache), and a specialized array cannot
// be a `List<T>` there, so the honest byte buffer could not be a list at all
// [kt-bytes]. The other is the surface: bytes want slicing, hex, a strict
// UTF-8 decode and an in-place fill, and none of those belong on `List`.
//
// The two shapes, exactly as `Str` and `Mut Str`:
//
//   * `Bytes` is a buffer you read — `size`, `get`, `slice`, `index_of`,
//     `to_str`, `to_hex`, `str_of_bytes`, and `for b in data`.
//   * `Mut Bytes` is a buffer you build — `add`, `append`, `set`, `clear` —
//     and it reaches the whole read surface by dropping its `Mut`, which
//     costs nothing on either backend [bytes-type].
//
// That is what makes the fill-a-buffer reads work: one `Mut Bytes` is filled,
// used and cleared for as long as a loop runs, instead of a fresh payload per
// chunk [fs-read-to].
intrinsic type Bytes canbe Mut

// The bytes of [elems], in order: `bytes_of(to_byte(0), to_byte(255))`.
// Bytes have no literal syntax — a `[...]` literal is a `List` [col-literal] —
// so this is the constructor.
intrinsic fn bytes_of(...elems: Byte[]) [] -> Bytes

// A buffer under construction, holding [parts] concatenated in order.
// `mut_bytes()` is the empty one, which is how a read buffer starts;
// mutability is asked for here, mirroring [mut_str] and [mut_list_of].
intrinsic fn mut_bytes(...parts: Bytes[]) [] -> Mut Bytes

// The UTF-8 bytes of [str] — the encoding both backends agree on, and the one
// [byte_size] counts.
intrinsic fn to_bytes(str: Str) [] -> Bytes => str

// The text [data] spells in UTF-8, or `None` when it is not valid UTF-8.
// Decoding is **strict** on both backends — a malformed byte is `None`, never
// a replacement character — which is the rule the filesystem's text reads
// follow too [fs-bytes].
intrinsic fn str_of_bytes(data: Bytes) [] -> Str? => data

// The number of bytes in [data].
intrinsic fn size(data: Bytes) [] -> Int => data

// The byte at [index], or `None` when [index] is outside the buffer.
intrinsic fn get(data: Bytes, index: Int) [] -> Byte? => data, index

// The bytes from [start] (inclusive) to [end] (exclusive), or `None` when
// that range does not lie within the buffer. A **copy**, not a view: Salvo
// states borrows in deductions, and this one does not borrow [proj-field].
intrinsic fn slice(data: Bytes, start: Int, end: Int) [] -> Bytes? => data, start, end

// The index of the first occurrence of [byte], or `None` when it does not
// occur. No `-1` sentinel: absence is `None` [type-nullable].
intrinsic fn index_of(data: Bytes, byte: Byte) [] -> Int? => data, byte

// Appends one byte to [data].
intrinsic fn add(data: Mut Bytes, byte: Byte) [] -> None => data: Mut, byte

// Appends every byte of [more] to [data].
intrinsic fn append(data: Mut Bytes, more: Bytes) [] -> None => data: Mut, more

// Replaces the byte at [index]. Out of range it does nothing — the buffer is
// the caller's, and growing it here would make a `set` an `add`, exactly as
// on `Mut Str` [set].
intrinsic fn set(data: Mut Bytes, index: Int, byte: Byte) [] -> None => data: Mut, index, byte

// Removes every byte from [data], keeping whatever room it had. This is the
// call that makes a buffer reusable across reads [fs-read-to].
intrinsic fn clear(data: Mut Bytes) [] -> None => data: Mut

// The numbers, as a list is written: `[0, 255, 200]`. Deliberately the same
// text a `List<Byte>` produces, so what a program prints did not change when
// the payload type did [interp-to-str].
intrinsic fn to_str(data: Bytes) [] -> Str => data

// Lower-case hex, two characters per byte and nothing between them:
// `00ffc8`. The form a checksum, a digest or a wire dump wants.
intrinsic fn to_hex(data: Bytes) [] -> Str => data

// [iter-pass] A fresh pass over the bytes of [data], in order — which is what
// makes `for b in data` and the sequence functions work on a buffer.
fn iter(data: Bytes) [] -> Mut BytesYield => data {
    return Mut BytesYield { data: data, at: 0 }
}

// [iter-protocol] The pass a buffer is walked by: the buffer plus a position
// in it. The buffer is borrowed, not copied [proj-field].
struct BytesYield : Yield<self, Byte> canbe Mut {
    // The buffer being walked.
    data: proj Bytes,
    // The index of the next byte to emit.
    at: Int
}

// Advances the pass, reporting the byte at its position or the end of the
// buffer.
fn next(p: Mut BytesYield) [] -> Emitted Byte | Finished => p: Mut {
    let b = get(p.data, p.at)
    if b is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(b)
}
