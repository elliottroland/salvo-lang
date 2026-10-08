use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;


/// [platform-type] The host's `Bytes`.
pub use crate::platform_core_bytes::Bytes;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<Bytes>(); };
const _: fn() = || { fn __each(x: &Bytes) -> impl Iterator<Item = u8> + '_ { crate::platform_core_bytes::each(x).map(|e| e.clone()) } let _ = __each; };

pub fn bytes_of(mut elems: Vec<u8>) -> crate::core_bytes::Bytes {
    let mut out: crate::core_bytes::Bytes = crate::core_bytes::empty_bytes_platform();
    for mut b in elems.iter().copied() {
        crate::core_bytes::add_platform(&mut out, b);
    }
    return out;
}

pub fn mut_bytes(mut parts: Vec<crate::core_bytes::Bytes>) -> crate::core_bytes::Bytes {
    let mut out: crate::core_bytes::Bytes = crate::core_bytes::empty_bytes_platform();
    for mut part in parts.iter().cloned() {
        crate::core_bytes::append_platform(&mut out, &part);
    }
    return out;
}

pub fn empty_bytes_platform() -> crate::core_bytes::Bytes {
    crate::platform_core_bytes::empty_bytes()
}

pub fn to_bytes_platform(str: &String) -> crate::core_bytes::Bytes {
    crate::platform_core_bytes::to_bytes(str)
}

pub fn str_of_bytes_platform(data: &crate::core_bytes::Bytes) -> Option<String> {
    crate::platform_core_bytes::str_of_bytes(data)
}

pub fn eq_platform(a: &crate::core_bytes::Bytes, b: &crate::core_bytes::Bytes) -> bool {
    crate::platform_core_bytes::eq(a, b)
}

pub fn size_platform(data: &crate::core_bytes::Bytes) -> i32 {
    crate::platform_core_bytes::size(data)
}

pub fn get_platform(data: &crate::core_bytes::Bytes, mut index: i32) -> Option<u8> {
    crate::platform_core_bytes::get(data, index)
}

pub fn slice_platform(data: &crate::core_bytes::Bytes, mut start: i32, mut end: i32) -> Option<crate::core_bytes::Bytes> {
    crate::platform_core_bytes::slice(data, start, end)
}

pub fn index_of_platform(data: &crate::core_bytes::Bytes, mut byte: u8) -> Option<i32> {
    crate::platform_core_bytes::index_of(data, byte)
}

pub fn add_platform(data: &mut crate::core_bytes::Bytes, mut byte: u8) {
    crate::platform_core_bytes::add(data, byte)
}

pub fn append_platform(data: &mut crate::core_bytes::Bytes, more: &crate::core_bytes::Bytes) {
    crate::platform_core_bytes::append(data, more)
}

pub fn set_platform(data: &mut crate::core_bytes::Bytes, mut index: i32, mut byte: u8) -> bool {
    crate::platform_core_bytes::set(data, index, byte)
}

pub fn clear_platform(data: &mut crate::core_bytes::Bytes) {
    crate::platform_core_bytes::clear(data)
}

pub fn to_str_platform(data: &crate::core_bytes::Bytes) -> String {
    crate::platform_core_bytes::to_str(data)
}

pub fn to_hex_platform(data: &crate::core_bytes::Bytes) -> String {
    crate::platform_core_bytes::to_hex(data)
}

pub fn iter(data: &crate::core_bytes::Bytes) -> crate::core_bytes::BytesYield<'_> {
    return crate::core_bytes::BytesYield { data: data, at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct BytesYield<'s> {
    pub data: &'s crate::core_bytes::Bytes,
    pub at: i32,
}

pub fn next(p: &mut crate::core_bytes::BytesYield<'_>) -> crate::unions::Union2<u8, crate::core_iterator::Finished> {
    let mut b: Option<u8> = crate::core_bytes::get_platform(p.data, p.at);
    if b.is_none() {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut b_1 = b.unwrap();
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<u8>(b_1));
}
