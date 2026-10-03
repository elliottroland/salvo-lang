#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
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
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;

use crate::collections::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

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
    return (Ord::cmp(&((a.chars().count() as i32)), &((b.chars().count() as i32))) as i32);
}

pub fn count_unique(xs: &Vec<i32>) -> i32 {
    return (xs.len() as i32);
}

pub fn main() {
    let console = crate::core_console::Console::shared(StdOutConsole::new());
    let mut primes = vec![2, 3, 5, 7];
    let mut vowels = SalvoSet::from_elements::<HostHash, HostEq, _>(vec!["a".to_string(), "e".to_string(), "i".to_string(), "o".to_string(), "u".to_string()]);
    let mut ages = SalvoMap::from_entries::<HostHash, HostEq, _>(vec![("ada".to_string(), 36), ("grace".to_string(), 45)]);
    println(&console, &(format!("1. list {}", format!("[{}]", primes.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    println(&console, &(format!("1. set {} of {}", vowels.to_string(), (vowels.len() as i32))));
    println(&console, &(format!("1. map {}", ages.to_string())));
    let mut note: Note = Note { text: "still a struct literal".to_string() };
    println(&console, &(format!("1. struct {}", note.text.clone())));
    let mut seen: SalvoSet<String> = SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]);
    seen.insert("first".to_string());
    println(&console, &(format!("1. empty then filled {}", seen.to_string())));
    let mut tally: SalvoMap<String, i32> = SalvoMap::from_entries::<HostHash, HostEq, _>(vec![("pear".to_string(), 1), ("apple".to_string(), 2)]);
    tally.insert("fig".to_string(), 3);
    tally.insert("pear".to_string(), 99);
    println(&console, &(format!("2. insertion order kept {}", tally.to_string())));
    let mut ranked: SalvoSortedSet<String> = SalvoSortedSet::from_elements::<HostOrd, _>(vec!["pear".to_string(), "apple".to_string(), "fig".to_string()]);
    println(&console, &(format!("2. key order {}", format!("{{{}}}", ranked.to_vec().iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut smallest = ranked.min().cloned();
    if smallest.is_some() {
        println(&console, &(format!("2. min is cheap here {}", smallest.as_ref().unwrap().clone())));
    }
    let mut corners: SalvoSet<Point> = SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]);
    corners.insert(Point { x: 0, y: 0 });
    let mut again = corners.insert(Point { x: 0, y: 0 });
    println(&console, &(format!("3. struct key: size {}, second add {}", (corners.len() as i32), again)));
    let mut labels: SalvoMap<Point, String> = SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]);
    labels.insert(Point { x: 1, y: 1 }, "diagonal".to_string());
    let mut found = labels.get(&Point { x: 1, y: 1 });
    if found.is_some() {
        println(&console, &(format!("3. looked up by value {}", found.unwrap().clone())));
    }
    let mut a = Point { x: 1, y: 2 };
    let mut b = Point { x: 1, y: 2 };
    let mut c = Point { x: 1, y: 9 };
    let mut same = eq__9(&a, &b);
    let mut before = cmp__5(&a, &c) < 0;
    println(&console, &(format!("4. equal {}, ordered {}", same, before)));
    let mut n1 = Note { text: "same".to_string() };
    let mut n2 = Note { text: "same".to_string() };
    let mut notes_equal = eq__10(&n1, &n2);
    println(&console, &(format!("4. plain struct equality {}", notes_equal)));
    let mut squares = (0..(4)).map(|i| i * i).collect::<Vec<_>>();
    println(&console, &(format!("5. generated {}", format!("[{}]", squares.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut deduped = SalvoSet::from_elements::<HostHash, HostEq, _>(primes.iter().cloned());
    println(&console, &(format!("5. to_set {}", deduped.to_string())));
    let mut words = vec!["alpha".to_string(), "be".to_string()];
    let mut lengths = SalvoMap::from_entries::<HostHash, HostEq, _>(words.iter().map(|w| (w.clone(), (w.chars().count() as i32))));
    println(&console, &(format!("5. to_map with a rule {}", lengths.to_string())));
    let mut filled = vec!["ada".to_string(), "grace".to_string()];
    println(&console, &(format!("6. first is {}, no optional", first(&filled))));
    let mut growing: Vec<i32> = vec![];
    growing.push(7);
    println(&console, &(format!("6. after add, first is {}", *first(&growing))));
    let mut ordered = sort::<i32>(&(vec![40, 10, 30, 20]), &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    println(&console, &(format!("6. sorted {}", format!("[{}]", ordered.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut __is1 = binary_search::<i32>(&ordered, &(30), &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    if __is1.is_some() {
        let mut at = __is1.unwrap();
        println(&console, &(format!("6. found 30 at {}", at)));
    }
    let mut live: Vec<i32> = mut_sort::<i32>(&(vec![10, 30]), &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    add_sorted::<i32>(&mut live, 20, &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    add_sorted::<i32>(&mut live, 5, &mut |__i0, __i1| (Ord::cmp(&(__i0), &(__i1)) as i32));
    println(&console, &(format!("6. still sorted {}", format!("[{}]", live.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut bylen = sort::<String>(&(vec!["alpha".to_string(), "be".to_string(), "z".to_string()]), &mut |__i0, __i1| by_len(__i0, __i1));
    println(&console, &(format!("6. by length {}", format!("[{}]", bylen.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut __is2 = binary_search::<String>(&bylen, &("hi".to_string()), &mut |__i0, __i1| by_len(__i0, __i1));
    if __is2.is_some() {
        let mut at_len = __is2.unwrap();
        println(&console, &(format!("6. a two-letter word at {}", at_len)));
    }
    let mut unique = deduped.iter().cloned().collect::<Vec<_>>();
    println(&console, &(format!("6. distinct {} of {}", format!("[{}]", unique.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")), count_unique(&unique))));
    let mut __loop1_pass = iter__6(&vowels);
    while let Union2::U1(mut v) = next__17(&mut __loop1_pass) {
        console.print(&v);
    }
    println(&console, &("".to_string()));
    let mut __loop2_pass = iter__5(&ages);
    while let Union2::U1(mut name) = next__11(&mut __loop2_pass) {
        let mut age = ages.get(&name);
        if age.is_some() {
            println(&console, &(format!("7. {} is {}", name, *age.unwrap())));
        }
    }
}

pub fn cmp__5(a: &Point, b: &Point) -> i32 {
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

pub fn hash__9(value: &Point) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.x), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.y), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__9(a: &Point, b: &Point) -> bool {
    if !((a.x) == (b.x)) {
        return false;
    }
    if !((a.y) == (b.y)) {
        return false;
    }
    return true;
}

pub fn eq__10(a: &Note, b: &Note) -> bool {
    if !(&a.text[..] == &b.text[..]) {
        return false;
    }
    return true;
}
