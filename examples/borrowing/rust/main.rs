#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
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

use crate::core_index::Idx__Int_qualifies;
use crate::core_list::ListYield;
use crate::core_index::NotEq__Int_qualifies;
use crate::core_list::add_platform;
use crate::core_list::at__loc;
use crate::core_seq::filter;
use crate::core_list::get__loc;
use crate::core_list::get_platform;
use crate::core_list::get_platform__loc;
use crate::core_list::iter;
use crate::core_list::next__ListYield;
use crate::core_console::println;
use crate::core_list::size_platform;
use crate::core_list::to_str;
use crate::core_list::update;
use crate::core_list::update2;


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

pub fn named<'a>(roster: &'a Vec<crate::Fighter>, name: &String) -> Option<&'a crate::Fighter> {
    for mut f in roster.iter() {
        if (&f.name[..] == &name[..]) {
            return Some(f);
        };
    }
    return None;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Window<'s> {
    pub roster: &'s Vec<crate::Fighter>,
    pub at: i32,
}

pub fn window(roster: &Vec<crate::Fighter>) -> crate::Window<'_> {
    return crate::Window { roster: roster, at: 0i32 };
}

pub fn peek<'a>(w: &crate::Window<'a>) -> Option<&'a crate::Fighter> {
    return crate::core_list::get_platform::<crate::Fighter>(w.roster, w.at);
}

pub fn heal(f: &mut crate::Fighter) {
    f.hp = i32::wrapping_add(f.hp, 10i32);
    return;
}

pub fn wounded(squad: &mut Vec<crate::Fighter>) -> Option<&mut crate::Fighter> {
    for mut f in (&mut *squad).iter_mut() {
        if (f.hp < 10i32) {
            return Some(&mut *f);
        };
    }
    return None;
}

pub fn wounded__loc(squad: &Vec<crate::Fighter>) -> Option<usize> {
    for __li1 in 0..squad.len() {
        let f = &squad[__li1];
        if (f.hp < 10i32) {
            return Some(__li1);
        };
    }
    return None;
}

pub fn rally_at<L: Clone>(squad: &mut Vec<crate::Fighter>, l: &L, at: &mut dyn FnMut(&Vec<crate::Fighter>, &L) -> Option<usize>) {
    crate::heal({
        let mut __nn_1: Option<&mut crate::Fighter> = { match at(&*squad, l) { Some(__l1) => Some(&mut squad[__l1]), None => None } };
        if __nn_1.is_none() {
            panic!("salvo: value is absent at main:100:10");
        } else {
            let mut __some_2 = __nn_1.unwrap();
            &mut *__some_2
        }
    });
    return;
}

pub fn duel(a: &mut crate::Fighter, d: &mut crate::Fighter) {
    a.hp = i32::wrapping_sub(a.hp, 1i32);
    d.hp = i32::wrapping_sub(d.hp, 2i32);
    return;
}

