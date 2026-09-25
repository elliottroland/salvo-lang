#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/fs.rs"]
pub mod core_fs;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/nonempty.rs"]
pub mod core_nonempty;
#[path = "core/range.rs"]
pub mod core_range;
#[path = "core/result.rs"]
pub mod core_result;
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
use crate::core_bytes::*;
use crate::core_console::*;
use crate::core_fs::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_range::*;
use crate::core_seq::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Fighter {
    pub name: String,
    pub hp: i32,
    pub energy: i32,
}

pub fn named<'a>(roster: &'a Vec<Fighter>, name: &String) -> Option<&'a Fighter> {
    for f in roster {
        if f.name.clone() == name.clone() {
            return Some(f);
        }
    }
    return None;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Window<'s> {
    pub roster: &'s Vec<Fighter>,
    pub at: i32,
}

pub fn window(roster: &Vec<Fighter>) -> Window<'_> {
    return Window { roster: roster, at: 0 };
}

pub fn peek<'s>(w: &Window<'s>) -> Option<&'s Fighter> {
    return w.roster.get((w.at) as i64 as usize);
}

pub fn heal(f: &mut Fighter) {
    f.hp = f.hp + 10;
    return;
}

pub fn wounded(squad: &mut Vec<Fighter>) -> Option<&Fighter> {
    for f in &*squad {
        if f.hp < 10 {
            return Some(f);
        }
    }
    return None;
}

pub fn wounded__loc(squad: &Vec<Fighter>) -> Option<usize> {
    for __li0 in 0..squad.len() {
        if squad[__li0].hp < 10 {
            return Some(__li0);
        }
    }
    return None;
}

pub fn rally_at<L: Clone>(squad: &mut Vec<Fighter>, l: &L, at: &mut dyn FnMut(&Vec<Fighter>, &L) -> Option<usize>) {
    heal({ let __l1 = at(&*squad, l).expect("salvo: value is absent at main:100:10"); &mut squad[__l1] });
    return;
}

pub fn duel(a: &mut Fighter, d: &mut Fighter) {
    a.hp = a.hp - 1;
    d.hp = d.hp - 2;
    return;
}

