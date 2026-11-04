use crate::std_core_iterator::Finished;
use crate::std_core_list::add_platform;
use crate::std_core_iterator::emitted;
use crate::std_core_iterator::finished;


pub fn map__It_Fn<It, T: Clone, U: Clone, E: Clone>(e: &E, it: &mut It, f: &mut dyn FnMut(&E, &T) -> U, next: &mut dyn FnMut(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> Vec<U> {
    let mut out: Vec<U> = vec![];
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(e, &mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut x: T = __emitted_3;
            crate::std_core_list::add_platform::<U>(&mut out, f(e, &x));
        } else {
            break;
        };
    }
    return out;
}

pub fn filter<'a, It, T: Clone, E: Clone>(e: &E, it: &'a mut It, keep: &mut dyn FnMut(&E, &T) -> bool, next: &mut dyn FnMut(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> Vec<T> {
    let mut out = vec![];
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(e, &mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut x: T = __emitted_3;
            if keep(e, &x) {
                crate::std_core_list::add_platform(&mut out, x);
            };
        } else {
            break;
        };
    }
    return out;
}

pub fn reduce__It_A_Fn<It, T: Clone, A: Clone, E: Clone>(e: &E, it: &mut It, init: &A, f: &mut dyn FnMut(&E, &A, &T) -> A, next: &mut dyn FnMut(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> A {
    let mut acc: A = init.clone();
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(e, &mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut x: T = __emitted_3;
            acc = f(e, &acc, &x);
        } else {
            break;
        };
    }
    return acc;
}

pub fn map__List_Fn<T: Clone, U: Clone, E: Clone>(e: &E, list: &Vec<T>, f: &mut dyn FnMut(&E, &T) -> U) -> Vec<U> {
    let mut out: Vec<U> = vec![];
    for mut x in list.iter() {
        crate::std_core_list::add_platform::<U>(&mut out, f(e, x));
    }
    return out;
}

pub fn filter_platform<'a, T: Clone, E: Clone>(e: &E, list: &'a Vec<T>, keep: &mut dyn FnMut(&E, &T) -> bool) -> Vec<&'a T> {
    crate::platform_std_core_seq::filter(list, &mut |__x0| keep(e, __x0))
}

pub fn reduce__List_A_Fn<T: Clone, A: Clone, E: Clone>(e: &E, list: &Vec<T>, mut init: A, f: &mut dyn FnMut(&E, &A, &T) -> A) -> A {
    let mut acc: A = init.clone();
    for mut x in list.iter() {
        acc = f(e, &acc, x);
    }
    return acc;
}

pub fn map_to<D: Clone, It: Clone, T: Clone, U: Clone>(mut dest: D, it: &mut It, f: &mut dyn FnMut(&T) -> U, add: &mut dyn FnMut(&mut D, U), next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> D {
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(&mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut x: T = __emitted_3;
            add(&mut dest, f(&x));
        } else {
            break;
        };
    }
    return dest;
}

pub fn filter_to<D: Clone, It: Clone, T: Clone>(mut dest: D, it: &mut It, keep: &mut dyn FnMut(&T) -> bool, add: &mut dyn FnMut(&mut D, T), copy: &mut dyn FnMut(&T) -> T, next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> D {
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(&mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut x: T = __emitted_3;
            if keep(&x) {
                add(&mut dest, copy(&x));
            };
        } else {
            break;
        };
    }
    return dest;
}

pub fn drop<T: Clone>(mut value: T) {
}

#[derive(Clone)]
pub struct Mapping<It, T, U, E> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync>,
    pub f: std::sync::Arc<dyn Fn(&E, &T) -> U + Send + Sync>,
}

impl<It: std::fmt::Debug, T: std::fmt::Debug, U: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for Mapping<It, T, U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mapping")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("f", &"<fn>")
            .finish()
    }
}

