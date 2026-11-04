use crate::std_core_iterator::Finished;
use crate::std_core_list::append_platform;
use crate::std_core_iterator::emitted;
use crate::std_core_iterator::finished;


pub fn iter<T: Clone>(array: &Vec<T>) -> crate::std_core_array::ArrayIter<'_, T> {
    return crate::std_core_array::ArrayIter { items: array, at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayIter<'s, T> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next<'a, T: Clone>(p: &mut crate::std_core_array::ArrayIter<'a, T>) -> crate::unions::Union2<&'a T, crate::std_core_iterator::Finished> {
    let mut elem: Option<&T> = p.items.get((p.at) as i64 as usize);
    if elem.is_none() {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut elem_1 = elem.unwrap();
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<&T>(elem_1));
}

pub fn drain<T>(mut items: Vec<T>, each: &mut dyn FnMut(T)) {
    let mut list: Vec<T> = vec![];
    crate::std_core_list::append_platform::<T>(&mut list, items);
    crate::std_core_list::drain::<T>(list, &mut *each);
}
