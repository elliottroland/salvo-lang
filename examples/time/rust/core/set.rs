use crate::core_iterator::Finished;
use crate::core_string::append_platform;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_string::mut_str;


/// [platform-type] The host's `Set`.
pub use crate::platform_core_set::Set;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + std::fmt::Debug + PartialEq + Eq>() {} __contract::<Set<i32>>(); };
const _: fn() = || { fn __each(x: &Set<i32>) -> impl Iterator<Item = i32> + '_ { crate::platform_core_set::each(x).map(|e| e.clone()) } let _ = __each; };
const _: fn() = || { fn __each_ref(x: &Set<i32>) -> impl Iterator<Item = &i32> + '_ { crate::platform_core_set::each(x) } fn __each_mut(x: &mut Set<i32>) -> impl Iterator<Item = &mut i32> + '_ { crate::platform_core_set::each_mut(x) } fn __into_each(x: Set<i32>) -> impl Iterator<Item = i32> { crate::platform_core_set::into_each(x) } let _ = (__each_ref, __each_mut, __into_each); };

pub fn set_of_platform<T: Clone>(mut elems: Vec<T>, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> crate::core_set::Set<T> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::set_of(elems, &mut hash, &mut eq)
}

pub fn mut_set_of_platform<T: Clone>(mut elems: Vec<T>, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> crate::core_set::Set<T> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::mut_set_of(elems, &mut hash, &mut eq)
}

pub fn set_by_platform<T: Clone>(mut size: i32, init: &mut dyn FnMut(i32) -> T, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> crate::core_set::Set<T> {
    let mut init = init;
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::set_by(size, &mut init, &mut hash, &mut eq)
}

pub fn mut_set_by_platform<T: Clone>(mut size: i32, init: &mut dyn FnMut(i32) -> T, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> crate::core_set::Set<T> {
    let mut init = init;
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::mut_set_by(size, &mut init, &mut hash, &mut eq)
}

pub fn to_set_platform<T: Clone>(list: &Vec<T>, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> crate::core_set::Set<T> {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::to_set(list, &mut hash, &mut eq)
}

pub fn add_platform<T: Clone>(set: &mut crate::core_set::Set<T>, mut elem: T, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::add(set, elem, &mut hash, &mut eq)
}

pub fn remove_platform<T: Clone>(set: &mut crate::core_set::Set<T>, elem: &T, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::remove(set, elem, &mut hash, &mut eq)
}

pub fn contains_platform<T: Clone>(set: &crate::core_set::Set<T>, elem: &T, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    let mut hash = hash;
    let mut eq = eq;
    crate::platform_core_set::contains(set, elem, &mut hash, &mut eq)
}

pub fn size_platform<T: Clone>(set: &crate::core_set::Set<T>) -> i32 {
    crate::platform_core_set::size(set)
}

pub fn to_str<T: Clone>(set: &crate::core_set::Set<T>, to_str: &mut dyn FnMut(&T) -> String) -> String {
    let mut out: String = crate::core_string::mut_str(vec![String::from("{")]);
    let mut i: i32 = 0i32;
    for mut x in crate::platform_core_set::each(set).map(|__x| __x.clone()) {
        if (i > 0i32) {
            crate::core_string::append_platform(&mut out, &String::from(", "));
        };
        crate::core_string::append_platform(&mut out, &to_str(&x));
        i = i32::wrapping_add(i, 1i32);
    }
    crate::core_string::append_platform(&mut out, &String::from("}"));
    return out;
}

pub fn eq<T: Clone>(a: &crate::core_set::Set<T>, b: &crate::core_set::Set<T>, hash: &mut dyn FnMut(&T) -> i64, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    if (crate::core_set::size_platform::<T>(a) != crate::core_set::size_platform::<T>(b)) {
        return false;
    };
    for mut x in crate::platform_core_set::each(a).map(|__x| __x.clone()) {
        if !(crate::core_set::contains_platform::<T>(b, &x, &mut *hash, &mut *eq)) {
            return false;
        };
    }
    return true;
}

pub fn to_list_platform<T: Clone>(set: &crate::core_set::Set<T>) -> Vec<T> {
    crate::platform_core_set::to_list(set)
}

pub fn iter<T: Clone>(set: &crate::core_set::Set<T>) -> crate::core_set::SetYield<T> {
    return crate::core_set::SetYield { items: crate::core_set::to_list_platform::<T>(set), at: 0i32 };
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetYield<T> {
    pub items: Vec<T>,
    pub at: i32,
}

impl<T: Clone + 'static + crate::wire::__Wire> crate::wire::__Wire for SetYield<T> {
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

pub fn next<T: Clone>(p: &mut crate::core_set::SetYield<T>) -> crate::unions::Union2<T, crate::core_iterator::Finished> {
    let mut elem: Option<T> = p.items.get((p.at) as i64 as usize).cloned();
    if elem.is_none() {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    p.at = i32::wrapping_add(p.at, 1i32);
    let mut elem_1 = elem.unwrap();
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<T>(elem_1));
}

pub fn NonEmpty__Set_qualifies<T: Clone>(set: &crate::core_set::Set<T>) -> bool {
    return (crate::core_set::size_platform::<T>(set) > 0i32);
}
