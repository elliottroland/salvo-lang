#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
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
#[path = "platform/std/core/set.rs"]
pub mod platform_std_core_set;
#[path = "platform/std/core/sorted.rs"]
pub mod platform_std_core_sorted;
#[path = "platform/std/core/string.rs"]
pub mod platform_std_core_string;

use crate::std_core_iterator::Finished;
use crate::std_core_map::Map;
use crate::std_core_map::MapKeyNext;
use crate::std_core_map::Map_platform;
use crate::std_core_map::Mut_Map_platform;
use crate::std_core_set::Mut_Set_platform;
use crate::std_core_sorted::Mut_SortedSet_platform;
use crate::std_core_set::Set;
use crate::std_core_set::SetNext;
use crate::std_core_set::Set_platform;
use crate::std_core_sorted::SortedSet;
use crate::std_core_list::add_sorted;
use crate::std_core_list::binary_search;
use crate::std_core_list::first;
use crate::std_core_map::get_platform;
use crate::std_core_list::list_by;
use crate::std_core_sorted::min_platform;
use crate::std_core_compare::mix_hash;
use crate::std_core_list::mut_sort;
use crate::std_core_set::next;
use crate::std_core_map::next__MapKeyNext;
use crate::std_core_console::println;
use crate::std_core_map::put_platform;
use crate::std_core_list::sort;
use crate::std_core_set::to_list_platform;
use crate::std_core_map::to_map;
use crate::std_core_set::to_set_platform;
use crate::std_core_sorted::to_str__SortedSet;


#[derive(Clone, Debug, PartialEq)]
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
    return (Ord::cmp(&(crate::std_core_string::size_platform(a)), &(crate::std_core_string::size_platform(b))) as i32);
}

pub fn count_unique(xs: &Vec<i32>) -> i32 {
    return crate::std_core_list::size_platform::<i32>(xs);
}

