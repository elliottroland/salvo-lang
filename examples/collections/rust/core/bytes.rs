use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn iter__2(data: &Vec<u8>) -> BytesYield<'_> {
    return BytesYield { data: data, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct BytesYield<'s> {
    pub data: &'s Vec<u8>,
    pub at: i32,
}

pub fn next__2(p: &mut BytesYield<'_>) -> Union2<u8, Finished> {
    let mut b = p.data.get((p.at) as usize).copied();
    if b.is_none() {
        return Union2::<u8, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<u8, Finished>::U1(emitted(b.unwrap()));
}
