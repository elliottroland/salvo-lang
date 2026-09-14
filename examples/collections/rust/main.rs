#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
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

use crate::collections::*;
use crate::core_array::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_nonempty::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
}

pub fn count_unique(xs: &Vec<i32>) -> i32 {
    return (xs.len() as i32);
}

pub fn main() {
    let mut console = StdOutConsole::new();
    let mut primes = vec![2, 3, 5, 7];
    let mut vowels = SalvoSet::from_elements(vec!["a".to_string(), "e".to_string(), "i".to_string(), "o".to_string(), "u".to_string()]);
    let mut ages = SalvoMap::from_entries(vec![("ada".to_string(), 36), ("grace".to_string(), 45)]);
    println(&mut console, &(format!("1. list {}", format!("[{}]", primes.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    println(&mut console, &(format!("1. set {} of {}", vowels.to_string(), (vowels.len() as i32))));
    println(&mut console, &(format!("1. map {}", ages.to_string())));
    let mut note: Note = Note { text: "still a struct literal".to_string() };
    println(&mut console, &(format!("1. struct {}", note.text.clone())));
    let mut seen: SalvoSet<String> = SalvoSet::from_elements(vec![]);
    seen.insert("first".to_string());
    println(&mut console, &(format!("1. empty then filled {}", seen.to_string())));
    let mut tally: SalvoMap<String, i32> = SalvoMap::from_entries(vec![("pear".to_string(), 1), ("apple".to_string(), 2)]);
    tally.insert("fig".to_string(), 3);
    tally.insert("pear".to_string(), 99);
    println(&mut console, &(format!("2. insertion order kept {}", tally.to_string())));
    let mut ranked: std::collections::BTreeSet<String> = vec!["pear".to_string(), "apple".to_string(), "fig".to_string()].into_iter().collect::<std::collections::BTreeSet<_>>();
    println(&mut console, &(format!("2. key order {}", format!("{{{}}}", ranked.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut smallest = ranked.iter().next().cloned();
    if smallest.is_some() {
        println(&mut console, &(format!("2. min is cheap here {}", smallest.as_ref().unwrap().clone())));
    }
    let mut corners: SalvoSet<Point> = SalvoSet::from_elements(vec![]);
    corners.insert(Point { x: 0, y: 0 });
    let mut again = corners.insert(Point { x: 0, y: 0 });
    println(&mut console, &(format!("3. struct key: size {}, second add {}", (corners.len() as i32), again)));
    let mut labels: SalvoMap<Point, String> = SalvoMap::from_entries(vec![]);
    labels.insert(Point { x: 1, y: 1 }, "diagonal".to_string());
    let mut found = labels.get(&Point { x: 1, y: 1 });
    if found.is_some() {
        println(&mut console, &(format!("3. looked up by value {}", found.unwrap().clone())));
    }
    let mut a = Point { x: 1, y: 2 };
    let mut b = Point { x: 1, y: 2 };
    let mut c = Point { x: 1, y: 9 };
    let mut same = a == b;
    let mut before = a < c;
    println(&mut console, &(format!("4. equal {}, ordered {}", same, before)));
    let mut n1 = Note { text: "same".to_string() };
    let mut n2 = Note { text: "same".to_string() };
    let mut notes_equal = n1 == n2;
    println(&mut console, &(format!("4. plain struct equality {}", notes_equal)));
    let mut squares = (0..(4)).map(|i| i * i).collect::<Vec<_>>();
    println(&mut console, &(format!("5. generated {}", format!("[{}]", squares.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut deduped = SalvoSet::from_elements(primes.iter().cloned());
    println(&mut console, &(format!("5. to_set {}", deduped.to_string())));
    let mut words = vec!["alpha".to_string(), "be".to_string()];
    let mut lengths = SalvoMap::from_entries(words.iter().map(|w| (w.clone(), (w.chars().count() as i32))));
    println(&mut console, &(format!("5. to_map with a rule {}", lengths.to_string())));
    let mut filled = non_empty_list(&("ada".to_string()), vec!["grace".to_string()]);
    println(&mut console, &(format!("6. first is {}, no optional", first(&filled))));
    let mut growing: Vec<i32> = vec![];
    growing.push(7);
    println(&mut console, &(format!("6. after add, first is {}", first(&growing))));
    let mut ordered = { let mut __v = vec![40, 10, 30, 20].clone(); __v.sort(); __v };
    println(&mut console, &(format!("6. sorted {}", format!("[{}]", ordered.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    if ({ let __e = 30; let __at = ordered.partition_point(|__x| __x < &__e); if __at < ordered.len() && ordered[__at] == __e { Some(__at as i32) } else { None } }.is_some()) {
        let mut at = { let __e = 30; let __at = ordered.partition_point(|__x| __x < &__e); if __at < ordered.len() && ordered[__at] == __e { Some(__at as i32) } else { None } }.unwrap();
        println(&mut console, &(format!("6. found 30 at {}", at)));
    }
    let mut live: Vec<i32> = { let mut __v = vec![10, 30].clone(); __v.sort(); __v };
    { let __e = 20; let __at = live.partition_point(|__x| __x < &__e); live.insert(__at, __e); };
    { let __e = 5; let __at = live.partition_point(|__x| __x < &__e); live.insert(__at, __e); };
    println(&mut console, &(format!("6. still sorted {}", format!("[{}]", live.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut unique = deduped.iter().cloned().collect::<Vec<_>>();
    println(&mut console, &(format!("6. distinct {} of {}", format!("[{}]", unique.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")), count_unique(&unique))));
    let mut __loop1_pass = iter__4(&vowels);
    while let Union2::U1(mut v) = next__4(&mut __loop1_pass) {
        console.print(&v);
    }
    println(&mut console, &("".to_string()));
    let mut __loop2_pass = iter__3(&ages);
    while let Union2::U1(mut name) = next__3(&mut __loop2_pass) {
        let mut age = ages.get(&name);
        if age.is_some() {
            println(&mut console, &(format!("7. {} is {}", name, *age.unwrap())));
        }
    }
}
