#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "std/core/array.rs"]
pub mod std_core_array;
#[path = "std/core/basic.rs"]
pub mod std_core_basic;
#[path = "std/core/checked.rs"]
pub mod std_core_checked;
#[path = "std/core/compare.rs"]
pub mod std_core_compare;
#[path = "std/core/console.rs"]
pub mod std_core_console;
#[path = "std/core/index.rs"]
pub mod std_core_index;
#[path = "std/core/iterator.rs"]
pub mod std_core_iterator;
#[path = "std/core/list.rs"]
pub mod std_core_list;
#[path = "std/core/map.rs"]
pub mod std_core_map;
#[path = "std/core/range.rs"]
pub mod std_core_range;
#[path = "std/core/seq.rs"]
pub mod std_core_seq;
#[path = "std/core/set.rs"]
pub mod std_core_set;
#[path = "std/core/sorted.rs"]
pub mod std_core_sorted;
#[path = "std/core/string.rs"]
pub mod std_core_string;
#[path = "platform/std/core/console.rs"]
pub mod platform_std_core_console;
#[path = "platform/std/core/list.rs"]
pub mod platform_std_core_list;
#[path = "platform/std/core/map.rs"]
pub mod platform_std_core_map;
#[path = "platform/std/core/seq.rs"]
pub mod platform_std_core_seq;
#[path = "platform/std/core/set.rs"]
pub mod platform_std_core_set;
#[path = "platform/std/core/sorted.rs"]
pub mod platform_std_core_sorted;
#[path = "platform/std/core/string.rs"]
pub mod platform_std_core_string;

use crate::std_core_console::Console;
use crate::std_core_iterator::Finished;
use crate::std_core_list::ListNext;
use crate::std_core_string::Mut_Str;
use crate::std_core_range::Range;
use crate::std_core_string::StrNext;
use crate::std_core_list::add_platform;
use crate::std_core_string::append_platform;
use crate::std_core_iterator::emitted;
use crate::std_core_seq::filter;
use crate::std_core_seq::filter_platform;
use crate::std_core_iterator::finished;
use crate::std_core_list::get_platform;
use crate::std_core_seq::map__It_Fn;
use crate::std_core_seq::map__List_Fn;
use crate::std_core_seq::map_to;
use crate::std_core_list::next__ListNext;
use crate::std_core_console::println;
use crate::std_core_range::range__Int_Int_Int;
use crate::std_core_seq::reduce__It_A_Fn;
use crate::std_core_seq::reduce__List_A_Fn;


pub fn describe_container(console: &crate::std_core_console::Console, xs: &Vec<i32>) {
    let mut sum: i32 = 0i32;
    for mut n in xs.iter().copied() {
        sum = i32::wrapping_add(sum, n);
    }
    crate::std_core_console::println(console, &format!("1. list of {} sums to {}", crate::std_core_list::size_platform::<i32>(xs), sum));
    let mut letters: String = crate::std_core_string::Mut_Str(vec![]);
    for mut c in String::from("salvo").chars() {
        crate::std_core_string::append_platform(&mut letters, &format!("{}.", c));
    }
    crate::std_core_console::println(console, &format!("1. string: {}", letters));
    let mut arr: Vec<i32> = vec![10i32, 20i32, 30i32];
    let mut from_array: i32 = 0i32;
    for mut n in arr.iter().copied() {
        from_array = i32::wrapping_add(from_array, n);
    }
    crate::std_core_console::println(console, &format!("1. array sums to {}", from_array));
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

pub fn next__Countdown(p: &mut crate::Countdown) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    if (p.at <= 0i32) {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut now: i32 = p.at;
    p.at = i32::wrapping_sub(p.at, 1i32);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(now));
}

pub fn skip(p: &mut crate::Countdown) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    if (p.at <= 1i32) {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut now: i32 = p.at;
    p.at = i32::wrapping_sub(p.at, 2i32);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(now));
}

