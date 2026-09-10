use crate::core_iterator::*;
use crate::core_list::*;
use crate::unions::*;

pub fn iter<T: Clone + 'static>(array: Vec<T>) -> ArrayYield<T> {
    return ArrayYield { items: array, at: 0 };
}

#[derive(Clone, Debug)]
pub struct ArrayYield<T: Clone + 'static> {
    pub items: Vec<T>,
    pub at: i32,
}

pub fn next<T: Clone + 'static>(p: &mut ArrayYield<T>) -> Union2<T, Finished> {
    let mut elem = p.items.get((p.at) as usize).cloned();
    if elem.is_none() {
        return Union2::<T, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<T, Finished>::U1(emitted(elem.as_ref().unwrap().clone()));
}
