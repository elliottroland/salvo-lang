// [buffer-type] std's fixed-length runs, for `core.buffer`'s `IntBuffer` and
// `LongBuffer` (ROADMAP §0j 6g): a `Vec`, no class, no boxing. `Mut` is the
// same type, passed as `&mut` [type-canbe-mut]. Callers prove every index
// (`Idx`), so the reads and writes index directly.

pub type IntBuffer = Vec<i32>;
pub type MutIntBuffer = Vec<i32>;
pub type LongBuffer = Vec<i64>;
pub type MutLongBuffer = Vec<i64>;

// [platform-iterable] The host's loop over a buffer: its slots.
pub fn each_int_buffer(buf: &IntBuffer) -> std::iter::Copied<std::slice::Iter<'_, i32>> {
    buf.iter().copied()
}

pub fn each_long_buffer(buf: &LongBuffer) -> std::iter::Copied<std::slice::Iter<'_, i64>> {
    buf.iter().copied()
}

pub fn new_int(size: i32, fill: i32) -> IntBuffer {
    vec![fill; size.max(0) as usize]
}

pub fn int_size(buf: &IntBuffer) -> i32 {
    buf.len() as i32
}

pub fn int_get(buf: &IntBuffer, index: i32) -> i32 {
    buf[index as usize]
}

pub fn int_replace(buf: &mut IntBuffer, index: i32, value: i32) -> i32 {
    std::mem::replace(&mut buf[index as usize], value)
}

pub fn int_clear(buf: &mut IntBuffer, fill: i32) {
    buf.fill(fill);
}

pub fn new_long(size: i32, fill: i64) -> LongBuffer {
    vec![fill; size.max(0) as usize]
}

pub fn long_size(buf: &LongBuffer) -> i32 {
    buf.len() as i32
}

pub fn long_get(buf: &LongBuffer, index: i32) -> i64 {
    buf[index as usize]
}

pub fn long_replace(buf: &mut LongBuffer, index: i32, value: i64) -> i64 {
    std::mem::replace(&mut buf[index as usize], value)
}

pub fn long_clear(buf: &mut LongBuffer, fill: i64) {
    buf.fill(fill);
}
