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

pub fn NonEmpty__SortedSet_qualifies<T: Clone>(set: &SalvoSortedSet<T>) -> bool {
    return (set.len() as i32) > 0;
}

pub fn NonEmpty__SortedMap_qualifies<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> bool {
    return (map.len() as i32) > 0;
}

pub fn min<T: Clone>(set: &SalvoSortedSet<T>) -> T {
    return set.min().cloned().expect("salvo: value is absent at core.sorted:160:12");
}

pub fn max<T: Clone>(set: &SalvoSortedSet<T>) -> T {
    return set.max().cloned().expect("salvo: value is absent at core.sorted:164:12");
}

pub fn first_key<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> K {
    return map.first_key().cloned().expect("salvo: value is absent at core.sorted:168:12");
}

pub fn last_key<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> K {
    return map.last_key().cloned().expect("salvo: value is absent at core.sorted:172:12");
}
