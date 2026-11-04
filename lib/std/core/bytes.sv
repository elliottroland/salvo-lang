// Raw binary data: the `Bytes` buffer, which every byte-shaped API in std
// takes and answers (file and stream reads, network payloads, encodings).
// Part of `std.core`, so every file sees it without an import.
//
// A `Bytes` is a sequence of `Byte`s, each an unsigned octet from 0 to 255.
// A `Byte` is made from an `Int` with `to_byte`, which keeps the low 8 bits,
// and read back with `to_int`. A buffer works like a string: a `Bytes` is
// read, and a `Mut Bytes` is one being filled, through [add], [append],
// [set] and [clear]. A `Mut Bytes` can be passed wherever a `Bytes` is
// expected, at no cost.
//
// ```
// let header = bytes_of(to_byte(202), to_byte(254))
// let buf = mut_bytes(header)
// append(buf, to_bytes("hi"))
// println("${size(buf)} bytes: ${to_hex(buf)}")          // 4 bytes: cafe6869
// let tail = slice(buf, 2, 4)                            // [104, 105]
// if tail is Bytes t {
//     println(str_of_bytes(t) ?: "not text")             // hi
// }
// ```
//
// **Text and bytes.** [to_bytes] encodes a string as UTF-8, and
// [str_of_bytes] decodes UTF-8 **strictly**: bytes that are not valid UTF-8
// answer `None`, never text with replacement characters. Text reads from
// files and streams follow the same rule.
//
// **One buffer for a whole loop.** The fill-a-buffer reads of `std.stream`
// (`read_to`) append to a `Mut Bytes` you own, so a loop that reads a large
// file in chunks can [clear] and reuse one buffer instead of getting a new
// one per chunk. Clearing keeps the room the buffer had grown to.
//
// **Absence is `None`.** [get], [slice] and [index_of] answer `None` for an
// index outside the buffer or a byte that does not occur, rather than
// failing or answering `-1`.
// @compilerrules [kt-bytes, type-bytes, fs-read-to, platform-value-type, platform-type]

// A value platform type (ROADMAP 0.7): the host implements it in
// `lib/platform/std/core/bytes.{rs,kt}`, its copy is the host's own copy,
// and its mutable kind is the host's `MutBytes`. It is not a `List<Byte>`
// because a list of octets boxes every element on the JVM (`UByte` is a
// value class with no box cache) and a specialized array cannot be a
// `List<T>` there; and slicing, hex, a strict UTF-8 decode and an in-place
// fill do not belong on `List`.

// A buffer of bytes. `Bytes` is read; `Mut Bytes` is one being built, and
// reaches every reading function as well.
//
// Bytes have no literal: make a buffer with [bytes_of] from single bytes,
// [mut_bytes] from other buffers, or [to_bytes] from text. `for b in data`
// walks the bytes in order, and `==` compares two buffers byte for byte.
//
// ```
// let data = bytes_of(to_byte(1), to_byte(2), to_byte(3))
// let total = 0
// for b in data {
//     total = total + to_int(b)
// }
// println("${total}")                                    // 6
// ```
// @compilerrules [kt-bytes, type-bytes, fs-read-to, platform-value-type, platform-type]
export iterable platform type Bytes canbe Mut : Iter<self, BytesIter, Byte>

// A buffer holding [elems], in order. This is the constructor for a fixed
// buffer, since bytes have no literal syntax.
//
// ```
// let magic = bytes_of(to_byte(127), to_byte(69), to_byte(76), to_byte(70))
// println(to_hex(magic))                       // 7f454c46
// ```
// @compilerrules [col-no-literal]
export fn bytes_of(...elems: Byte[]) [] -> Bytes => elems {
    let out = empty_bytes()
    for b in elems {
        add(out, b)
    }
    return out
}

// A buffer to fill, holding [parts] concatenated in order. `mut_bytes()` is
// the empty one, which is how a read buffer usually starts.
//
// ```
// let buf = mut_bytes()
// add(buf, to_byte(10))
// append(buf, to_bytes("ok"))
// println("${buf}")                                      // [10, 111, 107]
// ```
export fn mut_bytes(...parts: Bytes[]) [] -> Mut Bytes => parts {
    let out = empty_bytes()
    for part in parts {
        append(out, part)
    }
    return out
}

// An empty buffer to build: what [mut_bytes] and [bytes_of] start from.
platform fn empty_bytes() [] -> Mut Bytes

// The UTF-8 encoding of [str], the same on every backend. Its [size] is
// [byte_size] of the string.
//
// ```
// let data = to_bytes("hé")                              // [104, 195, 169]
// ```
export platform fn to_bytes(str: Str) [] -> Bytes => str

