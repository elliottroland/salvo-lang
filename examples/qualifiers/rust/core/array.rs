use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_sorted::*;
use crate::unions::*;

pub fn iter<T: Clone>(array: &Vec<T>) -> ArrayYield<'_, T> {
    return ArrayYield { items: array, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next<'s, T: Clone>(p: &mut ArrayYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = p.items.get((p.at) as usize);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::U1(emitted(elem.unwrap()));
}
