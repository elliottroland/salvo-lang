#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
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
#[path = "std/core/nonempty.rs"]
pub mod std_core_nonempty;
#[path = "std/core/ref.rs"]
pub mod std_core_ref;
#[path = "std/core/seq.rs"]
pub mod std_core_seq;
#[path = "std/core/string.rs"]
pub mod std_core_string;
#[path = "platform/std/core/console.rs"]
pub mod platform_std_core_console;
#[path = "platform/std/core/list.rs"]
pub mod platform_std_core_list;
#[path = "platform/std/core/seq.rs"]
pub mod platform_std_core_seq;
#[path = "platform/std/core/string.rs"]
pub mod platform_std_core_string;

use crate::std_core_iterator::Finished;
use crate::std_core_index::Idx__Int_qualifies;
use crate::std_core_list::ListIter;
use crate::std_core_list::List__T_TArray;
use crate::std_core_list::Mut_List;
use crate::std_core_ref::NotSame_qualifies;
use crate::std_core_list::add_platform;
use crate::std_core_list::at_platform;
use crate::std_core_seq::filter;
use crate::std_core_list::get;
use crate::std_core_list::get_platform;
use crate::std_core_list::iter;
use crate::std_core_list::next__ListIter;
use crate::std_core_console::println;
use crate::std_core_list::size_platform;
use crate::std_core_list::to_str;
use crate::std_core_list::update;


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
    return crate::std_core_list::get_platform::<crate::Fighter>(w.roster, w.at);
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
    crate::std_core_list::add_platform::<String>(&mut camp.banners, banner);
    return;
}