// The text [data] spells in UTF-8, or `None` when it is not valid UTF-8.
//
// The decode is **strict** on every backend: one malformed byte makes the
// answer `None`, never a replacement character. The text reads of files and
// streams follow the same rule.
//
// ```
// let ok = str_of_bytes(to_bytes("hé"))                  // "hé"
// let bad = str_of_bytes(bytes_of(to_byte(255)))         // None
// ```
// @compilerrules [fs-bytes]
export platform fn str_of_bytes(data: Bytes) [] -> Str? => data

// Whether two buffers hold the same bytes in the same order, which is what
// `==` means on `Bytes`.
// @compilerrules [op-equality, fn-attached]
export platform fn eq(a: Bytes, b: Bytes) [] -> Bool => a, b

// The number of bytes in [data].
export platform fn size(data: Bytes) [] -> Int => data

// The byte at [index], or `None` when [index] is outside the buffer.
//
// ```
// let data = to_bytes("abc")
// let b = get(data, 0)                                   // 97
// let none = get(data, 3)                                // None
// ```
export platform fn get(data: Bytes, index: Int) [] -> Byte? => data, index

// The bytes from [start] (inclusive) to [end] (exclusive), or `None` when
// that range does not lie within the buffer.
//
// The answer is a **copy**, not a view: it does not borrow [data], and
// changing one buffer later does not change the other.
//
// ```
// let data = to_bytes("salvo")
// let mid = slice(data, 1, 4)                            // the bytes of "alv"
// let none = slice(data, 3, 9)                           // None
// ```
// @compilerrules [proj-field]
export platform fn slice(data: Bytes, start: Int, end: Int) [] -> Bytes? => data, start, end

// The index of the first occurrence of [byte] in [data], or `None` when it
// does not occur.
//
// ```
// let at = index_of(to_bytes("a=b"), to_byte(61))        // 1: '=' is 61
// ```
// @compilerrules [type-nullable]
export platform fn index_of(data: Bytes, byte: Byte) [] -> Int? => data, byte

// Appends one byte to the end of [data].
export platform fn add(data: Mut Bytes, byte: Byte) [] -> None => data: Mut, byte

// Appends every byte of [more] to the end of [data].
export platform fn append(data: Mut Bytes, more: Bytes) [] -> None => data: Mut, more

// Replaces the byte at [index] with [byte], and answers whether it did.
//
// Out of range it writes nothing and answers `false`: the buffer never
// grows here, which is what [add] is for. Ignore the answer where the index
// is known to be good.
//
// ```
// let buf = mut_bytes(to_bytes("cat"))
// let done = set(buf, 0, to_byte(98))                    // true: now "bat"
// let missed = set(buf, 3, to_byte(115))                 // false: unchanged
// ```
// @compilerrules [col-bounds]
export platform fn set(data: Mut Bytes, index: Int, byte: Byte) [] -> Bool
=> data: Mut, index, byte

// Removes every byte from [data], keeping the room it had grown to. This is
// what lets one buffer serve every step of a read loop.
// @compilerrules [fs-read-to]
export platform fn clear(data: Mut Bytes) [] -> None => data: Mut

// The bytes as decimal numbers, written the way a list is: `[0, 255, 200]`.
// This is what interpolating a buffer prints; [to_hex] is the compact form.
// @compilerrules [interp-to-str]
export platform fn to_str(data: Bytes) [] -> Str => data

// The bytes in lower-case hexadecimal, two characters per byte and nothing
// between them: `00ffc8`. The form a checksum, a digest or a dump of wire
// data is usually shown in.
//
// ```
// let hex = to_hex(bytes_of(to_byte(0), to_byte(255), to_byte(200)))   // "00ffc8"
// ```
export platform fn to_hex(data: Bytes) [] -> Str => data

// A fresh iterator over the bytes of [data], in order. This is what lets a
// `for` loop and every function of `std.core.seq` walk a buffer.
// @compilerrules [iter-mint]
export fn iter(data: Bytes) [] -> Mut BytesIter => data {
    return Mut BytesIter { data: data, at: 0 }
}

// The iterator [iter] answers: a buffer and a position in it. It borrows the
// buffer rather than copying it.
// @compilerrules [iter-protocol, proj-field]
export struct BytesIter : Next<self, Byte> canbe Mut {
    // The buffer being walked.
    data: proj Bytes,
    // The index of the next byte to emit.
    at: Int
}

// Answers the byte at the iterator's position and moves past it, or answers
// the end once every byte has been emitted.
export fn next(p: Mut BytesIter) [] -> Emitted Byte | Finished => p: Mut {
    let b = get(p.data, p.at)
    if b is None {
        return finished()
    }
    p.at = p.at + 1
    return emitted(b)
}
