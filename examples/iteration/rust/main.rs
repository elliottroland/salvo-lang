#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/basic.rs"]
pub mod core_basic;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/compare.rs"]
pub mod core_compare;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/index.rs"]
pub mod core_index;
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

use crate::core_console::Console;
use crate::core_iterator::Finished;
use crate::core_list::ListYield;
use crate::core_string::StrYield;
use crate::core_list::add_platform;
use crate::core_string::append_platform;
use crate::core_iterator::emitted;
use crate::core_seq::filter;
use crate::core_seq::filter_platform;
use crate::core_iterator::finished;
use crate::core_list::get_platform;
use crate::core_seq::map__It_Fn;
use crate::core_seq::map__List_Fn;
use crate::core_seq::map_to;
use crate::core_string::mut_str;
use crate::core_list::next__ListYield;
use crate::core_console::println;
use crate::core_range::range__Int_Int_Int;
use crate::core_seq::reduce__It_A_Fn;
use crate::core_seq::reduce__List_A_Fn;


pub fn describe_container(console: &crate::core_console::Console, xs: &Vec<i32>) {
    let mut sum: i32 = 0i32;
    for mut n in xs.iter().copied() {
        sum = i32::wrapping_add(sum, n);
    }
    crate::core_console::println(console, &format!("1. list of {} sums to {}", crate::core_list::size_platform::<i32>(xs), sum));
    let mut letters: String = crate::core_string::mut_str(vec![]);
    for mut c in String::from("salvo").chars() {
        crate::core_string::append_platform(&mut letters, &format!("{}.", c));
    }
    crate::core_console::println(console, &format!("1. string: {}", letters));
    let mut arr: Vec<i32> = vec![10i32, 20i32, 30i32];
    let mut from_array: i32 = 0i32;
    for mut n in arr.iter().copied() {
        from_array = i32::wrapping_add(from_array, n);
    }
    crate::core_console::println(console, &format!("1. array sums to {}", from_array));
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

pub fn countdown(mut from: i32) -> crate::Countdown {
    return crate::Countdown { at: from };
}

pub fn next__Countdown(p: &mut crate::Countdown) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (p.at <= 0i32) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut now: i32 = p.at;
    p.at = i32::wrapping_sub(p.at, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(now));
}

pub fn skip(p: &mut crate::Countdown) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (p.at <= 1i32) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut now: i32 = p.at;
    p.at = i32::wrapping_sub(p.at, 2i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(now));
}

pub fn take(console: &crate::core_console::Console, p: &mut crate::Countdown, mut count: i32) {
    let mut seen: i32 = 0i32;
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::next__Countdown(&mut *p);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_3;
            crate::core_console::println(console, &format!("2. got {}", n));
            seen = i32::wrapping_add(seen, 1i32);
            if ((seen) == (count)) {
                break;
            };
        } else {
            break;
        };
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

pub fn halving(mut start: i32) -> crate::__Iter_halving_Int {
    return crate::__Iter_halving_Int { start: start, at: start };
}

pub fn next__Iter_halving_Int(__p: &mut crate::__Iter_halving_Int) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (__p.at <= 0i32) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut now: i32 = __p.at;
    __p.at = i32::wrapping_div(__p.at, 2i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(now));
}

pub fn halving_from_ten() -> crate::__Iter_halving_Int {
    return crate::halving(10i32);
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

pub fn iter(bag: &crate::Bag) -> crate::__Iter_iter_Bag<'_> {
    return crate::__Iter_iter_Bag { items: &bag.items, at: 0i32 };
}

pub fn next__Iter_iter_Bag(__p: &mut crate::__Iter_iter_Bag<'_>) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    let mut e: Option<i32> = crate::core_list::get_platform::<i32>(__p.items, __p.at).copied();
    if e.is_none() {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    __p.at = i32::wrapping_add(__p.at, 1i32);
    let mut e_1 = e.unwrap();
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(e_1));
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

pub fn fibs(mut count: i32) -> crate::__Iter_fibs_Int {
    return crate::__Iter_fibs_Int { count: count, a: 0i32, b: 1i32, made: 0i32 };
}

pub fn next__Iter_fibs_Int(console: &crate::core_console::Console, __p: &mut crate::__Iter_fibs_Int) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    if (__p.made >= __p.count) {
        crate::core_console::println(console, &String::from("3. finished"));
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut now: i32 = __p.a;
    let mut sum: i32 = i32::wrapping_add(__p.a, __p.b);
    __p.a = __p.b;
    __p.b = sum;
    __p.made = i32::wrapping_add(__p.made, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(now));
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

pub fn naturals(mut from: i32) -> crate::__Iter_naturals_Int {
    return crate::__Iter_naturals_Int { from: from, at: from };
}

pub fn next__Iter_naturals_Int(__p: &mut crate::__Iter_naturals_Int) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    let mut now: i32 = __p.at;
    __p.at = i32::wrapping_add(__p.at, 1i32);
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<i32>(now));
}

