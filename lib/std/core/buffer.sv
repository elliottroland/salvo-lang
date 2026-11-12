// Fixed-length arrays of numbers: an `IntBuffer` holds `Int`s and a
// `LongBuffer` holds `Long`s. They are for tables read and written by
// index in a tight loop, like the slots a hash table probes.
//
// A buffer is a host array of the raw numbers (a Kotlin `IntArray` or
// `LongArray`, a Rust `Vec<i32>` or `Vec<i64>`), where a `List<Int>` would
// box every element on the JVM. Reading and writing a slot are O(1).
//
// ```
// let table = int_buffer(8, -1)            // eight slots, every one -1
// let slot = 3
// if slot is Idx(table) {
//     let was = replace(table, slot, 42)   // -1: written in place
//     let now = get(table, slot)           // 42, an Int: the index is proven
// }
// clear(table, -1)                         // every slot back to -1
// println("${size(table)} ${table}")       // 8 [-1, -1, -1, -1, -1, -1, -1, -1]
// ```
//
// **The length never changes.** A buffer is made full, with every slot
// holding a given value, and has no `add` or `remove`; growing one means
// building a bigger one and copying across. Since no write moves a
// boundary, an index proven in range with `i is Idx(buf)` stays proven
// across [replace] and [clear], and the total [get] at it answers the number
// itself, with no optional.
//
// **A plain value.** An `IntBuffer` cannot be changed; a `Mut IntBuffer`,
// which [int_buffer] answers, can. `copy(buf)` copies the whole array.
// `for x in buf` walks the slots in order, and a buffer prints as a list does. A
// buffer has no `==`, and no wire form, so it stays on its node.
// @compilerrules [col-buffer, net-noremote]

// A fixed-length array of `Int`s, every slot written when it is made.
//
// ```
// let b = int_buffer(3, 7)                 // [7, 7, 7]
// ```
export iterable platform type IntBuffer canbe Mut : Iter<self, IntBufferIter, Int>

// A fixed-length array of `Long`s: [IntBuffer] over `Long`.
export iterable platform type LongBuffer canbe Mut : Iter<self, LongBufferIter, Long>

// Makes an `IntBuffer` of a given number of slots, every one holding a
// given value.
//
// A size of 0 or less gives an empty buffer.
//
// ```
// let table = int_buffer(4, -1)            // [-1, -1, -1, -1]
// ```
//
// @param size  the number of slots
// @param fill  the value every slot starts with
// @compilerrules [col-buffer]
export fn int_buffer(size: Int, fill: Int) [] -> Mut IntBuffer => size, fill {
    return new_int(size, fill)
}

// Makes a `LongBuffer`: [int_buffer] over `Long`.
//
// ```
// let b = long_buffer(2, 3L)               // [3, 3]
// ```
// @compilerrules [col-buffer]
export fn long_buffer(size: Int, fill: Long) [] -> Mut LongBuffer => size, fill {
    return new_long(size, fill)
}

// The number of slots in the buffer.
export fn size(buf: IntBuffer) [] -> Int => buf {
    return int_size(buf)
}

// The number of slots in a `LongBuffer`.
export fn size(buf: LongBuffer) [] -> Int => buf {
    return long_size(buf)
}

// The value in a slot, or `None` when the index is out of range at either
// end.
//
// An index proven in range (`i is Idx(buf)`) picks the overload below,
// which answers the value itself.
//
// ```
// let b = int_buffer(3, 7)
// let n = get(b, 0) ?: 0                   // 7
// let none = get(b, 3)                     // None
// ```
export fn get(buf: IntBuffer, index: Int) [] -> Int? => buf, index {
    if index is Idx(buf) {
        return int_get(buf, index + 0)
    }
    return None
}

// The value in a slot of a `LongBuffer`, or `None` when the index is out
// of range.
export fn get(buf: LongBuffer, index: Int) [] -> Long? => buf, index {
    if index is Idx(buf) {
        return long_get(buf, index + 0)
    }
    return None
}

