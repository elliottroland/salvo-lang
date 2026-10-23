use crate::core_list::NonEmpty__List_qualifies;
use crate::core_list::add_platform;


pub fn ok<T>(mut value: T) -> T {
    return value;
}

pub fn err<T>(mut value: T) -> T {
    return value;
}

pub fn Ok<T>(mut value: T) -> T {
    return value;
}

pub fn Err<T>(mut value: T) -> T {
    return value;
}

pub fn errs<E: Clone>(mut first: E) -> Vec<E> {
    return vec![first];
}

pub fn Errs<T>(mut value: T) -> T {
    return value;
}

pub fn both<A: Clone, B: Clone, E: Clone>(mut a: crate::unions::Union2<A, Vec<E>>, mut b: crate::unions::Union2<B, Vec<E>>, copy: &mut dyn FnMut(&E) -> E) -> crate::unions::Union2<(A, B), Vec<E>> {
    return if matches!(a, crate::unions::Union2::U1(_)) {
        let mut a_1 = match &a { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        if matches!(b, crate::unions::Union2::U1(_)) {
            let mut b_2 = match &b { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::Ok::<(A, B)>((a_1.clone(), b_2.clone())));
        } else if matches!(b, crate::unions::Union2::U2(_)) {
            let mut b_3 = match b { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::Errs::<Vec<E>>(b_3));
        } else {
            panic!("salvo: unhandled union arm at core.result:70:13");
        }
    } else if matches!(a, crate::unions::Union2::U2(_)) {
        let mut a_4 = match a { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        if matches!(b, crate::unions::Union2::U1(_)) {
            let mut b_5 = match &b { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::Errs::<Vec<E>>(a_4));
        } else if matches!(b, crate::unions::Union2::U2(_)) {
            let mut b_6 = match b { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::Errs::<Vec<E>>(crate::core_result::joined::<E>(a_4, b_6, &mut *copy)));
        } else {
            panic!("salvo: unhandled union arm at core.result:76:13");
        }
    } else {
        panic!("salvo: unhandled union arm at core.result:68:5");
    };
}

pub fn map2<A: Clone, B: Clone, C: Clone, E: Clone>(mut a: crate::unions::Union2<A, Vec<E>>, mut b: crate::unions::Union2<B, Vec<E>>, f: &mut dyn FnMut(A, B) -> C, copy: &mut dyn FnMut(&E) -> E) -> crate::unions::Union2<C, Vec<E>> {
    return if matches!(a, crate::unions::Union2::U1(_)) {
        let mut a_1 = match a { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        if matches!(b, crate::unions::Union2::U1(_)) {
            let mut b_2 = match b { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::Ok::<C>(f(a_1, b_2)));
        } else if matches!(b, crate::unions::Union2::U2(_)) {
            let mut b_3 = match b { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::Errs::<Vec<E>>(b_3));
        } else {
            panic!("salvo: unhandled union arm at core.result:90:13");
        }
    } else if matches!(a, crate::unions::Union2::U2(_)) {
        let mut a_4 = match a { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        if matches!(b, crate::unions::Union2::U1(_)) {
            let mut b_5 = match &b { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::Errs::<Vec<E>>(a_4));
        } else if matches!(b, crate::unions::Union2::U2(_)) {
            let mut b_6 = match b { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::Errs::<Vec<E>>(crate::core_result::joined::<E>(a_4, b_6, &mut *copy)));
        } else {
            panic!("salvo: unhandled union arm at core.result:96:13");
        }
    } else {
        panic!("salvo: unhandled union arm at core.result:88:5");
    };
}

pub fn joined<X: Clone>(mut a: Vec<X>, mut b: Vec<X>, copy: &mut dyn FnMut(&X) -> X) -> Vec<X> {
    let mut errors: Vec<X> = vec![];
    for mut e in a.iter() {
        crate::core_list::add_platform::<X>(&mut errors, copy(e));
    }
    for mut e in b.iter() {
        crate::core_list::add_platform::<X>(&mut errors, copy(e));
    }
    if crate::core_list::NonEmpty__List_qualifies(&errors) {
        return errors;
    };
    return panic!("salvo: two non-empty lists joined are not empty at core.result:118:5");
}