pub fn main() {
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut primes: Vec<i32> = vec![2i32, 3i32, 5i32, 7i32];
    let mut vowels: crate::std_core_set::Set<String> = crate::std_core_set::Set_platform::<String>(vec![String::from("a"), String::from("e"), String::from("i"), String::from("o"), String::from("u")], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    let mut ages: crate::std_core_map::Map<String, i32> = crate::std_core_map::Map_platform::<String, i32>(vec![(String::from("ada"), 36i32), (String::from("grace"), 45i32)], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_console::println(&__handle_2, &format!("1. list {}", crate::std_core_list::to_str::<i32>(&primes, &mut |__a0: &i32| format!("{}", *__a0))));
    crate::std_core_console::println(&__handle_2, &format!("1. set {} of {}", crate::std_core_set::to_str::<String>(&vowels, &mut |__a0: &String| format!("{}", __a0)), crate::std_core_set::size_platform::<String>(&vowels)));
    crate::std_core_console::println(&__handle_2, &format!("1. map {}", crate::std_core_map::to_str::<String, i32>(&ages, &mut |__a0: &String| format!("{}", __a0), &mut |__a0: &i32| format!("{}", *__a0), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))));
    let mut note: crate::Note = crate::Note { text: String::from("still a struct literal") };
    crate::std_core_console::println(&__handle_2, &format!("1. struct {}", note.text));
    let mut seen: crate::std_core_set::Set<String> = crate::std_core_set::Mut_Set_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_set::add_platform::<String>(&mut seen, String::from("first"), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_console::println(&__handle_2, &format!("1. empty then filled {}", crate::std_core_set::to_str::<String>(&seen, &mut |__a0: &String| format!("{}", __a0))));
    let mut tally: crate::std_core_map::Map<String, i32> = crate::std_core_map::Mut_Map_platform::<String, i32>(vec![(String::from("pear"), 1i32), (String::from("apple"), 2i32)], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_map::put_platform::<String, i32>(&mut tally, String::from("fig"), 3i32, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_map::put_platform::<String, i32>(&mut tally, String::from("pear"), 99i32, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_console::println(&__handle_2, &format!("2. insertion order kept {}", crate::std_core_map::to_str::<String, i32>(&tally, &mut |__a0: &String| format!("{}", __a0), &mut |__a0: &i32| format!("{}", *__a0), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))));
    let mut ranked: crate::std_core_sorted::SortedSet<String> = crate::std_core_sorted::Mut_SortedSet_platform::<String>(vec![String::from("pear"), String::from("apple"), String::from("fig")], &mut |__a0: &String, __a1: &String| (Ord::cmp(&__a0[..], &__a1[..]) as i32));
    crate::std_core_console::println(&__handle_2, &format!("2. key order {}", crate::std_core_sorted::to_str__SortedSet::<String>(&ranked, &mut |__a0: &String| format!("{}", __a0))));
    let mut smallest: Option<String> = crate::std_core_sorted::min_platform::<String>(&ranked);
    if smallest.is_some() {
        let mut smallest_3 = smallest.as_ref().unwrap();
        crate::std_core_console::println(&__handle_2, &format!("2. min is cheap here {}", smallest_3));
    };
    let mut corners: crate::std_core_set::Set<crate::Point> = crate::std_core_set::Mut_Set_platform::<crate::Point>(vec![], &mut |__a0: &crate::Point| crate::hash(__a0), &mut |__a0: &crate::Point, __a1: &crate::Point| crate::eq__Point_Point(__a0, __a1));
    crate::std_core_set::add_platform::<crate::Point>(&mut corners, crate::Point { x: 0i32, y: 0i32 }, &mut |__a0: &crate::Point| crate::hash(__a0), &mut |__a0: &crate::Point, __a1: &crate::Point| crate::eq__Point_Point(__a0, __a1));
    let mut again: bool = crate::std_core_set::add_platform::<crate::Point>(&mut corners, crate::Point { x: 0i32, y: 0i32 }, &mut |__a0: &crate::Point| crate::hash(__a0), &mut |__a0: &crate::Point, __a1: &crate::Point| crate::eq__Point_Point(__a0, __a1));
    crate::std_core_console::println(&__handle_2, &format!("3. struct key: size {}, second add {}", crate::std_core_set::size_platform::<crate::Point>(&corners), again));
    let mut labels: crate::std_core_map::Map<crate::Point, String> = crate::std_core_map::Mut_Map_platform::<crate::Point, String>(vec![], &mut |__a0: &crate::Point| crate::hash(__a0), &mut |__a0: &crate::Point, __a1: &crate::Point| crate::eq__Point_Point(__a0, __a1));
    crate::std_core_map::put_platform::<crate::Point, String>(&mut labels, crate::Point { x: 1i32, y: 1i32 }, String::from("diagonal"), &mut |__a0: &crate::Point| crate::hash(__a0), &mut |__a0: &crate::Point, __a1: &crate::Point| crate::eq__Point_Point(__a0, __a1));
    let mut __tmp1 = crate::Point { x: 1i32, y: 1i32 };
    let mut found: Option<&String> = crate::std_core_map::get_platform::<crate::Point, String>(&labels, &__tmp1, &mut |__a0: &crate::Point| crate::hash(__a0), &mut |__a0: &crate::Point, __a1: &crate::Point| crate::eq__Point_Point(__a0, __a1));
    if found.is_some() {
        let mut found_4 = found.unwrap();
        crate::std_core_console::println(&__handle_2, &format!("3. looked up by value {}", found_4));
    };
    let mut a: crate::Point = crate::Point { x: 1i32, y: 2i32 };
    let mut b: crate::Point = crate::Point { x: 1i32, y: 2i32 };
    let mut c: crate::Point = crate::Point { x: 1i32, y: 9i32 };
    let mut same: bool = crate::eq__Point_Point(&a, &b);
    let mut before: bool = (crate::cmp(&a, &c) < 0i32);
    crate::std_core_console::println(&__handle_2, &format!("4. equal {}, ordered {}", same, before));
    let mut n1: crate::Note = crate::Note { text: String::from("same") };
    let mut n2: crate::Note = crate::Note { text: String::from("same") };
    let mut notes_equal: bool = crate::eq__Note_Note(&n1, &n2);
    crate::std_core_console::println(&__handle_2, &format!("4. plain struct equality {}", notes_equal));
    let mut squares: Vec<i32> = crate::std_core_list::list_by::<i32>(4i32, &mut |mut i| -> i32 {
        i32::wrapping_mul(i, i)
    });
    crate::std_core_console::println(&__handle_2, &format!("5. generated {}", crate::std_core_list::to_str::<i32>(&squares, &mut |__a0: &i32| format!("{}", *__a0))));
    let mut deduped: crate::std_core_set::Set<i32> = crate::std_core_set::to_set_platform::<i32>(&primes, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
    crate::std_core_console::println(&__handle_2, &format!("5. to_set {}", crate::std_core_set::to_str::<i32>(&deduped, &mut |__a0: &i32| format!("{}", *__a0))));
    let mut words: Vec<String> = vec![String::from("alpha"), String::from("be")];
    let mut lengths: crate::std_core_map::Map<String, i32> = crate::std_core_map::to_map::<String, String, i32>(&words, &mut |mut w| -> (String, i32) {
        {
            let mut __part_5: i32 = crate::std_core_string::size_platform(w);
            (w.clone(), __part_5)
        }
    }, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    crate::std_core_console::println(&__handle_2, &format!("5. to_map with a rule {}", crate::std_core_map::to_str::<String, i32>(&lengths, &mut |__a0: &String| format!("{}", __a0), &mut |__a0: &i32| format!("{}", *__a0), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))));
    let mut filled: Vec<String> = vec![String::from("ada"), String::from("grace")];
    crate::std_core_console::println(&__handle_2, &format!("6. first is {}, no optional", crate::std_core_list::first::<String>(&filled)));
    let mut growing: Vec<i32> = vec![];
    crate::std_core_list::add_platform::<i32>(&mut growing, 7i32);
    crate::std_core_console::println(&__handle_2, &format!("6. after add, first is {}", (*crate::std_core_list::first::<i32>(&growing))));
    let mut ordered: Vec<i32> = crate::std_core_list::sort::<i32>(&vec![40i32, 10i32, 30i32, 20i32], &mut |__a0: &i32, __a1: &i32| (Ord::cmp(&(*__a0), &(*__a1)) as i32));
    crate::std_core_console::println(&__handle_2, &format!("6. sorted {}", crate::std_core_list::to_str::<i32>(&ordered, &mut |__a0: &i32| format!("{}", *__a0))));
    let mut __subject_6: Option<i32> = crate::std_core_list::binary_search::<i32>(&ordered, &30i32, &mut |__a0: &i32, __a1: &i32| (Ord::cmp(&(*__a0), &(*__a1)) as i32));
    if __subject_6.is_some() {
        let mut at = __subject_6.unwrap();
        crate::std_core_console::println(&__handle_2, &format!("6. found 30 at {}", at));
    };
    let mut live: Vec<i32> = crate::std_core_list::mut_sort::<i32>(&vec![10i32, 30i32], &mut |__a0: &i32, __a1: &i32| (Ord::cmp(&(*__a0), &(*__a1)) as i32));
    crate::std_core_list::add_sorted::<i32>(&mut live, 20i32, &mut |__a0: &i32, __a1: &i32| (Ord::cmp(&(*__a0), &(*__a1)) as i32));
    crate::std_core_list::add_sorted::<i32>(&mut live, 5i32, &mut |__a0: &i32, __a1: &i32| (Ord::cmp(&(*__a0), &(*__a1)) as i32));
    crate::std_core_console::println(&__handle_2, &format!("6. still sorted {}", crate::std_core_list::to_str::<i32>(&live, &mut |__a0: &i32| format!("{}", *__a0))));
    let mut bylen: Vec<String> = crate::std_core_list::sort::<String>(&vec![String::from("alpha"), String::from("be"), String::from("z")], &mut |__a0: &String, __a1: &String| crate::by_len(__a0, __a1));
    crate::std_core_console::println(&__handle_2, &format!("6. by length {}", crate::std_core_list::to_str::<String>(&bylen, &mut |__a0: &String| format!("{}", __a0))));
    let mut __subject_7: Option<i32> = crate::std_core_list::binary_search::<String>(&bylen, &String::from("hi"), &mut |__a0: &String, __a1: &String| crate::by_len(__a0, __a1));
    if __subject_7.is_some() {
        let mut at_len = __subject_7.unwrap();
        crate::std_core_console::println(&__handle_2, &format!("6. a two-letter word at {}", at_len));
    };
    let mut unique: Vec<i32> = crate::std_core_set::to_list_platform::<i32>(&deduped);
    crate::std_core_console::println(&__handle_2, &format!("6. distinct {} of {}", crate::std_core_list::to_str::<i32>(&unique, &mut |__a0: &i32| format!("{}", *__a0)), crate::count_unique(&unique)));
    let mut __pass_8: crate::std_core_set::SetNext<String> = crate::std_core_set::iter::<String>(&vowels);
    loop {
        let mut __step_9: crate::unions::Union2<String, crate::std_core_iterator::Finished> = crate::std_core_set::next(&mut __pass_8);
        if matches!(__step_9, crate::unions::Union2::U1(_)) {
            let mut __emitted_10 = match __step_9 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut v: String = __emitted_10;
            __handle_2.print(&v);
        } else {
            break;
        };
    }
    crate::std_core_console::println(&__handle_2, &String::from(""));
    let mut __pass_11: crate::std_core_map::MapKeyNext<String> = crate::std_core_map::iter::<String, i32>(&ages);
    loop {
        let mut __step_12: crate::unions::Union2<String, crate::std_core_iterator::Finished> = crate::std_core_map::next__MapKeyNext(&mut __pass_11);
        if matches!(__step_12, crate::unions::Union2::U1(_)) {
            let mut __emitted_13 = match __step_12 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut name: String = __emitted_13;
            let mut age: Option<i32> = crate::std_core_map::get_platform::<String, i32>(&ages, &name, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])).copied();
            if age.is_some() {
                let mut age_14 = age.unwrap();
                crate::std_core_console::println(&__handle_2, &format!("7. {} is {}", name, age_14));
            };
        } else {
            break;
        };
    }
}

pub fn cmp(a: &crate::Point, b: &crate::Point) -> i32 {
    let mut c__c1: i32 = (Ord::cmp(&(a.x), &(b.x)) as i32);
    if (c__c1 != 0i32) {
        return c__c1;
    };
    let mut c__c2: i32 = (Ord::cmp(&(a.y), &(b.y)) as i32);
    if (c__c2 != 0i32) {
        return c__c2;
    };
    return 0i32;
}

pub fn hash(value: &crate::Point) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.x), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.y), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__Point_Point(a: &crate::Point, b: &crate::Point) -> bool {
    if !(((a.x) == (b.x))) {
        return false;
    };
    if !(((a.y) == (b.y))) {
        return false;
    };
    return true;
}

pub fn eq__Note_Note(a: &crate::Note, b: &crate::Note) -> bool {
    if !((&a.text[..] == &b.text[..])) {
        return false;
    };
    return true;
}
