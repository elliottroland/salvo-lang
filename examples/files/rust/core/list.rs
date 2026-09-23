use crate::core_array::*;
use crate::core_bytes::*;
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

pub fn first<T: Clone>(list: &Vec<T>) -> &T {
    return list.get((0) as i64 as usize).expect("salvo: value is absent at core.list:153:12");
}

pub fn iter__3<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next__5<'s, T: Clone>(p: &mut ListYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = p.items.get((p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::U1(emitted(elem.unwrap()));
}

pub fn sort<T: Clone>(list: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Vec<T> {
    return { let mut __cmp = cmp; let mut __v = list.clone(); __v.sort_by(|__a, __b| __cmp(__a, __b).cmp(&0)); __v };
}

pub fn mut_sort<T: Clone>(list: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Vec<T> {
    return { let mut __cmp = cmp; let mut __v = list.clone(); __v.sort_by(|__a, __b| __cmp(__a, __b).cmp(&0)); __v };
}

pub fn add_sorted<T: Clone>(list: &mut Vec<T>, elem: T, cmp: &mut dyn FnMut(&T, &T) -> i32) {
    { let mut __cmp = cmp; let __e = elem; let __at = list.partition_point(|__x| __cmp(__x, &__e) < 0); list.insert(__at, __e); };
}

pub fn binary_search<T: Clone>(list: &Vec<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Option<i32> {
    return { let mut __cmp = cmp; let __e = elem; let __at = list.partition_point(|__x| __cmp(__x, &__e) < 0); if __at < list.len() && __cmp(&list[__at], &__e) == 0 { Some(__at as i32) } else { None } };
}
