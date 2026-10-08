use crate::core_list::Enumerated;
use crate::core_iterator::Finished;
use crate::core_index::Idx__Int_qualifies;
use crate::core_list::enumerate;
use crate::core_list::get;
use crate::core_list::next__Iter_enumerate_List;
use crate::core_list::size_platform;


pub fn eq__List_List<T: Clone>(a: &Vec<T>, b: &Vec<T>, eq: &mut dyn FnMut(&T, &T) -> bool) -> bool {
    if !(((crate::core_list::size_platform::<T>(a)) == (crate::core_list::size_platform::<T>(b)))) {
        return false;
    };
    let mut __pass_1: crate::core_list::__Iter_enumerate_List<'_, T> = crate::core_list::enumerate::<T>(a);
    loop {
        let mut __step_2: crate::unions::Union2<crate::core_list::Enumerated<'_, T>, crate::core_iterator::Finished> = crate::core_list::next__Iter_enumerate_List(&mut __pass_1);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut p: crate::core_list::Enumerated<'_, T> = __emitted_3;
            let mut j: i32 = p.index;
            if crate::core_index::Idx__Int_qualifies(j, b, &mut |__a0| crate::core_list::size_platform(__a0)) {
                if !(eq(p.elem, crate::core_list::get::<T>(b, j))) {
                    return false;
                };
            };
        } else {
            break;
        };
    }
    return true;
}

pub fn cmp__List_List<T: Clone>(a: &Vec<T>, b: &Vec<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> i32 {
    let mut __pass_1: crate::core_list::__Iter_enumerate_List<'_, T> = crate::core_list::enumerate::<T>(a);
    loop {
        let mut __step_2: crate::unions::Union2<crate::core_list::Enumerated<'_, T>, crate::core_iterator::Finished> = crate::core_list::next__Iter_enumerate_List(&mut __pass_1);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut p: crate::core_list::Enumerated<'_, T> = __emitted_3;
            let mut j: i32 = p.index;
            if crate::core_index::Idx__Int_qualifies(j, b, &mut |__a0| crate::core_list::size_platform(__a0)) {
                let mut c: i32 = cmp(p.elem, crate::core_list::get::<T>(b, j));
                if !(((c) == (0i32))) {
                    return c;
                };
            } else {
                return 1i32;
            };
        } else {
            break;
        };
    }
    if (crate::core_list::size_platform::<T>(a) < crate::core_list::size_platform::<T>(b)) {
        return i32::wrapping_neg(1i32);
    };
    return 0i32;
}

pub fn hash__List<T: Clone>(value: &Vec<T>, hash: &mut dyn FnMut(&T) -> i64) -> i64 {
    let mut h: i64 = 7i64;
    for mut x in value.iter() {
        h = crate::core_compare::mix_hash(h, hash(x));
    }
    return h;
}

pub fn eq__TupleAB_TupleAB<A: Clone, B: Clone>(a: &(A, B), b: &(A, B), eq: &mut dyn FnMut(&A, &A) -> bool, eq__1: &mut dyn FnMut(&B, &B) -> bool) -> bool {
    return (eq(&a.0, &b.0) && eq__1(&a.1, &b.1));
}

pub fn cmp__TupleAB_TupleAB<A: Clone, B: Clone>(a: &(A, B), b: &(A, B), cmp: &mut dyn FnMut(&A, &A) -> i32, cmp__1: &mut dyn FnMut(&B, &B) -> i32) -> i32 {
    let mut c: i32 = cmp(&a.0, &b.0);
    if !(((c) == (0i32))) {
        return c;
    };
    return cmp__1(&a.1, &b.1);
}

pub fn hash__TupleAB<A: Clone, B: Clone>(value: &(A, B), hash: &mut dyn FnMut(&A) -> i64, hash__1: &mut dyn FnMut(&B) -> i64) -> i64 {
    return crate::core_compare::mix_hash(crate::core_compare::mix_hash(7i64, hash(&value.0)), hash__1(&value.1));
}

pub fn eq__TupleABC_TupleABC<A: Clone, B: Clone, C: Clone>(a: &(A, B, C), b: &(A, B, C), eq: &mut dyn FnMut(&A, &A) -> bool, eq__1: &mut dyn FnMut(&B, &B) -> bool, eq__2: &mut dyn FnMut(&C, &C) -> bool) -> bool {
    return ((eq(&a.0, &b.0) && eq__1(&a.1, &b.1)) && eq__2(&a.2, &b.2));
}

pub fn cmp__TupleABC_TupleABC<A: Clone, B: Clone, C: Clone>(a: &(A, B, C), b: &(A, B, C), cmp: &mut dyn FnMut(&A, &A) -> i32, cmp__1: &mut dyn FnMut(&B, &B) -> i32, cmp__2: &mut dyn FnMut(&C, &C) -> i32) -> i32 {
    let mut c: i32 = cmp(&a.0, &b.0);
    if !(((c) == (0i32))) {
        return c;
    };
    let mut d: i32 = cmp__1(&a.1, &b.1);
    if !(((d) == (0i32))) {
        return d;
    };
    return cmp__2(&a.2, &b.2);
}

pub fn hash__TupleABC<A: Clone, B: Clone, C: Clone>(value: &(A, B, C), hash: &mut dyn FnMut(&A) -> i64, hash__1: &mut dyn FnMut(&B) -> i64, hash__2: &mut dyn FnMut(&C) -> i64) -> i64 {
    return crate::core_compare::mix_hash(crate::core_compare::mix_hash(crate::core_compare::mix_hash(7i64, hash(&value.0)), hash__1(&value.1)), hash__2(&value.2));
}

pub fn mix_hash(mut seed: i64, mut value: i64) -> i64 {
    return i64::wrapping_add(i64::wrapping_mul(seed, 31i64), value);
}