pub fn next__Mapping<It, T: Clone, U: Clone, E: Clone>(e: &E, m: &mut crate::std_core_seq::Mapping<It, T, U, E>) -> crate::unions::Union2<U, crate::std_core_iterator::Finished> {
    let mut step = m.step.clone();
    let mut x: crate::unions::Union2<T, crate::std_core_iterator::Finished> = step(e, &mut m.src);
    if matches!(x, crate::unions::Union2::U2(_)) {
        let mut x_1 = match &x { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut f = m.f.clone();
    let mut x_2 = match &x { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut y: U = f(e, x_2);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<U>(y));
}

pub fn mapping<It, T: Clone, U: Clone, E: Clone>(mut it: It, f: impl Fn(&E, &T) -> U + Send + Sync + 'static, next: impl Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync + 'static) -> crate::std_core_seq::Mapping<It, T, U, E> {
    return crate::std_core_seq::Mapping { src: it, step: std::sync::Arc::new(next), f: std::sync::Arc::new(f) };
}

pub fn detach__Mapping<It, T: Clone, U: Clone, E: Clone>(mut it: crate::std_core_seq::Mapping<It, T, U, E>) -> It {
    return it.src;
}

#[derive(Clone)]
pub struct Filtering<It, T, E> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync>,
    pub keep: std::sync::Arc<dyn Fn(&E, &T) -> bool + Send + Sync>,
}

impl<It: std::fmt::Debug, T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for Filtering<It, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filtering")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("keep", &"<fn>")
            .finish()
    }
}

pub fn next__Filtering<It, T: Clone, E: Clone>(e: &E, t: &mut crate::std_core_seq::Filtering<It, T, E>) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> {
    let mut step = t.step.clone();
    let mut keep = t.keep.clone();
    loop {
        let mut x: crate::unions::Union2<T, crate::std_core_iterator::Finished> = step(e, &mut t.src);
        if matches!(x, crate::unions::Union2::U2(_)) {
            let mut x_1 = match &x { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_iterator::finished());
        };
        let mut x_2 = match x { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        if keep(e, &x_2) {
            return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<T>(x_2));
        };
    }
    panic!("salvo: unreachable at std.core.seq:308:5");
    return crate::unions::Union2::U2(crate::std_core_iterator::finished());
}

pub fn filtering<It, T: Clone, E: Clone>(mut it: It, keep: impl Fn(&E, &T) -> bool + Send + Sync + 'static, next: impl Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync + 'static) -> crate::std_core_seq::Filtering<It, T, E> {
    return crate::std_core_seq::Filtering { src: it, step: std::sync::Arc::new(next), keep: std::sync::Arc::new(keep) };
}

pub fn detach__Filtering<It, T: Clone, E: Clone>(mut it: crate::std_core_seq::Filtering<It, T, E>) -> It {
    return it.src;
}

#[derive(Clone)]
pub struct Taking<It, T, E> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync>,
    pub left: i32,
}

impl<It: std::fmt::Debug, T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for Taking<It, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Taking")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("left", &self.left)
            .finish()
    }
}

pub fn next__Taking<It, T: Clone, E: Clone>(e: &E, t: &mut crate::std_core_seq::Taking<It, T, E>) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> {
    if (t.left <= 0i32) {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    t.left = i32::wrapping_sub(t.left, 1i32);
    let mut step = t.step.clone();
    return step(e, &mut t.src);
}

pub fn taking<It, T: Clone, E: Clone>(mut it: It, mut n: i32, next: impl Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync + 'static) -> crate::std_core_seq::Taking<It, T, E> {
    return crate::std_core_seq::Taking { src: it, step: std::sync::Arc::new(next), left: n };
}

pub fn detach__Taking<It, T: Clone, E: Clone>(mut it: crate::std_core_seq::Taking<It, T, E>) -> It {
    return it.src;
}

#[derive(Clone)]
pub struct TakingWhile<It, T, E> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync>,
    pub keep: std::sync::Arc<dyn Fn(&E, &T) -> bool + Send + Sync>,
    pub done: bool,
}

impl<It: std::fmt::Debug, T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for TakingWhile<It, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TakingWhile")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("keep", &"<fn>")
            .field("done", &self.done)
            .finish()
    }
}

pub fn next__TakingWhile<It, T: Clone, E: Clone>(e: &E, t: &mut crate::std_core_seq::TakingWhile<It, T, E>) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> {
    if t.done {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut step = t.step.clone();
    let mut x: crate::unions::Union2<T, crate::std_core_iterator::Finished> = step(e, &mut t.src);
    if matches!(x, crate::unions::Union2::U2(_)) {
        let mut x_1 = match &x { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        t.done = true;
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut keep = t.keep.clone();
    let mut x_2 = match x { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    if keep(e, &x_2) {
        return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<T>(x_2));
    };
    t.done = true;
    return crate::unions::Union2::U2(crate::std_core_iterator::finished());
}

pub fn taking_while<It, T: Clone, E: Clone>(mut it: It, keep: impl Fn(&E, &T) -> bool + Send + Sync + 'static, next: impl Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync + 'static) -> crate::std_core_seq::TakingWhile<It, T, E> {
    return crate::std_core_seq::TakingWhile { src: it, step: std::sync::Arc::new(next), keep: std::sync::Arc::new(keep), done: false };
}

pub fn detach__TakingWhile<It, T: Clone, E: Clone>(mut it: crate::std_core_seq::TakingWhile<It, T, E>) -> It {
    return it.src;
}

#[derive(Clone)]
pub struct Skipping<It, T, E> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync>,
    pub left: i32,
}

impl<It: std::fmt::Debug, T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for Skipping<It, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Skipping")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("left", &self.left)
            .finish()
    }
}

