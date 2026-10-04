use crate::collections::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn iter__6<T: Clone>(set: &SalvoSet<T>) -> SetYield<T> {
    return SetYield { items: set.iter().cloned().collect::<Vec<_>>(), at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetYield<T> {
    pub items: Vec<T>,
    pub at: i32,
}

impl<T: Clone + 'static + crate::wire::__Wire> crate::wire::__Wire for SetYield<T> {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.items, out);
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            items: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn next__19<T: Clone>(p: &mut SetYield<T>) -> Union2<T, Finished> {
    let mut elem = p.items.get((p.at) as i64 as usize).cloned();
    if elem.is_none() {
        return Union2::<T, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<T, Finished>::U1(emitted(elem.as_ref().unwrap().clone()));
}

pub fn NonEmpty__Set_qualifies<T: Clone>(set: &SalvoSet<T>) -> bool {
    return (set.len() as i32) > 0;
}