pub fn strike(__anchor: &mut Vec<Fighter>, __c0: usize, __c1: usize) {
    __anchor[__c0].energy = __anchor[__c0].energy - 1;
    __anchor[__c1].hp = __anchor[__c1].hp - 2;
    return;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Squad {
    pub banner: String,
    pub members: Vec<Fighter>,
}

pub fn rotate(squad: &mut Squad, __c1: usize, __c2: usize) {
    squad.members[__c1].energy = squad.members[__c1].energy - 1;
    squad.members[__c2].energy = squad.members[__c2].energy + 1;
    return;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camp {
    pub supplies: i32,
    pub banners: Vec<String>,
}

pub fn spend(camp: &mut Camp, n: i32) {
    camp.supplies = camp.supplies - n;
    return;
}

pub fn hoist(camp: &mut Camp, banner: String) {
    camp.banners.push(banner);
    return;
}

pub fn main() {
    let mut console = StdOutConsole::new();
    let mut roster: Vec<Fighter> = vec![Fighter { name: "Ada".to_string(), hp: 30, energy: 4 }, Fighter { name: "Bo".to_string(), hp: 8, energy: 9 }];
    let mut ada = named(&roster, &("Ada".to_string())).unwrap();
    println(&mut console, &(format!("1. found {}, hp {}", ada.name.clone(), ada.hp)));
    let mut names: Vec<String> = vec![];
    names.push(ada.name.clone());
    println(&mut console, &(format!("1. copied out {}", format!("[{}]", names.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    let mut w = window(&roster);
    w.at = 1;
    println(&mut console, &(format!("1. window at {}: {}", w.at, peek(&w).expect("salvo: value is absent at main:206:38").name.clone())));
    let mut pass = iter__3(&roster);
    let mut standing = filter::<ListYield<Fighter>, &Fighter>(&mut pass, &mut (|f: &&Fighter| {
    let f = *f; 
    f.hp > 10
}), &mut |__i0| next__5(__i0));
    println(&mut console, &(format!("1. {} of {} still standing", (standing.len() as i32), (roster.len() as i32))));
    let mut bench: Vec<Fighter> = vec![Fighter { name: "Cy".to_string(), hp: 12, energy: 2 }];
    bench.push(Fighter { name: "Dee".to_string(), hp: 6, energy: 7 });
    println(&mut console, &(format!("2. bench {}, front {} (a reading, not a handle)", (bench.len() as i32), bench.get((0) as i64 as usize).expect("salvo: value is absent at main:221:47").name.clone())));
    let mut squad: Vec<Fighter> = vec![Fighter { name: "Ada".to_string(), hp: 30, energy: 4 }, Fighter { name: "Bo".to_string(), hp: 8, energy: 9 }];
    let __h2 = (0) as usize;
    squad.get(__h2).expect("salvo: value is absent at main:232:16");
    squad[__h2].hp = squad[__h2].hp + 1;
    let mut n = (squad.len() as i32);
    squad[__h2].hp = squad[__h2].hp + n;
    println(&mut console, &(format!("2. {} at {} after a read in the middle", squad.get((0) as i64 as usize).expect("salvo: value is absent at main:236:19").name.clone(), squad.get((0) as i64 as usize).expect("salvo: value is absent at main:236:45").hp)));
    heal({ let __l3 = wounded__loc(&squad).expect("salvo: value is absent at main:239:10"); &mut squad[__l3] });
    rally_at::<i32>(&mut squad, &(1), &mut |__i0, __i1| at__loc(__i0, (__i1).clone()));
    println(&mut console, &(format!("2. after the searches: {} {}", squad.get((0) as i64 as usize).expect("salvo: value is absent at main:244:39").hp, squad.get((1) as i64 as usize).expect("salvo: value is absent at main:244:60").hp)));
    let mut i = 0;
    let mut j = 1;
    if NotEq_qualifies(j, i) {
        let __h4 = (i) as usize;
        squad.get(__h4).expect("salvo: value is absent at main:260:17");
        let __h5 = (j) as usize;
        squad.get(__h5).expect("salvo: value is absent at main:261:17");
        squad[__h4].hp = squad[__h4].hp + 1;
        squad[__h5].hp = squad[__h5].hp + 1;
        let (__pm6, __pm7) = salvo_pair_mut(&mut squad[..], __h4, __h5).expect("salvo: value is absent at main:264:9");
        duel(__pm6, __pm7);
    }
    if Idx_qualifies(i, &squad) {
        if Idx_qualifies(j, &squad) {
            update(&mut squad, &i, &mut (|f: &mut Fighter| {
    f.energy = f.energy + 1;
}));
            if NotEq_qualifies(j, i) {
                update2(&mut squad, &i, &j, &mut (|a: &mut Fighter, b: &mut Fighter| {
    a.energy = a.energy + 100;
    b.energy = b.energy + 200;
}));
            }
            println(&mut console, &(format!("3. {} {} (total reads: `Idx` survived)", get(&squad, &i).energy, get(&squad, &j).energy)));
        }
    }
    if Idx_qualifies(i, &squad) {
        if Idx_qualifies(j, &squad) {
            strike(&mut squad, (i) as usize, (j) as usize);
            strike(&mut squad, (i) as usize, (i) as usize);
        }
    }
    println(&mut console, &(format!("4. {} hp / {} energy after striking itself", squad.get((0) as i64 as usize).expect("salvo: value is absent at main:294:19").hp, squad.get((0) as i64 as usize).expect("salvo: value is absent at main:294:45").energy)));
    let mut team = Squad { banner: "Red".to_string(), members: vec![Fighter { name: "Cy".to_string(), hp: 12, energy: 2 }, Fighter { name: "Dee".to_string(), hp: 6, energy: 7 }] };
    rotate(&mut team, (i) as usize, (j) as usize);
    println(&mut console, &(format!("4. {}: {} {}", team.banner.clone(), team.members.get((0) as i64 as usize).expect("salvo: value is absent at main:302:35").energy, team.members.get((1) as i64 as usize).expect("salvo: value is absent at main:302:67").energy)));
    let mut camp = Camp { supplies: 10, banners: vec!["red".to_string()] };
    spend(&mut camp, 3);
    hoist(&mut camp, "blue".to_string());
    println(&mut console, &(format!("5. supplies {}, banners {}", camp.supplies, format!("[{}]", camp.banners.iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
}