// [get] at an index proven in range: answers the value itself.
//
// The fast path of a probe loop: no optional, and so on the JVM no boxed
// `Int`.
//
// ```
// let b = int_buffer(4, 0)
// let i = 2
// if i is Idx(b) {
//     let v = get(b, i)                    // 0, an Int
// }
// ```
// @compilerrules [col-idx]
export fn get(buf: IntBuffer, index: Idx(buf) Int) [] -> Int => buf, index {
    return int_get(buf, index + 0)
}

// [get] on a `LongBuffer` at an index proven in range: answers the value
// itself.
export fn get(buf: LongBuffer, index: Idx(buf) Int) [] -> Long => buf, index {
    return long_get(buf, index + 0)
}

// Writes a value into a slot at an index proven in range, and answers the
// value it held.
//
// The index is proven, so the write cannot miss. Every `Idx` claim on the
// buffer survives it.
//
// ```
// let b = int_buffer(4, 0)
// let i = 2
// if i is Idx(b) {
//     let old = replace(b, i, 5)           // 0; b is [0, 0, 5, 0]
// }
// ```
// @compilerrules [col-replace]
export fn replace(buf: Mut IntBuffer, index: Idx(buf) Int, value: Int) [] -> Int
=> buf: Mut, index, value {
    return int_replace(buf, index + 0, value)
}

// [replace] on a `LongBuffer`.
export fn replace(buf: Mut LongBuffer, index: Idx(buf) Int, value: Long) [] -> Long
=> buf: Mut, index, value {
    return long_replace(buf, index + 0, value)
}

// Sets every slot of the buffer to a value.
//
// The length does not change, so every `Idx` claim on the buffer survives.
//
// ```
// let b = int_buffer(3, 1)
// clear(b, 0)                              // [0, 0, 0]
// ```
export fn clear(buf: Mut IntBuffer, fill: Int) [] -> None => buf: Mut, fill {
    int_clear(buf, fill)
}

// [clear] on a `LongBuffer`.
export fn clear(buf: Mut LongBuffer, fill: Long) [] -> None => buf: Mut, fill {
    long_clear(buf, fill)
}

// A fresh iterator over the buffer's slots, in order.
//
// The iterator borrows the buffer. A `for` loop over a buffer needs no
// `iter` (it is the host's own loop); this is for passing a buffer to a
// function that takes an iterator, like those of `std.core.seq`.
//
// ```
// let b = int_buffer(3, 2)
// let total = reduce(iter(b), 0, (acc, x) -> acc + x)   // 6
// ```
// @compilerrules [platform-iterable]
export fn iter(buf: IntBuffer) [] -> Mut IntBufferIter => buf {
    return Mut IntBufferIter { buf: buf, at: 0 }
}

// The iterator [iter] answers over an `IntBuffer`: the buffer, borrowed,
// and the next slot.
export struct IntBufferIter : Next<self, Int> canbe Mut {
    // The buffer being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    buf: proj IntBuffer
    // The next slot to emit.
    at: Int
}

// Answers the next slot's value and moves past it, or the end once every
// slot has been emitted.
export fn next(p: Mut IntBufferIter) [] -> Emitted Int | Finished => p: Mut {
    if p.at >= int_size(p.buf) {
        return finished()
    }
    let value = int_get(p.buf, p.at)
    p.at = p.at + 1
    return emitted(value)
}

// [iter] over a `LongBuffer`.
export fn iter(buf: LongBuffer) [] -> Mut LongBufferIter => buf {
    return Mut LongBufferIter { buf: buf, at: 0 }
}

// The iterator [iter] answers over a `LongBuffer`: the buffer, borrowed,
// and the next slot.
export struct LongBufferIter : Next<self, Long> canbe Mut {
    // The buffer being walked, borrowed.
    // @compilerrules [proj-field, proj-infer]
    buf: proj LongBuffer
    // The next slot to emit.
    at: Int
}

// Answers the next slot's value of a `LongBuffer` walk and moves past it,
// or the end.
export fn next(p: Mut LongBufferIter) [] -> Emitted Long | Finished => p: Mut {
    if p.at >= long_size(p.buf) {
        return finished()
    }
    let value = long_get(p.buf, p.at)
    p.at = p.at + 1
    return emitted(value)
}

// The text form of a buffer, its slots in order as a list prints them,
// `[1, 2, 3]`: what `"${buf}"` prints.
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

// The text form of a `LongBuffer`, as [to_str] of an `IntBuffer`.
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
