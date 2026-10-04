use crate::core_bytes::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::unions::*;

/// [platform-type] The host's `Str`.
pub use crate::platform_core_string::Str;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<Str>(); };

pub fn mut_str(parts: Vec<String>) -> String {
    let mut out = empty_str_platform();
    for part in &parts {
        append_platform(&mut out, part);
    }
    return out;
}

pub fn empty_str_platform() -> String {
    crate::platform_core_string::empty_str()
}

pub fn size_platform(str: &String) -> i32 {
    crate::platform_core_string::size(str)
}

pub fn byte_size_platform(str: &String) -> i64 {
    crate::platform_core_string::byte_size(str)
}

pub fn char_at_platform(str: &String, index: i32) -> Option<char> {
    crate::platform_core_string::char_at(str, index)
}

pub fn iter__9(str: &String) -> StrYield<'_> {
    return StrYield { text: str, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrYield<'s> {
    pub text: &'s String,
    pub at: i32,
}

pub fn next__20(p: &mut StrYield<'_>) -> Union2<char, Finished> {
    let mut chr = char_at_platform(&p.text, p.at);
    if chr.is_none() {
        return Union2::<char, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<char, Finished>::U1(emitted(chr.unwrap()));
}

pub fn split_platform(str: &String, sep: &String) -> Vec<String> {
    crate::platform_core_string::split(str, sep)
}

pub fn index_of_platform(str: &String, needle: &String) -> Option<i32> {
    crate::platform_core_string::index_of(str, needle)
}

pub fn index_of__4(str: &String, needle: &String, from: i32) -> Option<i32> {
    return index_of_from_platform(str, needle, from);
}

pub fn index_of_from_platform(str: &String, needle: &String, from: i32) -> Option<i32> {
    crate::platform_core_string::index_of_from(str, needle, from)
}

pub fn last_index_of_platform(str: &String, needle: &String) -> Option<i32> {
    crate::platform_core_string::last_index_of(str, needle)
}

pub fn replace_platform(str: &String, from: &String, to: &String) -> String {
    crate::platform_core_string::replace(str, from, to)
}

pub fn trim_start_platform(str: &String) -> String {
    crate::platform_core_string::trim_start(str)
}

pub fn trim_end_platform(str: &String) -> String {
    crate::platform_core_string::trim_end(str)
}

pub fn contains_platform(str: &String, needle: &String) -> bool {
    crate::platform_core_string::contains(str, needle)
}

pub fn starts_with_platform(str: &String, prefix: &String) -> bool {
    crate::platform_core_string::starts_with(str, prefix)
}

pub fn ends_with_platform(str: &String, suffix: &String) -> bool {
    crate::platform_core_string::ends_with(str, suffix)
}

pub fn trim_platform(str: &String) -> String {
    crate::platform_core_string::trim(str)
}

pub fn trim_prefix_platform(str: &String, prefix: &String) -> String {
    crate::platform_core_string::trim_prefix(str, prefix)
}

pub fn trim_suffix_platform(str: &String, suffix: &String) -> String {
    crate::platform_core_string::trim_suffix(str, suffix)
}

pub fn substr_platform(str: &String, start: i32, end: i32) -> Option<String> {
    crate::platform_core_string::substr(str, start, end)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: i32,
    pub end: i32,
}

impl crate::wire::__Wire for Span {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.start, out);
        crate::wire::__Wire::__enc(&self.end, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            start: crate::wire::__Wire::__dec(r)?,
            end: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn SpanOf_qualifies(span: &Span, str: &String) -> bool {
    return span.start >= 0 && span.start <= span.end && span.end <= size_platform(str);
}

pub fn substr__2(str: &String, at: &Span) -> String {
    return substr_platform(str, at.start, at.end).expect("salvo: value is absent at core.string:150:12");
}

pub fn to_upper_platform(str: &String) -> String {
    crate::platform_core_string::to_upper(str)
}

pub fn to_lower_platform(str: &String) -> String {
    crate::platform_core_string::to_lower(str)
}

pub fn join_platform(parts: &Vec<String>, sep: &String) -> String {
    crate::platform_core_string::join(parts, sep)
}

pub fn parse_int_platform(str: &String) -> Option<i32> {
    crate::platform_core_string::parse_int(str)
}

pub fn append_platform(str: &mut String, text: &String) {
    crate::platform_core_string::append(str, text)
}

pub fn set_platform(str: &mut String, index: i32, chr: char) -> bool {
    crate::platform_core_string::set(str, index, chr)
}

pub fn clear_platform(str: &mut String) {
    crate::platform_core_string::clear(str)
}

pub fn is_empty__2(str: &String) -> bool {
    return size_platform(str) == 0;
}

pub fn repeat(str: &String, n: i32) -> String {
    let mut out = mut_str(vec![]);
    let mut i = 0;
    while i < n {
        append_platform(&mut out, str);
        i = i + 1;
    }
    return out;
}

pub fn lines(str: &String) -> Vec<String> {
    let mut parts = split_platform(str, &("\n".to_string()));
    if crate::core_list::size_platform(&parts) > 1 && ends_with_platform(str, &("\n".to_string())) {
        let mut _end = remove_back(&mut parts, 1);
    }
    let mut out = vec![];
    for p in &parts {
        crate::core_list::add_platform(&mut out, trim_suffix_platform(p, &("\r".to_string())));
    }
    return out;
}

pub fn split_once(str: &String, sep: &String) -> Option<(String, String)> {
    let mut at = index_of_platform(str, sep);
    if at.is_some() {
        let mut i = at.unwrap();
        let mut before = { let __pick1 = substr_platform(str, 0, i.clone()); if __pick1.is_some() { __pick1.as_ref().unwrap().clone() } else { "".to_string() } };
        let mut after = { let __pick2 = substr_platform(str, i + size_platform(sep), size_platform(str)); if __pick2.is_some() { __pick2.as_ref().unwrap().clone() } else { "".to_string() } };
        return Some((before, after));
    }
    return None;
}

pub fn split_last(str: &String, sep: &String) -> Option<(String, String)> {
    let mut at = last_index_of_platform(str, sep);
    if at.is_some() {
        let mut i = at.unwrap();
        let mut before = { let __pick3 = substr_platform(str, 0, i.clone()); if __pick3.is_some() { __pick3.as_ref().unwrap().clone() } else { "".to_string() } };
        let mut after = { let __pick4 = substr_platform(str, i + size_platform(sep), size_platform(str)); if __pick4.is_some() { __pick4.as_ref().unwrap().clone() } else { "".to_string() } };
        return Some((before, after));
    }
    return None;
}
