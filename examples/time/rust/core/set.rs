use crate::collections::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_nonempty::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn iter__6<T: Clone>(set: &SalvoSet<T>) -> SetYield<T> {
    return SetYield { items: set.iter().cloned().collect::<Vec<_>>(), at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetYield<T: Clone + 'static> {
    pub items: Vec<T>,
    pub at: i32,
}

pub fn next__11<T: Clone>(p: &mut SetYield<T>) -> Union2<T, Finished> {
    let mut elem = p.items.get((p.at) as i64 as usize).cloned();
    if elem.is_none() {
        return Union2::<T, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<T, Finished>::U1(emitted(elem.as_ref().unwrap().clone()));
}
