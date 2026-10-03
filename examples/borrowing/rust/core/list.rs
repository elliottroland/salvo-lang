use crate::collections::*;
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

pub fn at__loc<T: Clone>(list: &Vec<T>, index: i32) -> Option<usize> {
    return { let __i = (index) as i64 as usize; if __i < list.len() { Some(__i) } else { None } };
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
    return list.get((0) as i64 as usize).expect("salvo: value is absent at core.list:273:12");
}

pub fn last<T: Clone>(list: &Vec<T>) -> Option<&T> {
    return list.get(((list.len() as i32) - 1) as i64 as usize);
}

pub fn is_empty<T: Clone>(list: &Vec<T>) -> bool {
    return (list.len() as i32) == 0;
}

pub fn remove_front<T: Clone>(list: &mut Vec<T>, n: i32) -> Vec<T> {
    return list.salvo_remove_range(0, n);
}

pub fn remove_back<T: Clone>(list: &mut Vec<T>, n: i32) -> Vec<T> {
    let mut at = (list.len() as i32) - n;
    if at < 0 {
        at = 0;
    }
    return list.salvo_remove_range(at, (list.len() as i32));
}

pub fn remove_front_while<T: Clone>(list: &mut Vec<T>, keep: &mut impl FnMut(&T) -> bool) -> Vec<T> {
    let mut n = 0;
    while n < (list.len() as i32) && keep(&(list.get((n) as i64 as usize).expect("salvo: value is absent at core.list:316:34"))) {
        n = n + 1;
    }
    return list.salvo_remove_range(0, n);
}

pub fn remove_back_while<T: Clone>(list: &mut Vec<T>, keep: &mut impl FnMut(&T) -> bool) -> Vec<T> {
    let mut at = (list.len() as i32);
    while at > 0 && keep(&(list.get((at - 1) as i64 as usize).expect("salvo: value is absent at core.list:328:26"))) {
        at = at - 1;
    }
    return list.salvo_remove_range(at, (list.len() as i32));
}

pub fn sub_list<T: Clone>(list: &Vec<T>, from: i32, to: i32, copy: &mut dyn FnMut(&T) -> T) -> Vec<T> {
    let mut out = vec![];
    let mut i = from;
    if i < 0 {
        i = 0;
    }
    while i < to && i < (list.len() as i32) {
        out.push(copy(&list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:346:23")));
        i = i + 1;
    }
    return out;
}

pub fn find_first<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> Option<i32> {
    let mut i = 0;
    while i < (list.len() as i32) {
        if pick(&(list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:356:17"))) {
            return Some(i);
        }
        i = i + 1;
    }
    return None;
}

pub fn find_last<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> Option<i32> {
    let mut i = (list.len() as i32) - 1;
    while i >= 0 {
        if pick(&(list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:368:17"))) {
            return Some(i);
        }
        i = i - 1;
    }
    return None;
}

pub fn index_of<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> Option<i32> {
    let mut i = 0;
    while i < (list.len() as i32) {
        if eq(&list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:380:15"), elem) {
            return Some(i);
        }
        i = i + 1;
    }
    return None;
}

pub fn last_index_of<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> Option<i32> {
    let mut i = (list.len() as i32) - 1;
    while i >= 0 {
        if eq(&list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:392:15"), elem) {
            return Some(i);
        }
        i = i - 1;
    }
    return None;
}

pub fn contains<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    let mut i = 0;
    while i < (list.len() as i32) {
        if eq(&list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:404:15"), elem) {
            return true;
        }
        i = i + 1;
    }
    return false;
}

pub fn any<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> bool {
    return !(find_first(list, pick).is_none());
}

pub fn all<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> bool {
    let mut i = 0;
    while i < (list.len() as i32) {
        if !pick(&(list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:421:18"))) {
            return false;
        }
        i = i + 1;
    }
    return true;
}

pub fn count<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> i32 {
    let mut n = 0;
    let mut i = 0;
    while i < (list.len() as i32) {
        if pick(&(list.get((i) as i64 as usize).expect("salvo: value is absent at core.list:434:17"))) {
            n = n + 1;
        }
        i = i + 1;
    }
    return n;
}

pub fn partition<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool, copy: &mut dyn FnMut(&T) -> T) -> (Vec<T>, Vec<T>) {
    let mut yes = vec![];
    let mut no = vec![];
    let mut i = 0;
    while i < (list.len() as i32) {
        let mut x = list.get((i) as i64 as usize).unwrap();
        if pick(x) {
            yes.push(copy(x));
        } else {
            no.push(copy(x));
        }
        i = i + 1;
    }
    return (yes, no);
}

pub fn reverse<T: Clone>(list: &mut Vec<T>) {
    let mut i = 0;
    let mut j = (list.len() as i32) - 1;
    while i < j {
        ignore({ let __i = (i.clone()) as i64 as usize; let __j = (j.clone()) as i64 as usize; Checked { value: if __i < list.len() && __j < list.len() { list.swap(__i, __j); true } else { false } } });
        i = i + 1;
        j = j - 1;
    }
}

pub fn iter__4<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListYield<'s, T: Clone> {
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

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_reversed_List<'s, T: Clone> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn reversed__2<T: Clone>(list: &Vec<T>) -> __Iter_reversed_List<'_, T> {
    return __Iter_reversed_List { list: list, at: (list.len() as i32) - 1 };
}

pub fn next__6<'s, T: Clone>(__p: &mut __Iter_reversed_List<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = __p.list.get((__p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    __p.at = __p.at - 1;
    return Union2::U1(emitted(elem.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_indices_List<'s, T: Clone> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn indices<T: Clone>(list: &Vec<T>) -> __Iter_indices_List<'_, T> {
    return __Iter_indices_List { list: list, at: 0 };
}

pub fn next__7<T: Clone>(__p: &mut __Iter_indices_List<'_, T>) -> Union2<i32, Finished> {
    if __p.at >= (__p.list.len() as i32) {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at + 1;
    return Union2::<i32, Finished>::U1(emitted(index));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_rev_indices_List<'s, T: Clone> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn rev_indices<T: Clone>(list: &Vec<T>) -> __Iter_rev_indices_List<'_, T> {
    return __Iter_rev_indices_List { list: list, at: (list.len() as i32) - 1 };
}

pub fn next__8<T: Clone>(__p: &mut __Iter_rev_indices_List<'_, T>) -> Union2<i32, Finished> {
    if __p.at < 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at - 1;
    return Union2::<i32, Finished>::U1(emitted(index));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enumerated<'s, T: Clone> {
    pub index: i32,
    pub elem: &'s T,
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_enumerate_List<'s, T: Clone> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn enumerate<T: Clone>(list: &Vec<T>) -> __Iter_enumerate_List<'_, T> {
    return __Iter_enumerate_List { list: list, at: 0 };
}

pub fn next__9<'a, T: Clone>(__p: &mut __Iter_enumerate_List<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = __p.list.get((__p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::<Enumerated<'_, T>, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at + 1;
    return Union2::<Enumerated<'_, T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_enumerate_rev_List<'s, T: Clone> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn enumerate_rev<T: Clone>(list: &Vec<T>) -> __Iter_enumerate_rev_List<'_, T> {
    return __Iter_enumerate_rev_List { list: list, at: (list.len() as i32) - 1 };
}

pub fn next__10<'a, T: Clone>(__p: &mut __Iter_enumerate_rev_List<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = __p.list.get((__p.at) as i64 as usize);
    if elem.is_none() {
        return Union2::<Enumerated<'_, T>, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at - 1;
    return Union2::<Enumerated<'_, T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
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
