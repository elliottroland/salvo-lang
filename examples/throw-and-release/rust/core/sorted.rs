use crate::collections::*;
use crate::core_bytes::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_string::*;

pub fn iter__7<T: Clone>(set: &SalvoSortedSet<T>) -> SetYield<T> {
    return SetYield { items: set.to_vec(), at: 0 };
}

pub fn iter__8<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> MapKeyYield<K> {
    return MapKeyYield { items: map.keys(), at: 0 };
}
