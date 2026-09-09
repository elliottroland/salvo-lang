#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/seq.rs"]
pub mod core_seq;
#[path = "core/string.rs"]
pub mod core_string;

use crate::core_array::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_seq::*;
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Fibs {
    pub count: i32,
}

pub fn fibs(count: i32) -> Fibs {
    return Fibs { count: count };
}

#[derive(Clone, Debug)]
pub struct Naturals {
    pub from: i32,
}

pub fn naturals(from: i32) -> Naturals {
    return Naturals { from: from };
}

pub fn sum_of<It: Clone + 'static>(it: &mut It, next: &mut dyn FnMut(&mut It) -> Union2<i32, Finished>) -> i32 {
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
    let mut __loop3_pass = __Pass_Fibs::new(fibs(6));
    while let Some(mut n) = __loop3_pass.__advance(&mut console) {
        println(&mut console, &(format!("3. fib {}", n)));
    }
    __loop3_pass.__close(&mut console);
    { let __a1 = &(format!("3. again sums to {}", { let mut __mint1 = __Pass_Fibs::new(fibs(6)); let __call = sum_of::<__Pass_Fibs>(&mut __mint1, &mut |__p: &mut __Pass_Fibs| match __p.__advance(&mut console) { Some(__v) => Union2::<i32, Finished>::U1(__v), None => Union2::<i32, Finished>::U2(Finished {}) }); __mint1.__close(&mut console); __call })); println(&mut console, __a1) };
    let mut __loop4_pass = __Pass_Naturals::new(naturals(10));
    while let Some(mut n) = __loop4_pass.__advance() {
        if n > 12 {
            break;
        }
        println(&mut console, &(format!("3. natural {}", n)));
    }
    __loop4_pass.__close();
    let mut doubled = salvo_map(&xs[..], |n| *n * 2);
    let mut odd = salvo_filter(&xs[..], |n| *n % 2 == 1);
    let mut total = salvo_reduce(&xs[..], 0, |acc, n| *acc + *n);
    println(&mut console, &(format!("5. list: {} doubled, {} odd, total {}", (doubled.len() as i32), (odd.len() as i32), total)));
    let mut words = vec!["ann".to_string(), "bo".to_string(), "carol".to_string()];
    let mut lengths = map::<ListYield<String>, String, i32>(&mut (iter__2(words)), &mut (|w| (w.chars().count() as i32)), &mut |__i0| next__2(__i0));
    println(&mut console, &(format!("5. lengths: {}", reduce::<ListYield<i32>, i32, i32>(&mut (iter__2(lengths)), 0, &mut (|acc, n| *acc + *n), &mut |__i0| next__2(__i0)))));
    let mut vowels = filter::<StrYield, char>(&mut (iter__3("iteration".to_string())), &mut (|c| *c == 'i' || *c == 'o'), &mut |__i0| next__5(__i0));
    println(&mut console, &(format!("5. vowels: {}", (vowels.len() as i32))));
    { let __a2 = &(format!("5. fibs total {}", { let mut __mint2 = __Pass_Fibs::new(fibs(6)); let __call = reduce::<__Pass_Fibs, i32, i32>(&mut __mint2, 0, &mut (|acc, n| acc.clone() + n.clone()), &mut |__p: &mut __Pass_Fibs| match __p.__advance(&mut console) { Some(__v) => Union2::<i32, Finished>::U1(__v), None => Union2::<i32, Finished>::U2(Finished {}) }); __mint2.__close(&mut console); __call })); println(&mut console, __a2) };
    let mut squares = { let mut __mint3 = __Pass_Naturals::new(naturals(1)); let __call = map_lazy::<__Pass_Naturals, i32, i32>(__mint3, move |n: &i32| *n * *n, move |__p: &mut __Pass_Naturals| match __p.__advance() { Some(__v) => Union2::<i32, Finished>::U1(__v), None => Union2::<i32, Finished>::U2(Finished {}) });  __call };
    let mut big = filter_lazy::<MapYield<__Pass_Naturals, i32, i32>, i32>(squares, move |n: &i32| *n > 10, move |__i0| next__3(__i0));
    let mut __loop5_pass = big;
    while let Union2::U1(mut n) = next__4(&mut __loop5_pass) {
        println(&mut console, &(format!("6. big square {}", n)));
        if n > 50 {
            break;
        }
    }
    let mut collected = map_to::<Vec<i32>, Countdown, i32, i32>(vec![], &mut (countdown(3)), &mut (|n: &i32| *n * 10), &mut |__i0, __i1| __i0.push(__i1), &mut |__i0| next__6(__i0));
    println(&mut console, &(format!("6. collected {}", (collected.len() as i32))));
}

#[derive(Clone)]
pub struct __Pass_Fibs {
    f: Fibs,
    a: i32,
    b: i32,
    made: i32,
    sum: i32,
    __state: u32,
    __d0: bool,
}

impl __Pass_Fibs {
    pub fn new(f: Fibs) -> Self {
        Self {
            f,
            a: 0,
            b: 0,
            made: 0,
            sum: 0,
            __state: 0,
            __d0: false,
        }
    }

    pub fn __advance(&mut self, console: &mut dyn Console) -> Option<i32> {
        loop {
            match self.__state {
                0 => {
                    println(console, &("3. opening".to_string()));
                    self.__d0 = true;
                    self.a = 0;
                    self.b = 1;
                    self.made = 0;
                    self.__state = 1;
                    continue;
                }
                1 => {
                    if !(self.made < self.f.count) {
                        self.__state = 3;
                        continue;
                    }
                    let __v = self.a.clone();
                    self.__state = 2;
                    return Some(__v);
                }
                2 => {
                    self.sum = self.a + self.b;
                    self.a = self.b.clone();
                    self.b = self.sum.clone();
                    self.made = self.made + 1;
                    self.__state = 1;
                    continue;
                }
                3 => {
                    self.__run_d0(console);
                    self.__state = 4;
                    return None;
                }
                4 => {
                    self.__state = 4;
                    return None;
                }
                _ => return None,
            }
        }
    }

    pub fn __close(&mut self, console: &mut dyn Console) {
        self.__run_d0(console);
        self.__state = 4;
    }

    fn __run_d0(&mut self, console: &mut dyn Console) {
        if self.__d0 {
            self.__d0 = false;
            println(console, &("3. closing".to_string()));
        }
    }
}

#[derive(Clone)]
pub struct __Pass_Naturals {
    n: Naturals,
    i: i32,
    __state: u32,
}

impl __Pass_Naturals {
    pub fn new(n: Naturals) -> Self {
        Self {
            n,
            i: 0,
            __state: 0,
        }
    }

    pub fn __advance(&mut self) -> Option<i32> {
        loop {
            match self.__state {
                0 => {
                    self.i = self.n.from;
                    self.__state = 1;
                    continue;
                }
                1 => {
                    if !(true) {
                        self.__state = 3;
                        continue;
                    }
                    let __v = self.i.clone();
                    self.__state = 2;
                    return Some(__v);
                }
                2 => {
                    self.i = self.i + 1;
                    self.__state = 1;
                    continue;
                }
                3 => {
                    self.__state = 3;
                    return None;
                }
                _ => return None,
            }
        }
    }

    pub fn __close(&mut self) {
        self.__state = 3;
    }
}
