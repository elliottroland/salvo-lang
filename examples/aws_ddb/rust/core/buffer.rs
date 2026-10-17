use crate::core_iterator::Finished;
use crate::core_index::Idx__Int_qualifies;
use crate::core_string::append_platform;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_string::mut_str;
use crate::core_range::next;
use crate::core_range::range__Int;


/// [platform-type] The host's `IntBuffer`.
pub use crate::platform_core_buffer::IntBuffer;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<IntBuffer>(); };
const _: fn() = || { fn __each(x: &IntBuffer) -> impl Iterator<Item = i32> + '_ { crate::platform_core_buffer::each_int_buffer(x).map(|e| e.clone()) } let _ = __each; };

/// [platform-type] The host's `LongBuffer`.
pub use crate::platform_core_buffer::LongBuffer;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<LongBuffer>(); };
const _: fn() = || { fn __each(x: &LongBuffer) -> impl Iterator<Item = i64> + '_ { crate::platform_core_buffer::each_long_buffer(x).map(|e| e.clone()) } let _ = __each; };

pub fn int_buffer(mut size: i32, mut fill: i32) -> crate::core_buffer::IntBuffer {
    return crate::core_buffer::new_int_platform(size, fill);
}

pub fn long_buffer(mut size: i32, mut fill: i64) -> crate::core_buffer::LongBuffer {
    return crate::core_buffer::new_long_platform(size, fill);
}

pub fn size__IntBuffer(buf: &crate::core_buffer::IntBuffer) -> i32 {
    return crate::core_buffer::int_size_platform(buf);
}

pub fn size__LongBuffer(buf: &crate::core_buffer::LongBuffer) -> i32 {
    return crate::core_buffer::long_size_platform(buf);
}

pub fn get__IntBuffer_Int(buf: &crate::core_buffer::IntBuffer, mut index: i32) -> Option<i32> {
    if crate::core_index::Idx__Int_qualifies(index, buf, &mut |__a0| crate::core_buffer::size__IntBuffer(__a0)) {
        return Some(crate::core_buffer::int_get_platform(buf, i32::wrapping_add(index, 0i32)));
    };
    return None;
}

pub fn get__LongBuffer_Int(buf: &crate::core_buffer::LongBuffer, mut index: i32) -> Option<i64> {
    if crate::core_index::Idx__Int_qualifies(index, buf, &mut |__a0| crate::core_buffer::size__LongBuffer(__a0)) {
        return Some(crate::core_buffer::long_get_platform(buf, i32::wrapping_add(index, 0i32)));
    };
    return None;
}

pub fn get__IntBuffer_IdxInt(buf: &crate::core_buffer::IntBuffer, mut index: i32) -> i32 {
    return crate::core_buffer::int_get_platform(buf, i32::wrapping_add(index, 0i32));
}

pub fn get__LongBuffer_IdxInt(buf: &crate::core_buffer::LongBuffer, mut index: i32) -> i64 {
    return crate::core_buffer::long_get_platform(buf, i32::wrapping_add(index, 0i32));
}

pub fn replace__IntBuffer_Int_Int(buf: &mut crate::core_buffer::IntBuffer, mut index: i32, mut value: i32) -> i32 {
    return crate::core_buffer::int_replace_platform(&mut *buf, i32::wrapping_add(index, 0i32), value);
}

pub fn replace__LongBuffer_Int_Long(buf: &mut crate::core_buffer::LongBuffer, mut index: i32, mut value: i64) -> i64 {
    return crate::core_buffer::long_replace_platform(&mut *buf, i32::wrapping_add(index, 0i32), value);
}

pub fn clear__IntBuffer_Int(buf: &mut crate::core_buffer::IntBuffer, mut fill: i32) {
    crate::core_buffer::int_clear_platform(&mut *buf, fill);
}

pub fn clear__LongBuffer_Long(buf: &mut crate::core_buffer::LongBuffer, mut fill: i64) {
    crate::core_buffer::long_clear_platform(&mut *buf, fill);
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_iter_IntBuffer<'s> {
    pub buf: &'s crate::core_buffer::IntBuffer,
    pub at: i32,
}

pub fn iter__IntBuffer(buf: &crate::core_buffer::IntBuffer) -> crate::core_buffer::__Iter_iter_IntBuffer<'_> {
    return crate::core_buffer::__Iter_iter_IntBuffer { buf: buf, at: 0i32 };
}

