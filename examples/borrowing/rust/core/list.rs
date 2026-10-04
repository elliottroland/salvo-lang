use crate::collections::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;
use crate::unions::*;

/// [platform-type] The host's `List`.
pub use crate::platform_core_list::List;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<List<i32>>(); };

pub fn list_by<T: Clone>(size: i32, init: &mut impl FnMut(i32) -> T) -> Vec<T> {
    return mut_list_by(size, init);
}

pub fn mut_list_by<T: Clone>(size: i32, init: &mut impl FnMut(i32) -> T) -> Vec<T> {
    let mut out = vec![];
    let mut i = 0;
    while i < size {
        add_platform(&mut out, init(i.clone()));
        i = i + 1;
    }
    return out;
}

pub fn get_platform<T>(list: &Vec<T>, index: i32) -> Option<&T> {
    crate::platform_core_list::get(list, index)
}

pub fn get_platform__loc<T>(list: &Vec<T>, index: i32) -> Option<usize> {
    if index >= 0 && (index as usize) < list.len() { Some(index as usize) } else { None }
}

pub fn Idx_qualifies<T: Clone>(index: i32, list: &Vec<T>) -> bool {
    return index >= 0 && index < size_platform(list);
}

pub fn NotEq_qualifies(j: i32, i: i32) -> bool {
    return j != i;
}

pub fn get__Idx<'a, T>(list: &'a Vec<T>, index: &i32) -> &'a T {
    return get_platform(list, *index + 0).expect("salvo: value is absent at core.list:104:12");
}

pub fn get__Idx__loc<T>(list: &Vec<T>, index: &i32) -> usize {
    return get_platform__loc(list, *index + 0).expect("salvo: value is absent at core.list:104:12");
}

pub fn swap__Mut_Idx_Idx<T: Clone>(list: &mut Vec<T>, i: &i32, j: &i32) {
    ignore(swap__Mut(list, *i + 0, *j + 0));
    return;
}

pub fn at<T>(list: &mut Vec<T>, index: i32) -> Option<&T> {
    return get_platform(list, index);
}

pub fn at__loc<T>(list: &Vec<T>, index: i32) -> Option<usize> {
    return get_platform__loc(list, index);
}

pub fn update<T: Clone>(list: &mut Vec<T>, index: &i32, f: &mut impl FnMut(&mut T)) {
    f(list.get_mut((*index) as usize).expect("salvo: value is absent at core.list:147:7"));
    return;
}

pub fn update2<T: Clone>(list: &mut Vec<T>, i: &i32, j: &i32, f: &mut impl FnMut(&mut T, &mut T)) {
    let (__pm0, __pm1) = salvo_pair_mut(&mut list[..], (*i) as usize, (*j) as usize).expect("salvo: value is absent at core.list:160:5");
    f(__pm0, __pm1);
    return;
}

pub fn add_platform<T>(list: &mut Vec<T>, elem: T) {
    crate::platform_core_list::add(list, elem)
}

pub fn remove_first_platform<T>(list: &mut Vec<T>) -> Option<T> {
    crate::platform_core_list::remove_first(list)
}

pub fn remove_at_platform<T>(list: &mut Vec<T>, index: i32) -> Option<T> {
    crate::platform_core_list::remove_at(list, index)
}

pub fn insert_at_platform<T>(list: &mut Vec<T>, index: i32, elem: T) -> Option<T> {
    crate::platform_core_list::insert_at(list, index, elem)
}

pub fn remove_range_platform<T>(list: &mut Vec<T>, from: i32, to: i32) -> Vec<T> {
    crate::platform_core_list::remove_range(list, from, to)
}

pub fn swap__Mut<T>(list: &mut Vec<T>, i: i32, j: i32) -> Checked<bool> {
    return checked(swap_at_platform(list, i, j));
}

pub fn swap_at_platform<T>(list: &mut Vec<T>, i: i32, j: i32) -> bool {
    crate::platform_core_list::swap_at(list, i, j)
}

pub fn drain__2<T>(list: Vec<T>, each: &mut impl FnMut(T)) {
    let mut m = into_mut_platform(list);
    reverse(&mut m);
    while size_platform(&m) > 0 {
        each(remove_last_platform(&mut m).expect("salvo: value is absent at core.list:250:14"));
    }
    end_empty_platform(m);
}

pub fn into_mut_platform<T>(list: Vec<T>) -> Vec<T> {
    crate::platform_core_list::into_mut(list)
}

