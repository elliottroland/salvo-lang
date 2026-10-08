use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_list::size_platform;


pub fn Idx__Int_qualifies<C>(mut index: i32, c: &C, size: &mut dyn FnMut(&C) -> i32) -> bool {
    return ((index >= 0i32) && (index < size(c)));
}

pub fn NotEq__Int_qualifies(mut j: i32, mut i: i32) -> bool {
    return !(((j) == (i)));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_indices_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn indices<T: Clone>(list: &Vec<T>) -> crate::core_index::__Iter_indices_List<'_, T> {
    return crate::core_index::__Iter_indices_List { list: list, at: 0i32 };
}

pub fn next__Iter_indices_List<T: Clone>(__p: &mut crate::core_index::__Iter_indices_List<'_, T>) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (__p.at >= crate::core_list::size_platform::<T>(__p.list)) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut index: i32 = __p.at;
    __p.at = i32::wrapping_add(__p.at, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(index));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_rev_indices_List<'s, T> {
    pub list: &'s Vec<T>,
    pub at: i32,
}

pub fn rev_indices<T: Clone>(list: &Vec<T>) -> crate::core_index::__Iter_rev_indices_List<'_, T> {
    return crate::core_index::__Iter_rev_indices_List { list: list, at: i32::wrapping_sub(crate::core_list::size_platform::<T>(list), 1i32) };
}

pub fn next__Iter_rev_indices_List<T: Clone>(__p: &mut crate::core_index::__Iter_rev_indices_List<'_, T>) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (__p.at < 0i32) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut index: i32 = __p.at;
    __p.at = i32::wrapping_sub(__p.at, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(index));
}
