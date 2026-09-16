use crate::core_bytes::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_seq::*;
use crate::core_set::*;
use crate::core_string::*;

pub fn iter__6<T: Clone>(set: &std::collections::BTreeSet<T>) -> SetYield<T> {
    return SetYield { items: set.iter().cloned().collect::<Vec<_>>(), at: 0 };
}

pub fn iter__7<K: Clone, V: Clone>(map: &std::collections::BTreeMap<K, V>) -> MapKeyYield<K> {
    return MapKeyYield { items: map.keys().cloned().collect::<Vec<_>>(), at: 0 };
}
