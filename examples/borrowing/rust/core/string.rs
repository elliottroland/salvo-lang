use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::unions::*;

pub fn iter__9(str: &String) -> StrYield<'_> {
    return StrYield { text: str, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrYield<'s> {
    pub text: &'s String,
    pub at: i32,
}

pub fn next__12(p: &mut StrYield<'_>) -> Union2<char, Finished> {
    let mut chr = p.text.chars().nth((p.at) as i64 as usize);
    if chr.is_none() {
        return Union2::<char, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<char, Finished>::U1(emitted(chr.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: i32,
    pub end: i32,
}

pub fn SpanOf_qualifies(span: &Span, str: &String) -> bool {
    return span.start >= 0 && span.start <= span.end && span.end <= (str.chars().count() as i32);
}

pub fn substr(str: &String, at: &Span) -> String {
    return { let __s = &str[..]; let __i = at.start; let __j = at.end; let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } }.expect("salvo: value is absent at core.string:116:12");
}
