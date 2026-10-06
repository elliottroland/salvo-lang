use crate::core_list::List;
use crate::core_list::contains;
use crate::core_map::MapKeyYield;
use crate::core_seq::to_list;
use crate::core_set::SetYield;
use crate::core_string::Str;
use crate::core_string::append_platform;
use crate::core_string::mut_str;

/// [platform-type] The host's `SortedSet`.
pub use crate::platform_core_sorted::SortedSet;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq>() {} __contract::<SortedSet<i32>>(); };
const _: fn() = || { fn __each(x: &SortedSet<i32>) -> impl Iterator<Item = i32> + '_ { crate::platform_core_sorted::each_sorted_set(x).map(|e| e.clone()) } let _ = __each; };
const _: fn() = || { fn __each_ref(x: &SortedSet<i32>) -> impl Iterator<Item = &i32> + '_ { crate::platform_core_sorted::each_sorted_set(x) } fn __each_mut(x: &mut SortedSet<i32>) -> impl Iterator<Item = &mut i32> + '_ { crate::platform_core_sorted::each_sorted_set_mut(x) } fn __into_each(x: SortedSet<i32>) -> impl Iterator<Item = i32> { crate::platform_core_sorted::into_each_sorted_set(x) } let _ = (__each_ref, __each_mut, __into_each); };

pub fn sorted_set_of_platform<T: Clone>(elems: Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> SortedSet<T> {
    crate::platform_core_sorted::sorted_set_of(elems, cmp)
}

pub fn mut_sorted_set_of_platform<T: Clone>(elems: Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> SortedSet<T> {
    crate::platform_core_sorted::mut_sorted_set_of(elems, cmp)
}

pub fn add_platform<T: Clone>(set: &mut SortedSet<T>, elem: T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> bool {
    crate::platform_core_sorted::add(set, elem, cmp)
}

pub fn remove__SortedSet_T<T: Clone>(set: &mut SortedSet<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> bool {
    return remove_elem_platform::<T>(set, elem, &mut *cmp);
}

pub fn remove_elem_platform<T: Clone>(set: &mut SortedSet<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> bool {
    crate::platform_core_sorted::remove_elem(set, elem, cmp)
}

pub fn set_size_platform<T: Clone>(set: &SortedSet<T>) -> i32 {
    crate::platform_core_sorted::set_size(set)
}

pub fn remove_key_platform<K: Clone, V: Clone>(map: &mut SortedMap<K, V>, key: &K, cmp: &mut dyn FnMut(&K, &K) -> i32) -> Option<V> {
    crate::platform_core_sorted::remove_key(map, key, cmp)
}

pub fn map_size_platform<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> i32 {
    crate::platform_core_sorted::map_size(map)
}

pub fn contains_platform<T: Clone>(set: &SortedSet<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> bool {
    crate::platform_core_sorted::contains(set, elem, cmp)
}

pub fn size__SortedSet<T: Clone>(set: &SortedSet<T>) -> i32 {
    return set_size_platform(set);
}

pub fn min_platform<T: Clone>(set: &SortedSet<T>) -> Option<T> {
    crate::platform_core_sorted::min(set)
}

pub fn max_platform<T: Clone>(set: &SortedSet<T>) -> Option<T> {
    crate::platform_core_sorted::max(set)
}

pub fn to_list_platform<T: Clone>(set: &SortedSet<T>) -> Vec<T> {
    crate::platform_core_sorted::to_list(set)
}

pub fn to_str__SortedSet<T: Clone>(set: &SortedSet<T>, to_str: &mut dyn FnMut(&T) -> String) -> String {
    let mut out = mut_str(vec!["{".to_string()]);
    let mut i = 0;
    for mut x in crate::platform_core_sorted::each_sorted_set(set).map(|__x| __x.clone()) {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(to_str(&x)));
        i = i32::wrapping_add(i, 1);
    }
    crate::core_string::append_platform(&mut out, &("}".to_string()));
    return out;
}

pub fn iter__SortedSet<T: Clone>(set: &SortedSet<T>) -> SetYield<T> {
    return SetYield { items: to_list_platform(set), at: 0 };
}

/// [platform-type] The host's `SortedMap`.
pub use crate::platform_core_sorted::SortedMap;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq>() {} __contract::<SortedMap<i32, i32>>(); };
const _: fn() = || { fn __each(x: &SortedMap<i32, i32>) -> impl Iterator<Item = i32> + '_ { crate::platform_core_sorted::each_sorted_map(x).map(|e| e.clone()) } let _ = __each; };
const _: fn() = || { fn __each_ref(x: &SortedMap<i32, i32>) -> impl Iterator<Item = &i32> + '_ { crate::platform_core_sorted::each_sorted_map(x) } fn __each_mut(x: &mut SortedMap<i32, i32>) -> impl Iterator<Item = &mut i32> + '_ { crate::platform_core_sorted::each_sorted_map_mut(x) } fn __into_each(x: SortedMap<i32, i32>) -> impl Iterator<Item = i32> { crate::platform_core_sorted::into_each_sorted_map(x) } let _ = (__each_ref, __each_mut, __into_each); };

pub fn sorted_map_of_platform<K: Clone, V: Clone>(entries: Vec<(K, V)>, cmp: &mut dyn FnMut(&K, &K) -> i32) -> SortedMap<K, V> {
    crate::platform_core_sorted::sorted_map_of(entries, cmp)
}

pub fn mut_sorted_map_of_platform<K: Clone, V: Clone>(entries: Vec<(K, V)>, cmp: &mut dyn FnMut(&K, &K) -> i32) -> SortedMap<K, V> {
    crate::platform_core_sorted::mut_sorted_map_of(entries, cmp)
}

pub fn get_platform<'a, K: Clone, V: Clone>(map: &'a SortedMap<K, V>, key: &K, cmp: &mut dyn FnMut(&K, &K) -> i32) -> Option<&'a V> {
    crate::platform_core_sorted::get(map, key, cmp)
}

pub fn put_platform<K: Clone, V: Clone>(map: &mut SortedMap<K, V>, key: K, value: V, cmp: &mut dyn FnMut(&K, &K) -> i32) {
    crate::platform_core_sorted::put(map, key, value, cmp)
}

pub fn remove__SortedMap_K<K: Clone, V: Clone>(map: &mut SortedMap<K, V>, key: &K, cmp: &mut dyn FnMut(&K, &K) -> i32) -> Option<V> {
    return remove_key_platform::<K, V>(map, key, &mut *cmp);
}

pub fn contains_key_platform<K: Clone, V: Clone>(map: &SortedMap<K, V>, key: &K, cmp: &mut dyn FnMut(&K, &K) -> i32) -> bool {
    crate::platform_core_sorted::contains_key(map, key, cmp)
}

pub fn size__SortedMap<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> i32 {
    return map_size_platform(map);
}

pub fn first_key_platform<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> Option<K> {
    crate::platform_core_sorted::first_key(map)
}

pub fn last_key_platform<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> Option<K> {
    crate::platform_core_sorted::last_key(map)
}

pub fn keys_platform<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> Vec<K> {
    crate::platform_core_sorted::keys(map)
}

pub fn to_str__SortedMap<K: Clone, V: Clone>(map: &SortedMap<K, V>, to_str: &mut dyn FnMut(&K) -> String, to_str__1: &mut dyn FnMut(&V) -> String, cmp: &mut dyn FnMut(&K, &K) -> i32) -> String {
    let mut out = mut_str(vec!["{".to_string()]);
    let mut i = 0;
    for mut k in crate::platform_core_sorted::each_sorted_map(map).map(|__x| __x.clone()) {
        if i > 0 {
            crate::core_string::append_platform(&mut out, &(", ".to_string()));
        }
        crate::core_string::append_platform(&mut out, &(to_str(&k)));
        crate::core_string::append_platform(&mut out, &(": ".to_string()));
        crate::core_string::append_platform(&mut out, &(to_str__1(&get_platform::<K, V>(map, &k, &mut *cmp).expect("salvo: value is absent at core.sorted:148:28"))));
        i = i32::wrapping_add(i, 1);
    }
    crate::core_string::append_platform(&mut out, &("}".to_string()));
    return out;
}

pub fn eq__SortedSet_SortedSet<T: Clone>(a: &SortedSet<T>, b: &SortedSet<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> bool {
    if size__SortedSet(a) != size__SortedSet(b) {
        return false;
    }
    for mut x in crate::platform_core_sorted::each_sorted_set(a).map(|__x| __x.clone()) {
        if !contains_platform::<T>(b, &x, &mut *cmp) {
            return false;
        }
    }
    return true;
}

pub fn eq__SortedMap_SortedMap<K: Clone, V: Clone>(a: &SortedMap<K, V>, b: &SortedMap<K, V>, eq: &mut dyn FnMut(&V, &V) -> bool, cmp: &mut dyn FnMut(&K, &K) -> i32) -> bool {
    if size__SortedMap(a) != size__SortedMap(b) {
        return false;
    }
    for mut k in crate::platform_core_sorted::each_sorted_map(a).map(|__x| __x.clone()) {
        let mut theirs = get_platform::<K, V>(b, &k, &mut *cmp);
        if theirs.is_none() {
            return false;
        }
        if !eq(&get_platform::<K, V>(a, &k, &mut *cmp).expect("salvo: value is absent at core.sorted:181:16"), &theirs.unwrap().clone()) {
            return false;
        }
    }
    return true;
}

pub fn iter__SortedMap<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> MapKeyYield<K> {
    return MapKeyYield { items: keys_platform(map), at: 0 };
}

pub fn NonEmpty__SortedSet_qualifies<T: Clone>(set: &SortedSet<T>) -> bool {
    return size__SortedSet(set) > 0;
}

pub fn NonEmpty__SortedMap_qualifies<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> bool {
    return size__SortedMap(map) > 0;
}

pub fn min<T: Clone>(set: &SortedSet<T>) -> T {
    return min_platform(set).expect("salvo: value is absent at core.sorted:236:12");
}

pub fn max<T: Clone>(set: &SortedSet<T>) -> T {
    return max_platform(set).expect("salvo: value is absent at core.sorted:240:12");
}

pub fn first_key<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> K {
    return first_key_platform(map).expect("salvo: value is absent at core.sorted:244:12");
}

pub fn last_key<K: Clone, V: Clone>(map: &SortedMap<K, V>) -> K {
    return last_key_platform(map).expect("salvo: value is absent at core.sorted:248:12");
}
