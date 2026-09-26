use crate::collections::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;
use crate::unions::*;

pub fn Idx_qualifies<T: Clone>(index: i32, list: &Vec<T>) -> bool {
    return index >= 0 && index < (list.len() as i32);
}

pub fn NotEq_qualifies(j: i32, i: i32) -> bool {
    return j != i;
}

pub fn get<'a, T: Clone>(list: &'a Vec<T>, index: &i32) -> &'a T {
    return list.get((*index + 0) as i64 as usize).expect("salvo: value is absent at core.list:90:12");
}

pub fn get__loc<T: Clone>(list: &Vec<T>, index: &i32) -> usize {
    return { let __i = (*index + 0) as i64 as usize; if __i < list.len() { Some(__i) } else { None } }.expect("salvo: value is absent at core.list:90:12");
}

pub fn swap<T: Clone>(list: &mut Vec<T>, i: &i32, j: &i32) {
    ignore({ let __i = (*i + 0) as i64 as usize; let __j = (*j + 0) as i64 as usize; Checked { value: if __i < list.len() && __j < list.len() { list.swap(__i, __j); true } else { false } } });
    return;
}

pub fn at<T: Clone>(list: &mut Vec<T>, index: i32) -> Option<&T> {
    return list.get((index) as i64 as usize);
}

pub fn update<T: Clone>(list: &mut Vec<T>, index: &i32, f: &mut impl FnMut(&mut T)) {
    f(list.get_mut((*index) as usize).expect("salvo: value is absent at core.list:133:7"));
    return;
}

pub fn update2<T: Clone>(list: &mut Vec<T>, i: &i32, j: &i32, f: &mut impl FnMut(&mut T, &mut T)) {
    let (__pm0, __pm1) = salvo_pair_mut(&mut list[..], (*i) as usize, (*j) as usize).expect("salvo: value is absent at core.list:146:5");
    f(__pm0, __pm1);
    return;
}

pub fn NonEmpty__List_qualifies<T: Clone>(list: &Vec<T>) -> bool {
    return (list.len() as i32) > 0;
}

pub fn first<T: Clone>(list: &Vec<T>) -> &T {
    return list.get((0) as i64 as usize).expect("salvo: value is absent at core.list:259:12");
}

pub fn iter__3<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next__3<'s, T: Clone>(p: &mut ListYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = p.items.get((p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::U1(emitted(elem.unwrap()));
}

pub fn reversed<T: Clone>(list: &Vec<T>) -> ListRevYield<'_, T> {
    return ListRevYield { items: list, at: (list.len() as i32) - 1 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListRevYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next__4<'s, T: Clone>(p: &mut ListRevYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = p.items.get((p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at - 1;
    return Union2::U1(emitted(elem.unwrap()));
}

pub fn indices<T: Clone>(list: &Vec<T>) -> IdxYield<'_, T> {
    return IdxYield { items: list, at: 0, step: 1 };
}

pub fn rev_indices<T: Clone>(list: &Vec<T>) -> IdxYield<'_, T> {
    return IdxYield { items: list, at: (list.len() as i32) - 1, step: -1 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdxYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
    pub step: i32,
}

pub fn next__5<T: Clone>(p: &mut IdxYield<'_, T>) -> Union2<i32, Finished> {
    if p.at < 0 || p.at >= (p.items.len() as i32) {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut index = p.at;
    p.at = p.at + p.step;
    return Union2::<i32, Finished>::U1(emitted(index));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enumerated<'s, T: Clone + 'static> {
    pub index: i32,
    pub elem: &'s T,
}

pub fn enumerate<T: Clone>(list: &Vec<T>) -> ListEnumYield<'_, T> {
    return ListEnumYield { items: list, at: 0, step: 1 };
}

pub fn enumerate_rev<T: Clone>(list: &Vec<T>) -> ListEnumYield<'_, T> {
    return ListEnumYield { items: list, at: (list.len() as i32) - 1, step: -1 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListEnumYield<'s, T: Clone + 'static> {
    pub items: &'s Vec<T>,
    pub at: i32,
    pub step: i32,
}

pub fn next__6<'a, T: Clone>(p: &mut ListEnumYield<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = p.items.get((p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::<Enumerated<T>, Finished>::U2(finished());
    }
    let mut index = p.at;
    p.at = p.at + p.step;
    return Union2::<Enumerated<T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
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
