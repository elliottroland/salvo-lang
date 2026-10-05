use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub fn map<It: Clone, T: Clone, U: Clone>(it: &mut It, f: &mut impl FnMut(&T) -> U, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<U> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        crate::core_list::add_platform(&mut out, f(&x));
    }
    return out;
}

pub fn filter<It: Clone, T: Clone>(it: &mut It, keep: &mut impl FnMut(&T) -> bool, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<T> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        if keep(&x) {
            crate::core_list::add_platform(&mut out, x);
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

pub fn map__2<T: Clone, U: Clone>(list: &Vec<T>, f: &mut impl FnMut(&T) -> U) -> Vec<U> {
    let mut out = vec![];
    for mut x in crate::platform_core_list::each(list).map(|__x| __x.clone()) {
        crate::core_list::add_platform(&mut out, f(&x));
    }
    return out;
}

pub fn filter_platform<T: Clone>(list: &Vec<T>, keep: &mut dyn FnMut(&T) -> bool) -> Vec<T> {
    crate::platform_core_seq::filter(list, keep)
}

pub fn reduce__2<T: Clone, A: Clone>(list: &Vec<T>, init: A, f: &mut impl FnMut(&A, &T) -> A) -> A {
    let mut acc = init.clone();
    for mut x in crate::platform_core_list::each(list).map(|__x| __x.clone()) {
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
pub struct Mapping<It, T, U> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub f: std::sync::Arc<dyn Fn(&T) -> U + Send + Sync>,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug> std::fmt::Debug for Mapping<It, T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mapping")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("f", &"<fn>")
            .finish()
    }
}

pub fn next__13<It: Clone, T: Clone, U: Clone>(m: &mut Mapping<It, T, U>) -> Union2<U, Finished> {
    let mut step = &m.step;
    let mut x = step(&mut m.src);
    if matches!(x, Union2::U2(_)) {
        return Union2::<U, Finished>::U2(finished());
    }
    let mut f = &m.f;
    let mut y: U = f(x.u1());
    return Union2::<U, Finished>::U1(emitted(y));
}

pub fn mapping<It: Clone, T: Clone, U: Clone>(it: It, f: impl Fn(&T) -> U + Send + Sync + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> Mapping<It, T, U> {
    return Mapping { src: it, step: std::sync::Arc::new(next), f: std::sync::Arc::new(f) };
}

#[derive(Clone)]
pub struct Filtering<It, T> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub keep: std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for Filtering<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filtering")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("keep", &"<fn>")
            .finish()
    }
}

pub fn next__14<It: Clone, T: Clone>(t: &mut Filtering<It, T>) -> Union2<T, Finished> {
    let mut step = &t.step;
    let mut keep = &t.keep;
    loop {
        let mut x = step(&mut t.src);
        if matches!(x, Union2::U2(_)) {
            return Union2::<T, Finished>::U2(finished());
        }
        if keep(x.u1()) {
            return Union2::<T, Finished>::U1(emitted(x.u1().clone()));
        }
    }
    return Union2::<T, Finished>::U2(finished());
}

pub fn filtering<It: Clone, T: Clone>(it: It, keep: impl Fn(&T) -> bool + Send + Sync + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> Filtering<It, T> {
    return Filtering { src: it, step: std::sync::Arc::new(next), keep: std::sync::Arc::new(keep) };
}

#[derive(Clone)]
pub struct Taking<It, T> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub left: i32,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for Taking<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Taking")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("left", &self.left)
            .finish()
    }
}

pub fn next__15<It: Clone, T: Clone>(t: &mut Taking<It, T>) -> Union2<T, Finished> {
    if t.left <= 0 {
        return Union2::<T, Finished>::U2(finished());
    }
    t.left = t.left - 1;
    let mut step = &t.step;
    return step(&mut t.src);
}

pub fn taking<It: Clone, T: Clone>(it: It, n: i32, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> Taking<It, T> {
    return Taking { src: it, step: std::sync::Arc::new(next), left: n };
}

#[derive(Clone)]
pub struct TakingWhile<It, T> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub keep: std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>,
    pub done: bool,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for TakingWhile<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakingWhile")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("keep", &"<fn>")
            .field("done", &self.done)
            .finish()
    }
}

pub fn next__16<It: Clone, T: Clone>(t: &mut TakingWhile<It, T>) -> Union2<T, Finished> {
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

pub fn taking_while<It: Clone, T: Clone>(it: It, keep: impl Fn(&T) -> bool + Send + Sync + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> TakingWhile<It, T> {
    return TakingWhile { src: it, step: std::sync::Arc::new(next), keep: std::sync::Arc::new(keep), done: false };
}

#[derive(Clone)]
pub struct Skipping<It, T> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub left: i32,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for Skipping<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Skipping")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("left", &self.left)
            .finish()
    }
}

pub fn next__17<It: Clone, T: Clone>(t: &mut Skipping<It, T>) -> Union2<T, Finished> {
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

pub fn skipping<It: Clone, T: Clone>(it: It, n: i32, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> Skipping<It, T> {
    return Skipping { src: it, step: std::sync::Arc::new(next), left: n };
}

#[derive(Clone)]
pub struct SkippingWhile<It, T> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&mut It) -> Union2<T, Finished> + Send + Sync>,
    pub skip: std::sync::Arc<dyn Fn(&T) -> bool + Send + Sync>,
    pub started: bool,
}

impl<It: Clone + std::fmt::Debug, T: Clone + std::fmt::Debug> std::fmt::Debug for SkippingWhile<It, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkippingWhile")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("skip", &"<fn>")
            .field("started", &self.started)
            .finish()
    }
}

pub fn next__18<It: Clone, T: Clone>(t: &mut SkippingWhile<It, T>) -> Union2<T, Finished> {
    let mut step = &t.step;
    if t.started {
        return step(&mut t.src);
    }
    let mut skip = &t.skip;
    loop {
        let mut x = step(&mut t.src);
        if matches!(x, Union2::U2(_)) {
            return Union2::<T, Finished>::U2(finished());
        }
        if !skip(x.u1()) {
            t.started = true;
            return Union2::<T, Finished>::U1(emitted(x.u1().clone()));
        }
    }
    return Union2::<T, Finished>::U2(finished());
}

pub fn skipping_while<It: Clone, T: Clone>(it: It, skip: impl Fn(&T) -> bool + Send + Sync + 'static, next: impl Fn(&mut It) -> Union2<T, Finished> + Send + Sync + 'static) -> SkippingWhile<It, T> {
    return SkippingWhile { src: it, step: std::sync::Arc::new(next), skip: std::sync::Arc::new(skip), started: false };
}

pub fn to_list__2<It: Clone, T: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> Vec<T> {
    let mut out = vec![];
    while let Union2::U1(mut x) = next(it) {
        crate::core_list::add_platform(&mut out, x);
    }
    return out;
}

pub fn count__2<It: Clone, T: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> Union2<T, Finished>) -> i32 {
    let mut n = 0;
    while let Union2::U1(mut _x) = next(it) {
        n = n + 1;
    }
    return n;
}
