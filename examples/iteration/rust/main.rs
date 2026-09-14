#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/nonempty.rs"]
pub mod core_nonempty;
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
use crate::core_seq::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;
use crate::unions::*;

pub fn describe_container(console: &mut dyn Console, xs: &Vec<i32>) {
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

pub fn countdown(from: i32) -> Countdown {
    return Countdown { at: from };
}

pub fn next__6(p: &mut Countdown) -> Union2<i32, Finished> {
    if p.at <= 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = p.at;
    p.at = p.at - 1;
    return Union2::<i32, Finished>::U1(emitted(now));
}

pub fn take(console: &mut dyn Console, p: &mut Countdown, count: i32) {
    let mut seen = 0;
    while let Union2::U1(mut n) = next__6(p) {
        println(console, &(format!("2. got {}", n)));
        seen = seen + 1;
        if seen == count {
            break;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Halving {
    pub start: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Pass_Halving {
    pub at: i32,
}

pub fn iter__8(h: &Halving) -> __Pass_Halving {
    return __Pass_Halving { at: h.start };
}

pub fn next__7(__p: &mut __Pass_Halving) -> Union2<i32, Finished> {
    if __p.at <= 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let mut now = __p.at;
    __p.at = __p.at / 2;
    return Union2::<i32, Finished>::U1(emitted(now));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fibs {
    pub count: i32,
}

pub fn fibs(count: i32) -> Fibs {
    return Fibs { count: count };
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Pass_Fibs {
    pub count: i32,
    pub a: i32,
    pub b: i32,
    pub made: i32,
}

pub fn iter__9(f: &Fibs) -> __Pass_Fibs {
    return __Pass_Fibs { count: f.count, a: 0, b: 1, made: 0 };
}

pub fn next__8(console: &mut dyn Console, __p: &mut __Pass_Fibs) -> Union2<i32, Finished> {
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
pub struct Naturals {
    pub from: i32,
}

pub fn naturals(from: i32) -> Naturals {
    return Naturals { from: from };
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Pass_Naturals {
    pub at: i32,
}

pub fn iter__10(n: &Naturals) -> __Pass_Naturals {
    return __Pass_Naturals { at: n.from };
}

pub fn next__9(__p: &mut __Pass_Naturals) -> Union2<i32, Finished> {
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

pub fn main() {
    let mut console = StdOutConsole::new();
    let mut xs = vec![1, 2, 3, 4];
    describe_container(&mut console, &xs);
    let mut p = countdown(5);
    take(&mut console, &mut p, 2);
    println(&mut console, &(format!("2. rest sums to {}", sum_of::<Countdown>(&mut p, &mut |__i0| next__6(__i0)))));
    let mut h = Halving { start: 20 };
    let mut __loop3_pass = iter__8(&h);
    while let Union2::U1(mut n) = next__7(&mut __loop3_pass) {
        println(&mut console, &(format!("2b. halving {}", n)));
    }
    let mut hp = iter__8(&h);
    println(&mut console, &(format!("2b. summed from a held pass: {}", sum_of::<__Pass_Halving>(&mut hp, &mut |__i0| next__7(__i0)))));
    let mut __loop4_pass = iter__9(&(fibs(6)));
    while let Union2::U1(mut n) = next__8(&mut console, &mut __loop4_pass) {
        println(&mut console, &(format!("3. fib {}", n)));
    }
    let mut __loop5_pass = iter__10(&(naturals(10)));
    while let Union2::U1(mut n) = next__9(&mut __loop5_pass) {
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
    let mut lengths = map::<ListYield<String>, &String, i32>(&mut (iter__2(&words)), &mut (|w| { let w = *w; (w.chars().count() as i32) }), &mut |__i0| next__2(__i0));
    println(&mut console, &(format!("5. lengths: {}", reduce::<ListYield<i32>, &i32, i32>(&mut (iter__2(&lengths)), &(0), &mut (|acc, n| { let n = *n; *acc + *n }), &mut |__i0| next__2(__i0)))));
    let mut word = "iteration".to_string();
    let mut vowels = filter::<StrYield, char>(&mut (iter__7(&word)), &mut (|c| *c == 'i' || *c == 'o'), &mut |__i0| next__5(__i0));
    println(&mut console, &(format!("5. vowels: {}", (vowels.len() as i32))));
    println(&mut console, &(format!("5. halving total {}", reduce::<__Pass_Halving, i32, i32>(&mut (iter__8(&(Halving { start: 20 }))), &(0), &mut (|acc, n| *acc + *n), &mut |__i0| next__7(__i0)))));
    let mut collected = map_to::<Vec<i32>, Countdown, i32, i32>(vec![], &mut (countdown(3)), &mut (|n: &i32| *n * 10), &mut |__i0, __i1| __i0.push(__i1), &mut |__i0| next__6(__i0));
    println(&mut console, &(format!("6. collected {}", (collected.len() as i32))));
}