pub fn strike(__anchor: &mut Vec<crate::Fighter>, __c0: usize, __c1: usize) {
    __anchor[__c0].energy = i32::wrapping_sub(__anchor[__c0].energy, 1i32);
    __anchor[__c1].hp = i32::wrapping_sub(__anchor[__c1].hp, 2i32);
    return;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Squad {
    pub banner: String,
    pub members: Vec<crate::Fighter>,
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

pub fn rotate(squad: &mut crate::Squad, __c1: usize, __c2: usize) {
    squad.members[__c1].energy = i32::wrapping_sub(squad.members[__c1].energy, 1i32);
    squad.members[__c2].energy = i32::wrapping_add(squad.members[__c2].energy, 1i32);
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

pub fn spend(camp: &mut crate::Camp, mut n: i32) {
    camp.supplies = i32::wrapping_sub(camp.supplies, n);
    return;
}

pub fn hoist(camp: &mut crate::Camp, mut banner: String) {
    crate::core_list::add_platform::<String>(&mut camp.banners, banner);
    return;
}

pub fn main() {
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut roster: Vec<crate::Fighter> = vec![crate::Fighter { name: String::from("Ada"), hp: 30i32, energy: 4i32 }, crate::Fighter { name: String::from("Bo"), hp: 8i32, energy: 9i32 }];
    let mut ada: &crate::Fighter = {
        let mut __tmp1 = String::from("Ada");
        let mut __nn_3: Option<&crate::Fighter> = crate::named(&roster, &__tmp1);
        if __nn_3.is_none() {
            panic!("salvo: value is absent at main:192:15");
        } else {
            let mut __some_4 = __nn_3.unwrap();
            __some_4
        }
    };
    crate::core_console::println(&__handle_2, &format!("1. found {}, hp {}", ada.name, ada.hp));
    let mut names: Vec<String> = vec![];
    crate::core_list::add_platform::<String>(&mut names, (ada.name).clone());
    crate::core_console::println(&__handle_2, &format!("1. copied out {}", crate::core_list::to_str::<String>(&names, &mut |__a0: &String| format!("{}", __a0))));
    let mut w: crate::Window<'_> = crate::window(&roster);
    w.at = 1i32;
    crate::core_console::println(&__handle_2, &format!("1. window at {}: {}", w.at, {
        let mut __proj_7: &crate::Fighter = {
            let mut __nn_5: Option<&crate::Fighter> = crate::peek(&w);
            if __nn_5.is_none() {
                panic!("salvo: value is absent at main:206:38");
            } else {
                let mut __some_6 = __nn_5.unwrap();
                __some_6
            }
        };
        __proj_7.name.clone()
    }));
    let mut pass: crate::core_list::ListYield<'_, crate::Fighter> = crate::core_list::iter::<crate::Fighter>(&roster);
    let mut standing = crate::core_seq::filter::<crate::core_list::ListYield<'_, crate::Fighter>, &crate::Fighter>(&mut pass, &mut |mut f| -> bool {
        let f = *f;
        return (f.hp > 10i32);
    }, &mut |__a0: &mut crate::core_list::ListYield<'_, crate::Fighter>| crate::core_list::next__ListYield(&mut *__a0));
    crate::core_console::println(&__handle_2, &format!("1. {} of {} still standing", crate::core_list::size_platform(&standing), crate::core_list::size_platform::<crate::Fighter>(&roster)));
    let mut bench: Vec<crate::Fighter> = vec![crate::Fighter { name: String::from("Cy"), hp: 12i32, energy: 2i32 }];
    crate::core_list::add_platform::<crate::Fighter>(&mut bench, crate::Fighter { name: String::from("Dee"), hp: 6i32, energy: 7i32 });
    crate::core_console::println(&__handle_2, &format!("2. bench {}, front {} (a reading, not a handle)", crate::core_list::size_platform::<crate::Fighter>(&bench), {
        let mut __proj_10: &crate::Fighter = {
            let mut __nn_8: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&bench, 0i32);
            if __nn_8.is_none() {
                panic!("salvo: value is absent at main:221:47");
            } else {
                let mut __some_9 = __nn_8.unwrap();
                __some_9
            }
        };
        __proj_10.name.clone()
    }));
    let mut squad: Vec<crate::Fighter> = vec![crate::Fighter { name: String::from("Ada"), hp: 30i32, energy: 4i32 }, crate::Fighter { name: String::from("Bo"), hp: 8i32, energy: 9i32 }];
    let __h2: usize = crate::core_list::get_platform__loc(&squad, 0i32).expect("salvo: value is absent at main:232:16");
    squad[__h2].hp = i32::wrapping_add(squad[__h2].hp, 1i32);
    let mut n: i32 = crate::core_list::size_platform::<crate::Fighter>(&squad);
    squad[__h2].hp = i32::wrapping_add(squad[__h2].hp, n);
    crate::core_console::println(&__handle_2, &format!("2. {} at {} after a read in the middle", {
        let __h3: usize = crate::core_list::get_platform__loc(&squad, 0i32).expect("salvo: value is absent at main:236:19");
        squad[__h3].name.clone()
    }, {
        let __h4: usize = crate::core_list::get_platform__loc(&squad, 0i32).expect("salvo: value is absent at main:236:45");
        squad[__h4].hp
    }));
    crate::heal({
        let mut __nn_19: Option<&mut crate::Fighter> = crate::wounded(&mut squad);
        if __nn_19.is_none() {
            panic!("salvo: value is absent at main:239:10");
        } else {
            let mut __some_20 = __nn_19.unwrap();
            &mut *__some_20
        }
    });
    crate::rally_at::<i32>(&mut squad, &1i32, &mut |__a0: &Vec<crate::Fighter>, __a1: &i32| crate::core_list::at__loc(__a0, *__a1));
    crate::core_console::println(&__handle_2, &format!("2. after the searches: {} {}", {
        let __h5: usize = crate::core_list::get_platform__loc(&squad, 0i32).expect("salvo: value is absent at main:244:39");
        squad[__h5].hp
    }, {
        let __h6: usize = crate::core_list::get_platform__loc(&squad, 1i32).expect("salvo: value is absent at main:244:60");
        squad[__h6].hp
    }));
    let mut i: i32 = 0i32;
    let mut j: i32 = 1i32;
    if crate::core_index::NotEq__Int_qualifies(j, i.clone()) {
        let __h7: usize = crate::core_list::get_platform__loc(&squad, i).expect("salvo: value is absent at main:260:17");
        let __h8: usize = crate::core_list::get_platform__loc(&squad, j).expect("salvo: value is absent at main:261:17");
        squad[__h7].hp = i32::wrapping_add(squad[__h7].hp, 1i32);
        squad[__h8].hp = i32::wrapping_add(squad[__h8].hp, 1i32);
        { let (__pm9, __pm10) = crate::seq::salvo_pair_mut(&mut squad[..], __h7, __h8).expect("salvo: value is absent"); crate::duel(__pm9, __pm10) };
    };
    if crate::core_index::Idx__Int_qualifies(i, &squad, &mut |__a0| crate::core_list::size_platform(__a0)) {
        if crate::core_index::Idx__Int_qualifies(j, &squad, &mut |__a0| crate::core_list::size_platform(__a0)) {
            crate::core_list::update::<crate::Fighter>(&mut squad, i, &mut |mut f| {
                f.energy = i32::wrapping_add(f.energy, 1i32);
            });
            if crate::core_index::NotEq__Int_qualifies(j, i.clone()) {
                crate::core_list::update2::<crate::Fighter>(&mut squad, i, j, &mut |mut a, mut b| {
                    a.energy = i32::wrapping_add(a.energy, 100i32);
                    b.energy = i32::wrapping_add(b.energy, 200i32);
                });
            };
            crate::core_console::println(&__handle_2, &format!("3. {} {} (total reads: `Idx` survived)", {
                let __h11: usize = crate::core_list::get__loc(&squad, i);
                squad[__h11].energy
            }, {
                let __h12: usize = crate::core_list::get__loc(&squad, j);
                squad[__h12].energy
            }));
        };
    };
    if crate::core_index::Idx__Int_qualifies(i, &squad, &mut |__a0| crate::core_list::size_platform(__a0)) {
        if crate::core_index::Idx__Int_qualifies(j, &squad, &mut |__a0| crate::core_list::size_platform(__a0)) {
            { let __c13 = crate::core_list::get__loc(&squad, i); let __c14 = crate::core_list::get__loc(&squad, j); crate::strike(&mut squad, __c13, __c14) };
            { let __c15 = crate::core_list::get__loc(&squad, i); let __c16 = crate::core_list::get__loc(&squad, i); crate::strike(&mut squad, __c15, __c16) };
        };
    };
    crate::core_console::println(&__handle_2, &format!("4. {} hp / {} energy after striking itself", {
        let __h17: usize = crate::core_list::get_platform__loc(&squad, 0i32).expect("salvo: value is absent at main:294:19");
        squad[__h17].hp
    }, {
        let __h18: usize = crate::core_list::get_platform__loc(&squad, 0i32).expect("salvo: value is absent at main:294:45");
        squad[__h18].energy
    }));
    let mut team: crate::Squad = crate::Squad { banner: String::from("Red"), members: vec![crate::Fighter { name: String::from("Cy"), hp: 12i32, energy: 2i32 }, crate::Fighter { name: String::from("Dee"), hp: 6i32, energy: 7i32 }] };
    { let __c19 = crate::core_list::get_platform__loc(&team.members, i).expect("salvo: value is absent at main:301:18"); let __c20 = crate::core_list::get_platform__loc(&team.members, j).expect("salvo: value is absent at main:301:41"); crate::rotate(&mut team, __c19, __c20) };
    crate::core_console::println(&__handle_2, &format!("4. {}: {} {}", team.banner, {
        let __h21: usize = crate::core_list::get_platform__loc(&team.members, 0i32).expect("salvo: value is absent at main:302:35");
        team.members[__h21].energy
    }, {
        let __h22: usize = crate::core_list::get_platform__loc(&team.members, 1i32).expect("salvo: value is absent at main:302:67");
        team.members[__h22].energy
    }));
    let mut camp: crate::Camp = crate::Camp { supplies: 10i32, banners: vec![String::from("red")] };
    crate::spend(&mut camp, 3i32);
    crate::hoist(&mut camp, String::from("blue"));
    crate::core_console::println(&__handle_2, &format!("5. supplies {}, banners {}", camp.supplies, crate::core_list::to_str::<String>(&camp.banners, &mut |__a0: &String| format!("{}", __a0))));
}
