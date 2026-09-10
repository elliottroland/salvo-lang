use crate::core_iterator::*;
use crate::core_list::*;
use crate::unions::*;

pub fn map<It: Clone + 'static, T: Clone + 'static, U: Clone + 'static>(it: &mut It, f: &mut impl FnMut(&T) -> U, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<U> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        out.push(f(&x));
    }
    return out;
}

pub fn filter<It: Clone + 'static, T: Clone + 'static>(it: &mut It, keep: &mut impl FnMut(&T) -> bool, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<T> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        if keep(&x) {
            out.push(x);
        }
    }
    return out;
}

pub fn reduce<It: Clone + 'static, T: Clone + 'static, A: Clone + 'static>(it: &mut It, init: A, f: &mut impl FnMut(&A, &T) -> A, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> A {
    let mut acc = init.clone();
    while let Union2::U1(mut x) = next(it) {
        acc = f(&acc, &x);
    }
    return acc;
}

pub fn map_to<D: Clone + 'static, It: Clone + 'static, T: Clone + 'static, U: Clone + 'static>(mut dest: D, it: &mut It, f: &mut impl FnMut(&T) -> U, add: &mut dyn FnMut(&mut D, U), next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> D {
    while let Union2::U1(mut x) = next(it) {
        add(&mut dest, f(&x));
    }
    return dest;
}

pub fn filter_to<D: Clone + 'static, It: Clone + 'static, T: Clone + 'static>(mut dest: D, it: &mut It, keep: &mut impl FnMut(&T) -> bool, add: &mut dyn FnMut(&mut D, T), next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> D {
    while let Union2::U1(mut x) = next(it) {
        if keep(&x) {
            add(&mut dest, x);
        }
    }
    return dest;
}