pub fn sum_of<It: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<i32, crate::core_iterator::Finished>) -> i32 {
    let mut total: i32 = 0i32;
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::core_iterator::Finished> = next(&mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_3;
            total = i32::wrapping_add(total, n);
        } else {
            break;
        };
    }
    return total;
}

pub fn total<'c, C: Clone, __It0: Clone>(c: &'c C, iter: &mut dyn FnMut(&'c C) -> __It0, next: &mut dyn FnMut(&mut __It0) -> crate::unions::Union2<i32, crate::core_iterator::Finished>) -> i32 {
    let mut total: i32 = 0i32;
    let mut __pass_1: __It0 = iter(c);
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::core_iterator::Finished> = next(&mut __pass_1);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_3;
            total = i32::wrapping_add(total, n);
        } else {
            break;
        };
    }
    return total;
}

pub fn first<__It0: Clone>(it: &mut __It0, next: &mut dyn FnMut(&mut __It0) -> crate::unions::Union2<i32, crate::core_iterator::Finished>) -> i32 {
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::core_iterator::Finished> = next(&mut *it);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_3;
            return n;
        } else {
            break;
        };
    }
    return i32::wrapping_neg(1i32);
}

pub fn main() {
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut xs: Vec<i32> = vec![1i32, 2i32, 3i32, 4i32];
    crate::describe_container(&__handle_2, &xs);
    let mut p: crate::Countdown = crate::countdown(5i32);
    crate::take(&__handle_2, &mut p, 2i32);
    { let __arg3 = { let __part2 = crate::sum_of::<crate::Countdown>(&mut p, &mut |__a0: &mut crate::Countdown| crate::next__Countdown(&mut *__a0)); format!("2. rest sums to {}", __part2) }; crate::core_console::println(&__handle_2, &__arg3) };
    let mut q: crate::Countdown = crate::countdown(6i32);
    loop {
        let mut __step_4: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::skip(&mut q);
        if matches!(__step_4, crate::unions::Union2::U1(_)) {
            let mut __emitted_5 = match &__step_4 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_5;
            crate::core_console::println(&__handle_2, &format!("2. skip {}", n));
        } else {
            break;
        };
    }
    let mut __pass_6: crate::__Iter_halving_Int = crate::halving(20i32);
    loop {
        let mut __step_7: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::next__Iter_halving_Int(&mut __pass_6);
        if matches!(__step_7, crate::unions::Union2::U1(_)) {
            let mut __emitted_8 = match &__step_7 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_8;
            crate::core_console::println(&__handle_2, &format!("2b. halving {}", n));
        } else {
            break;
        };
    }
    let mut hp: crate::__Iter_halving_Int = crate::halving(20i32);
    { let __arg6 = { let __part5 = crate::sum_of::<crate::__Iter_halving_Int>(&mut hp, &mut |__a0: &mut crate::__Iter_halving_Int| crate::next__Iter_halving_Int(&mut *__a0)); format!("2b. summed from a held iterator: {}", __part5) }; crate::core_console::println(&__handle_2, &__arg6) };
    crate::core_console::println(&__handle_2, &{ let __part7 = crate::first::<crate::__Iter_halving_Int>(&mut crate::halving_from_ten(), &mut |__a0: &mut crate::__Iter_halving_Int| crate::next__Iter_halving_Int(&mut *__a0)); format!("2b. first from a pattern-typed fn: {}", __part7) });
    let mut bag: crate::Bag = crate::Bag { items: vec![7i32, 8i32] };
    let mut __pass_9: crate::__Iter_iter_Bag<'_> = crate::iter(&bag);
    loop {
        let mut __step_10: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::next__Iter_iter_Bag(&mut __pass_9);
        if matches!(__step_10, crate::unions::Union2::U1(_)) {
            let mut __emitted_11 = match &__step_10 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_11;
            crate::core_console::println(&__handle_2, &format!("2c. bag {}", n));
        } else {
            break;
        };
    }
    crate::core_console::println(&__handle_2, &format!("2c. total of a bag {}, of a list {}", crate::total::<crate::Bag, crate::__Iter_iter_Bag<'_>>(&bag, &mut |__a0: &crate::Bag| crate::iter(__a0), &mut |__a0: &mut crate::__Iter_iter_Bag<'_>| crate::next__Iter_iter_Bag(&mut *__a0)), crate::total::<Vec<i32>, crate::core_list::ListYield<'_, i32>>(&xs, &mut |__a0: &Vec<i32>| crate::core_list::iter(__a0), &mut |__a0: &mut crate::core_list::ListYield<'_, i32>| (match crate::core_list::next__ListYield(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) }))));
    let mut __pass_12: crate::__Iter_fibs_Int = crate::fibs(6i32);
    loop {
        let mut __step_13: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::next__Iter_fibs_Int(&__handle_2, &mut __pass_12);
        if matches!(__step_13, crate::unions::Union2::U1(_)) {
            let mut __emitted_14 = match &__step_13 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_14;
            crate::core_console::println(&__handle_2, &format!("3. fib {}", n));
        } else {
            break;
        };
    }
    let mut __pass_15: crate::__Iter_naturals_Int = crate::naturals(10i32);
    loop {
        let mut __step_16: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::next__Iter_naturals_Int(&mut __pass_15);
        if matches!(__step_16, crate::unions::Union2::U1(_)) {
            let mut __emitted_17 = match &__step_16 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_17;
            if (n > 12i32) {
                break;
            };
            crate::core_console::println(&__handle_2, &format!("3. natural {}", n));
        } else {
            break;
        };
    }
    let mut doubled: Vec<i32> = crate::core_seq::map__List_Fn::<i32, i32>(&xs, &mut |mut n| -> i32 {
        i32::wrapping_mul(*n, 2i32)
    });
    let mut odd: Vec<i32> = crate::core_seq::filter_platform::<i32>(&xs, &mut |mut n| -> bool {
        ((i32::wrapping_rem(*n, 2i32)) == (1i32))
    });
    let mut total: i32 = crate::core_seq::reduce__List_A_Fn::<i32, i32>(&xs, 0i32, &mut |mut acc, mut n| -> i32 {
        i32::wrapping_add(*acc, *n)
    });
    crate::core_console::println(&__handle_2, &format!("5. list: {} doubled, {} odd, total {}", crate::core_list::size_platform::<i32>(&doubled), crate::core_list::size_platform::<i32>(&odd), total));
    let mut words: Vec<String> = vec![String::from("ann"), String::from("bo"), String::from("carol")];
    let mut lengths: Vec<i32> = crate::core_seq::map__It_Fn::<crate::core_list::ListYield<'_, String>, &String, i32>(&mut crate::core_list::iter::<String>(&words), &mut |mut w| -> i32 {
        let w = *w;
        crate::core_string::size_platform(w)
    }, &mut |__a0: &mut crate::core_list::ListYield<'_, String>| crate::core_list::next__ListYield(&mut *__a0));
    crate::core_console::println(&__handle_2, &{ let __part8 = crate::core_seq::reduce__It_A_Fn::<crate::core_list::ListYield<'_, i32>, i32, i32>(&mut crate::core_list::iter::<i32>(&lengths), &0i32, &mut |mut acc, mut n| -> i32 {
        i32::wrapping_add(*acc, *n)
    }, &mut |__a0: &mut crate::core_list::ListYield<'_, i32>| (match crate::core_list::next__ListYield(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) })); format!("5. lengths: {}", __part8) });
    let mut word: String = String::from("iteration");
    let mut vowels: Vec<char> = crate::core_seq::filter::<crate::core_string::StrYield<'_>, char>(&mut crate::core_string::iter(&word), &mut |mut c| -> bool {
        (((*c) == ('i')) || ((*c) == ('o')))
    }, &mut |__a0: &mut crate::core_string::StrYield<'_>| crate::core_string::next(&mut *__a0));
    crate::core_console::println(&__handle_2, &format!("5. vowels: {}", crate::core_list::size_platform::<char>(&vowels)));
    crate::core_console::println(&__handle_2, &{ let __part9 = crate::core_seq::reduce__It_A_Fn::<crate::__Iter_halving_Int, i32, i32>(&mut crate::halving(20i32), &0i32, &mut |mut acc, mut n| -> i32 {
        i32::wrapping_add(*acc, *n)
    }, &mut |__a0: &mut crate::__Iter_halving_Int| crate::next__Iter_halving_Int(&mut *__a0)); format!("5. halving total {}", __part9) });
    let mut collected: Vec<i32> = crate::core_seq::map_to::<Vec<i32>, crate::Countdown, i32, i32>(vec![], &mut crate::countdown(3i32), &mut |mut n| -> i32 {
        i32::wrapping_mul(*n, 10i32)
    }, &mut |__a0: &mut Vec<i32>, __a1: i32| crate::core_list::add_platform(&mut *__a0, __a1), &mut |__a0: &mut crate::Countdown| crate::next__Countdown(&mut *__a0));
    crate::core_console::println(&__handle_2, &format!("6. collected {}", crate::core_list::size_platform::<i32>(&collected)));
    let mut evens: String = crate::core_string::mut_str(vec![]);
    let mut __pass_18: crate::core_range::__Iter_range_Int_Int_Int = crate::core_range::range__Int_Int_Int(0i32, 10i32, 2i32);
    loop {
        let mut __step_19: crate::unions::Union2<i32, crate::core_iterator::Finished> = crate::core_range::next(&mut __pass_18);
        if matches!(__step_19, crate::unions::Union2::U1(_)) {
            let mut __emitted_20 = match &__step_19 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut i: i32 = __emitted_20;
            crate::core_string::append_platform(&mut evens, &format!("{} ", i));
        } else {
            break;
        };
    }
    crate::core_console::println(&__handle_2, &format!("7. evens {}", evens));
}
