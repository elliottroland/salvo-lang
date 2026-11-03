// [col-buffer] `std.core.buffer`: fixed-length runs of `Int` and `Long` (user
// decisions 2026-10-05, ROADMAP §0j 6g), the index tables a hash structure
// written in Salvo probes. A `List<Int>` would box every slot on the JVM; a
// buffer is a Kotlin `IntArray`/`LongArray` and a Rust `Vec<i32>`/`Vec<i64>`.
//
// No `add` or `remove`: a buffer's length is fixed when it is made, and
// growing one builds a new one. So a write moves no boundary, and every `Idx`
// claim on a buffer survives `replace` and `clear` (`std.core.index`). A plain
// buffer is immutable, and `copy` copies the array. A host array has no wire
// form, so a buffer stays on its node [net-noremote].

// A run of `Int`s, all written when it is made.
export iterable platform type IntBuffer canbe Mut : Iter<self, IntBufferIter, Int>

// A run of `Long`s, the same over `Long`.
export iterable platform type LongBuffer canbe Mut : Iter<self, LongBufferIter, Long>

// A buffer of [size] slots, every one holding [fill]. A
// negative size is an empty buffer.
// @compilerrules [col-buffer]
export fn int_buffer(size: Int, fill: Int) [] -> Mut IntBuffer => size, fill {
    return new_int(size, fill)
}

// The `Long` form of [int_buffer].
// @compilerrules [col-buffer]
export fn long_buffer(size: Int, fill: Long) [] -> Mut LongBuffer => size, fill {
    return new_long(size, fill)
}

// The number of slots.
export fn size(buf: IntBuffer) [] -> Int => buf {
    return int_size(buf)
}

export fn size(buf: LongBuffer) [] -> Int => buf {
    return long_size(buf)
}

// The value in slot [index], or `None` past either end.
export fn get(buf: IntBuffer, index: Int) [] -> Int? => buf, index {
    if index is Idx(buf) {
        return int_get(buf, index + 0)
    }
    return None
}

export fn get(buf: LongBuffer, index: Int) [] -> Long? => buf, index {
    if index is Idx(buf) {
        return long_get(buf, index + 0)
    }
    return None
}

// The **total** read, the hot path of a hash probe: the index is
// proven in range, and the answer is the value itself — no boxed `Int?` on
// the JVM.
// @compilerrules [col-idx]
export fn get(buf: IntBuffer, index: Idx(buf) Int) [] -> Int => buf, index {
    return int_get(buf, index + 0)
}

export fn get(buf: LongBuffer, index: Idx(buf) Int) [] -> Long => buf, index {
    return long_get(buf, index + 0)
}

// Writes [value] into slot [index] and answers what it held. The
// index is proven, so nothing can be out of range; `Idx` claims survive it.
// @compilerrules [col-replace]
export fn replace(buf: Mut IntBuffer, index: Idx(buf) Int, value: Int) [] -> Int
=> buf: Mut, index, value {
    return int_replace(buf, index + 0, value)
}

export fn replace(buf: Mut LongBuffer, index: Idx(buf) Int, value: Long) [] -> Long
=> buf: Mut, index, value {
    return long_replace(buf, index + 0, value)
}

// Sets every slot to [fill]. The length does not change, so `Idx` claims
// survive it.
export fn clear(buf: Mut IntBuffer, fill: Int) [] -> None => buf: Mut, fill {
    int_clear(buf, fill)
}

export fn clear(buf: Mut LongBuffer, fill: Long) [] -> None => buf: Mut, fill {
    long_clear(buf, fill)
}

// The slots in order, for a generic fn over `Iter`; a
// `for` over a buffer is the host's own loop.
// @compilerrules [platform-iterable]
export fn iter(buf: IntBuffer) [] -> Mut IntBufferIter => buf {
    return Mut IntBufferIter { buf: buf, at: 0 }
}

// The iterator [iter] mints over an `IntBuffer`: the buffer, borrowed, and
// the next slot.
export struct IntBufferIter : Next<self, Int> canbe Mut {
    // The buffer being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    buf: proj IntBuffer,
    // The next slot to emit.
    at: Int
}

// Advances a buffer walk: the next slot's value, or the end.
export fn next(p: Mut IntBufferIter) [] -> Emitted Int | Finished => p: Mut {
    if p.at >= int_size(p.buf) {
        return finished()
    }
    let value = int_get(p.buf, p.at)
    p.at = p.at + 1
    return emitted(value)
}

export fn iter(buf: LongBuffer) [] -> Mut LongBufferIter => buf {
    return Mut LongBufferIter { buf: buf, at: 0 }
}

// The iterator [iter] mints over a `LongBuffer`.
export struct LongBufferIter : Next<self, Long> canbe Mut {
    // The buffer being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    buf: proj LongBuffer,
    // The next slot to emit.
    at: Int
}

export fn next(p: Mut LongBufferIter) [] -> Emitted Long | Finished => p: Mut {
    if p.at >= long_size(p.buf) {
        return finished()
    }
    let value = long_get(p.buf, p.at)
    p.at = p.at + 1
    return emitted(value)
}

// The slots in order, as a list prints: `[1, 2, 3]`.
export fn to_str(buf: IntBuffer) [] -> Str => buf {
    let out: Mut Str = Mut Str("[")
    for i in range(size(buf)) {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(int_get(buf, i)))
    }
    append(out, "]")
    return out
}

export fn to_str(buf: LongBuffer) [] -> Str => buf {
    let out: Mut Str = Mut Str("[")
    for i in range(size(buf)) {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_str(long_get(buf, i)))
    }
    append(out, "]")
    return out
}

// The host's side: two platform fns of one module may not overload, so each
// kind's carry its name.
platform fn new_int(size: Int, fill: Int) [] -> Mut IntBuffer => size, fill
platform fn int_size(buf: IntBuffer) [] -> Int => buf
// The index is in range: the callers prove it.
platform fn int_get(buf: IntBuffer, index: Int) [] -> Int => buf, index
platform fn int_replace(buf: Mut IntBuffer, index: Int, value: Int) [] -> Int => buf: Mut, index, value
platform fn int_clear(buf: Mut IntBuffer, fill: Int) [] -> None => buf: Mut, fill

platform fn new_long(size: Int, fill: Long) [] -> Mut LongBuffer => size, fill
platform fn long_size(buf: LongBuffer) [] -> Int => buf
platform fn long_get(buf: LongBuffer, index: Int) [] -> Long => buf, index
platform fn long_replace(buf: Mut LongBuffer, index: Int, value: Long) [] -> Long => buf: Mut, index, value
platform fn long_clear(buf: Mut LongBuffer, fill: Long) [] -> None => buf: Mut, fill
