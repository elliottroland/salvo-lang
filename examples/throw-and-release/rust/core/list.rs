use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_string::*;
use crate::unions::*;

pub fn iter__2<T: Clone + 'static>(list: Vec<T>) -> ListYield<T> {
    return ListYield { items: list, at: 0 };
}

#[derive(Clone, Debug)]
pub struct ListYield<T: Clone + 'static> {
    pub items: Vec<T>,
    pub at: i32,
}

pub fn next__2<T: Clone + 'static>(p: &mut ListYield<T>) -> Union2<T, Finished> {
    let mut elem = p.items.get((p.at) as usize).cloned();
    if elem.is_none() {
        return Union2::<T, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<T, Finished>::U1(emitted(elem.as_ref().unwrap().clone()));
}
