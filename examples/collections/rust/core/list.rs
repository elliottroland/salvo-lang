use crate::seq::*;
use crate::unions::*;
use crate::core_checked::Checked;
use crate::core_checked::checked;
use crate::core_checked::ignore;
use crate::core_index::Idx__Int_qualifies;
use crate::core_index::indices;
use crate::core_index::next__Iter_indices_List;
use crate::core_index::next__Iter_rev_indices_List;
use crate::core_index::rev_indices;
use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_string::mut_str;

/// [platform-type] The host's `List`.
pub use crate::platform_core_list::List;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<List<i32>>(); };
const _: fn() = || { fn __each(x: &List<i32>) -> impl Iterator<Item = i32> + '_ { crate::platform_core_list::each(x).map(|e| e.clone()) } let _ = __each; };
const _: fn() = || { fn __each_ref(x: &List<i32>) -> impl Iterator<Item = &i32> + '_ { crate::platform_core_list::each(x) } fn __each_mut(x: &mut List<i32>) -> impl Iterator<Item = &mut i32> + '_ { crate::platform_core_list::each_mut(x) } fn __into_each(x: List<i32>) -> impl Iterator<Item = i32> { crate::platform_core_list::into_each(x) } let _ = (__each_ref, __each_mut, __into_each); };

pub fn list_by<T: Clone>(size: i32, init: &mut impl FnMut(i32) -> T) -> Vec<T> {
    return mut_list_by(size, init);
}

pub fn mut_list_by<T: Clone>(size: i32, init: &mut impl FnMut(i32) -> T) -> Vec<T> {
    let mut out = vec![];
    let mut i = 0;
    while i < size {
        add_platform(&mut out, init(i.clone()));
        i = i32::wrapping_add(i, 1);
    }
    return out;
}

pub fn get_platform<T>(list: &Vec<T>, index: i32) -> Option<&T> {
    crate::platform_core_list::get(list, index)
}

pub fn get_platform__loc<T>(list: &Vec<T>, index: i32) -> Option<usize> {
    if index >= 0 && (index as usize) < list.len() { Some(index as usize) } else { None }
}

pub fn get<'a, T>(list: &'a Vec<T>, index: &i32) -> &'a T {
    return get_at_platform(list, i32::wrapping_add(*index, 0));
}

pub fn get__loc<T>(list: &Vec<T>, index: &i32) -> usize {
    return get_at_platform__loc(list, i32::wrapping_add(*index, 0));
}

pub fn get_at_platform<T>(list: &Vec<T>, index: i32) -> &T {
    crate::platform_core_list::get_at(list, index)
}

pub fn get_at_platform__loc<T>(list: &Vec<T>, index: i32) -> usize {
    index as usize
}

pub fn swap__MutList_IdxInt_IdxInt<T: Clone>(list: &mut Vec<T>, i: &i32, j: &i32) {
    ignore(swap__MutList_Int_Int(list, i32::wrapping_add(*i, 0), i32::wrapping_add(*j, 0)));
    return;
}

pub fn replace_platform<T>(list: &mut Vec<T>, index: &i32, value: T) -> T {
    crate::platform_core_list::replace(list, *index, value)
}

pub fn at<T>(list: &mut Vec<T>, index: i32) -> Option<&T> {
    return get_platform(list, index);
}

pub fn update<T: Clone>(list: &mut Vec<T>, index: &i32, f: &mut impl FnMut(&mut T)) {
    f(list.get_mut((*index) as usize).expect("salvo: value is absent at core.list:134:7"));
    return;
}

