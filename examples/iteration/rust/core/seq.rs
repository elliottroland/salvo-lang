use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_string::*;
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

#[derive(Clone)]
pub struct MapYield<It: Clone + 'static, T: Clone + 'static, U: Clone + 'static> {
    pub source: It,
    pub f: std::rc::Rc<dyn Fn(&T) -> U>,
    pub step: std::rc::Rc<dyn Fn(&mut It) -> Union2<T, Finished>>,
}

impl<It: Clone + 'static + std::fmt::Debug, T: Clone + 'static + std::fmt::Debug, U: Clone + 'static + std::fmt::Debug> std::fmt::Debug for MapYield<It, T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapYield")
            .field("source", &self.source)
            .field("f", &"<fn>")
            .field("step", &"<fn>")
            .finish()
    }
}

pub fn map_lazy<It: Clone + 'static, T: Clone + 'static, U: Clone + 'static>(mut it: It, f: impl Fn(&T) -> U + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + 'static) -> MapYield<It, T, U> {
    return MapYield { source: it, f: std::rc::Rc::new(f), step: std::rc::Rc::new(next) };
}

pub fn next__3<It: Clone + 'static, T: Clone + 'static, U: Clone + 'static>(pass: &mut MapYield<It, T, U>) -> Union2<U, Finished> {
    let mut advance = pass.step.clone();
    let mut step = advance(&mut pass.source);
    match step {
        Union2::U1(_) => {
            let mut f = pass.f.clone();
            return Union2::<U, Finished>::U1(emitted(f(&(step.u1().clone()))));
        }
        Union2::U2(_) => {
            return Union2::<U, Finished>::U2(finished());
        }
    }
}

#[derive(Clone)]
pub struct FilterYield<It: Clone + 'static, T: Clone + 'static> {
    pub source: It,
    pub keep: std::rc::Rc<dyn Fn(&T) -> bool>,
    pub step: std::rc::Rc<dyn Fn(&mut It) -> Union2<T, Finished>>,
}

impl<It: Clone + 'static + std::fmt::Debug, T: Clone + 'static + std::fmt::Debug> std::fmt::Debug for FilterYield<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterYield")
            .field("source", &self.source)
            .field("keep", &"<fn>")
            .field("step", &"<fn>")
            .finish()
    }
}

pub fn filter_lazy<It: Clone + 'static, T: Clone + 'static>(mut it: It, keep: impl Fn(&T) -> bool + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + 'static) -> FilterYield<It, T> {
    return FilterYield { source: it, keep: std::rc::Rc::new(keep), step: std::rc::Rc::new(next) };
}

pub fn next__4<It: Clone + 'static, T: Clone + 'static>(pass: &mut FilterYield<It, T>) -> Union2<T, Finished> {
    let mut advance = pass.step.clone();
    let mut keep = pass.keep.clone();
    let mut going = true;
    while going {
        let mut step = advance(&mut pass.source);
        match step {
            Union2::U1(_) => {
                if keep(&(step.u1().clone())) {
                    return Union2::<T, Finished>::U1(emitted(step.u1().clone()));
                }
            }
            Union2::U2(_) => {
                going = false;
            }
        }
    }
    return Union2::<T, Finished>::U2(finished());
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
