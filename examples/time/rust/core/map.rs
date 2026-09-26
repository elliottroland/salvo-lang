use crate::collections::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn KeyOf_qualifies<K: Clone, V: Clone>(key: &K, map: &SalvoMap<K, V>) -> bool {
    return map.contains_key(&key);
}

pub fn get__2<'a, K: Clone, V: Clone>(map: &'a SalvoMap<K, V>, key: &K) -> &'a V {
    return map.get(&key).unwrap();
}

pub fn iter__4<K: Clone, V: Clone>(map: &SalvoMap<K, V>) -> MapKeyYield<K> {
    return MapKeyYield { items: map.keys().cloned().collect::<Vec<_>>(), at: 0 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapKeyYield<K: Clone + 'static> {
    pub items: Vec<K>,
    pub at: i32,
}

pub fn next__7<K: Clone>(p: &mut MapKeyYield<K>) -> Union2<K, Finished> {
    let mut key = p.items.get((p.at) as i64 as usize).cloned();
    if key.is_none() {
        return Union2::<K, Finished>::U2(finished());
    }
    p.at = p.at + 1;
    return Union2::<K, Finished>::U1(emitted(key.as_ref().unwrap().clone()));
}