pub fn update2<T: Clone>(list: &mut Vec<T>, i: &i32, j: &i32, f: &mut impl FnMut(&mut T, &mut T)) {
    let (__pm0, __pm1) = salvo_pair_mut(&mut list[..], (*i) as usize, (*j) as usize).expect("salvo: value is absent at core.list:147:5");
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

pub fn swap__MutList_Int_Int<T>(list: &mut Vec<T>, i: i32, j: i32) -> Checked<bool> {
    return checked(swap_at_platform(list, i, j));
}

pub fn swap_at_platform<T>(list: &mut Vec<T>, i: i32, j: i32) -> bool {
    crate::platform_core_list::swap_at(list, i, j)
}

pub fn drain<T>(list: Vec<T>, each: &mut impl FnMut(T)) {
    let mut m = into_mut_platform(list);
    reverse(&mut m);
    while size_platform(&m) > 0 {
        each(remove_last_platform(&mut m).expect("salvo: value is absent at core.list:237:14"));
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

pub fn first<T>(list: &Vec<T>) -> &T {
    return get_platform(list, 0).expect("salvo: value is absent at core.list:294:12");
}

pub fn size_platform<T>(list: &Vec<T>) -> i32 {
    crate::platform_core_list::size(list)
}

pub fn last<T>(list: &Vec<T>) -> Option<&T> {
    return get_platform(list, i32::wrapping_sub(size_platform(list), 1));
}

pub fn is_empty<T>(list: &Vec<T>) -> bool {
    return size_platform(list) == 0;
}

pub fn remove_front<T>(list: &mut Vec<T>, n: i32) -> Vec<T> {
    return remove_range_platform(list, 0, n);
}

pub fn remove_back<T>(list: &mut Vec<T>, n: i32) -> Vec<T> {
    let mut at = i32::wrapping_sub(size_platform(list), n);
    if at < 0 {
        at = 0;
    }
    return remove_range_platform(list, at, size_platform(list));
}

pub fn remove_front_while<T>(list: &mut Vec<T>, keep: &mut impl FnMut(&T) -> bool) -> Vec<T> {
    let mut n = 0;
    while n < size_platform(list) && keep(&(get_platform(list, n).expect("salvo: value is absent at core.list:337:34"))) {
        n = i32::wrapping_add(n, 1);
    }
    return remove_range_platform(list, 0, n);
}

pub fn remove_back_while<T>(list: &mut Vec<T>, keep: &mut impl FnMut(&T) -> bool) -> Vec<T> {
    let mut at = size_platform(list);
    while at > 0 && keep(&(get_platform(list, i32::wrapping_sub(at, 1)).expect("salvo: value is absent at core.list:349:26"))) {
        at = i32::wrapping_sub(at, 1);
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
        add_platform(&mut out, copy(&get_platform(list, i).expect("salvo: value is absent at core.list:367:23")));
        i = i32::wrapping_add(i, 1);
    }
    return out;
}

pub fn find_first<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> Option<i32> {
    let mut __loop1_pass = indices(list);
    while let Union2::U1(mut i) = next__Iter_indices_List(&mut __loop1_pass) {
        if pick(&(get(list, &i))) {
            return Some(i);
        }
    }
    return None;
}

pub fn find_last<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> Option<i32> {
    let mut __loop2_pass = rev_indices(list);
    while let Union2::U1(mut i) = next__Iter_rev_indices_List(&mut __loop2_pass) {
        if pick(&(get(list, &i))) {
            return Some(i);
        }
    }
    return None;
}

pub fn index_of<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> Option<i32> {
    let mut __loop3_pass = indices(list);
    while let Union2::U1(mut i) = next__Iter_indices_List(&mut __loop3_pass) {
        if eq(&get(list, &i), elem) {
            return Some(i);
        }
    }
    return None;
}

pub fn last_index_of<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> Option<i32> {
    let mut __loop4_pass = rev_indices(list);
    while let Union2::U1(mut i) = next__Iter_rev_indices_List(&mut __loop4_pass) {
        if eq(&get(list, &i), elem) {
            return Some(i);
        }
    }
    return None;
}

pub fn contains<T: Clone>(list: &Vec<T>, elem: &T, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    let mut i = 0;
    while i < size_platform(list) {
        if eq(&get_platform(list, i).expect("salvo: value is absent at core.list:417:15"), elem) {
            return true;
        }
        i = i32::wrapping_add(i, 1);
    }
    return false;
}

pub fn any<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> bool {
    return !(find_first(list, pick).is_none());
}

pub fn all<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> bool {
    let mut i = 0;
    while i < size_platform(list) {
        if !pick(&(get_platform(list, i).expect("salvo: value is absent at core.list:434:18"))) {
            return false;
        }
        i = i32::wrapping_add(i, 1);
    }
    return true;
}

pub fn count<T: Clone>(list: &Vec<T>, pick: &mut impl FnMut(&T) -> bool) -> i32 {
    let mut n = 0;
    let mut i = 0;
    while i < size_platform(list) {
        if pick(&(get_platform(list, i).expect("salvo: value is absent at core.list:447:17"))) {
            n = i32::wrapping_add(n, 1);
        }
        i = i32::wrapping_add(i, 1);
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
        i = i32::wrapping_add(i, 1);
    }
    return (yes, no);
}

pub fn reverse<T>(list: &mut Vec<T>) {
    let mut i = 0;
    let mut j = i32::wrapping_sub(size_platform(list), 1);
    while i < j {
        ignore(swap__MutList_Int_Int(list, i.clone(), j.clone()));
        i = i32::wrapping_add(i, 1);
        j = i32::wrapping_sub(j, 1);
    }
}

pub fn iter<T: Clone>(list: &Vec<T>) -> ListYield<'_, T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListYield<'s, T> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next__ListYield<'s, T: Clone>(p: &mut ListYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = get_platform(&p.items, p.at);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = i32::wrapping_add(p.at, 1);
    return Union2::U1(emitted(elem.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_reversed_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn reversed<T: Clone>(list: &Vec<T>) -> __Iter_reversed_List<'_, T> {
    return __Iter_reversed_List { list: list, at: i32::wrapping_sub(size_platform(list), 1) };
}

pub fn next__Iter_reversed_List<'s, T: Clone>(__p: &mut __Iter_reversed_List<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = get_platform(&__p.list, __p.at);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    __p.at = i32::wrapping_sub(__p.at, 1);
    return Union2::U1(emitted(elem.unwrap()));
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

pub fn next__Iter_enumerate_List<'a, T: Clone>(__p: &mut __Iter_enumerate_List<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = get_platform(&__p.list, __p.at);
    if elem.is_none() {
        return Union2::<Enumerated<'_, T>, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = i32::wrapping_add(__p.at, 1);
    return Union2::<Enumerated<'_, T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_enumerate_rev_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn enumerate_rev<T: Clone>(list: &Vec<T>) -> __Iter_enumerate_rev_List<'_, T> {
    return __Iter_enumerate_rev_List { list: list, at: i32::wrapping_sub(size_platform(list), 1) };
}

pub fn next__Iter_enumerate_rev_List<'a, T: Clone>(__p: &mut __Iter_enumerate_rev_List<'a, T>) -> Union2<Enumerated<'a, T>, Finished> {
    let mut elem = get_platform(&__p.list, __p.at);
    if elem.is_none() {
        return Union2::<Enumerated<'_, T>, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = i32::wrapping_sub(__p.at, 1);
    return Union2::<Enumerated<'_, T>, Finished>::U1(emitted(Enumerated { index: index, elem: elem.unwrap() }));
}

pub fn to_str<T: Clone>(list: &Vec<T>, to_str: &mut dyn FnMut(&T) -> String) -> String {
    let mut out = mut_str(vec!["[".to_string()]);
    let mut i = 0;
    for mut x in crate::platform_core_list::each(list).map(|__x| __x.clone()) {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(to_str(&x)));
        i = i32::wrapping_add(i, 1);
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
    let mut found = { let __pick1 = search_sorted_by_platform(list, elem, cmp); if __pick1.is_some() { __pick1.unwrap() } else { return None } };
    if Idx__Int_qualifies(found, &*list, &mut |__i0| size_platform(&__i0)) {
        return Some(found);
    }
    return None;
}
