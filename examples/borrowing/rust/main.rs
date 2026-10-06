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

use crate::seq::*;
use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;
use crate::core_index::Idx__Int_qualifies;
use crate::core_index::NotEq__Int_qualifies;
use crate::core_list::ListYield;
use crate::core_list::at;
use crate::core_list::at__loc;
use crate::core_list::get;
use crate::core_list::iter;
use crate::core_list::next__ListYield;
use crate::core_list::to_str;
use crate::core_list::update2;
use crate::core_list::update;
use crate::core_seq::filter;

#[derive(Clone, Debug, PartialEq)]
pub struct Fighter {
    pub name: String,
    pub hp: i32,
    pub energy: i32,
}

impl crate::wire::__Wire for Fighter {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
        crate::wire::__Wire::__enc(&self.hp, out);
        crate::wire::__Wire::__enc(&self.energy, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
            hp: crate::wire::__Wire::__dec(r)?,
            energy: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn named<'a>(roster: &'a Vec<Fighter>, name: &String) -> Option<&'a Fighter> {
    for f in crate::platform_core_list::each(roster) {
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
    return crate::core_list::get_platform(&w.roster, w.at);
}

pub fn heal(f: &mut Fighter) {
    f.hp = i32::wrapping_add(f.hp, 10);
    return;
}

pub fn wounded(squad: &mut Vec<Fighter>) -> Option<&Fighter> {
    for f in crate::platform_core_list::each(&*squad) {
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
    a.hp = i32::wrapping_sub(a.hp, 1);
    d.hp = i32::wrapping_sub(d.hp, 2);
    return;
}

pub fn strike(__anchor: &mut Vec<Fighter>, __c0: usize, __c1: usize) {
    __anchor[__c0].energy = i32::wrapping_sub(__anchor[__c0].energy, 1);
    __anchor[__c1].hp = i32::wrapping_sub(__anchor[__c1].hp, 2);
    return;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Squad {
    pub banner: String,
    pub members: Vec<Fighter>,
}

impl crate::wire::__Wire for Squad {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.banner, out);
        crate::wire::__Wire::__enc(&self.members, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            banner: crate::wire::__Wire::__dec(r)?,
            members: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn rotate(squad: &mut Squad, __c1: usize, __c2: usize) {
    squad.members[__c1].energy = i32::wrapping_sub(squad.members[__c1].energy, 1);
    squad.members[__c2].energy = i32::wrapping_add(squad.members[__c2].energy, 1);
    return;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camp {
    pub supplies: i32,
    pub banners: Vec<String>,
}

impl crate::wire::__Wire for Camp {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.supplies, out);
        crate::wire::__Wire::__enc(&self.banners, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            supplies: crate::wire::__Wire::__dec(r)?,
            banners: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn spend(camp: &mut Camp, n: i32) {
    camp.supplies = i32::wrapping_sub(camp.supplies, n);
    return;
}

pub fn hoist(camp: &mut Camp, banner: String) {
    crate::core_list::add_platform(&mut camp.banners, banner);
    return;
}

pub fn main() {
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let mut roster: Vec<Fighter> = vec![Fighter { name: "Ada".to_string(), hp: 30, energy: 4 }, Fighter { name: "Bo".to_string(), hp: 8, energy: 9 }];
    let mut ada = named(&roster, &("Ada".to_string())).unwrap();
    println(&console, &(format!("1. found {}, hp {}", ada.name.clone(), ada.hp)));
    let mut names: Vec<String> = vec![];
    crate::core_list::add_platform(&mut names, ada.name.clone());
    println(&console, &(format!("1. copied out {}", to_str::<String>(&names, &mut |__i0| format!("{}", __i0)))));
    let mut w = window(&roster);
    w.at = 1;
    println(&console, &(format!("1. window at {}: {}", w.at, peek(&w).expect("salvo: value is absent at main:206:38").name.clone())));
    let mut pass = iter(&roster);
    let mut standing = filter::<ListYield<'_, Fighter>, &Fighter>(&mut pass, &mut (|f: &&Fighter| {
    let f = *f; 
    f.hp > 10
}), &mut |__i0| next__ListYield(__i0));
    println(&console, &(format!("1. {} of {} still standing", crate::core_list::size_platform(&standing), crate::core_list::size_platform(&roster))));
    let mut bench: Vec<Fighter> = vec![Fighter { name: "Cy".to_string(), hp: 12, energy: 2 }];
    crate::core_list::add_platform(&mut bench, Fighter { name: "Dee".to_string(), hp: 6, energy: 7 });
    println(&console, &(format!("2. bench {}, front {} (a reading, not a handle)", crate::core_list::size_platform(&bench), crate::core_list::get_platform(&bench, 0).expect("salvo: value is absent at main:221:47").name.clone())));
    let mut squad: Vec<Fighter> = vec![Fighter { name: "Ada".to_string(), hp: 30, energy: 4 }, Fighter { name: "Bo".to_string(), hp: 8, energy: 9 }];
    let __h2 = (0) as usize;
    squad.get(__h2).expect("salvo: value is absent at main:232:16");
    squad[__h2].hp = i32::wrapping_add(squad[__h2].hp, 1);
    let mut n = crate::core_list::size_platform(&squad);
    squad[__h2].hp = i32::wrapping_add(squad[__h2].hp, n);
    println(&console, &(format!("2. {} at {} after a read in the middle", crate::core_list::get_platform(&squad, 0).expect("salvo: value is absent at main:236:19").name.clone(), crate::core_list::get_platform(&squad, 0).expect("salvo: value is absent at main:236:45").hp)));
    heal({ let __l3 = wounded__loc(&squad).expect("salvo: value is absent at main:239:10"); &mut squad[__l3] });
    rally_at::<i32>(&mut squad, &(1), &mut |__i0, __i1| at__loc(__i0, (__i1).clone()));
    println(&console, &(format!("2. after the searches: {} {}", crate::core_list::get_platform(&squad, 0).expect("salvo: value is absent at main:244:39").hp, crate::core_list::get_platform(&squad, 1).expect("salvo: value is absent at main:244:60").hp)));
    let mut i = 0;
    let mut j = 1;
    if NotEq__Int_qualifies(j, i) {
        let __h4 = (i) as usize;
        squad.get(__h4).expect("salvo: value is absent at main:260:17");
        let __h5 = (j) as usize;
        squad.get(__h5).expect("salvo: value is absent at main:261:17");
        squad[__h4].hp = i32::wrapping_add(squad[__h4].hp, 1);
        squad[__h5].hp = i32::wrapping_add(squad[__h5].hp, 1);
        let (__pm6, __pm7) = salvo_pair_mut(&mut squad[..], __h4, __h5).expect("salvo: value is absent at main:264:9");
        duel(__pm6, __pm7);
    }
    if Idx__Int_qualifies(i, &squad, &mut |__i0| crate::core_list::size_platform(&__i0)) {
        if Idx__Int_qualifies(j, &squad, &mut |__i0| crate::core_list::size_platform(&__i0)) {
            update(&mut squad, &i, &mut (|f: &mut Fighter| {
    f.energy = i32::wrapping_add(f.energy, 1);
}));
            if NotEq__Int_qualifies(j, i) {
                update2(&mut squad, &i, &j, &mut (|a: &mut Fighter, b: &mut Fighter| {
    a.energy = i32::wrapping_add(a.energy, 100);
    b.energy = i32::wrapping_add(b.energy, 200);
}));
            }
            println(&console, &(format!("3. {} {} (total reads: `Idx` survived)", get(&squad, &i).energy, get(&squad, &j).energy)));
        }
    }
    if Idx__Int_qualifies(i, &squad, &mut |__i0| crate::core_list::size_platform(&__i0)) {
        if Idx__Int_qualifies(j, &squad, &mut |__i0| crate::core_list::size_platform(&__i0)) {
            strike(&mut squad, (i) as usize, (j) as usize);
            strike(&mut squad, (i) as usize, (i) as usize);
        }
    }
    println(&console, &(format!("4. {} hp / {} energy after striking itself", crate::core_list::get_platform(&squad, 0).expect("salvo: value is absent at main:294:19").hp, crate::core_list::get_platform(&squad, 0).expect("salvo: value is absent at main:294:45").energy)));
    let mut team = Squad { banner: "Red".to_string(), members: vec![Fighter { name: "Cy".to_string(), hp: 12, energy: 2 }, Fighter { name: "Dee".to_string(), hp: 6, energy: 7 }] };
    rotate(&mut team, (i) as usize, (j) as usize);
    println(&console, &(format!("4. {}: {} {}", team.banner.clone(), crate::core_list::get_platform(&team.members, 0).expect("salvo: value is absent at main:302:35").energy, crate::core_list::get_platform(&team.members, 1).expect("salvo: value is absent at main:302:67").energy)));
    let mut camp = Camp { supplies: 10, banners: vec!["red".to_string()] };
    spend(&mut camp, 3);
    hoist(&mut camp, "blue".to_string());
    println(&console, &(format!("5. supplies {}, banners {}", camp.supplies, to_str::<String>(&camp.banners, &mut |__i0| format!("{}", __i0)))));
}
