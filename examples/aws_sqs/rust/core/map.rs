use crate::core_iterator::Finished;
use crate::core_string::append_platform;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_string::mut_str;


/// [platform-type] The host's `Map`.
pub use crate::platform_core_map::Map;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq>() {} __contract::<Map<i32, i32>>(); };
const _: fn() = || { fn __each(x: &Map<i32, i32>) -> impl Iterator<Item = i32> + '_ { crate::platform_core_map::each(x).map(|e| e.clone()) } let _ = __each; };
const _: fn() = || { fn __each_ref(x: &Map<i32, i32>) -> impl Iterator<Item = &i32> + '_ { crate::platform_core_map::each(x) } fn __each_mut(x: &mut Map<i32, i32>) -> impl Iterator<Item = &mut i32> + '_ { crate::platform_core_map::each_mut(x) } fn __into_each(x: Map<i32, i32>) -> impl Iterator<Item = i32> { crate::platform_core_map::into_each(x) } let _ = (__each_ref, __each_mut, __into_each); };

pub fn map_of_platform<K: Clone, V>(mut entries: Vec<(K, V)>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::map_of(entries, &mut hash, &mut eq)
}

pub fn mut_map_of_platform<K: Clone, V>(mut entries: Vec<(K, V)>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::mut_map_of(entries, &mut hash, &mut eq)
}

pub fn map_by_platform<K: Clone, V>(mut size: i32, init: &mut dyn FnMut(i32) -> (K, V), hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::core_map::Map<K, V> {
    let mut init = init;
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::map_by(size, &mut init, &mut hash, &mut eq)
}

pub fn mut_map_by_platform<K: Clone, V>(mut size: i32, init: &mut dyn FnMut(i32) -> (K, V), hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::core_map::Map<K, V> {
    let mut init = init;
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::mut_map_by(size, &mut init, &mut hash, &mut eq)
}

pub fn to_map_platform<K: Clone, V: Clone>(pairs: &Vec<(K, V)>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::to_map(pairs, &mut hash, &mut eq)
}

pub fn to_map<T: Clone, K: Clone, V: Clone>(items: &Vec<T>, entry: &mut dyn FnMut(&T) -> (K, V), hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::core_map::Map<K, V> {
    let mut map: crate::core_map::Map<K, V> = crate::core_map::mut_map_of_platform::<K, V>(vec![], &mut *hash, &mut *eq);
    for mut x in items.iter() {
        let mut __destructured_1: (K, V) = entry(x);
        let mut k: K = __destructured_1.0;
        let mut v: V = __destructured_1.1;
        crate::core_map::put_platform::<K, V>(&mut map, k, v, &mut *hash, &mut *eq);
    }
    return map;
}

pub fn get_platform<'a, K: Clone, V: Clone>(map: &'a crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<&'a V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::get(map, key, &mut hash, &mut eq)
}

pub fn get_platform__loc<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<usize> {
    crate::platform_core_map::slot_of(map, key, hash, eq)
}

pub fn at<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<usize> {
    return crate::core_map::get_platform__loc(map, key, &mut *hash, &mut *eq);
}

pub fn put_platform<K: Clone, V: Clone>(map: &mut crate::core_map::Map<K, V>, mut key: K, mut value: V, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::put(map, key, value, &mut hash, &mut eq)
}

pub fn replace_platform<K: Clone, V>(map: &mut crate::core_map::Map<K, V>, mut key: K, mut value: V, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::replace(map, key, value, &mut hash, &mut eq)
}

pub fn remove_platform<K: Clone, V>(map: &mut crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::remove(map, key, &mut hash, &mut eq)
}

pub fn contains_key_platform<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> bool {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::contains_key(map, key, &mut hash, &mut eq)
}

pub fn KeyOf_qualifies<K: Clone, V: Clone>(key: &K, map: &crate::core_map::Map<K, V>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> bool {
    return crate::core_map::contains_key_platform::<K, V>(map, key, &mut *hash, &mut *eq);
}

pub fn get_present_platform<'a, K: Clone, V: Clone>(map: &'a crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> &'a V {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_map::get_present(map, key, &mut hash, &mut eq)
}

pub fn get<'a, K: Clone, V: Clone>(map: &'a crate::core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> &'a V {
    return crate::core_map::get_present_platform::<K, V>(map, key, &mut *hash, &mut *eq);
}

pub fn size_platform<K: Clone, V>(map: &crate::core_map::Map<K, V>) -> i32 {
    crate::platform_core_map::size(map)
}

pub fn drain<K: Clone, V>(mut map: crate::core_map::Map<K, V>, each: &mut dyn FnMut(V)) {
    crate::core_list::drain::<V>(crate::core_map::into_values_platform::<K, V>(map), &mut *each);
}

pub fn into_values_platform<K: Clone, V>(mut map: crate::core_map::Map<K, V>) -> Vec<V> {
    crate::platform_core_map::into_values(map)
}

pub fn to_str<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>, to_str: &mut dyn FnMut(&K) -> String, to_str__1: &mut dyn FnMut(&V) -> String, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> String {
    let mut out: String = crate::core_string::mut_str(vec![String::from("{")]);
    let mut i: i32 = 0i32;
    for mut k in crate::platform_core_map::each(map).map(|__x| __x.clone()) {
        if (i > 0i32) {
            crate::core_string::append_platform(&mut out, &String::from(", "));
        };
        crate::core_string::append_platform(&mut out, &to_str(&k));
        crate::core_string::append_platform(&mut out, &String::from(": "));
        crate::core_string::append_platform(&mut out, &to_str__1({
            let mut __nn_1: Option<&V> = crate::core_map::get_platform::<K, V>(map, &k, &mut *hash, &mut *eq);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at core.map:173:28");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }));
        i = i32::wrapping_add(i, 1i32);
    }
    crate::core_string::append_platform(&mut out, &String::from("}"));
    return out;
}

pub fn eq<K: Clone, V: Clone>(a: &crate::core_map::Map<K, V>, b: &crate::core_map::Map<K, V>, eq: &mut dyn FnMut(&V, &V) -> bool, hash: &mut dyn FnMut(&K) -> i64, eq__1: &mut dyn FnMut(&K, &K) -> bool) -> bool {
    if (crate::core_map::size_platform::<K, V>(a) != crate::core_map::size_platform::<K, V>(b)) {
        return false;
    };
    for mut k in crate::platform_core_map::each(a).map(|__x| __x.clone()) {
        let mut theirs: Option<&V> = crate::core_map::get_platform::<K, V>(b, &k, &mut *hash, &mut *eq__1);
        if theirs.is_none() {
            return false;
        };
        let mut theirs_3 = theirs.unwrap();
        if !(eq({
            let mut __nn_1: Option<&V> = crate::core_map::get_platform::<K, V>(a, &k, &mut *hash, &mut *eq__1);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at core.map:195:16");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }, theirs_3)) {
            return false;
        };
    }
    return true;
}

pub fn keys_platform<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>) -> Vec<K> {
    crate::platform_core_map::keys(map)
}

pub fn iter<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>) -> crate::core_map::MapKeyYield<K> {
    return crate::core_map::MapKeyYield { items: crate::core_map::keys_platform::<K, V>(map), at: 0i32 };
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

pub fn next__MapKeyYield<K: Clone>(p: &mut crate::core_map::MapKeyYield<K>) -> crate::unions::Union2<K, crate::core_iterator::Finished> {
    let mut key: Option<K> = p.items.get((p.at) as i64 as usize).cloned();
    if key.is_none() {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut key_1 = key.unwrap();
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<K>(key_1));
}

pub fn slot_count_platform<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>) -> i32 {
    crate::platform_core_map::slot_count(map)
}

pub fn key_at_platform<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>, mut at: i32) -> Option<&K> {
    crate::platform_core_map::key_at(map, at)
}

pub fn value_at_platform<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>, mut at: i32) -> &V {
    crate::platform_core_map::value_at(map, at)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapEntry<'s, K, V> {
    pub key: &'s K,
    pub value: &'s V,
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_entries_Map<'s, K, V> {
    pub map: &'s crate::core_map::Map<K, V>,
    pub at: i32,
}

pub fn entries<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>) -> crate::core_map::__Iter_entries_Map<'_, K, V> {
    return crate::core_map::__Iter_entries_Map { map: map, at: 0i32 };
}

pub fn next__Iter_entries_Map<'a, K: Clone, V: Clone>(__p: &mut crate::core_map::__Iter_entries_Map<'a, K, V>) -> crate::unions::Union2<crate::core_map::MapEntry<'a, K, V>, crate::core_iterator::Finished> {
    loop {
        if !((__p.at < crate::core_map::slot_count_platform::<K, V>(__p.map))) {
            break;
        };
        let mut here: i32 = __p.at;
        __p.at = i32::wrapping_add(__p.at, 1i32);
        let mut key: Option<&K> = crate::core_map::key_at_platform::<K, V>(__p.map, here);
        if !(key.is_none()) {
            let mut value: &V = crate::core_map::value_at_platform::<K, V>(__p.map, here);
            let mut key_1 = key.unwrap();
            return crate::unions::Union2::U1(crate::core_iterator::emitted::<crate::core_map::MapEntry<'_, K, V>>(crate::core_map::MapEntry { key: key_1, value: value }));
        };
    }
    return crate::unions::Union2::U2(crate::core_iterator::finished());
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_values_Map<'s, K, V> {
    pub map: &'s crate::core_map::Map<K, V>,
    pub at: i32,
}

pub fn values<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>) -> crate::core_map::__Iter_values_Map<'_, K, V> {
    return crate::core_map::__Iter_values_Map { map: map, at: 0i32 };
}

pub fn next__Iter_values_Map<'a, K: Clone, V: Clone>(__p: &mut crate::core_map::__Iter_values_Map<'a, K, V>) -> crate::unions::Union2<&'a V, crate::core_iterator::Finished> {
    loop {
        if !((__p.at < crate::core_map::slot_count_platform::<K, V>(__p.map))) {
            break;
        };
        let mut here: i32 = __p.at;
        __p.at = i32::wrapping_add(__p.at, 1i32);
        if !((crate::core_map::key_at_platform::<K, V>(__p.map, here)).is_none()) {
            let mut value: &V = crate::core_map::value_at_platform::<K, V>(__p.map, here);
            return crate::unions::Union2::U1(crate::core_iterator::emitted::<&V>(value));
        };
    }
    return crate::unions::Union2::U2(crate::core_iterator::finished());
}

pub fn NonEmpty__Map_qualifies<K: Clone, V: Clone>(map: &crate::core_map::Map<K, V>) -> bool {
    return (crate::core_map::size_platform::<K, V>(map) > 0i32);
}
