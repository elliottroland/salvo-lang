use crate::std_core_iterator::Finished;
use crate::std_core_string::Mut_Str;
use crate::std_core_string::append_platform;
use crate::std_core_iterator::emitted;
use crate::std_core_iterator::finished;


/// [platform-type] The host's `Map`.
pub use crate::platform_std_core_map::Map;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq>() {} __contract::<Map<i32, i32>>(); };
const _: fn() = || { fn __each(x: &Map<i32, i32>) -> impl Iterator<Item = i32> + '_ { crate::platform_std_core_map::each(x).map(|e| e.clone()) } let _ = __each; };
const _: fn() = || { fn __each_ref(x: &Map<i32, i32>) -> impl Iterator<Item = &i32> + '_ { crate::platform_std_core_map::each(x) } fn __each_mut(x: &mut Map<i32, i32>) -> impl Iterator<Item = &mut i32> + '_ { crate::platform_std_core_map::each_mut(x) } fn __into_each(x: Map<i32, i32>) -> impl Iterator<Item = i32> { crate::platform_std_core_map::into_each(x) } let _ = (__each_ref, __each_mut, __into_each); };

pub fn Map_platform<K: Clone, V>(mut entries: Vec<(K, V)>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::std_core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::Map(entries, &mut hash, &mut eq)
}

pub fn Mut_Map_platform<K: Clone, V>(mut entries: Vec<(K, V)>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::std_core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::Mut_Map(entries, &mut hash, &mut eq)
}

pub fn map_by_platform<K: Clone, V, E: Clone>(e: &E, mut size: i32, init: &mut dyn FnMut(&E, i32) -> (K, V), hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::std_core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::map_by(size, &mut |__x0| init(e, __x0), &mut hash, &mut eq)
}

pub fn mut_map_by_platform<K: Clone, V, E: Clone>(e: &E, mut size: i32, init: &mut dyn FnMut(&E, i32) -> (K, V), hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::std_core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::mut_map_by(size, &mut |__x0| init(e, __x0), &mut hash, &mut eq)
}

pub fn to_map_platform<K: Clone, V: Clone>(pairs: &Vec<(K, V)>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::std_core_map::Map<K, V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::to_map(pairs, &mut hash, &mut eq)
}

pub fn to_map<T: Clone, K: Clone, V: Clone, E: Clone>(e: &E, items: &Vec<T>, entry: &mut dyn FnMut(&E, &T) -> (K, V), hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> crate::std_core_map::Map<K, V> {
    let mut map: crate::std_core_map::Map<K, V> = crate::std_core_map::Mut_Map_platform::<K, V>(vec![], &mut *hash, &mut *eq);
    for mut x in items.iter() {
        let mut __destructured_1: (K, V) = entry(e, x);
        let mut k: K = __destructured_1.0;
        let mut v: V = __destructured_1.1;
        crate::std_core_map::put_platform::<K, V>(&mut map, k, v, &mut *hash, &mut *eq);
    }
    return map;
}

pub fn get_platform<'a, K: Clone, V: Clone>(map: &'a crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<&'a V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::get(map, key, &mut hash, &mut eq)
}

pub fn at_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<usize> {
    crate::platform_std_core_map::slot_of(map, key, hash, eq)
}

pub fn put_platform<K: Clone, V: Clone>(map: &mut crate::std_core_map::Map<K, V>, mut key: K, mut value: V, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::put(map, key, value, &mut hash, &mut eq)
}

pub fn replace_platform<K: Clone, V>(map: &mut crate::std_core_map::Map<K, V>, mut key: K, mut value: V, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::replace(map, key, value, &mut hash, &mut eq)
}

pub fn remove_platform<K: Clone, V>(map: &mut crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> Option<V> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::remove(map, key, &mut hash, &mut eq)
}

pub fn contains_key_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> bool {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::contains_key(map, key, &mut hash, &mut eq)
}

pub fn KeyOf_qualifies<K: Clone, V: Clone>(key: &K, map: &crate::std_core_map::Map<K, V>, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> bool {
    return crate::std_core_map::contains_key_platform::<K, V>(map, key, &mut *hash, &mut *eq);
}

pub fn get_present_platform<'a, K: Clone, V: Clone>(map: &'a crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> &'a V {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_std_core_map::get_present(map, key, &mut hash, &mut eq)
}

pub fn get<'a, K: Clone, V: Clone>(map: &'a crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> &'a V {
    return crate::std_core_map::get_present_platform::<K, V>(map, key, &mut *hash, &mut *eq);
}

