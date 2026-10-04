use crate::core_deque::*;
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

pub fn next__20(p: &mut StrYield<'_>) -> Union2<char, Finished> {
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
    return span.start >= 0 && span.start <= span.end && span.end <= (str.chars().count() as i32);
}

pub fn substr(str: &String, at: &Span) -> String {
    return { let __s = &str[..]; let __i = at.start; let __j = at.end; let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } }.expect("salvo: value is absent at core.string:134:12");
}

pub fn is_empty__2(str: &String) -> bool {
    return (str.chars().count() as i32) == 0;
}

pub fn repeat(str: &String, n: i32) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < n {
        out.push_str(&str[..]);
        i = i + 1;
    }
    return out;
}

pub fn lines(str: &String) -> Vec<String> {
    let mut parts = str.split(&"\n".to_string()[..]).map(|__p| __p.to_string()).collect::<Vec<String>>();
    if ((parts.len() as i32) > 1 && str.ends_with(&"\n".to_string()[..])) {
        let mut _end = remove_back(&mut parts, 1);
    }
    let mut out = vec![];
    for p in &parts {
        out.push({ let __s = &p[..]; __s.strip_suffix(&"\r".to_string()[..]).unwrap_or(__s).to_string() });
    }
    return out;
}

pub fn split_once(str: &String, sep: &String) -> Option<(String, String)> {
    let mut at = { let __s = &str[..]; __s.find(&sep[..]).map(|__b| __s[..__b].chars().count() as i32) };
    if at.is_some() {
        let mut i = at.unwrap();
        let mut before = { let __pick1 = { let __s = &str[..]; let __i = 0; let __j = i.clone(); let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } }; if __pick1.is_some() { __pick1.as_ref().unwrap().clone() } else { "".to_string() } };
        let mut after = { let __pick2 = { let __s = &str[..]; let __i = i + (sep.chars().count() as i32); let __j = (str.chars().count() as i32); let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } }; if __pick2.is_some() { __pick2.as_ref().unwrap().clone() } else { "".to_string() } };
        return Some((before, after));
    }
    return None;
}

pub fn split_last(str: &String, sep: &String) -> Option<(String, String)> {
    let mut at = { let __s = &str[..]; __s.rfind(&sep[..]).map(|__b| __s[..__b].chars().count() as i32) };
    if at.is_some() {
        let mut i = at.unwrap();
        let mut before = { let __pick3 = { let __s = &str[..]; let __i = 0; let __j = i.clone(); let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } }; if __pick3.is_some() { __pick3.as_ref().unwrap().clone() } else { "".to_string() } };
        let mut after = { let __pick4 = { let __s = &str[..]; let __i = i + (sep.chars().count() as i32); let __j = (str.chars().count() as i32); let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } }; if __pick4.is_some() { __pick4.as_ref().unwrap().clone() } else { "".to_string() } };
        return Some((before, after));
    }
    return None;
}
