#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
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
#[path = "platform/core/set.rs"]
pub mod platform_core_set;
#[path = "platform/core/sorted.rs"]
pub mod platform_core_sorted;
#[path = "platform/core/string.rs"]
pub mod platform_core_string;

use crate::unions::*;
use crate::core_compare::mix_hash;
use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;
use crate::core_list::add_sorted;
use crate::core_list::at;
use crate::core_list::binary_search;
use crate::core_list::first;
use crate::core_list::list_by;
use crate::core_list::mut_sort;
use crate::core_list::sort;
use crate::core_list::to_str as to_str__core_list;
use crate::core_map::Map;
use crate::core_map::iter as iter__core_map;
use crate::core_map::next__MapKeyYield;
use crate::core_map::to_map;
use crate::core_map::to_str as to_str__core_map;
use crate::core_set::Set;
use crate::core_set::iter as iter__core_set;
use crate::core_set::next;
use crate::core_set::to_str as to_str__core_set;
use crate::core_sorted::SortedSet;
use crate::core_sorted::to_str__SortedSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl crate::wire::__Wire for Point {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.x, out);
        crate::wire::__Wire::__enc(&self.y, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            x: crate::wire::__Wire::__dec(r)?,
            y: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
}

impl crate::wire::__Wire for Note {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.text, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            text: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn by_len(a: &String, b: &String) -> i32 {
    return (Ord::cmp(&(crate::core_string::size_platform(a)), &(crate::core_string::size_platform(b))) as i32);
}

pub fn count_unique(xs: &Vec<i32>) -> i32 {
    return crate::core_list::size_platform(xs);
}

pub fn main() {
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let mut primes = vec![2, 3, 5, 7];
    let mut vowels = crate::core_set::set_of_platform::<String>(vec!["a".to_string(), "e".to_string(), "i".to_string(), "o".to_string(), "u".to_string()], &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    let mut ages = crate::core_map::map_of_platform::<String, i32>(vec![("ada".to_string(), 36), ("grace".to_string(), 45)], &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    println(&console, &(format!("1. list {}", to_str__core_list::<i32>(&primes, &mut |__i0| format!("{}", __i0)))));
    println(&console, &(format!("1. set {} of {}", to_str__core_set::<String>(&vowels, &mut |__i0| format!("{}", __i0)), crate::core_set::size_platform(&vowels))));
    println(&console, &(format!("1. map {}", to_str__core_map::<String, i32>(&ages, &mut |__i0| format!("{}", __i0), &mut |__i0| format!("{}", __i0), &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..])))));
    let mut note: Note = Note { text: "still a struct literal".to_string() };
    println(&console, &(format!("1. struct {}", note.text.clone())));
    let mut seen: Set<String> = crate::core_set::mut_set_of_platform::<String>(vec![], &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    crate::core_set::add_platform::<String>(&mut seen, "first".to_string(), &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    println(&console, &(format!("1. empty then filled {}", to_str__core_set::<String>(&seen, &mut |__i0| format!("{}", __i0)))));
    let mut tally: Map<String, i32> = crate::core_map::mut_map_of_platform::<String, i32>(vec![("pear".to_string(), 1), ("apple".to_string(), 2)], &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    crate::core_map::put_platform::<String, i32>(&mut tally, "fig".to_string(), 3, &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    crate::core_map::put_platform::<String, i32>(&mut tally, "pear".to_string(), 99, &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    println(&console, &(format!("2. insertion order kept {}", to_str__core_map::<String, i32>(&tally, &mut |__i0| format!("{}", __i0), &mut |__i0| format!("{}", __i0), &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..])))));
    let mut ranked: SortedSet<String> = crate::core_sorted::mut_sorted_set_of_platform::<String>(vec!["pear".to_string(), "apple".to_string(), "fig".to_string()], &mut |__i0, __i1| (Ord::cmp(&__i0[..], &__i1[..]) as i32));
    println(&console, &(format!("2. key order {}", to_str__SortedSet::<String>(&ranked, &mut |__i0| format!("{}", __i0)))));
    let mut smallest = crate::core_sorted::min_platform(&ranked);
    if smallest.is_some() {
        println(&console, &(format!("2. min is cheap here {}", smallest.as_ref().unwrap().clone())));
    }
    let mut corners: Set<Point> = crate::core_set::mut_set_of_platform::<Point>(vec![], &mut |__i0| hash(__i0), &mut |__i0, __i1| eq__Point_Point(__i0, __i1));
    crate::core_set::add_platform::<Point>(&mut corners, Point { x: 0, y: 0 }, &mut |__i0| hash(__i0), &mut |__i0, __i1| eq__Point_Point(__i0, __i1));
    let mut again = crate::core_set::add_platform::<Point>(&mut corners, Point { x: 0, y: 0 }, &mut |__i0| hash(__i0), &mut |__i0, __i1| eq__Point_Point(__i0, __i1));
    println(&console, &(format!("3. struct key: size {}, second add {}", crate::core_set::size_platform(&corners), again)));
    let mut labels: Map<Point, String> = crate::core_map::mut_map_of_platform::<Point, String>(vec![], &mut |__i0| hash(__i0), &mut |__i0, __i1| eq__Point_Point(__i0, __i1));
    crate::core_map::put_platform::<Point, String>(&mut labels, Point { x: 1, y: 1 }, "diagonal".to_string(), &mut |__i0| hash(__i0), &mut |__i0, __i1| eq__Point_Point(__i0, __i1));
    let mut found = crate::core_map::get_platform::<Point, String>(&labels, &(Point { x: 1, y: 1 }), &mut |__i0| hash(__i0), &mut |__i0, __i1| eq__Point_Point(__i0, __i1));
    if found.is_some() {
        println(&console, &(format!("3. looked up by value {}", found.unwrap().clone())));
    }
    let mut a = Point { x: 1, y: 2 };
    let mut b = Point { x: 1, y: 2 };
    let mut c = Point { x: 1, y: 9 };
    let mut same = eq__Point_Point(&a, &b);
    let mut before = cmp(&a, &c) < 0;
    println(&console, &(format!("4. equal {}, ordered {}", same, before)));
    let mut n1 = Note { text: "same".to_string() };
    let mut n2 = Note { text: "same".to_string() };
    let mut notes_equal = eq__Note_Note(&n1, &n2);
    println(&console, &(format!("4. plain struct equality {}", notes_equal)));
    let mut squares = list_by(4, &mut (|i| i32::wrapping_mul(i, i)));
    println(&console, &(format!("5. generated {}", to_str__core_list::<i32>(&squares, &mut |__i0| format!("{}", __i0)))));
    let mut deduped = crate::core_set::to_set_platform::<i32>(&primes, &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(__i0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| ((__i0) == (__i1)));
    println(&console, &(format!("5. to_set {}", to_str__core_set::<i32>(&deduped, &mut |__i0| format!("{}", __i0)))));
    let mut words = vec!["alpha".to_string(), "be".to_string()];
    let mut lengths = to_map::<String, String, i32>(&words, &mut (|w| (w.clone(), crate::core_string::size_platform(w))), &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
    println(&console, &(format!("5. to_map with a rule {}", to_str__core_map::<String, i32>(&lengths, &mut |__i0| format!("{}", __i0), &mut |__i0| format!("{}", __i0), &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..])))));
    let mut filled = vec!["ada".to_string(), "grace".to_string()];
    println(&console, &(format!("6. first is {}, no optional", first(&filled))));
    let mut growing: Vec<i32> = vec![];
    crate::core_list::add_platform(&mut growing, 7);
    println(&console, &(format!("6. after add, first is {}", *first(&growing))));
    let mut ordered = sort::<i32>(&(vec![40, 10, 30, 20]), &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    println(&console, &(format!("6. sorted {}", to_str__core_list::<i32>(&ordered, &mut |__i0| format!("{}", __i0)))));
    let mut __is1 = binary_search::<i32>(&ordered, &(30), &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    if __is1.is_some() {
        let mut at = __is1.unwrap();
        println(&console, &(format!("6. found 30 at {}", at)));
    }
    let mut live: Vec<i32> = mut_sort::<i32>(&(vec![10, 30]), &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    add_sorted::<i32>(&mut live, 20, &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    add_sorted::<i32>(&mut live, 5, &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    println(&console, &(format!("6. still sorted {}", to_str__core_list::<i32>(&live, &mut |__i0| format!("{}", __i0)))));
    let mut bylen = sort::<String>(&(vec!["alpha".to_string(), "be".to_string(), "z".to_string()]), &mut |__i0, __i1| by_len(__i0, __i1));
    println(&console, &(format!("6. by length {}", to_str__core_list::<String>(&bylen, &mut |__i0| format!("{}", __i0)))));
    let mut __is2 = binary_search::<String>(&bylen, &("hi".to_string()), &mut |__i0, __i1| by_len(__i0, __i1));
    if __is2.is_some() {
        let mut at_len = __is2.unwrap();
        println(&console, &(format!("6. a two-letter word at {}", at_len)));
    }
    let mut unique = crate::core_set::to_list_platform(&deduped);
    println(&console, &(format!("6. distinct {} of {}", to_str__core_list::<i32>(&unique, &mut |__i0| format!("{}", __i0)), count_unique(&unique))));
    let mut __loop1_pass = iter__core_set(&vowels);
    while let Union2::U1(mut v) = next(&mut __loop1_pass) {
        console.print(&v);
    }
    println(&console, &("".to_string()));
    let mut __loop2_pass = iter__core_map(&ages);
    while let Union2::U1(mut name) = next__MapKeyYield(&mut __loop2_pass) {
        let mut age = crate::core_map::get_platform::<String, i32>(&ages, &name, &mut |__i0| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__i0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__i0, __i1| (&__i0[..] == &__i1[..]));
        if age.is_some() {
            println(&console, &(format!("7. {} is {}", name, *age.unwrap())));
        }
    }
}

pub fn cmp(a: &Point, b: &Point) -> i32 {
    let mut c__c1 = (Ord::cmp(&(a.x), &(b.x)) as i32);
    if c__c1 != 0 {
        return c__c1;
    }
    let mut c__c2 = (Ord::cmp(&(a.y), &(b.y)) as i32);
    if c__c2 != 0 {
        return c__c2;
    }
    return 0;
}

pub fn hash(value: &Point) -> i64 {
    let mut h = 17i64;
    h = mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.x), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.y), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__Point_Point(a: &Point, b: &Point) -> bool {
    if !((a.x) == (b.x)) {
        return false;
    }
    if !((a.y) == (b.y)) {
        return false;
    }
    return true;
}

pub fn eq__Note_Note(a: &Note, b: &Note) -> bool {
    if !(&a.text[..] == &b.text[..]) {
        return false;
    }
    return true;
}
