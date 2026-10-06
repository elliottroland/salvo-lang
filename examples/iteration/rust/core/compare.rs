use crate::unions::*;
use crate::core_index::Idx__Int_qualifies;
use crate::core_list::Enumerated;
use crate::core_list::List;
use crate::core_list::__Iter_enumerate_List;
use crate::core_list::enumerate;
use crate::core_list::get;
use crate::core_list::next__Iter_enumerate_List;
use crate::core_string::Str;

pub fn eq__List_List<T: Clone>(a: &Vec<T>, b: &Vec<T>, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    if crate::core_list::size_platform(a) != crate::core_list::size_platform(b) {
        return false;
    }
    let mut __loop1_pass = enumerate(a);
    while let Union2::U1(mut p) = next__Iter_enumerate_List(&mut __loop1_pass) {
        let mut j = &p.index;
        if Idx__Int_qualifies(*j, &*b, &mut |__i0| crate::core_list::size_platform(&__i0)) {
            if !eq(&p.elem, &get(b, j)) {
                return false;
            }
        }
    }
    return true;
}

pub fn cmp__List_List<T: Clone>(a: &Vec<T>, b: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> i32 {
    let mut __loop2_pass = enumerate(a);
    while let Union2::U1(mut p) = next__Iter_enumerate_List(&mut __loop2_pass) {
        let mut j = &p.index;
        if Idx__Int_qualifies(*j, &*b, &mut |__i0| crate::core_list::size_platform(&__i0)) {
            let mut c = cmp(&p.elem, &get(b, j));
            if c != 0 {
                return c;
            }
        } else {
            return 1;
        }
    }
    if crate::core_list::size_platform(a) < crate::core_list::size_platform(b) {
        return -1;
    }
    return 0;
}

pub fn hash__List<T: Clone>(value: &Vec<T>, hash: &mut dyn FnMut(&T) -> i64) -> i64 {
    let mut h = 7i64;
    for mut x in crate::platform_core_list::each(value).map(|__x| __x.clone()) {
        h = mix_hash(h, hash(&x));
    }
    return h;
}

pub fn eq__TupleAB_TupleAB<A: Clone, B: Clone>(a: &(A, B), b: &(A, B), eq: &mut dyn FnMut(&A, &A) -> bool, eq__1: &mut dyn FnMut(&B, &B) -> bool) -> bool {
    return eq(&a.0, &b.0) && eq__1(&a.1, &b.1);
}

pub fn cmp__TupleAB_TupleAB<A: Clone, B: Clone>(a: &(A, B), b: &(A, B), cmp: &mut dyn FnMut(&A, &A) -> i32, cmp__1: &mut dyn FnMut(&B, &B) -> i32) -> i32 {
    let mut c = cmp(&a.0, &b.0);
    if c != 0 {
        return c;
    }
    return cmp__1(&a.1, &b.1);
}

pub fn hash__TupleAB<A: Clone, B: Clone>(value: &(A, B), hash: &mut dyn FnMut(&A) -> i64, hash__1: &mut dyn FnMut(&B) -> i64) -> i64 {
    return mix_hash(mix_hash(7i64, hash(&value.0)), hash__1(&value.1));
}

pub fn eq__TupleABC_TupleABC<A: Clone, B: Clone, C: Clone>(a: &(A, B, C), b: &(A, B, C), eq: &mut dyn FnMut(&A, &A) -> bool, eq__1: &mut dyn FnMut(&B, &B) -> bool, eq__2: &mut dyn FnMut(&C, &C) -> bool) -> bool {
    return eq(&a.0, &b.0) && eq__1(&a.1, &b.1) && eq__2(&a.2, &b.2);
}

pub fn cmp__TupleABC_TupleABC<A: Clone, B: Clone, C: Clone>(a: &(A, B, C), b: &(A, B, C), cmp: &mut dyn FnMut(&A, &A) -> i32, cmp__1: &mut dyn FnMut(&B, &B) -> i32, cmp__2: &mut dyn FnMut(&C, &C) -> i32) -> i32 {
    let mut c = cmp(&a.0, &b.0);
    if c != 0 {
        return c;
    }
    let mut d = cmp__1(&a.1, &b.1);
    if d != 0 {
        return d;
    }
    return cmp__2(&a.2, &b.2);
}

pub fn hash__TupleABC<A: Clone, B: Clone, C: Clone>(value: &(A, B, C), hash: &mut dyn FnMut(&A) -> i64, hash__1: &mut dyn FnMut(&B) -> i64, hash__2: &mut dyn FnMut(&C) -> i64) -> i64 {
    return mix_hash(mix_hash(mix_hash(7i64, hash(&value.0)), hash__1(&value.1)), hash__2(&value.2));
}

pub fn mix_hash(seed: i64, value: i64) -> i64 {
    return i64::wrapping_add(i64::wrapping_mul(seed, 31i64), value);
}
