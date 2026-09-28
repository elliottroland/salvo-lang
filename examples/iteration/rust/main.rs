#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/range.rs"]
pub mod core_range;
#[path = "core/seq.rs"]
pub mod core_seq;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;

use crate::core_array::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_range::*;
use crate::core_seq::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;
use crate::unions::*;

pub fn describe_container(console: &mut crate::core_console::__Handle_Console, xs: &Vec<i32>) {
    let mut sum = 0;
    for n in xs {
        sum = sum + *n;
    }
    println(console, &(format!("1. list of {} sums to {}", (xs.len() as i32), sum)));
    let mut letters = String::new();
    for mut c in "salvo".to_string().chars() {
        letters.push_str(&format!("{}.", c)[..]);
    }
    println(console, &(format!("1. string: {}", letters)));
    let mut arr = vec![10, 20, 30];
    let mut from_array = 0;
    for n in &arr {
        from_array = from_array + *n;
    }
    println(console, &(format!("1. array sums to {}", from_array)));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Countdown {
    pub at: i32,
}

impl crate::wire::__Wire for Countdown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn countdown(from: i32) -> Countdown {
    return Countdown { at: from };
}

pub fn next__15(p: &mut Countdown) -> Union2<i32, Finished> {
    if p.at <= 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = p.at;
    p.at = p.at - 1;
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn skip(p: &mut Countdown) -> Union2<i32, Finished> {
    if p.at <= 1 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = p.at;
    p.at = p.at - 2;
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn take(console: &mut crate::core_console::__Handle_Console, p: &mut Countdown, count: i32) {
    let mut seen = 0;
    while let Union2::U1(mut n) = next__15(p) {
        println(console, &(format!("2. got {}", n)));
        seen = seen + 1;
        if seen == count {
            break;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_halving_Int {
    pub start: i32,
    pub at: i32,
}

impl crate::wire::__Wire for __Iter_halving_Int {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.start, out);
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            start: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn halving(start: i32) -> __Iter_halving_Int {
    return __Iter_halving_Int { start: start, at: start };
}

pub fn next__16(__p: &mut __Iter_halving_Int) -> Union2<i32, Finished> {
    if __p.at <= 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = __p.at;
    __p.at = __p.at / 2;
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn halving_from_ten() -> __Iter_halving_Int {
    return halving(10);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bag {
    pub items: Vec<i32>,
}

impl crate::wire::__Wire for Bag {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.items, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            items: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_iter_Bag<'s> {
    pub items: &'s Vec<i32>,
    pub at: i32,
}

pub fn iter__9(bag: &Bag) -> __Iter_iter_Bag<'_> {
    return __Iter_iter_Bag { items: &bag.items, at: 0 };
}

pub fn next__17(__p: &mut __Iter_iter_Bag<'_>) -> Union2<i32, Finished> {
    let mut e = __p.items.get((__p.at) as i64 as usize);
    if e.is_none() {
        return Union2::<i32, Finished>::U2(finished());
    }
    __p.at = __p.at + 1;
    return Union2::<i32, Finished>::U1(emitted(*e.unwrap()));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_fibs_Int {
    pub count: i32,
    pub a: i32,
    pub b: i32,
    pub made: i32,
}

impl crate::wire::__Wire for __Iter_fibs_Int {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.count, out);
        crate::wire::__Wire::__enc(&self.a, out);
        crate::wire::__Wire::__enc(&self.b, out);
        crate::wire::__Wire::__enc(&self.made, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            count: crate::wire::__Wire::__dec(r)?,
            a: crate::wire::__Wire::__dec(r)?,
            b: crate::wire::__Wire::__dec(r)?,
            made: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn fibs(count: i32) -> __Iter_fibs_Int {
    return __Iter_fibs_Int { count: count, a: 0, b: 1, made: 0 };
}

pub fn next__18(console: &mut crate::core_console::__Handle_Console, __p: &mut __Iter_fibs_Int) -> Union2<i32, Finished> {
    if __p.made >= __p.count {
        println(console, &("3. finished".to_string()));
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = __p.a;
    let mut sum = __p.a + __p.b;
    __p.a = __p.b;
    __p.b = sum.clone();
    __p.made = __p.made + 1;
    return Union2::<i32, Finished>::U1(emitted(now));
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_naturals_Int {
    pub from: i32,
    pub at: i32,
}

impl crate::wire::__Wire for __Iter_naturals_Int {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.from, out);
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            from: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn naturals(from: i32) -> __Iter_naturals_Int {
    return __Iter_naturals_Int { from: from, at: from };
}

pub fn next__19(__p: &mut __Iter_naturals_Int) -> Union2<i32, Finished> {
    let mut now = __p.at;
    __p.at = __p.at + 1;
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn sum_of<It: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> Union2<i32, Finished>) -> i32 {
    let mut total = 0;
    while let Union2::U1(mut n) = next(it) {
        total = total + n;
    }
    return total;
}

pub fn total<'c, C: Clone, __It0: Clone>(c: &'c C, iter: &mut dyn FnMut(&'c C) -> __It0, next: &mut dyn FnMut(&mut __It0) -> Union2<i32, Finished>) -> i32 {
    let mut total = 0;
    let mut __loop3_pass = iter(c);
    while let Union2::U1(mut n) = next(&mut __loop3_pass) {
        total = total + n;
    }
    return total;
}

pub fn first__2<__It0: Clone>(it: &mut __It0, next: &mut dyn FnMut(&mut __It0) -> Union2<i32, Finished>) -> i32 {
    while let Union2::U1(mut n) = next(it) {
        return n;
    }
    return -1;
}

pub fn main() {
    let mut console = crate::core_console::__Handle_Console::new(StdOutConsole::new());
    let mut xs = vec![1, 2, 3, 4];
    describe_container(&mut console, &xs);
    let mut p = countdown(5);
    take(&mut console, &mut p, 2);
    println(&mut console, &(format!("2. rest sums to {}", sum_of::<Countdown>(&mut p, &mut |__i0| next__15(__i0)))));
    let mut q = countdown(6);
    while let Union2::U1(mut n) = skip(&mut q) {
        println(&mut console, &(format!("2. skip {}", n)));
    }
    let mut __loop5_pass = halving(20);
    while let Union2::U1(mut n) = next__16(&mut __loop5_pass) {
        println(&mut console, &(format!("2b. halving {}", n)));
    }
    let mut hp = halving(20);
    println(&mut console, &(format!("2b. summed from a held iterator: {}", sum_of::<__Iter_halving_Int>(&mut hp, &mut |__i0| next__16(__i0)))));
    println(&mut console, &(format!("2b. first from a pattern-typed fn: {}", first__2::<__Iter_halving_Int>(&mut (halving_from_ten()), &mut |__i0| next__16(__i0)))));
    let mut bag = Bag { items: vec![7, 8] };
    let mut __loop6_pass = iter__9(&bag);
    while let Union2::U1(mut n) = next__17(&mut __loop6_pass) {
        println(&mut console, &(format!("2c. bag {}", n)));
    }
    println(&mut console, &(format!("2c. total of a bag {}, of a list {}", total::<Bag, __Iter_iter_Bag<'_>>(&bag, &mut |__i0| iter__9(__i0), &mut |__i0| next__17(__i0)), total::<Vec<i32>, ListYield<'_, i32>>(&xs, &mut |__i0| iter__3(__i0), &mut |__i0| match next__3(__i0) { Union2::U1(__e) => Union2::U1(*__e), Union2::U2(__f) => Union2::U2(__f) }))));
    let mut __loop7_pass = fibs(6);
    while let Union2::U1(mut n) = next__18(&mut console, &mut __loop7_pass) {
        println(&mut console, &(format!("3. fib {}", n)));
    }
    let mut __loop8_pass = naturals(10);
    while let Union2::U1(mut n) = next__19(&mut __loop8_pass) {
        if n > 12 {
            break;
        }
        println(&mut console, &(format!("3. natural {}", n)));
    }
    let mut doubled = salvo_map(&xs[..], |n| *n * 2);
    let mut odd = salvo_filter(&xs[..], |n| *n % 2 == 1);
    let mut total = salvo_reduce(&xs[..], 0, |acc, n| *acc + *n);
    println(&mut console, &(format!("5. list: {} doubled, {} odd, total {}", (doubled.len() as i32), (odd.len() as i32), total)));
    let mut words = vec!["ann".to_string(), "bo".to_string(), "carol".to_string()];
    let mut lengths = map::<ListYield<'_, String>, &String, i32>(&mut (iter__3(&words)), &mut (|w| { let w = *w; (w.chars().count() as i32) }), &mut |__i0| next__3(__i0));
    println(&mut console, &(format!("5. lengths: {}", reduce::<ListYield<'_, i32>, &i32, i32>(&mut (iter__3(&lengths)), &(0), &mut (|acc, n| { let n = *n; *acc + *n }), &mut |__i0| next__3(__i0)))));
    let mut word = "iteration".to_string();
    let mut vowels = filter::<StrYield<'_>, char>(&mut (iter__8(&word)), &mut (|c| *c == 'i' || *c == 'o'), &mut |__i0| next__12(__i0));
    println(&mut console, &(format!("5. vowels: {}", (vowels.len() as i32))));
    println(&mut console, &(format!("5. halving total {}", reduce::<__Iter_halving_Int, i32, i32>(&mut (halving(20)), &(0), &mut (|acc, n| *acc + *n), &mut |__i0| next__16(__i0)))));
    let mut collected = map_to::<Vec<i32>, Countdown, i32, i32>(vec![], &mut (countdown(3)), &mut (|n: &i32| *n * 10), &mut |__i0, __i1| __i0.push(__i1), &mut |__i0| next__15(__i0));
    println(&mut console, &(format!("6. collected {}", (collected.len() as i32))));
    let mut evens = String::new();
    let mut __loop9_pass = range(0, 10, 2);
    while let Union2::U1(mut i) = next__10(&mut __loop9_pass) {
        evens.push_str(&format!("{} ", i)[..]);
    }
    println(&mut console, &(format!("7. evens {}", evens)));
}
