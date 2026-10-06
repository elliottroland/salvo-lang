// [buffer-type] `core.buffer`: fixed-length runs of `Int` and `Long` (user
// decisions 2026-10-05, ROADMAP §0j 6g), the index tables a hash structure
// written in Salvo probes. A `List<Int>` would box every slot on the JVM; a
// buffer is a Kotlin `IntArray`/`LongArray` and a Rust `Vec<i32>`/`Vec<i64>`.
//
// No `add` or `remove`: a buffer's length is fixed when it is made, and
// growing one builds a new one. So a write moves no boundary, and every `Idx`
// claim on a buffer survives `replace` and `clear` (`core.index`). A plain
// buffer is immutable, and `copy` copies the array. A host array has no wire
// form, so a buffer stays on its node [noremote].

// A run of `Int`s, all written when it is made.
export iterable platform type IntBuffer canbe Mut : Iter<self, Int>

// A run of `Long`s, the same over `Long`.
export iterable platform type LongBuffer canbe Mut : Iter<self, Long>

// [buffer-type] A buffer of [size] slots, every one holding [fill]. A
// negative size is an empty buffer.
export fn int_buffer(size: Int, fill: Int) [] -> Mut IntBuffer => size, fill {
    return new_int(size, fill)
}

// [buffer-type] The `Long` form of [int_buffer].
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

// [col-idx] The **total** read, the hot path of a hash probe: the index is
// proven in range, and the answer is the value itself — no boxed `Int?` on
// the JVM.
export fn get(buf: IntBuffer, index: Idx(buf) Int) [] -> Int => buf, index {
    return int_get(buf, index + 0)
}

export fn get(buf: LongBuffer, index: Idx(buf) Int) [] -> Long => buf, index {
    return long_get(buf, index + 0)
}

// [col-replace] Writes [value] into slot [index] and answers what it held. The
// index is proven, so nothing can be out of range; `Idx` claims survive it.
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

// [platform-iterable] The slots in order, for a generic fn over `Iter`; a
// `for` over a buffer is the host's own loop.
export iter fn iter(buf: IntBuffer) -> Emitted Int | Finished {
    state {
        at: Int = 0
    }
    if at >= int_size(buf) {
        return finished()
    }
    let value = int_get(buf, at)
    at = at + 1
    return emitted(value)
}

export iter fn iter(buf: LongBuffer) -> Emitted Long | Finished {
    state {
        at: Int = 0
    }
    if at >= long_size(buf) {
        return finished()
    }
    let value = long_get(buf, at)
    at = at + 1
    return emitted(value)
}

// The slots in order, as a list prints: `[1, 2, 3]`.
export fn to_str(buf: IntBuffer) [] -> Str => buf {
    let out: Mut Str = mut_str("[")
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
    let out: Mut Str = mut_str("[")
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
