use crate::core_array::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_range::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn map<It: Clone, T: Clone, U: Clone>(it: &mut It, f: &mut impl FnMut(&T) -> U, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<U> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        out.push(f(&x));
    }
    return out;
}

pub fn filter<It: Clone, T: Clone>(it: &mut It, keep: &mut impl FnMut(&T) -> bool, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<T> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        if keep(&x) {
            out.push(x);
        }
    }
    return out;
}

pub fn reduce<It: Clone, T: Clone, A: Clone>(it: &mut It, init: &A, f: &mut impl FnMut(&A, &T) -> A, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> A {
    let mut acc = init.clone();
    while let Union2::U1(mut x) = next(it) {
        acc = f(&acc, &x);
    }
    return acc;
}

pub fn map_to<D: Clone, It: Clone, T: Clone, U: Clone>(mut dest: D, it: &mut It, f: &mut impl FnMut(&T) -> U, add: &mut dyn FnMut(&mut D, U), next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> D {
    while let Union2::U1(mut x) = next(it) {
        add(&mut dest, f(&x));
    }
    return dest;
}

pub fn filter_to<D: Clone, It: Clone, T: Clone>(mut dest: D, it: &mut It, keep: &mut impl FnMut(&T) -> bool, add: &mut dyn FnMut(&mut D, T), copy: &mut dyn FnMut(&T) -> T, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> D {
    while let Union2::U1(mut x) = next(it) {
        if keep(&x) {
            add(&mut dest, copy(&x));
        }
    }
    return dest;
}

pub fn drop<T: Clone>(value: T) {
}

#[derive(Clone)]
pub struct Take<It: Clone, T: Clone> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub left: i32,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for Take<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Take")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("left", &self.left)
            .finish()
    }
}

pub fn next__13<It: Clone, T: Clone>(t: &mut Take<It, T>) -> Union2<T, Finished> {
    if t.left <= 0 {
        return Union2::<T, Finished>::U2(finished());
    }
    t.left = t.left - 1;
    let mut step = &t.step;
    return step(&mut t.src);
}

pub fn take<It: Clone, T: Clone>(it: It, n: i32, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> Take<It, T> {
    return Take { src: it, step: std::sync::Arc::new(next), left: n };
}

#[derive(Clone)]
pub struct TakeWhile<It: Clone, T: Clone> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub keep: std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>,
    pub done: bool,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for TakeWhile<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakeWhile")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("keep", &"<fn>")
            .field("done", &self.done)
            .finish()
    }
}

pub fn next__14<It: Clone, T: Clone>(t: &mut TakeWhile<It, T>) -> Union2<T, Finished> {
    if t.done {
        return Union2::<T, Finished>::U2(finished());
    }
    let mut step = &t.step;
    let mut x = step(&mut t.src);
    if matches!(x, Union2::U2(_)) {
        t.done = true;
        return Union2::<T, Finished>::U2(finished());
    }
    let mut keep = &t.keep;
    if keep(x.u1()) {
        return Union2::<T, Finished>::U1(emitted(x.u1().clone()));
    }
    t.done = true;
    return Union2::<T, Finished>::U2(finished());
}

pub fn take_while<It: Clone, T: Clone>(it: It, keep: impl Fn(&T) -> bool + Send + Sync + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> TakeWhile<It, T> {
    return TakeWhile { src: it, step: std::sync::Arc::new(next), keep: std::sync::Arc::new(keep), done: false };
}

#[derive(Clone)]
pub struct Skip<It: Clone, T: Clone> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub left: i32,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for Skip<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Skip")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("left", &self.left)
            .finish()
    }
}

pub fn next__15<It: Clone, T: Clone>(t: &mut Skip<It, T>) -> Union2<T, Finished> {
    let mut step = &t.step;
    while t.left > 0 {
        t.left = t.left - 1;
        let mut x = step(&mut t.src);
        if matches!(x, Union2::U2(_)) {
            return Union2::<T, Finished>::U2(finished());
        }
    }
    return step(&mut t.src);
}

pub fn skip<It: Clone, T: Clone>(it: It, n: i32, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> Skip<It, T> {
    return Skip { src: it, step: std::sync::Arc::new(next), left: n };
}

#[derive(Clone)]
pub struct SkipWhile<It: Clone, T: Clone> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub skip: std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>,
    pub started: bool,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for SkipWhile<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkipWhile")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("skip", &"<fn>")
            .field("started", &self.started)
            .finish()
    }
}

pub fn next__16<It: Clone, T: Clone>(t: &mut SkipWhile<It, T>) -> Union2<T, Finished> {
    let mut step = &t.step;
    if t.started {
        return step(&mut t.src);
    }
    let mut passing = &t.skip;
    loop {
        let mut x = step(&mut t.src);
        if matches!(x, Union2::U2(_)) {
            return Union2::<T, Finished>::U2(finished());
        }
        if !passing(x.u1()) {
            t.started = true;
            return Union2::<T, Finished>::U1(emitted(x.u1().clone()));
        }
    }
    return Union2::<T, Finished>::U2(finished());
}

pub fn skip_while<It: Clone, T: Clone>(it: It, skip: impl Fn(&T) -> bool + Send + Sync + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> SkipWhile<It, T> {
    return SkipWhile { src: it, step: std::sync::Arc::new(next), skip: std::sync::Arc::new(skip), started: false };
}

pub fn collect<It: Clone, T: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<T> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        out.push(x);
    }
    return out;
}
