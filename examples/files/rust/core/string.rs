use crate::core_iterator::Finished;
use crate::core_list::add_platform;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_list::remove_back;


/// [platform-type] The host's `Str`.
pub use crate::platform_core_string::Str;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<Str>(); };
const _: fn() = || { fn __each(x: &Str) -> impl Iterator<Item = char> + '_ { crate::platform_core_string::each(x).map(|e| e.clone()) } let _ = __each; };

pub fn mut_str(mut parts: Vec<String>) -> String {
    let mut out: String = crate::core_string::empty_str_platform();
    for mut part in parts.iter().cloned() {
        crate::core_string::append_platform(&mut out, &part);
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

pub fn char_at_platform(str: &String, mut index: i32) -> Option<char> {
    crate::platform_core_string::char_at(str, index)
}

pub fn iter(str: &String) -> crate::core_string::StrYield<'_> {
    return crate::core_string::StrYield { text: str, at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrYield<'s> {
    pub text: &'s String,
    pub at: i32,
}

pub fn next(p: &mut crate::core_string::StrYield<'_>) -> crate::unions::Union2<char, crate::core_iterator::Finished> {
    let mut chr: Option<char> = crate::core_string::char_at_platform(p.text, p.at);
    if chr.is_none() {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut chr_1 = chr.unwrap();
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<char>(chr_1));
}

pub fn split_platform(str: &String, sep: &String) -> Vec<String> {
    crate::platform_core_string::split(str, sep)
}

pub fn index_of_platform(str: &String, needle: &String) -> Option<i32> {
    crate::platform_core_string::index_of(str, needle)
}

pub fn index_of(str: &String, needle: &String, mut from: i32) -> Option<i32> {
    return crate::core_string::index_of_from_platform(str, needle, from);
}

pub fn index_of_from_platform(str: &String, needle: &String, mut from: i32) -> Option<i32> {
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

pub fn substr_platform(str: &String, mut start: i32, mut end: i32) -> Option<String> {
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

pub fn SpanOf__Span_qualifies(span: &crate::core_string::Span, str: &String) -> bool {
    return (((span.start >= 0i32) && (span.start <= span.end)) && (span.end <= crate::core_string::size_platform(str)));
}

pub fn substr(str: &String, at: &crate::core_string::Span) -> String {
    return {
        let mut __nn_1: Option<String> = crate::core_string::substr_platform(str, at.start, at.end);
        if __nn_1.is_none() {
            panic!("salvo: value is absent at core.string:150:12");
        } else {
            let mut __some_2 = __nn_1.unwrap();
            __some_2
        }
    };
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

pub fn set_platform(str: &mut String, mut index: i32, mut chr: char) -> bool {
    crate::platform_core_string::set(str, index, chr)
}

pub fn clear_platform(str: &mut String) {
    crate::platform_core_string::clear(str)
}

pub fn is_empty(str: &String) -> bool {
    return (crate::core_string::size_platform(str) == 0i32);
}

pub fn repeat(str: &String, mut n: i32) -> String {
    let mut out: String = crate::core_string::mut_str(vec![]);
    let mut i: i32 = 0i32;
    loop {
        if !((i < n)) {
            break;
        };
        crate::core_string::append_platform(&mut out, str);
        i = i32::wrapping_add(i, 1i32);
    }
    return out;
}

pub fn lines(str: &String) -> Vec<String> {
    let mut parts: Vec<String> = crate::core_string::split_platform(str, &String::from("\n"));
    if ((crate::core_list::size_platform::<String>(&parts) > 1i32) && crate::core_string::ends_with_platform(str, &String::from("\n"))) {
        let mut _end: Vec<String> = crate::core_list::remove_back::<String>(&mut parts, 1i32);
    };
    let mut out: Vec<String> = vec![];
    for mut p in parts.iter() {
        crate::core_list::add_platform::<String>(&mut out, crate::core_string::trim_suffix_platform(p, &String::from("\r")));
    }
    return out;
}

pub fn split_once(str: &String, sep: &String) -> Option<(String, String)> {
    let mut at: Option<i32> = crate::core_string::index_of_platform(str, sep);
    if at.is_some() {
        let mut i = at.unwrap();
        let mut before: String = {
            let mut __elv_1: Option<String> = crate::core_string::substr_platform(str, 0i32, i);
            if __elv_1.is_none() {
                String::from("")
            } else {
                let mut __some_2 = __elv_1.unwrap();
                __some_2
            }
        };
        let mut after: String = {
            let mut __elv_3: Option<String> = crate::core_string::substr_platform(str, i32::wrapping_add(i, crate::core_string::size_platform(sep)), crate::core_string::size_platform(str));
            if __elv_3.is_none() {
                String::from("")
            } else {
                let mut __some_4 = __elv_3.unwrap();
                __some_4
            }
        };
        return Some((before.clone(), after.clone()));
    };
    return None;
}

pub fn split_last(str: &String, sep: &String) -> Option<(String, String)> {
    let mut at: Option<i32> = crate::core_string::last_index_of_platform(str, sep);
    if at.is_some() {
        let mut i = at.unwrap();
        let mut before: String = {
            let mut __elv_1: Option<String> = crate::core_string::substr_platform(str, 0i32, i);
            if __elv_1.is_none() {
                String::from("")
            } else {
                let mut __some_2 = __elv_1.unwrap();
                __some_2
            }
        };
        let mut after: String = {
            let mut __elv_3: Option<String> = crate::core_string::substr_platform(str, i32::wrapping_add(i, crate::core_string::size_platform(sep)), crate::core_string::size_platform(str));
            if __elv_3.is_none() {
                String::from("")
            } else {
                let mut __some_4 = __elv_3.unwrap();
                __some_4
            }
        };
        return Some((before.clone(), after.clone()));
    };
    return None;
}