pub fn at<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> usize {
    return crate::std_core_map::at_present_platform(map, key, &mut *hash, &mut *eq);
}

pub fn at_present_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, key: &K, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> usize {
    crate::platform_std_core_map::slot_of(map, key, hash, eq).expect("salvo: a `KeyOf` key is present")
}

pub fn size_platform<K: Clone, V>(map: &crate::std_core_map::Map<K, V>) -> i32 {
    crate::platform_std_core_map::size(map)
}

pub fn drain<K: Clone, V, E: Clone>(e: &E, mut map: crate::std_core_map::Map<K, V>, each: &mut dyn FnMut(&E, V)) {
    crate::std_core_list::drain::<V, E>(e, crate::std_core_map::into_values_platform::<K, V>(map), &mut *each);
}

pub fn into_values_platform<K: Clone, V>(mut map: crate::std_core_map::Map<K, V>) -> Vec<V> {
    crate::platform_std_core_map::into_values(map)
}

pub fn to_str<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, to_str: &mut dyn FnMut(&K) -> String, to_str__1: &mut dyn FnMut(&V) -> String, hash: &mut dyn FnMut(&K) -> i64, eq: &mut dyn FnMut(&K, &K) -> bool) -> String {
    let mut out: String = crate::std_core_string::Mut_Str(vec![String::from("{")]);
    let mut i: i32 = 0i32;
    for mut k in crate::platform_std_core_map::each(map).map(|__x| __x.clone()) {
        if (i > 0i32) {
            crate::std_core_string::append_platform(&mut out, &String::from(", "));
        };
        crate::std_core_string::append_platform(&mut out, &to_str(&k));
        crate::std_core_string::append_platform(&mut out, &String::from(": "));
        crate::std_core_string::append_platform(&mut out, &to_str__1({
            let mut __pick1: Option<&V> = crate::std_core_map::get_platform::<K, V>(map, &k, &mut *hash, &mut *eq);
            if __pick1.is_some() {
                let mut __pick1_1 = __pick1.unwrap();
                __pick1_1
            } else {
                panic!("salvo: unreachable at std.core.map:367:43")
            }
        }));
        i = i32::wrapping_add(i, 1i32);
    }
    crate::std_core_string::append_platform(&mut out, &String::from("}"));
    return out;
}

pub fn eq<K: Clone, V: Clone>(a: &crate::std_core_map::Map<K, V>, b: &crate::std_core_map::Map<K, V>, eq: &mut dyn FnMut(&V, &V) -> bool, hash: &mut dyn FnMut(&K) -> i64, eq__1: &mut dyn FnMut(&K, &K) -> bool) -> bool {
    if (crate::std_core_map::size_platform::<K, V>(a) != crate::std_core_map::size_platform::<K, V>(b)) {
        return false;
    };
    for mut k in crate::platform_std_core_map::each(a).map(|__x| __x.clone()) {
        let mut theirs: Option<&V> = crate::std_core_map::get_platform::<K, V>(b, &k, &mut *hash, &mut *eq__1);
        if theirs.is_none() {
            return false;
        };
        let mut theirs_2 = theirs.unwrap();
        if !(eq({
            let mut __pick2: Option<&V> = crate::std_core_map::get_platform::<K, V>(a, &k, &mut *hash, &mut *eq__1);
            if __pick2.is_some() {
                let mut __pick2_1 = __pick2.unwrap();
                __pick2_1
            } else {
                panic!("salvo: unreachable at std.core.map:395:29")
            }
        }, theirs_2)) {
            return false;
        };
    }
    return true;
}

