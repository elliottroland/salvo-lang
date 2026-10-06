use crate::unions::*;
use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_list::List;

pub fn Idx__Int_qualifies<C: Clone>(index: i32, c: &C, size: &mut dyn FnMut(&C) -> i32) -> bool {
    return index >= 0 && index < size(c);
}

pub fn NotEq__Int_qualifies(j: i32, i: i32) -> bool {
    return j != i;
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_indices_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn indices<T: Clone>(list: &Vec<T>) -> __Iter_indices_List<'_, T> {
    return __Iter_indices_List { list: list, at: 0 };
}

pub fn next__Iter_indices_List<T: Clone>(__p: &mut __Iter_indices_List<'_, T>) -> Union2<i32, Finished> {
    if __p.at >= crate::core_list::size_platform(&__p.list) {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = i32::wrapping_add(__p.at, 1);
    return Union2::<i32, Finished>::U1(emitted(index));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_rev_indices_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn rev_indices<T: Clone>(list: &Vec<T>) -> __Iter_rev_indices_List<'_, T> {
    return __Iter_rev_indices_List { list: list, at: i32::wrapping_sub(crate::core_list::size_platform(list), 1) };
}

pub fn next__Iter_rev_indices_List<T: Clone>(__p: &mut __Iter_rev_indices_List<'_, T>) -> Union2<i32, Finished> {
    if __p.at < 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut index = __p.at;
    __p.at = i32::wrapping_sub(__p.at, 1);
    return Union2::<i32, Finished>::U1(emitted(index));
}