pub fn main() {
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut roster: Vec<crate::Fighter> = crate::std_core_list::List__T_TArray::<crate::Fighter>(crate::Fighter { name: String::from("Ada"), hp: 30i32, energy: 4i32 }, vec![crate::Fighter { name: String::from("Bo"), hp: 8i32, energy: 9i32 }]);
    let mut ada: &crate::Fighter = {
        let mut __tmp1 = String::from("Ada");
        let mut __pick2: Option<&crate::Fighter> = crate::named(&roster, &__tmp1);
        if __pick2.is_some() {
            let mut __pick2_3 = __pick2.unwrap();
            __pick2_3
        } else {
            panic!("salvo: unreachable at main:191:39")
        }
    };
    crate::std_core_console::println(&__handle_2, &format!("1. found {}, hp {}", ada.name, ada.hp));
    let mut names: Vec<String> = vec![];
    crate::std_core_list::add_platform::<String>(&mut names, (ada.name).clone());
    crate::std_core_console::println(&__handle_2, &format!("1. copied out {}", crate::std_core_list::to_str::<String>(&names, &mut |__a0: &String| format!("{}", __a0))));
    let mut w: crate::Window<'_> = crate::window(&roster);
    w.at = 1i32;
    crate::std_core_console::println(&__handle_2, &format!("1. window at {}: {}", w.at, {
        let mut __proj_5: &crate::Fighter = {
            let mut __pick3: Option<&crate::Fighter> = crate::peek(&w);
            if __pick3.is_some() {
                let mut __pick3_4 = __pick3.unwrap();
                __pick3_4
            } else {
                panic!("salvo: unreachable at main:205:50")
            }
        };
        __proj_5.name.clone()
    }));
    let mut pass: crate::std_core_list::ListIter<'_, crate::Fighter> = crate::std_core_list::iter::<crate::Fighter>(&roster);
    let mut __tmp2 = ();
    let mut standing: Vec<&crate::Fighter> = crate::std_core_seq::filter::<crate::std_core_list::ListIter<'_, crate::Fighter>, &crate::Fighter, ()>(&__tmp2, &mut pass, &mut |mut __leff0, mut f| -> bool {
        let f = *f;
        return (f.hp > 10i32);
    }, &mut |mut __fx0, mut __a0| -> crate::unions::Union2<&crate::Fighter, crate::std_core_iterator::Finished> {
        crate::std_core_list::next__ListIter(&mut *__a0)
    });
    crate::std_core_console::println(&__handle_2, &format!("1. {} of {} still standing", crate::std_core_list::size_platform(&standing), crate::std_core_list::size_platform::<crate::Fighter>(&roster)));
    let mut bench: Vec<crate::Fighter> = crate::std_core_list::Mut_List::<crate::Fighter>(crate::Fighter { name: String::from("Cy"), hp: 12i32, energy: 2i32 }, vec![]);
    crate::std_core_list::add_platform::<crate::Fighter>(&mut bench, crate::Fighter { name: String::from("Dee"), hp: 6i32, energy: 7i32 });
    crate::std_core_console::println(&__handle_2, &format!("2. bench {}, front {} (a reading, not a handle)", crate::std_core_list::size_platform::<crate::Fighter>(&bench), {
        let mut __proj_7: &crate::Fighter = {
            let mut __pick4: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&bench, 0i32);
            if __pick4.is_some() {
                let mut __pick4_6 = __pick4.unwrap();
                __pick4_6
            } else {
                panic!("salvo: unreachable at main:220:65")
            }
        };
        __proj_7.name.clone()
    }));
    let mut squad: Vec<crate::Fighter> = crate::std_core_list::List__T_TArray::<crate::Fighter>(crate::Fighter { name: String::from("Ada"), hp: 30i32, energy: 4i32 }, vec![crate::Fighter { name: String::from("Bo"), hp: 8i32, energy: 9i32 }]);
    let __h3: usize = crate::std_core_list::at_platform(&squad, 0i32).expect("salvo: value is absent at main:231:32");
    squad[__h3].hp = i32::wrapping_add(squad[__h3].hp, 1i32);
    let mut n: i32 = crate::std_core_list::size_platform::<crate::Fighter>(&squad);
    squad[__h3].hp = i32::wrapping_add(squad[__h3].hp, n);
    crate::std_core_console::println(&__handle_2, &format!("2. {} at {} after a read in the middle", {
        let mut __proj_10: &crate::Fighter = {
            let mut __pick6: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick6.is_some() {
                let mut __pick6_9 = __pick6.unwrap();
                __pick6_9
            } else {
                panic!("salvo: unreachable at main:235:37")
            }
        };
        __proj_10.name.clone()
    }, {
        let mut __proj_12: &crate::Fighter = {
            let mut __pick7: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick7.is_some() {
                let mut __pick7_11 = __pick7.unwrap();
                __pick7_11
            } else {
                panic!("salvo: unreachable at main:235:82")
            }
        };
        __proj_12.hp
    }));
    crate::heal({
        let __h4: Option<usize> = crate::wounded(&squad);
        if __h4.map(|__x| &squad[__x]).is_some() {
            let __h5 = __h4.expect("salvo: value is absent");
            &mut squad[__h5]
        } else {
            panic!("salvo: unreachable at main:238:28")
        }
    });
    crate::rally_at::<i32>(&mut squad, &1i32, &mut |__a0: &Vec<crate::Fighter>, __a1: &i32| crate::std_core_list::at_platform(__a0, *__a1));
    crate::std_core_console::println(&__handle_2, &format!("2. after the searches: {} {}", {
        let mut __proj_15: &crate::Fighter = {
            let mut __pick9: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick9.is_some() {
                let mut __pick9_14 = __pick9.unwrap();
                __pick9_14
            } else {
                panic!("salvo: unreachable at main:243:57")
            }
        };
        __proj_15.hp
    }, {
        let mut __proj_17: &crate::Fighter = {
            let mut __pick10: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&squad, 1i32);
            if __pick10.is_some() {
                let mut __pick10_16 = __pick10.unwrap();
                __pick10_16
            } else {
                panic!("salvo: unreachable at main:243:97")
            }
        };
        __proj_17.hp
    }));
    let mut i: i32 = 0i32;
    let mut j: i32 = 1i32;
    let __h6: usize = crate::std_core_list::at_platform(&squad, i).expect("salvo: value is absent at main:259:29");
    let __h7: usize = crate::std_core_list::at_platform(&squad, j).expect("salvo: value is absent at main:260:29");
    if crate::std_core_ref::NotSame_qualifies::<crate::Fighter>(&squad[__h7], &squad[__h6]) {
        squad[__h6].hp = i32::wrapping_add(squad[__h6].hp, 1i32);
        squad[__h7].hp = i32::wrapping_add(squad[__h7].hp, 1i32);
        { let (__pm10, __pm11) = if __h6 != __h7 { let (__u8, __w9) = crate::seq::salvo_pair_mut(&mut squad[..], __h6, __h7).expect("salvo: value is absent"); (&mut (*__u8), &mut (*__w9)) } else { panic!("salvo: two handles to one element") }; crate::duel(__pm10, __pm11) };
        squad[__h6].energy = i32::wrapping_add(squad[__h6].energy, 100i32);
        squad[__h7].energy = i32::wrapping_add(squad[__h7].energy, 200i32);
    };
    if crate::std_core_index::Idx__Int_qualifies::<Vec<crate::Fighter>>(i, &squad, &mut |__a0: &Vec<crate::Fighter>| crate::std_core_list::size_platform(__a0)) {
        if crate::std_core_index::Idx__Int_qualifies::<Vec<crate::Fighter>>(j, &squad, &mut |__a0: &Vec<crate::Fighter>| crate::std_core_list::size_platform(__a0)) {
            crate::std_core_list::update::<crate::Fighter, ()>(&(), &mut squad, i, &mut |mut __leff0, mut f| {
                f.energy = i32::wrapping_add(f.energy, 1i32);
            });
            crate::std_core_console::println(&__handle_2, &format!("3. {} {} (total reads: `Idx` survived)", {
                let mut __proj_20: &crate::Fighter = crate::std_core_list::get::<crate::Fighter>(&squad, i);
                __proj_20.energy
            }, {
                let mut __proj_21: &crate::Fighter = crate::std_core_list::get::<crate::Fighter>(&squad, j);
                __proj_21.energy
            }));
        };
    };
    { let __q12 = crate::std_core_list::at_platform(&squad, i).expect("salvo: value is absent at main:282:35"); let __c13 = __q12; let __q14 = crate::std_core_list::at_platform(&squad, j).expect("salvo: value is absent at main:282:67"); let __c15 = __q14; crate::strike(&mut squad, __c13, __c15) };
    { let __q16 = crate::std_core_list::at_platform(&squad, i).expect("salvo: value is absent at main:285:35"); let __c17 = __q16; let __q18 = crate::std_core_list::at_platform(&squad, i).expect("salvo: value is absent at main:285:67"); let __c19 = __q18; crate::strike(&mut squad, __c17, __c19) };
    crate::std_core_console::println(&__handle_2, &format!("4. {} hp / {} energy after striking itself", {
        let mut __proj_27: &crate::Fighter = {
            let mut __pick17: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick17.is_some() {
                let mut __pick17_26 = __pick17.unwrap();
                __pick17_26
            } else {
                panic!("salvo: unreachable at main:286:37")
            }
        };
        __proj_27.hp
    }, {
        let mut __proj_29: &crate::Fighter = {
            let mut __pick18: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&squad, 0i32);
            if __pick18.is_some() {
                let mut __pick18_28 = __pick18.unwrap();
                __pick18_28
            } else {
                panic!("salvo: unreachable at main:286:82")
            }
        };
        __proj_29.energy
    }));
    let mut team: crate::Squad = crate::Squad { banner: String::from("Red"), members: crate::std_core_list::List__T_TArray::<crate::Fighter>(crate::Fighter { name: String::from("Cy"), hp: 12i32, energy: 2i32 }, vec![crate::Fighter { name: String::from("Dee"), hp: 6i32, energy: 7i32 }]) };
    { let __q20 = crate::std_core_list::at_platform(&team.members, i).expect("salvo: value is absent at main:293:49"); let __c21 = __q20; let __q22 = crate::std_core_list::at_platform(&team.members, j).expect("salvo: value is absent at main:293:88"); let __c23 = __q22; crate::rotate(&mut team.members, __c21, __c23) };
    crate::std_core_console::println(&__handle_2, &format!("4. {}: {} {}", team.banner, {
        let mut __proj_33: &crate::Fighter = {
            let mut __pick21: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&team.members, 0i32);
            if __pick21.is_some() {
                let mut __pick21_32 = __pick21.unwrap();
                __pick21_32
            } else {
                panic!("salvo: unreachable at main:294:60")
            }
        };
        __proj_33.energy
    }, {
        let mut __proj_35: &crate::Fighter = {
            let mut __pick22: Option<&crate::Fighter> = crate::std_core_list::get_platform::<crate::Fighter>(&team.members, 1i32);
            if __pick22.is_some() {
                let mut __pick22_34 = __pick22.unwrap();
                __pick22_34
            } else {
                panic!("salvo: unreachable at main:294:111")
            }
        };
        __proj_35.energy
    }));
    let mut camp: crate::Camp = crate::Camp { supplies: 10i32, banners: crate::std_core_list::Mut_List::<String>(String::from("red"), vec![]) };
    crate::spend(&mut camp, 3i32);
    crate::hoist(&mut camp, String::from("blue"));
    crate::std_core_console::println(&__handle_2, &format!("5. supplies {}, banners {}", camp.supplies, crate::std_core_list::to_str::<String>(&camp.banners, &mut |__a0: &String| format!("{}", __a0))));
}
