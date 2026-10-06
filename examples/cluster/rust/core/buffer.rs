use crate::unions::*;
use crate::core_index::Idx__Int_qualifies;
use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_list::at;
use crate::core_range::next;
use crate::core_range::range__Int;
use crate::core_string::mut_str;

/// [platform-type] The host's `IntBuffer`.
pub use crate::platform_core_buffer::IntBuffer;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<IntBuffer>(); };
const _: fn() = || { fn __each(x: &IntBuffer) -> impl Iterator<Item = i32> + '_ { crate::platform_core_buffer::each_int_buffer(x).map(|e| e.clone()) } let _ = __each; };

/// [platform-type] The host's `LongBuffer`.
pub use crate::platform_core_buffer::LongBuffer;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<LongBuffer>(); };
const _: fn() = || { fn __each(x: &LongBuffer) -> impl Iterator<Item = i64> + '_ { crate::platform_core_buffer::each_long_buffer(x).map(|e| e.clone()) } let _ = __each; };

pub fn int_buffer(size: i32, fill: i32) -> IntBuffer {
    return new_int_platform(size, fill);
}

pub fn long_buffer(size: i32, fill: i64) -> LongBuffer {
    return new_long_platform(size, fill);
}

pub fn size__IntBuffer(buf: &IntBuffer) -> i32 {
    return int_size_platform(buf);
}

pub fn size__LongBuffer(buf: &LongBuffer) -> i32 {
    return long_size_platform(buf);
}

pub fn get__IntBuffer_Int(buf: &IntBuffer, index: i32) -> Option<i32> {
    if Idx__Int_qualifies(index, &*buf, &mut |__i0| size__IntBuffer(&__i0)) {
        return Some(int_get_platform(buf, i32::wrapping_add(index, 0)));
    }
    return None;
}

pub fn get__LongBuffer_Int(buf: &LongBuffer, index: i32) -> Option<i64> {
    if Idx__Int_qualifies(index, &*buf, &mut |__i0| size__LongBuffer(&__i0)) {
        return Some(long_get_platform(buf, i32::wrapping_add(index, 0)));
    }
    return None;
}

pub fn get__IntBuffer_IdxInt(buf: &IntBuffer, index: &i32) -> i32 {
    return int_get_platform(buf, i32::wrapping_add(*index, 0));
}

pub fn get__LongBuffer_IdxInt(buf: &LongBuffer, index: &i32) -> i64 {
    return long_get_platform(buf, i32::wrapping_add(*index, 0));
}

pub fn replace__IntBuffer_Int_Int(buf: &mut IntBuffer, index: &i32, value: i32) -> i32 {
    return int_replace_platform(buf, i32::wrapping_add(*index, 0), value);
}

pub fn replace__LongBuffer_Int_Long(buf: &mut LongBuffer, index: &i32, value: i64) -> i64 {
    return long_replace_platform(buf, i32::wrapping_add(*index, 0), value);
}

pub fn clear__IntBuffer_Int(buf: &mut IntBuffer, fill: i32) {
    int_clear_platform(buf, fill);
}

pub fn clear__LongBuffer_Long(buf: &mut LongBuffer, fill: i64) {
    long_clear_platform(buf, fill);
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_iter_IntBuffer<'s> {
    pub buf: &'s IntBuffer,
    pub at: i32,
}

pub fn iter__IntBuffer(buf: &IntBuffer) -> __Iter_iter_IntBuffer<'_> {
    return __Iter_iter_IntBuffer { buf: buf, at: 0 };
}

pub fn next__Iter_iter_IntBuffer(__p: &mut __Iter_iter_IntBuffer<'_>) -> Union2<i32, Finished> {
    if __p.at >= int_size_platform(&__p.buf) {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut value = int_get_platform(&__p.buf, __p.at);
    __p.at = i32::wrapping_add(__p.at, 1);
    return Union2::<i32, Finished>::U1(emitted(value));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_iter_LongBuffer<'s> {
    pub buf: &'s LongBuffer,
    pub at: i32,
}

pub fn iter__LongBuffer(buf: &LongBuffer) -> __Iter_iter_LongBuffer<'_> {
    return __Iter_iter_LongBuffer { buf: buf, at: 0 };
}

pub fn next__Iter_iter_LongBuffer(__p: &mut __Iter_iter_LongBuffer<'_>) -> Union2<i64, Finished> {
    if __p.at >= long_size_platform(&__p.buf) {
        return Union2::<i64, Finished>::U2(finished());
    }
    let mut value = long_get_platform(&__p.buf, __p.at);
    __p.at = i32::wrapping_add(__p.at, 1);
    return Union2::<i64, Finished>::U1(emitted(value));
}

pub fn to_str__IntBuffer(buf: &IntBuffer) -> String {
    let mut out: String = mut_str(vec!["[".to_string()]);
    let mut __loop1_pass = range__Int(size__IntBuffer(buf));
    while let Union2::U1(mut i) = next(&mut __loop1_pass) {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(format!("{}", int_get_platform(buf, i))));
    }
    crate::core_string::append_platform(&mut out, &("]".to_string()));
    return out;
}

pub fn to_str__LongBuffer(buf: &LongBuffer) -> String {
    let mut out: String = mut_str(vec!["[".to_string()]);
    let mut __loop2_pass = range__Int(size__LongBuffer(buf));
    while let Union2::U1(mut i) = next(&mut __loop2_pass) {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(format!("{}", long_get_platform(buf, i))));
    }
    crate::core_string::append_platform(&mut out, &("]".to_string()));
    return out;
}

pub fn new_int_platform(size: i32, fill: i32) -> IntBuffer {
    crate::platform_core_buffer::new_int(size, fill)
}

pub fn int_size_platform(buf: &IntBuffer) -> i32 {
    crate::platform_core_buffer::int_size(buf)
}

pub fn int_get_platform(buf: &IntBuffer, index: i32) -> i32 {
    crate::platform_core_buffer::int_get(buf, index)
}

pub fn int_replace_platform(buf: &mut IntBuffer, index: i32, value: i32) -> i32 {
    crate::platform_core_buffer::int_replace(buf, index, value)
}

pub fn int_clear_platform(buf: &mut IntBuffer, fill: i32) {
    crate::platform_core_buffer::int_clear(buf, fill)
}

pub fn new_long_platform(size: i32, fill: i64) -> LongBuffer {
    crate::platform_core_buffer::new_long(size, fill)
}

pub fn long_size_platform(buf: &LongBuffer) -> i32 {
    crate::platform_core_buffer::long_size(buf)
}

pub fn long_get_platform(buf: &LongBuffer, index: i32) -> i64 {
    crate::platform_core_buffer::long_get(buf, index)
}

pub fn long_replace_platform(buf: &mut LongBuffer, index: i32, value: i64) -> i64 {
    crate::platform_core_buffer::long_replace(buf, index, value)
}

pub fn long_clear_platform(buf: &mut LongBuffer, fill: i64) {
    crate::platform_core_buffer::long_clear(buf, fill)
}
