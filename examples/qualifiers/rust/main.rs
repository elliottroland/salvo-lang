#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions/mod.rs"]
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
#[path = "core/compare.rs"]
pub mod core_compare;
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

use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;
use crate::core_list::first;

pub fn NonEmpty__List_qualifies<T: Clone>(list: &Vec<T>) -> bool {
    return crate::core_list::size_platform(list) > 0;
}

pub fn head(list: &Vec<i32>) -> i32 {
    let mut first = crate::core_list::get_platform(list, 0);
    return *first.expect("salvo: value is absent at main:30:12");
}

pub fn celsius(degrees: i32) -> i32 {
    return degrees;
}

pub fn describe__Int(temp: i32) -> String {
    return format!("{} (no unit)", temp);
}

pub fn describe__CelsiusInt(temp: &i32) -> String {
    return format!("{}°C", *temp);
}

pub fn sum(list: &Vec<i32>) -> i32 {
    let mut total = 0;
    for n in crate::platform_core_list::each(list) {
        total = i32::wrapping_add(total, *n);
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

pub fn authenticate(mut request: Request) -> Request {
    return request;
}

pub fn freshen(mut request: Request) -> Request {
    return request;
}

pub fn touch(request: &mut Request) {
    request.touches = i32::wrapping_add(request.touches, 1);
}

pub fn handle__Request(request: &Request) -> String {
    return format!("plain {}", request.path.clone());
}

pub fn handle__AuthenticatedRequest(request: &Request) -> String {
    return format!("authenticated {}", request.path.clone());
}

pub fn handle__FreshRequest(request: &Request) -> String {
    return format!("fresh {}", request.path.clone());
}

pub fn main() {
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let mut xs: Vec<i32> = vec![];
    crate::core_list::add_platform(&mut xs, 3);
    println(&console, &(format!("1. head after add: {}", head(&xs))));
    let mut maybe_empty = vec![7, 8];
    if NonEmpty__List_qualifies(&maybe_empty) {
        println(&console, &(format!("2. checked at run time, head is {}", head(&maybe_empty))));
    }
    let mut plain = 21;
    let mut warm = celsius(21);
    println(&console, &(format!("2. {} vs {}", describe__Int(plain), describe__CelsiusInt(&warm))));
    if true {
        println(&console, &(format!("2. widened: {}", describe__Int(warm))));
    }
    println(&console, &(format!("3. sum {}, head still {}", sum(&xs), head(&xs))));
    compact(&mut xs);
    crate::core_list::add_platform(&mut xs, 9);
    println(&console, &(format!("3. after compact and add, head is {}", head(&xs))));
    let mut session = authenticate(Request { path: "/orders".to_string(), touches: 0 });
    let mut fresh = freshen(Request { path: "/health".to_string(), touches: 0 });
    println(&console, &(format!("4. before: {} / {}", handle__AuthenticatedRequest(&session), handle__FreshRequest(&fresh))));
    touch(&mut session);
    touch(&mut fresh);
    println(&console, &(format!("4. after:  {} / {}", handle__AuthenticatedRequest(&session), handle__Request(&fresh))));
}