pub fn end_empty_platform<T>(mut list: Vec<T>) {
    crate::platform_core_list::end_empty(list)
}

pub fn remove_last_platform<T>(list: &mut Vec<T>) -> Option<T> {
    crate::platform_core_list::remove_last(list)
}

pub fn first_platform<T>(list: &Vec<T>) -> Option<&T> {
    crate::platform_core_list::first(list)
}

pub fn NonEmpty__List_qualifies<T: Clone>(list: &Vec<T>) -> bool {
    return size_platform(list) > 0;
}

pub fn first__NonEmpty<T>(list: &Vec<T>) -> &T {
    return get_platform(list, 0).expect("salvo: value is absent at core.list:307:12");
}

pub fn size_platform<T>(list: &Vec<T>) -> i32 {
    crate::platform_core_list::size(list)
}

pub fn last__2<T>(list: &Vec<T>) -> Option<&T> {
    return get_platform(list, size_platform(list) - 1);
}

pub fn is_empty<T>(list: &Vec<T>) -> bool {
    return size_platform(list) == 0;
}

pub fn remove_front<T>(list: &mut Vec<T>, n: i32) -> Vec<T> {
    return remove_range_platform(list, 0, n);
}

pub fn remove_back<T>(list: &mut Vec<T>, n: i32) -> Vec<T> {
    let mut at = size_platform(list) - n;
    if at < 0 {
        at = 0;
    }
    return remove_range_platform(list, at, size_platform(list));
}

pub fn remove_front_while<T>(list: &mut Vec<T>, keep: &mut impl FnMut(&T) -> bool) -> Vec<T> {
    let mut n = 0;
    while n < size_platform(list) && keep(&(get_platform(list, n).expect("salvo: value is absent at core.list:350:34"))) {
        n = n + 1;
    }
    return remove_range_platform(list, 0, n);
}

pub fn remove_back_while<T>(list: &mut Vec<T>, keep: &mut impl FnMut(&T) -> bool) -> Vec<T> {
    let mut at = size_platform(list);
    while at > 0 && keep(&(get_platform(list, at - 1).expect("salvo: value is absent at core.list:362:26"))) {
        at = at - 1;
    }
    return remove_range_platform(list, at, size_platform(list));
}

pub fn sub_list<T: Clone>(list: &Vec<T>, from: i32, to: i32, copy: &mut dyn FnMut(&T) -> T) -> Vec<T> {
    let mut out = vec![];
    let mut i = from;
    if i < 0 {
        i = 0;
    }
    while i < to && i < size_platform(list) {
        add_platform(&mut out, copy(&get_platform(list, i).expect("salvo: value is absent at core.list:380:23")));
        i = i + 1;
    }
    return out;
}

pub fn find_first<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> Option<i32> {
    let mut i = 0;
    while i < size_platform(list) {
        if pick(&(get_platform(list, i).expect("salvo: value is absent at core.list:390:17"))) {
            return Some(i);
        }
        i = i + 1;
    }
    return None;
}

pub fn find_last<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> Option<i32> {
    let mut i = size_platform(list) - 1;
    while i >= 0 {
        if pick(&(get_platform(list, i).expect("salvo: value is absent at core.list:402:17"))) {
            return Some(i);
        }
        i = i - 1;
    }
    return None;
}

pub fn index_of__2<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> Option<i32> {
    let mut i = 0;
    while i < size_platform(list) {
        if eq(&get_platform(list, i).expect("salvo: value is absent at core.list:414:15"), elem) {
            return Some(i);
        }
        i = i + 1;
    }
    return None;
}

pub fn last_index_of<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> Option<i32> {
    let mut i = size_platform(list) - 1;
    while i >= 0 {
        if eq(&get_platform(list, i).expect("salvo: value is absent at core.list:426:15"), elem) {
            return Some(i);
        }
        i = i - 1;
    }
    return None;
}