pub fn keys_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>) -> Vec<K> {
    crate::platform_std_core_map::keys(map)
}

pub fn iter<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>) -> crate::std_core_map::MapKeyIter<K> {
    return crate::std_core_map::MapKeyIter { items: crate::std_core_map::keys_platform::<K, V>(map), at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapKeyIter<K> {
    pub items: Vec<K>,
    pub at: i32,
}

impl<K: Clone + 'static + crate::wire::__Wire> crate::wire::__Wire for MapKeyIter<K> {
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

pub fn next__MapKeyIter<K: Clone>(p: &mut crate::std_core_map::MapKeyIter<K>) -> crate::unions::Union2<K, crate::std_core_iterator::Finished> {
    let mut key: Option<K> = p.items.get((p.at) as i64 as usize).cloned();
    if key.is_none() {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut key_1 = key.unwrap();
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<K>(key_1));
}

pub fn slot_count_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>) -> i32 {
    crate::platform_std_core_map::slot_count(map)
}

pub fn key_at_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, mut at: i32) -> Option<&K> {
    crate::platform_std_core_map::key_at(map, at)
}

pub fn value_at_platform<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>, mut at: i32) -> &V {
    crate::platform_std_core_map::value_at(map, at)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapEntry<'s, K, V> {
    pub key: &'s K,
    pub value: &'s V,
}

pub fn entries<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>) -> crate::std_core_map::MapEntriesIter<'_, K, V> {
    return crate::std_core_map::MapEntriesIter { map: map, at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapEntriesIter<'s, K, V> {
    pub map: &'s crate::std_core_map::Map<K, V>,
    pub at: i32,
}

pub fn next__MapEntriesIter<'a, K: Clone, V: Clone>(p: &mut crate::std_core_map::MapEntriesIter<'a, K, V>) -> crate::unions::Union2<crate::std_core_map::MapEntry<'a, K, V>, crate::std_core_iterator::Finished> {
    loop {
        if !((p.at < crate::std_core_map::slot_count_platform::<K, V>(p.map))) {
            break;
        };
        let mut here: i32 = p.at;
        p.at = i32::wrapping_add(p.at, 1i32);
        let mut key: Option<&K> = crate::std_core_map::key_at_platform::<K, V>(p.map, here);
        if !(key.is_none()) {
            let mut value: &V = crate::std_core_map::value_at_platform::<K, V>(p.map, here);
            let mut key_1 = key.unwrap();
            return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<crate::std_core_map::MapEntry<'_, K, V>>(crate::std_core_map::MapEntry { key: key_1, value: value }));
        };
    }
    return crate::unions::Union2::U2(crate::std_core_iterator::finished());
}

pub fn values<K: Clone, V: Clone>(map: &crate::std_core_map::Map<K, V>) -> crate::std_core_map::MapValuesIter<'_, K, V> {
    return crate::std_core_map::MapValuesIter { map: map, at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapValuesIter<'s, K, V> {
    pub map: &'s crate::std_core_map::Map<K, V>,
    pub at: i32,
}

pub fn next__MapValuesIter<'a, K: Clone, V: Clone>(p: &mut crate::std_core_map::MapValuesIter<'a, K, V>) -> crate::unions::Union2<&'a V, crate::std_core_iterator::Finished> {
    loop {
        if !((p.at < crate::std_core_map::slot_count_platform::<K, V>(p.map))) {
            break;
        };
        let mut here: i32 = p.at;
        p.at = i32::wrapping_add(p.at, 1i32);
        if !((crate::std_core_map::key_at_platform::<K, V>(p.map, here)).is_none()) {
            let mut value: &V = crate::std_core_map::value_at_platform::<K, V>(p.map, here);
            return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<&V>(value));
        };
    }
    return crate::unions::Union2::U2(crate::std_core_iterator::finished());
}
