use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_map::*;
use crate::core_nonempty::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn NonEmpty__List_qualifies<T: Clone>(list: &Vec<T>) -> bool {
    return (list.len() as i32) > 0;
}

pub fn non_empty_list<T: Clone>(first: &T, rest: Vec<T>) -> Vec<T> {
    return { let mut __v = Vec::new(); __v.push(first.clone()); __v.extend(rest.iter().cloned()); __v };
}

pub fn first<T: Clone>(list: &Vec<T>) -> &T {
    return list.get((0) as usize).unwrap();
}

pub fn iter__2<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
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
