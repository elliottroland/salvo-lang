#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/compare.rs"]
pub mod core_compare;
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
#[path = "platform/core/console.rs"]
pub mod platform_core_console;
#[path = "platform/core/list.rs"]
pub mod platform_core_list;
#[path = "platform/core/map.rs"]
pub mod platform_core_map;
#[path = "platform/core/seq.rs"]
pub mod platform_core_seq;
#[path = "platform/core/set.rs"]
pub mod platform_core_set;
#[path = "platform/core/sorted.rs"]
pub mod platform_core_sorted;
#[path = "platform/core/string.rs"]
pub mod platform_core_string;

use crate::unions::*;
use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;
use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;
use crate::core_list::ListYield;
use crate::core_list::at;
use crate::core_list::iter as iter__core_list;
use crate::core_list::next__ListYield;
use crate::core_range::next as next__core_range;
use crate::core_range::range__Int_Int_Int;
use crate::core_seq::filter;
use crate::core_seq::map__It_Fn;
use crate::core_seq::map__List_Fn;
use crate::core_seq::map_to;
use crate::core_seq::reduce__It_A_Fn;
use crate::core_seq::reduce__List_A_Fn;
use crate::core_string::StrYield;
use crate::core_string::iter as iter__core_string;
use crate::core_string::mut_str;
use crate::core_string::next as next__core_string;