pub fn next__Skipping<It, T: Clone, E: Clone>(e: &E, t: &mut crate::std_core_seq::Skipping<It, T, E>) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> {
    let mut step = t.step.clone();
    loop {
        if !((t.left > 0i32)) {
            break;
        };
        t.left = i32::wrapping_sub(t.left, 1i32);
        let mut x: crate::unions::Union2<T, crate::std_core_iterator::Finished> = step(e, &mut t.src);
        if matches!(x, crate::unions::Union2::U2(_)) {
            let mut x_1 = match &x { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_iterator::finished());
        };
    }
    return step(e, &mut t.src);
}

pub fn skipping<It, T: Clone, E: Clone>(mut it: It, mut n: i32, next: impl Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync + 'static) -> crate::std_core_seq::Skipping<It, T, E> {
    return crate::std_core_seq::Skipping { src: it, step: std::sync::Arc::new(next), left: n };
}

pub fn detach__Skipping<It, T: Clone, E: Clone>(mut it: crate::std_core_seq::Skipping<It, T, E>) -> It {
    return it.src;
}

#[derive(Clone)]
pub struct SkippingWhile<It, T, E> {
    pub src: It,
    pub step: std::sync::Arc<dyn Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync>,
    pub skip: std::sync::Arc<dyn Fn(&E, &T) -> bool + Send + Sync>,
    pub started: bool,
}

impl<It: std::fmt::Debug, T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for SkippingWhile<It, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkippingWhile")
            .field("src", &self.src)
            .field("step", &"<fn>")
            .field("skip", &"<fn>")
            .field("started", &self.started)
            .finish()
    }
}

pub fn next__SkippingWhile<It, T: Clone, E: Clone>(e: &E, t: &mut crate::std_core_seq::SkippingWhile<It, T, E>) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> {
    let mut step = t.step.clone();
    if t.started {
        return step(e, &mut t.src);
    };
    let mut skip = t.skip.clone();
    loop {
        let mut x: crate::unions::Union2<T, crate::std_core_iterator::Finished> = step(e, &mut t.src);
        if matches!(x, crate::unions::Union2::U2(_)) {
            let mut x_1 = match &x { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_iterator::finished());
        };
        let mut x_2 = match &x { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        if !(skip(e, x_2)) {
            t.started = true;
            let mut x_3 = match x { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<T>(x_3));
        };
    }
    panic!("salvo: unreachable at std.core.seq:519:5");
    return crate::unions::Union2::U2(crate::std_core_iterator::finished());
}

pub fn skipping_while<It, T: Clone, E: Clone>(mut it: It, skip: impl Fn(&E, &T) -> bool + Send + Sync + 'static, next: impl Fn(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished> + Send + Sync + 'static) -> crate::std_core_seq::SkippingWhile<It, T, E> {
    return crate::std_core_seq::SkippingWhile { src: it, step: std::sync::Arc::new(next), skip: std::sync::Arc::new(skip), started: false };
}

pub fn detach__SkippingWhile<It, T: Clone, E: Clone>(mut it: crate::std_core_seq::SkippingWhile<It, T, E>) -> It {
    return it.src;
}

pub fn to_list<'a, It, T: Clone, E: Clone>(e: &E, it: &'a mut It, next: &mut dyn FnMut(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> Vec<T> {
    let mut out = vec![];
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(e, &mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut x: T = __emitted_3;
            crate::std_core_list::add_platform(&mut out, x);
        } else {
            break;
        };
    }
    return out;
}

pub fn count<It, T: Clone, E: Clone>(e: &E, it: &mut It, next: &mut dyn FnMut(&E, &mut It) -> crate::unions::Union2<T, crate::std_core_iterator::Finished>) -> i32 {
    let mut n: i32 = 0i32;
    loop {
        let mut __step_2: crate::unions::Union2<T, crate::std_core_iterator::Finished> = next(e, &mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match __step_2 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut _x: T = __emitted_3;
            n = i32::wrapping_add(n, 1i32);
        } else {
            break;
        };
    }
    return n;
}