pub fn take(console: &crate::std_core_console::Console, p: &mut crate::Countdown, mut count: i32) {
    let mut seen: i32 = 0i32;
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::next__Countdown(&mut *p);
        if matches!(__step_2, crate::unions::Union2::U1(_)) {
            let mut __emitted_3 = match &__step_2 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_3;
            crate::std_core_console::println(console, &format!("2. got {}", n));
            seen = i32::wrapping_add(seen, 1i32);
            if (seen == count) {
                break;
            };
        } else {
            break;
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Halving {
    pub start: i32,
    pub at: i32,
}

impl crate::wire::__Wire for Halving {
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

pub fn halving(mut start: i32) -> crate::Halving {
    return crate::Halving { start: start, at: start };
}

pub fn next__Halving(__p: &mut crate::Halving) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    if (__p.at <= 0i32) {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut now: i32 = __p.at;
    __p.at = i32::wrapping_div(__p.at, 2i32);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(now));
}

pub fn halving_from_ten() -> crate::Halving {
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
pub struct BagIter<'s> {
    pub bag: &'s crate::Bag,
    pub at: i32,
}

pub fn iter(bag: &crate::Bag) -> crate::BagIter<'_> {
    return crate::BagIter { bag: bag, at: 0i32 };
}

pub fn next__BagIter(__p: &mut crate::BagIter<'_>) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    let mut e: Option<i32> = crate::std_core_list::get_platform::<i32>(&__p.bag.items, __p.at).copied();
    if e.is_none() {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    __p.at = i32::wrapping_add(__p.at, 1i32);
    let mut e_1 = e.unwrap();
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(e_1));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fibs {
    pub count: i32,
    pub a: i32,
    pub b: i32,
    pub made: i32,
}

impl crate::wire::__Wire for Fibs {
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

pub fn fibs(mut count: i32) -> crate::Fibs {
    return crate::Fibs { count: count, a: 0i32, b: 1i32, made: 0i32 };
}

pub fn next__Fibs(console: &crate::std_core_console::Console, __p: &mut crate::Fibs) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    if (__p.made >= __p.count) {
        crate::std_core_console::println(console, &String::from("3. finished"));
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    let mut now: i32 = __p.a;
    let mut sum: i32 = i32::wrapping_add(__p.a, __p.b);
    __p.a = __p.b;
    __p.b = sum;
    __p.made = i32::wrapping_add(__p.made, 1i32);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(now));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Naturals {
    pub from: i32,
    pub at: i32,
}

impl crate::wire::__Wire for Naturals {
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

pub fn naturals(mut from: i32) -> crate::Naturals {
    return crate::Naturals { from: from, at: from };
}

pub fn next__Naturals(__p: &mut crate::Naturals) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    let mut now: i32 = __p.at;
    __p.at = i32::wrapping_add(__p.at, 1i32);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(now));
}

pub fn sum_of<It: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished>) -> i32 {
    let mut total: i32 = 0i32;
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = next(&mut *it);
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

pub fn total<'c, C: Clone, It: Clone>(c: &'c C, iter: &mut dyn FnMut(&'c C) -> It, next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished>) -> i32 {
    let mut total: i32 = 0i32;
    let mut __pass_1: It = iter(c);
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = next(&mut __pass_1);
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

pub fn first<It: Clone>(it: &mut It, next: &mut dyn FnMut(&mut It) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished>) -> i32 {
    loop {
        let mut __step_2: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = next(&mut *it);
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
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut xs: Vec<i32> = vec![1i32, 2i32, 3i32, 4i32];
    crate::describe_container(&__handle_2, &xs);
    let mut p: crate::Countdown = crate::countdown(5i32);
    crate::take(&__handle_2, &mut p, 2i32);
    { let __arg3 = { let __part2 = crate::sum_of::<crate::Countdown>(&mut p, &mut |__a0: &mut crate::Countdown| crate::next__Countdown(&mut *__a0)); format!("2. rest sums to {}", __part2) }; crate::std_core_console::println(&__handle_2, &__arg3) };
    let mut q: crate::Countdown = crate::countdown(6i32);
    loop {
        let mut __step_4: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::skip(&mut q);
        if matches!(__step_4, crate::unions::Union2::U1(_)) {
            let mut __emitted_5 = match &__step_4 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_5;
            crate::std_core_console::println(&__handle_2, &format!("2. skip {}", n));
        } else {
            break;
        };
    }
    let mut __pass_6: crate::Halving = crate::halving(20i32);
    loop {
        let mut __step_7: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::next__Halving(&mut __pass_6);
        if matches!(__step_7, crate::unions::Union2::U1(_)) {
            let mut __emitted_8 = match &__step_7 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_8;
            crate::std_core_console::println(&__handle_2, &format!("2b. halving {}", n));
        } else {
            break;
        };
    }
    let mut hp: crate::Halving = crate::halving(20i32);
    { let __arg6 = { let __part5 = crate::sum_of::<crate::Halving>(&mut hp, &mut |__a0: &mut crate::Halving| crate::next__Halving(&mut *__a0)); format!("2b. summed from a held iterator: {}", __part5) }; crate::std_core_console::println(&__handle_2, &__arg6) };
    crate::std_core_console::println(&__handle_2, &{ let __part7 = crate::first::<crate::Halving>(&mut crate::halving_from_ten(), &mut |__a0: &mut crate::Halving| crate::next__Halving(&mut *__a0)); format!("2b. first from a fn returning one: {}", __part7) });
    let mut bag: crate::Bag = crate::Bag { items: vec![7i32, 8i32] };
    let mut __pass_9: crate::BagIter<'_> = crate::iter(&bag);
    loop {
        let mut __step_10: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::next__BagIter(&mut __pass_9);
        if matches!(__step_10, crate::unions::Union2::U1(_)) {
            let mut __emitted_11 = match &__step_10 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_11;
            crate::std_core_console::println(&__handle_2, &format!("2c. bag {}", n));
        } else {
            break;
        };
    }
    crate::std_core_console::println(&__handle_2, &format!("2c. total of a bag {}, of a list {}", crate::total::<crate::Bag, crate::BagIter<'_>>(&bag, &mut |__a0: &crate::Bag| crate::iter(__a0), &mut |__a0: &mut crate::BagIter<'_>| crate::next__BagIter(&mut *__a0)), crate::total::<Vec<i32>, crate::std_core_list::ListNext<'_, i32>>(&xs, &mut |__a0: &Vec<i32>| crate::std_core_list::iter(__a0), &mut |__a0: &mut crate::std_core_list::ListNext<'_, i32>| (match crate::std_core_list::next__ListNext(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) }))));
    let mut __pass_12: crate::Fibs = crate::fibs(6i32);
    loop {
        let mut __step_13: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::next__Fibs(&__handle_2, &mut __pass_12);
        if matches!(__step_13, crate::unions::Union2::U1(_)) {
            let mut __emitted_14 = match &__step_13 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_14;
            crate::std_core_console::println(&__handle_2, &format!("3. fib {}", n));
        } else {
            break;
        };
    }
    let mut __pass_15: crate::Naturals = crate::naturals(10i32);
    loop {
        let mut __step_16: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::next__Naturals(&mut __pass_15);
        if matches!(__step_16, crate::unions::Union2::U1(_)) {
            let mut __emitted_17 = match &__step_16 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut n: i32 = __emitted_17;
            if (n > 12i32) {
                break;
            };
            crate::std_core_console::println(&__handle_2, &format!("3. natural {}", n));
        } else {
            break;
        };
    }
    let mut doubled: Vec<i32> = crate::std_core_seq::map__List_Fn::<i32, i32>(&xs, &mut |mut n| -> i32 {
        i32::wrapping_mul(*n, 2i32)
    });
    let mut odd: Vec<i32> = crate::std_core_seq::filter_platform::<i32>(&xs, &mut |mut n| -> bool {
        (i32::wrapping_rem(*n, 2i32) == 1i32)
    }).into_iter().copied().collect::<Vec<_>>();
    let mut total: i32 = crate::std_core_seq::reduce__List_A_Fn::<i32, i32>(&xs, 0i32, &mut |mut acc, mut n| -> i32 {
        i32::wrapping_add(*acc, *n)
    });
    crate::std_core_console::println(&__handle_2, &format!("5. list: {} doubled, {} odd, total {}", crate::std_core_list::size_platform::<i32>(&doubled), crate::std_core_list::size_platform::<i32>(&odd), total));
    let mut words: Vec<String> = vec![String::from("ann"), String::from("bo"), String::from("carol")];
    let mut lengths: Vec<i32> = crate::std_core_seq::map__It_Fn::<crate::std_core_list::ListNext<'_, String>, &String, i32>(&mut crate::std_core_list::iter::<String>(&words), &mut |mut w| -> i32 {
        let w = *w;
        crate::std_core_string::size_platform(w)
    }, &mut |__a0: &mut crate::std_core_list::ListNext<'_, String>| crate::std_core_list::next__ListNext(&mut *__a0));
    crate::std_core_console::println(&__handle_2, &{ let __part8 = crate::std_core_seq::reduce__It_A_Fn::<crate::std_core_list::ListNext<'_, i32>, i32, i32>(&mut crate::std_core_list::iter::<i32>(&lengths), &0i32, &mut |mut acc, mut n| -> i32 {
        i32::wrapping_add(*acc, *n)
    }, &mut |__a0: &mut crate::std_core_list::ListNext<'_, i32>| (match crate::std_core_list::next__ListNext(&mut *__a0) { crate::unions::Union2::U1(__v) => crate::unions::Union2::U1(*__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U2(__v) })); format!("5. lengths: {}", __part8) });
    let mut word: String = String::from("iteration");
    let mut vowels: Vec<char> = crate::std_core_seq::filter::<crate::std_core_string::StrNext<'_>, char>(&mut crate::std_core_string::iter(&word), &mut |mut c| -> bool {
        ((*c == 'i') || (*c == 'o'))
    }, &mut |__a0: &mut crate::std_core_string::StrNext<'_>| crate::std_core_string::next(&mut *__a0));
    crate::std_core_console::println(&__handle_2, &format!("5. vowels: {}", crate::std_core_list::size_platform::<char>(&vowels)));
    crate::std_core_console::println(&__handle_2, &{ let __part9 = crate::std_core_seq::reduce__It_A_Fn::<crate::Halving, i32, i32>(&mut crate::halving(20i32), &0i32, &mut |mut acc, mut n| -> i32 {
        i32::wrapping_add(*acc, *n)
    }, &mut |__a0: &mut crate::Halving| crate::next__Halving(&mut *__a0)); format!("5. halving total {}", __part9) });
    let mut collected: Vec<i32> = crate::std_core_seq::map_to::<Vec<i32>, crate::Countdown, i32, i32>(vec![], &mut crate::countdown(3i32), &mut |mut n| -> i32 {
        i32::wrapping_mul(*n, 10i32)
    }, &mut |__a0: &mut Vec<i32>, __a1: i32| crate::std_core_list::add_platform(&mut *__a0, __a1), &mut |__a0: &mut crate::Countdown| crate::next__Countdown(&mut *__a0));
    crate::std_core_console::println(&__handle_2, &format!("6. collected {}", crate::std_core_list::size_platform::<i32>(&collected)));
    let mut evens: String = crate::std_core_string::Mut_Str(vec![]);
    let mut __pass_18: crate::std_core_range::Range = crate::std_core_range::range__Int_Int_Int(0i32, 10i32, 2i32);
    loop {
        let mut __step_19: crate::unions::Union2<i32, crate::std_core_iterator::Finished> = crate::std_core_range::next(&mut __pass_18);
        if matches!(__step_19, crate::unions::Union2::U1(_)) {
            let mut __emitted_20 = match &__step_19 { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut i: i32 = __emitted_20;
            crate::std_core_string::append_platform(&mut evens, &format!("{} ", i));
        } else {
            break;
        };
    }
    crate::std_core_console::println(&__handle_2, &format!("7. evens {}", evens));
}
