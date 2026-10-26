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

use crate::std_core_list::add_platform;
use crate::std_core_list::get_platform;
use crate::std_core_console::println;
use crate::std_core_list::size_platform;


pub fn NonEmpty__List_qualifies<T: Clone>(list: &Vec<T>) -> bool {
    return (crate::std_core_list::size_platform::<T>(list) > 0i32);
}

pub fn head(list: &Vec<i32>) -> i32 {
    let mut first: Option<i32> = crate::std_core_list::get_platform::<i32>(list, 0i32).copied();
    return {
        let mut __pick_1 = &first;
        if __pick_1.is_some() {
            let mut __picked_2 = __pick_1.unwrap();
            __picked_2
        } else {
            panic!("salvo: unreachable at main:30:21")
        }
    };
}

pub fn celsius(mut degrees: i32) -> i32 {
    return degrees;
}

pub fn describe__Int(mut temp: i32) -> String {
    return format!("{} (no unit)", temp);
}

pub fn describe__CelsiusInt(mut temp: i32) -> String {
    return format!("{}°C", temp);
}

pub fn sum(list: &Vec<i32>) -> i32 {
    let mut total: i32 = 0i32;
    for mut n in list.iter().copied() {
        total = i32::wrapping_add(total, n);
    }
    return total;
}

pub fn compact(list: &mut Vec<i32>) {
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub path: String,
    pub touches: i32,
}

impl crate::wire::__Wire for Request {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
        crate::wire::__Wire::__enc(&self.touches, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
            touches: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn authenticate(mut request: crate::Request) -> crate::Request {
    return request;
}

pub fn freshen(mut request: crate::Request) -> crate::Request {
    return request;
}

pub fn touch(request: &mut crate::Request) {
    request.touches = i32::wrapping_add(request.touches, 1i32);
}

pub fn handle__Request(request: &crate::Request) -> String {
    return format!("plain {}", request.path);
}

pub fn handle__AuthenticatedRequest(request: &crate::Request) -> String {
    return format!("authenticated {}", request.path);
}

pub fn handle__FreshRequest(request: &crate::Request) -> String {
    return format!("fresh {}", request.path);
}

pub fn main() {
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut xs: Vec<i32> = vec![];
    crate::std_core_list::add_platform::<i32>(&mut xs, 3i32);
    crate::std_core_console::println(&__handle_2, &format!("1. head after add: {}", crate::head(&xs)));
    let mut maybe_empty: Vec<i32> = vec![7i32, 8i32];
    if crate::std_core_list::NonEmpty__List_qualifies(&maybe_empty) {
        crate::std_core_console::println(&__handle_2, &format!("2. checked at run time, head is {}", crate::head(&maybe_empty)));
    };
    let mut plain: i32 = 21i32;
    let mut warm: i32 = crate::celsius(21i32);
    crate::std_core_console::println(&__handle_2, &format!("2. {} vs {}", crate::describe__Int(plain), crate::describe__CelsiusInt(warm)));
    {
        crate::std_core_console::println(&__handle_2, &format!("2. widened: {}", crate::describe__Int(warm)));
    };
    crate::std_core_console::println(&__handle_2, &format!("3. sum {}, head still {}", crate::sum(&xs), crate::head(&xs)));
    crate::compact(&mut xs);
    crate::std_core_list::add_platform::<i32>(&mut xs, 9i32);
    crate::std_core_console::println(&__handle_2, &format!("3. after compact and add, head is {}", crate::head(&xs)));
    let mut session: crate::Request = crate::authenticate(crate::Request { path: String::from("/orders"), touches: 0i32 });
    let mut fresh: crate::Request = crate::freshen(crate::Request { path: String::from("/health"), touches: 0i32 });
    crate::std_core_console::println(&__handle_2, &format!("4. before: {} / {}", crate::handle__AuthenticatedRequest(&session), crate::handle__FreshRequest(&fresh)));
    crate::touch(&mut session);
    crate::touch(&mut fresh);
    crate::std_core_console::println(&__handle_2, &format!("4. after:  {} / {}", crate::handle__AuthenticatedRequest(&session), crate::handle__Request(&fresh)));
}