pub fn describe_container(console: &crate::core_console::Console, xs: &Vec<i32>) {
    let mut sum = 0;
    for n in crate::platform_core_list::each(xs) {
        sum = i32::wrapping_add(sum, *n);
    }
    println(console, &(format!("1. list of {} sums to {}", crate::core_list::size_platform(xs), sum)));
    let mut letters = mut_str(vec![]);
    for mut c in crate::platform_core_string::each(&("salvo".to_string())).map(|__x| __x.clone()) {
        crate::core_string::append_platform(&mut letters, &(format!("{}.", c)));
    }
    println(console, &(format!("1. string: {}", letters)));
    let mut arr = vec![10, 20, 30];
    let mut from_array = 0;
    for n in &arr {
        from_array = i32::wrapping_add(from_array, *n);
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

pub fn next__Countdown(p: &mut Countdown) -> Union2<i32, Finished> {
    if p.at <= 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = p.at;
    p.at = i32::wrapping_sub(p.at, 1);
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn skip(p: &mut Countdown) -> Union2<i32, Finished> {
    if p.at <= 1 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = p.at;
    p.at = i32::wrapping_sub(p.at, 2);
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn take(console: &crate::core_console::Console, p: &mut Countdown, count: i32) {
    let mut seen = 0;
    while let Union2::U1(mut n) = next__Countdown(p) {
        println(console, &(format!("2. got {}", n)));
        seen = i32::wrapping_add(seen, 1);
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

pub fn next__Iter_halving_Int(__p: &mut __Iter_halving_Int) -> Union2<i32, Finished> {
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

pub fn iter(bag: &Bag) -> __Iter_iter_Bag<'_> {
    return __Iter_iter_Bag { items: &bag.items, at: 0 };
}

pub fn next__Iter_iter_Bag(__p: &mut __Iter_iter_Bag<'_>) -> Union2<i32, Finished> {
    let mut e = crate::core_list::get_platform(&__p.items, __p.at);
    if e.is_none() {
        return Union2::<i32, Finished>::U2(finished());
    }
    __p.at = i32::wrapping_add(__p.at, 1);
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

pub fn next__Iter_fibs_Int(console: &crate::core_console::Console, __p: &mut __Iter_fibs_Int) -> Union2<i32, Finished> {
    if __p.made >= __p.count {
        println(console, &("3. finished".to_string()));
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = __p.a;
    let mut sum = i32::wrapping_add(__p.a, __p.b);
    __p.a = __p.b;
    __p.b = sum.clone();
    __p.made = i32::wrapping_add(__p.made, 1);
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

pub fn next__Iter_naturals_Int(__p: &mut __Iter_naturals_Int) -> Union2<i32, Finished> {
    let mut now = __p.at;
    __p.at = i32::wrapping_add(__p.at, 1);
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn sum_of<It: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> Union2<i32, Finished>) -> i32 {
    let mut total = 0;
    while let Union2::U1(mut n) = next(it) {
        total = i32::wrapping_add(total, n);
    }
    return total;
}

pub fn total<'c, C: Clone, __It0: Clone>(c: &'c C, iter: &mut dyn FnMut(&'c C) -> __It0, next: &mut dyn FnMut(&mut __It0) -> Union2<i32, Finished>) -> i32 {
    let mut total = 0;
    let mut __loop3_pass = iter(c);
    while let Union2::U1(mut n) = next(&mut __loop3_pass) {
        total = i32::wrapping_add(total, n);
    }
    return total;
}

pub fn first<__It0: Clone>(it: &mut __It0, next: &mut dyn FnMut(&mut __It0) -> Union2<i32, Finished>) -> i32 {
    while let Union2::U1(mut n) = next(it) {
        return n;
    }
    return -1;
}

pub fn main() {
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let mut xs = vec![1, 2, 3, 4];
    describe_container(&console, &xs);
    let mut p = countdown(5);
    take(&console, &mut p, 2);
    println(&console, &(format!("2. rest sums to {}", sum_of::<Countdown>(&mut p, &mut |__i0| next__Countdown(__i0)))));
    let mut q = countdown(6);
    while let Union2::U1(mut n) = skip(&mut q) {
        println(&console, &(format!("2. skip {}", n)));
    }
    let mut __loop5_pass = halving(20);
    while let Union2::U1(mut n) = next__Iter_halving_Int(&mut __loop5_pass) {
        println(&console, &(format!("2b. halving {}", n)));
    }
    let mut hp = halving(20);
    println(&console, &(format!("2b. summed from a held iterator: {}", sum_of::<__Iter_halving_Int>(&mut hp, &mut |__i0| next__Iter_halving_Int(__i0)))));
    println(&console, &(format!("2b. first from a pattern-typed fn: {}", first::<__Iter_halving_Int>(&mut (halving_from_ten()), &mut |__i0| next__Iter_halving_Int(__i0)))));
    let mut bag = Bag { items: vec![7, 8] };
    let mut __loop6_pass = iter(&bag);
    while let Union2::U1(mut n) = next__Iter_iter_Bag(&mut __loop6_pass) {
        println(&console, &(format!("2c. bag {}", n)));
    }
    println(&console, &(format!("2c. total of a bag {}, of a list {}", total::<Bag, __Iter_iter_Bag<'_>>(&bag, &mut |__i0| iter(__i0), &mut |__i0| next__Iter_iter_Bag(__i0)), total::<Vec<i32>, ListYield<'_, i32>>(&xs, &mut |__i0| iter__core_list(__i0), &mut |__i0| match next__ListYield(__i0) { Union2::U1(__e) => Union2::U1(*__e), Union2::U2(__f) => Union2::U2(__f) }))));
    let mut __loop7_pass = fibs(6);
    while let Union2::U1(mut n) = next__Iter_fibs_Int(&console, &mut __loop7_pass) {
        println(&console, &(format!("3. fib {}", n)));
    }
    let mut __loop8_pass = naturals(10);
    while let Union2::U1(mut n) = next__Iter_naturals_Int(&mut __loop8_pass) {
        if n > 12 {
            break;
        }
        println(&console, &(format!("3. natural {}", n)));
    }
    let mut doubled = map__List_Fn(&xs, &mut (|n| i32::wrapping_mul(*n, 2)));
    let mut odd = crate::core_seq::filter_platform(&xs, &mut (|n| *n % 2 == 1));
    let mut total = reduce__List_A_Fn(&xs, 0, &mut (|acc, n| i32::wrapping_add(*acc, *n)));
    println(&console, &(format!("5. list: {} doubled, {} odd, total {}", crate::core_list::size_platform(&doubled), crate::core_list::size_platform(&odd), total)));
    let mut words = vec!["ann".to_string(), "bo".to_string(), "carol".to_string()];
    let mut lengths = map__It_Fn::<ListYield<'_, String>, &String, i32>(&mut (iter__core_list(&words)), &mut (|w| { let w = *w; crate::core_string::size_platform(w) }), &mut |__i0| next__ListYield(__i0));
    println(&console, &(format!("5. lengths: {}", reduce__It_A_Fn::<ListYield<'_, i32>, &i32, i32>(&mut (iter__core_list(&lengths)), &(0), &mut (|acc, n| { let n = *n; i32::wrapping_add(*acc, *n) }), &mut |__i0| next__ListYield(__i0)))));
    let mut word = "iteration".to_string();
    let mut vowels = filter::<StrYield<'_>, char>(&mut (iter__core_string(&word)), &mut (|c| *c == 'i' || *c == 'o'), &mut |__i0| next__core_string(__i0));
    println(&console, &(format!("5. vowels: {}", crate::core_list::size_platform(&vowels))));
    println(&console, &(format!("5. halving total {}", reduce__It_A_Fn::<__Iter_halving_Int, i32, i32>(&mut (halving(20)), &(0), &mut (|acc, n| i32::wrapping_add(*acc, *n)), &mut |__i0| next__Iter_halving_Int(__i0)))));
    let mut collected = map_to::<Vec<i32>, Countdown, i32, i32>(vec![], &mut (countdown(3)), &mut (|n: &i32| i32::wrapping_mul(*n, 10)), &mut |__i0, __i1| crate::core_list::add_platform(__i0, __i1), &mut |__i0| next__Countdown(__i0));
    println(&console, &(format!("6. collected {}", crate::core_list::size_platform(&collected))));
    let mut evens = mut_str(vec![]);
    let mut __loop9_pass = range__Int_Int_Int(0, 10, 2);
    while let Union2::U1(mut i) = next__core_range(&mut __loop9_pass) {
        crate::core_string::append_platform(&mut evens, &(format!("{} ", i)));
    }
    println(&console, &(format!("7. evens {}", evens)));
}
