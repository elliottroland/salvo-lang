use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;


pub fn iter<T: Clone>(array: &Vec<T>) -> crate::core_array::ArrayYield<'_, T> {
    return crate::core_array::ArrayYield { items: array, at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayYield<'s, T> {
    pub items: &'s Vec<T>,
    pub at: i32,
}

pub fn next<'a, T: Clone>(p: &mut crate::core_array::ArrayYield<'a, T>) -> crate::unions::Union2<&'a T, crate::core_iterator::Finished> {
    let mut elem: Option<&T> = p.items.get((p.at) as i64 as usize);
    if elem.is_none() {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut elem_1 = elem.unwrap();
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<&T>(elem_1));
}
