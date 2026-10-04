use crate::collections::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn KeyOf_qualifies<K: Clone, V: Clone>(key: &K, map: &SalvoMap<K, V>) -> bool {
    return map.contains_key(&key);
}

pub fn get__4<'a, K: Clone, V: Clone>(map: &'a SalvoMap<K, V>, key: &K) -> &'a V {
    return map.get(&key).unwrap();
}

pub fn iter__5<K: Clone, V: Clone>(map: &SalvoMap<K, V>) -> MapKeyYield<K> {
    return MapKeyYield { items: map.keys().cloned().collect::<Vec<_>>(), at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapKeyYield<K> {
    pub items: Vec<K>,
    pub at: i32,
}

impl<K: Clone + 'static + crate::wire::__Wire> crate::wire::__Wire for MapKeyYield<K> {
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

pub fn next__11<K: Clone>(p: &mut MapKeyYield<K>) -> Union2<K, Finished> {
    let mut key = p.items.get((p.at) as i64 as usize).cloned();
    if key.is_none() {
        return Union2::<K, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<K, Finished>::U1(emitted(key.as_ref().unwrap().clone()));
}

pub fn NonEmpty__Map_qualifies<K: Clone, V: Clone>(map: &SalvoMap<K, V>) -> bool {
    return (map.len() as i32) > 0;
}
