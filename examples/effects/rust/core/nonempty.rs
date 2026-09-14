use crate::collections::*;
use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_seq::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;

pub fn NonEmpty__Set_qualifies<T: Clone>(set: &SalvoSet<T>) -> bool {
    return (set.len() as i32) > 0;
}

pub fn NonEmpty__Map_qualifies<K: Clone, V: Clone>(map: &SalvoMap<K, V>) -> bool {
    return (map.len() as i32) > 0;
}

pub fn NonEmpty__SortedSet_qualifies<T: Clone>(set: &std::collections::BTreeSet<T>) -> bool {
    return (set.len() as i32) > 0;
}

pub fn NonEmpty__SortedMap_qualifies<K: Clone, V: Clone>(map: &std::collections::BTreeMap<K, V>) -> bool {
    return (map.len() as i32) > 0;
}

pub fn min<T: Clone>(set: &std::collections::BTreeSet<T>) -> T {
    return set.iter().next().cloned().unwrap();
}

pub fn max<T: Clone>(set: &std::collections::BTreeSet<T>) -> T {
    return set.iter().next_back().cloned().unwrap();
}

pub fn first_key<K: Clone, V: Clone>(map: &std::collections::BTreeMap<K, V>) -> K {
    return map.keys().next().cloned().unwrap();
}

pub fn last_key<K: Clone, V: Clone>(map: &std::collections::BTreeMap<K, V>) -> K {
    return map.keys().next_back().cloned().unwrap();
}
