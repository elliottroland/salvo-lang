use crate::core_iterator::*;
use crate::core_list::*;
use crate::unions::*;

pub fn iter__8(str: &String) -> StrYield<'_> {
    return StrYield { text: str, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrYield<'s> {
    pub text: &'s String,
    pub at: i32,
}

pub fn next__8(p: &mut StrYield<'_>) -> Union2<char, Finished> {
    let mut chr = p.text.chars().nth((p.at) as usize);
    if chr.is_none() {
        return Union2::<char, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<char, Finished>::U1(emitted(chr.unwrap()));
}
