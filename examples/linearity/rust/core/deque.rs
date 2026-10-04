use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

/// [platform-type] The host's `Deque`.
pub use crate::platform_core_deque::Deque;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq + std::hash::Hash>() {} __contract::<Deque<i32>>(); };

pub fn empty_deque_platform<T>() -> std::collections::VecDeque<T> {
    crate::platform_core_deque::empty_deque()
}

pub fn deque_from_platform<T: Clone>(first: T, rest: Vec<T>) -> std::collections::VecDeque<T> {
    crate::platform_core_deque::deque_from(first, rest)
}

pub fn into_mut_platform<T>(d: std::collections::VecDeque<T>) -> std::collections::VecDeque<T> {
    crate::platform_core_deque::into_mut(d)
}

pub fn end_empty_platform<T>(mut d: std::collections::VecDeque<T>) {
    crate::platform_core_deque::end_empty(d)
}

pub fn deque_of<T>() -> std::collections::VecDeque<T> {
    return empty_deque_platform();
}

pub fn deque_of__2<T>(first: T) -> std::collections::VecDeque<T> {
    let mut d = empty_deque_platform();
    add_last_platform(&mut d, first);
    return d;
}

pub fn deque_of__3<T: Clone>(first: T, rest: Vec<T>) -> std::collections::VecDeque<T> {
    return deque_from_platform(first, rest);
}

pub fn mut_deque_of<T>() -> std::collections::VecDeque<T> {
    return empty_deque_platform();
}

pub fn mut_deque_of__2<T>(first: T) -> std::collections::VecDeque<T> {
    let mut d = empty_deque_platform();
    add_last_platform(&mut d, first);
    return d;
}

pub fn mut_deque_of__3<T: Clone>(first: T, rest: Vec<T>) -> std::collections::VecDeque<T> {
    return deque_from_platform(first, rest);
}

pub fn deque_by<T: Clone>(size: i32, init: &mut impl FnMut(i32) -> T) -> std::collections::VecDeque<T> {
    return mut_deque_by(size, init);
}

pub fn mut_deque_by<T: Clone>(size: i32, init: &mut impl FnMut(i32) -> T) -> std::collections::VecDeque<T> {
    let mut d = empty_deque_platform();
    let mut i = 0;
    while i < size {
        add_last_platform(&mut d, init(i.clone()));
        i = i + 1;
    }
    return d;
}

pub fn add_last_platform<T>(d: &mut std::collections::VecDeque<T>, elem: T) {
    crate::platform_core_deque::add_last(d, elem)
}

pub fn add_first_platform<T>(d: &mut std::collections::VecDeque<T>, elem: T) {
    crate::platform_core_deque::add_first(d, elem)
}

pub fn remove_first_platform<T>(d: &mut std::collections::VecDeque<T>) -> Option<T> {
    crate::platform_core_deque::remove_first(d)
}

pub fn remove_last_platform<T>(d: &mut std::collections::VecDeque<T>) -> Option<T> {
    crate::platform_core_deque::remove_last(d)
}

pub fn remove_at_platform<T>(d: &mut std::collections::VecDeque<T>, index: i32) -> Option<T> {
    crate::platform_core_deque::remove_at(d, index)
}

pub fn get_platform<T>(d: &std::collections::VecDeque<T>, index: i32) -> Option<&T> {
    crate::platform_core_deque::get(d, index)
}

pub fn first_platform<T>(d: &std::collections::VecDeque<T>) -> Option<&T> {
    crate::platform_core_deque::first(d)
}

pub fn last_platform<T>(d: &std::collections::VecDeque<T>) -> Option<&T> {
    crate::platform_core_deque::last(d)
}

pub fn size_platform<T>(d: &std::collections::VecDeque<T>) -> i32 {
    crate::platform_core_deque::size(d)
}

pub fn drain<T>(d: std::collections::VecDeque<T>, each: &mut impl FnMut(T)) {
    let mut m = into_mut_platform(d);
    while size_platform(&m) > 0 {
        each(remove_first_platform(&mut m).expect("salvo: value is absent at core.deque:101:14"));
    }
    end_empty_platform(m);
}

pub fn to_str__2<T: Clone>(d: &std::collections::VecDeque<T>, to_str: &mut dyn FnMut(&T) -> String) -> String {
    let mut out = mut_str(vec!["[".to_string()]);
    let mut i = 0;
    let mut __loop1_pass = iter__3(d);
    while let Union2::U1(mut x) = next__3(&mut __loop1_pass) {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(to_str(&x)));
        i = i + 1;
    }
    crate::core_string::append_platform(&mut out, &("]".to_string()));
    return out;
}

pub fn to_list<T: Clone>(d: &std::collections::VecDeque<T>, copy: &mut dyn FnMut(&T) -> T) -> Vec<T> {
    let mut out = vec![];
    let mut __loop2_pass = iter__3(d);
    while let Union2::U1(mut x) = next__3(&mut __loop2_pass) {
        crate::core_list::add_platform(&mut out, copy(&x));
    }
    return out;
}

pub fn to_deque<T: Clone>(list: &Vec<T>, copy: &mut dyn FnMut(&T) -> T) -> std::collections::VecDeque<T> {
    let mut out = empty_deque_platform();
    for mut x in list.clone() {
        add_last_platform(&mut out, copy(&x));
    }
    return out;
}

pub fn iter__3<T: Clone>(d: &std::collections::VecDeque<T>) -> DequeYield<'_, T> {
    return DequeYield { items: d, at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct DequeYield<'s, T> {
    pub items: &'s std::collections::VecDeque<T>,
    pub at: i32,
}

pub fn next__3<'s, T: Clone>(p: &mut DequeYield<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = get_platform(&p.items, p.at);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::U1(emitted(elem.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_reversed_Deque<'s, T> {
    pub d: &'s std::collections::VecDeque<T>,
    pub at: i32,
}

pub fn reversed<T: Clone>(d: &std::collections::VecDeque<T>) -> __Iter_reversed_Deque<'_, T> {
    return __Iter_reversed_Deque { d: d, at: size_platform(d) - 1 };
}

pub fn next__4<'s, T: Clone>(__p: &mut __Iter_reversed_Deque<'s, T>) -> Union2<&'s T, Finished> {
    let mut elem = get_platform(&__p.d, __p.at);
    if elem.is_none() {
        return Union2::U2(finished());
    }
    __p.at = __p.at - 1;
    return Union2::U1(emitted(elem.unwrap()));
}
