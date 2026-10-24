#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
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
#[path = "core/ref.rs"]
pub mod core_ref;
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
use crate::core_ref::NotSame_qualifies;
use crate::core_list::add_platform;
use crate::core_list::at_platform;
use crate::core_seq::filter;
use crate::core_list::get;
use crate::core_list::get_platform;
use crate::core_list::iter;
use crate::core_list::next__ListYield;
use crate::core_console::println;
use crate::core_list::size_platform;
use crate::core_list::to_str;
use crate::core_list::update;


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

pub fn wounded(squad: &Vec<crate::Fighter>) -> Option<usize> {
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
        let __h1: Option<usize> = at(&*squad, l);
        if __h1.map(|__x| &squad[__x]).is_some() {
            let __h2 = __h1.expect("salvo: value is absent");
            &mut squad[__h2]
        } else {
            panic!("salvo: unreachable at main:102:26")
        }
    });
    return;
}

pub fn duel(a: &mut crate::Fighter, d: &mut crate::Fighter) {
    a.hp = i32::wrapping_sub(a.hp, 1i32);
    d.hp = i32::wrapping_sub(d.hp, 2i32);
    return;
}

pub fn strike(c: &mut Vec<crate::Fighter>, __c1: usize, __c2: usize) {
    c[__c1].energy = i32::wrapping_sub(c[__c1].energy, 1i32);
    c[__c2].hp = i32::wrapping_sub(c[__c2].hp, 2i32);
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

pub fn rotate(members: &mut Vec<crate::Fighter>, __c1: usize, __c2: usize) {
    members[__c1].energy = i32::wrapping_sub(members[__c1].energy, 1i32);
    members[__c2].energy = i32::wrapping_add(members[__c2].energy, 1i32);
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
    let mut __use_1: crate::core_console::__Platform_StdConsole = crate::core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut roster: Vec<crate::Fighter> = vec![crate::Fighter { name: String::from("Ada"), hp: 30i32, energy: 4i32 }, crate::Fighter { name: String::from("Bo"), hp: 8i32, energy: 9i32 }];
    let mut ada: &crate::Fighter = {
        let mut __tmp1 = String::from("Ada");
        let mut __pick_3: Option<&crate::Fighter> = crate::named(&roster, &__tmp1);
        if __pick_3.is_some() {
            let mut __picked_4 = __pick_3.unwrap();
            __picked_4
        } else {
            panic!("salvo: unreachable at main:191:39")
        }
    };
    crate::core_console::println(&__handle_2, &format!("1. found {}, hp {}", ada.name, ada.hp));
    let mut names: Vec<String> = vec![];
    crate::core_list::add_platform::<String>(&mut names, (ada.name).clone());
    crate::core_console::println(&__handle_2, &format!("1. copied out {}", crate::core_list::to_str::<String>(&names, &mut |__a0: &String| format!("{}", __a0))));
    let mut w: crate::Window<'_> = crate::window(&roster);
    w.at = 1i32;
    crate::core_console::println(&__handle_2, &format!("1. window at {}: {}", w.at, {
        let mut __proj_9: &crate::Fighter = {
            let mut __pick_6: Option<&crate::Fighter> = crate::peek(&w);
            if __pick_6.is_some() {
                let mut __picked_7 = __pick_6.unwrap();
                __picked_7
            } else {
                panic!("salvo: unreachable at main:205:50")
            }
        };
        __proj_9.name.clone()
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
        let mut __proj_13: &crate::Fighter = {
            let mut __pick_10: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&bench, 0i32);
            if __pick_10.is_some() {
                let mut __picked_11 = __pick_10.unwrap();
                __picked_11
            } else {
                panic!("salvo: unreachable at main:220:65")
            }
        };
        __proj_13.name.clone()
    }));
    let mut squad: Vec<crate::Fighter> = vec![crate::Fighter { name: String::from("Ada"), hp: 30i32, energy: 4i32 }, crate::Fighter { name: String::from("Bo"), hp: 8i32, energy: 9i32 }];
    let __h2: usize = crate::core_list::at_platform(&squad, 0i32).expect("salvo: value is absent at main:231:32");
    squad[__h2].hp = i32::wrapping_add(squad[__h2].hp, 1i32);
    let mut n: i32 = crate::core_list::size_platform::<crate::Fighter>(&squad);
    squad[__h2].hp = i32::wrapping_add(squad[__h2].hp, n);
    crate::core_console::println(&__handle_2, &format!("2. {} at {} after a read in the middle", {
        let mut __proj_20: &crate::Fighter = {
            let mut __pick_17: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick_17.is_some() {
                let mut __picked_18 = __pick_17.unwrap();
                __picked_18
            } else {
                panic!("salvo: unreachable at main:235:37")
            }
        };
        __proj_20.name.clone()
    }, {
        let mut __proj_24: &crate::Fighter = {
            let mut __pick_21: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick_21.is_some() {
                let mut __picked_22 = __pick_21.unwrap();
                __picked_22
            } else {
                panic!("salvo: unreachable at main:235:82")
            }
        };
        __proj_24.hp
    }));
    crate::heal({
        let __h3: Option<usize> = crate::wounded(&squad);
        if __h3.map(|__x| &squad[__x]).is_some() {
            let __h4 = __h3.expect("salvo: value is absent");
            &mut squad[__h4]
        } else {
            panic!("salvo: unreachable at main:238:28")
        }
    });
    crate::rally_at::<i32>(&mut squad, &1i32, &mut |__a0: &Vec<crate::Fighter>, __a1: &i32| crate::core_list::at_platform(__a0, *__a1));
    crate::core_console::println(&__handle_2, &format!("2. after the searches: {} {}", {
        let mut __proj_31: &crate::Fighter = {
            let mut __pick_28: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick_28.is_some() {
                let mut __picked_29 = __pick_28.unwrap();
                __picked_29
            } else {
                panic!("salvo: unreachable at main:243:57")
            }
        };
        __proj_31.hp
    }, {
        let mut __proj_35: &crate::Fighter = {
            let mut __pick_32: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&squad, 1i32);
            if __pick_32.is_some() {
                let mut __picked_33 = __pick_32.unwrap();
                __picked_33
            } else {
                panic!("salvo: unreachable at main:243:97")
            }
        };
        __proj_35.hp
    }));
    let mut i: i32 = 0i32;
    let mut j: i32 = 1i32;
    let __h5: usize = crate::core_list::at_platform(&squad, i).expect("salvo: value is absent at main:259:29");
    let __h6: usize = crate::core_list::at_platform(&squad, j).expect("salvo: value is absent at main:260:29");
    if crate::core_ref::NotSame_qualifies(&squad[__h6], &squad[__h5]) {
        squad[__h5].hp = i32::wrapping_add(squad[__h5].hp, 1i32);
        squad[__h6].hp = i32::wrapping_add(squad[__h6].hp, 1i32);
        { let (__pm9, __pm10) = if __h5 != __h6 { let (__u7, __w8) = crate::seq::salvo_pair_mut(&mut squad[..], __h5, __h6).expect("salvo: value is absent"); (&mut (*__u7), &mut (*__w8)) } else { panic!("salvo: two handles to one element") }; crate::duel(__pm9, __pm10) };
        squad[__h5].energy = i32::wrapping_add(squad[__h5].energy, 100i32);
        squad[__h6].energy = i32::wrapping_add(squad[__h6].energy, 200i32);
    };
    if crate::core_index::Idx__Int_qualifies(i, &squad, &mut |__a0| crate::core_list::size_platform(__a0)) {
        if crate::core_index::Idx__Int_qualifies(j, &squad, &mut |__a0| crate::core_list::size_platform(__a0)) {
            crate::core_list::update::<crate::Fighter>(&mut squad, i, &mut |mut f| {
                f.energy = i32::wrapping_add(f.energy, 1i32);
            });
            crate::core_console::println(&__handle_2, &format!("3. {} {} (total reads: `Idx` survived)", {
                let mut __proj_42: &crate::Fighter = crate::core_list::get::<crate::Fighter>(&squad, i);
                __proj_42.energy
            }, {
                let mut __proj_43: &crate::Fighter = crate::core_list::get::<crate::Fighter>(&squad, j);
                __proj_43.energy
            }));
        };
    };
    { let __q11 = crate::core_list::at_platform(&squad, i).expect("salvo: value is absent at main:282:35"); let __c12 = __q11; let __q13 = crate::core_list::at_platform(&squad, j).expect("salvo: value is absent at main:282:67"); let __c14 = __q13; crate::strike(&mut squad, __c12, __c14) };
    { let __q15 = crate::core_list::at_platform(&squad, i).expect("salvo: value is absent at main:285:35"); let __c16 = __q15; let __q17 = crate::core_list::at_platform(&squad, i).expect("salvo: value is absent at main:285:67"); let __c18 = __q17; crate::strike(&mut squad, __c16, __c18) };
    crate::core_console::println(&__handle_2, &format!("4. {} hp / {} energy after striking itself", {
        let mut __proj_59: &crate::Fighter = {
            let mut __pick_56: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick_56.is_some() {
                let mut __picked_57 = __pick_56.unwrap();
                __picked_57
            } else {
                panic!("salvo: unreachable at main:286:37")
            }
        };
        __proj_59.hp
    }, {
        let mut __proj_63: &crate::Fighter = {
            let mut __pick_60: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick_60.is_some() {
                let mut __picked_61 = __pick_60.unwrap();
                __picked_61
            } else {
                panic!("salvo: unreachable at main:286:82")
            }
        };
        __proj_63.energy
    }));
    let mut team: crate::Squad = crate::Squad { banner: String::from("Red"), members: vec![crate::Fighter { name: String::from("Cy"), hp: 12i32, energy: 2i32 }, crate::Fighter { name: String::from("Dee"), hp: 6i32, energy: 7i32 }] };
    { let __q19 = crate::core_list::at_platform(&team.members, i).expect("salvo: value is absent at main:293:49"); let __c20 = __q19; let __q21 = crate::core_list::at_platform(&team.members, j).expect("salvo: value is absent at main:293:88"); let __c22 = __q21; crate::rotate(&mut team.members, __c20, __c22) };
    crate::core_console::println(&__handle_2, &format!("4. {}: {} {}", team.banner, {
        let mut __proj_73: &crate::Fighter = {
            let mut __pick_70: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&team.members, 0i32);
            if __pick_70.is_some() {
                let mut __picked_71 = __pick_70.unwrap();
                __picked_71
            } else {
                panic!("salvo: unreachable at main:294:60")
            }
        };
        __proj_73.energy
    }, {
        let mut __proj_77: &crate::Fighter = {
            let mut __pick_74: Option<&crate::Fighter> = crate::core_list::get_platform::<crate::Fighter>(&team.members, 1i32);
            if __pick_74.is_some() {
                let mut __picked_75 = __pick_74.unwrap();
                __picked_75
            } else {
                panic!("salvo: unreachable at main:294:111")
            }
        };
        __proj_77.energy
    }));
    let mut camp: crate::Camp = crate::Camp { supplies: 10i32, banners: vec![String::from("red")] };
    crate::spend(&mut camp, 3i32);
    crate::hoist(&mut camp, String::from("blue"));
    crate::core_console::println(&__handle_2, &format!("5. supplies {}, banners {}", camp.supplies, crate::core_list::to_str::<String>(&camp.banners, &mut |__a0: &String| format!("{}", __a0))));
}
