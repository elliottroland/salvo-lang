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

pub fn NonEmpty__SortedSet_qualifies<T: Clone>(set: &SalvoSortedSet<T>) -> bool {
    return (set.len() as i32) > 0;
}

pub fn NonEmpty__SortedMap_qualifies<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> bool {
    return (map.len() as i32) > 0;
}

pub fn min<T: Clone>(set: &SalvoSortedSet<T>) -> T {
    return set.min().cloned().expect("salvo: value is absent at core.nonempty:58:12");
}

pub fn max<T: Clone>(set: &SalvoSortedSet<T>) -> T {
    return set.max().cloned().expect("salvo: value is absent at core.nonempty:62:12");
}

pub fn first_key<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> K {
    return map.first_key().cloned().expect("salvo: value is absent at core.nonempty:66:12");
}

pub fn last_key<K: Clone, V: Clone>(map: &SalvoSortedMap<K, V>) -> K {
    return map.last_key().cloned().expect("salvo: value is absent at core.nonempty:70:12");
}