pub fn contains<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    let mut i = 0;
    while i < size_platform(list) {
        if eq(&get_platform(list, i).expect("salvo: value is absent at core.list:438:15"), elem) {
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
    while i < size_platform(list) {
        if !pick(&(get_platform(list, i).expect("salvo: value is absent at core.list:455:18"))) {
            return false;
        }
        i = i + 1;
    }
    return true;
}

pub fn count<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> i32 {
    let mut n = 0;
    let mut i = 0;
    while i < size_platform(list) {
        if pick(&(get_platform(list, i).expect("salvo: value is absent at core.list:468:17"))) {
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
    while i < size_platform(list) {
        let mut x = get_platform(list, i).unwrap();
        if pick(x) {
            add_platform(&mut yes, copy(x));
        } else {
            add_platform(&mut no, copy(x));
        }
        i = i + 1;
    }
    return (yes, no);
}

pub fn reverse<T>(list: &mut Vec<T>) {
    let mut i = 0;
    let mut j = size_platform(list) - 1;
    while i < j {
        ignore(swap__Mut(list, i.clone(), j.clone()));
        i = i + 1;
        j = j - 1;
    }
}

pub fn iter__4<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListYield<'s, T> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next__5<'s, T: Clone>(p: &mut ListYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = get_platform(&p.items, p.at);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::U1(emitted(elem.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_reversed_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn reversed__2<T: Clone>(list: &Vec<T>) -> __Iter_reversed_List<'_, T> {
    return __Iter_reversed_List { list: list, at: size_platform(list) - 1 };
}

pub fn next__6<'s, T: Clone>(__p: &mut __Iter_reversed_List<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = get_platform(&__p.list, __p.at);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    __p.at = __p.at - 1;
    return Union2::U1(emitted(elem.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_indices_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn indices<T: Clone>(list: &Vec<T>) -> __Iter_indices_List<'_, T> {
    return __Iter_indices_List { list: list, at: 0 };
}

pub fn next__7<T: Clone>(__p: &mut __Iter_indices_List<'_, T>) -> Union2<i32, Finished> {
    if __p.at >= size_platform(&__p.list) {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at + 1;
    return Union2::<i32, Finished>::U1(emitted(index));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_rev_indices_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn rev_indices<T: Clone>(list: &Vec<T>) -> __Iter_rev_indices_List<'_, T> {
    return __Iter_rev_indices_List { list: list, at: size_platform(list) - 1 };
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
pub struct Enumerated<'s, T> {
    pub index: i32,
    pub elem: &'s T,
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_enumerate_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn enumerate<T: Clone>(list: &Vec<T>) -> __Iter_enumerate_List<'_, T> {
    return __Iter_enumerate_List { list: list, at: 0 };
}

pub fn next__9<'a, T: Clone>(__p: &mut __Iter_enumerate_List<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = get_platform(&__p.list, __p.at);
    if elem.is_none() {
        return Union2::<Enumerated<'_, T>, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at + 1;
    return Union2::<Enumerated<'_, T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_enumerate_rev_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn enumerate_rev<T: Clone>(list: &Vec<T>) -> __Iter_enumerate_rev_List<'_, T> {
    return __Iter_enumerate_rev_List { list: list, at: size_platform(list) - 1 };
}

pub fn next__10<'a, T: Clone>(__p: &mut __Iter_enumerate_rev_List<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = get_platform(&__p.list, __p.at);
    if elem.is_none() {
        return Union2::<Enumerated<'_, T>, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = __p.at - 1;
    return Union2::<Enumerated<'_, T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
}

pub fn to_str__3<T: Clone>(list: &Vec<T>, to_str: &mut dyn FnMut(&T) -> String) -> String {
    let mut out = mut_str(vec!["[".to_string()]);
    let mut i = 0;
    for mut x in list.clone() {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(to_str(&x)));
        i = i + 1;
    }
    crate::core_string::append_platform(&mut out, &("]".to_string()));
    return out;
}

pub fn sort_by_platform<T: Clone>(list: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Vec<T> {
    crate::platform_core_list::sort_by(list, cmp)
}

pub fn insert_sorted_by_platform<T: Clone>(list: &mut Vec<T>, elem: T, cmp: &mut dyn FnMut(&T, &T) -> i32) {
    crate::platform_core_list::insert_sorted_by(list, elem, cmp)
}

pub fn search_sorted_by_platform<T: Clone>(list: &Vec<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Option<i32> {
    crate::platform_core_list::search_sorted_by(list, elem, cmp)
}

pub fn sort<T: Clone>(list: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Vec<T> {
    return sort_by_platform(list, cmp);
}

pub fn mut_sort<T: Clone>(list: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Vec<T> {
    return sort_by_platform(list, cmp);
}

pub fn add_sorted<T: Clone>(list: &mut Vec<T>, elem: T, cmp: &mut dyn FnMut(&T, &T) -> i32) {
    insert_sorted_by_platform(list, elem, cmp);
}

pub fn binary_search<T: Clone>(list: &Vec<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Option<i32> {
    return search_sorted_by_platform(list, elem, cmp);
}
