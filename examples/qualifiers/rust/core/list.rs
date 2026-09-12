use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_string::*;
use crate::unions::*;

pub fn iter__2<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug)]
pub struct ListYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next__2<'s, T: Clone>(p: &mut ListYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = p.items.get((p.at) as usize);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::U1(emitted(elem.unwrap()));
}
