use crate::core_iterator::*;
use crate::core_list::*;
use crate::unions::*;

pub fn iter__3(str: String) -> StrYield {
    return StrYield { text: str, at: 0 };
}

#[derive(Clone, Debug)]
pub struct StrYield {
    pub text: String,
    pub at: i32,
}

pub fn next__3(p: &mut StrYield) -> Union2<char, Finished> {
    let mut chr = p.text.chars().nth((p.at) as usize);
    if chr.is_none() {
        return Union2::<char, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<char, Finished>::U1(emitted(chr.unwrap()));
}