pub fn next__Iter_iter_IntBuffer(__p: &mut crate::core_buffer::__Iter_iter_IntBuffer<'_>) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (__p.at >= crate::core_buffer::int_size_platform(__p.buf)) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut value: i32 = crate::core_buffer::int_get_platform(__p.buf, __p.at);
    __p.at = i32::wrapping_add(__p.at, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(value));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_iter_LongBuffer<'s> {
    pub buf: &'s crate::core_buffer::LongBuffer,
    pub at: i32,
}

pub fn iter__LongBuffer(buf: &crate::core_buffer::LongBuffer) -> crate::core_buffer::__Iter_iter_LongBuffer<'_> {
    return crate::core_buffer::__Iter_iter_LongBuffer { buf: buf, at: 0i32 };
}

pub fn next__Iter_iter_LongBuffer(__p: &mut crate::core_buffer::__Iter_iter_LongBuffer<'_>) -> crate::unions::Union2<i64, crate::core_iterator::Finished> {
    if (__p.at >= crate::core_buffer::long_size_platform(__p.buf)) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut value: i64 = crate::core_buffer::long_get_platform(__p.buf, __p.at);
    __p.at = i32::wrapping_add(__p.at, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i64>(value));
}

pub fn to_str__IntBuffer(buf: &crate::core_buffer::IntBuffer) -> String {
    let mut out: String = crate::core_string::mut_str(vec![String::from("[")]);
    let mut __pass_1: crate::core_range::__Iter_range_Int_Int_Int = crate::core_range::range__Int(crate::core_buffer::size__IntBuffer(buf));
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::core_range::next(&mut __pass_1);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut i: i32 = __emitted_3;
            if (i > 0i32) {
                crate::core_string::append_platform(&mut out, &String::from(", "));
            };
            crate::core_string::append_platform(&mut out, &format!("{}", crate::core_buffer::int_get_platform(buf, i)));
        } else {
            break;
        };
    }
    crate::core_string::append_platform(&mut out, &String::from("]"));
    return out;
}

pub fn to_str__LongBuffer(buf: &crate::core_buffer::LongBuffer) -> String {
    let mut out: String = crate::core_string::mut_str(vec![String::from("[")]);
    let mut __pass_1: crate::core_range::__Iter_range_Int_Int_Int = crate::core_range::range__Int(crate::core_buffer::size__LongBuffer(buf));
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::core_range::next(&mut __pass_1);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut i: i32 = __emitted_3;
            if (i > 0i32) {
                crate::core_string::append_platform(&mut out, &String::from(", "));
            };
            crate::core_string::append_platform(&mut out, &format!("{}", crate::core_buffer::long_get_platform(buf, i)));
        } else {
            break;
        };
    }
    crate::core_string::append_platform(&mut out, &String::from("]"));
    return out;
}

pub fn new_int_platform(mut size: i32, mut fill: i32) -> crate::core_buffer::IntBuffer {
    crate::platform_core_buffer::new_int(size, fill)
}

pub fn int_size_platform(buf: &crate::core_buffer::IntBuffer) -> i32 {
    crate::platform_core_buffer::int_size(buf)
}

pub fn int_get_platform(buf: &crate::core_buffer::IntBuffer, mut index: i32) -> i32 {
    crate::platform_core_buffer::int_get(buf, index)
}

pub fn int_replace_platform(buf: &mut crate::core_buffer::IntBuffer, mut index: i32, mut value: i32) -> i32 {
    crate::platform_core_buffer::int_replace(buf, index, value)
}

pub fn int_clear_platform(buf: &mut crate::core_buffer::IntBuffer, mut fill: i32) {
    crate::platform_core_buffer::int_clear(buf, fill)
}

pub fn new_long_platform(mut size: i32, mut fill: i64) -> crate::core_buffer::LongBuffer {
    crate::platform_core_buffer::new_long(size, fill)
}

pub fn long_size_platform(buf: &crate::core_buffer::LongBuffer) -> i32 {
    crate::platform_core_buffer::long_size(buf)
}

pub fn long_get_platform(buf: &crate::core_buffer::LongBuffer, mut index: i32) -> i64 {
    crate::platform_core_buffer::long_get(buf, index)
}

pub fn long_replace_platform(buf: &mut crate::core_buffer::LongBuffer, mut index: i32, mut value: i64) -> i64 {
    crate::platform_core_buffer::long_replace(buf, index, value)
}

pub fn long_clear_platform(buf: &mut crate::core_buffer::LongBuffer, mut fill: i64) {
    crate::platform_core_buffer::long_clear(buf, fill)
}
