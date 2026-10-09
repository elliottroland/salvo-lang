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
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "throw.rs"]
pub mod throw;
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

use crate::core_console::Console;
use crate::core_string::parse_int_platform;
use crate::core_console::println;
use crate::core_string::size_platform;


#[derive(Clone, Debug, PartialEq)]
pub struct FileHandle {
    pub name: String,
}

impl crate::wire::__Wire for FileHandle {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn open_file(console: &crate::core_console::Console, mut name: String) -> crate::FileHandle {
    crate::core_console::println(console, &format!("1. open {}", name));
    return crate::FileHandle { name: name.clone() };
}

pub fn close(console: &crate::core_console::Console, mut handle: crate::FileHandle) {
    crate::core_console::println(console, &format!("1. close {}", handle.name));
    std::mem::drop(handle);
}

pub fn read_size(console: &crate::core_console::Console, mut name: String, mut want: i32) -> i32 {
    let mut there_is: i32 = crate::core_string::size_platform(&name);
    let mut handle: crate::FileHandle = crate::open_file(console, name);
    if (want > there_is) {
        crate::core_console::println(console, &String::from("1. asked for more than there is"));
        crate::close(console, handle);
        return there_is;
    };
    crate::close(console, handle);
    return want;
}

pub fn parse_port(text: &String) -> std::ops::ControlFlow<String, i32> {
    let mut n: Option<i32> = crate::core_string::parse_int_platform(text);
    if n.is_none() {
        return std::ops::ControlFlow::Break(format!("not a number: {}", text));
    };
    let mut n_1 = n.unwrap();
    if (n_1 < 1i32) {
        return std::ops::ControlFlow::Break(String::from("port must be positive"));
    };
    let mut n_2 = n.unwrap();
    return std::ops::ControlFlow::Continue(n_2);
}

pub fn port_of(config: &String) -> std::ops::ControlFlow<String, i32> {
    let mut port: i32 = (match crate::parse_port(config) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => return std::ops::ControlFlow::Break(__m) });
    return std::ops::ControlFlow::Continue(i32::wrapping_mul(port, 1i32));
}

pub fn port_from_file(console: &crate::core_console::Console, mut name: String, text: &String) -> std::ops::ControlFlow<String, i32> {
    let mut handle: crate::FileHandle = crate::open_file(console, name);
    let mut from: String = (handle.name).clone();
    crate::close(console, handle);
    crate::core_console::println(console, &format!("2. reading a port out of {}", from));
    return std::ops::ControlFlow::Continue((match crate::parse_port(text) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => return std::ops::ControlFlow::Break(__m) }));
}

pub fn strict_port(text: &String) -> std::ops::ControlFlow<crate::unions::Union2<String, i32>, i32> {
    if (crate::core_string::size_platform(text) == 0i32) {
        return std::ops::ControlFlow::Break(crate::unions::Union2::U1(String::from("empty")));
    };
    let mut n: Option<i32> = crate::core_string::parse_int_platform(text);
    if n.is_none() {
        return std::ops::ControlFlow::Break(crate::unions::Union2::U2(crate::core_string::size_platform(text)));
    };
    let mut n_1 = n.unwrap();
    return std::ops::ControlFlow::Continue(n_1);
}

pub fn report(console: &crate::core_console::Console, label: &String, config: &String) {
    let mut outcome: crate::unions::Union2<i32, String> = ('try_1: {
        crate::unions::Union2::U1((match crate::port_of(config) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => break 'try_1 crate::unions::Union2::U2(__m) }))
    });
    if matches!(outcome, crate::unions::Union2::U1(_)) {
        let mut outcome_1 = match &outcome { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(console, &format!("3. {}: port {}", label, outcome_1));
    } else {
        let mut outcome_2 = match &outcome { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("3. {}: rejected — {}", label, outcome_2));
    };
}

pub fn main() {
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut small: i32 = crate::read_size(&__handle_2, String::from("notes.txt"), 3i32);
    crate::core_console::println(&__handle_2, &format!("1. read {}", small));
    let mut clamped: i32 = crate::read_size(&__handle_2, String::from("notes.txt"), 99i32);
    crate::core_console::println(&__handle_2, &format!("1. read {}", clamped));
    crate::report(&__handle_2, &String::from("good"), &String::from("8080"));
    crate::report(&__handle_2, &String::from("bad"), &String::from("http"));
    let mut guarded: crate::unions::Union2<i32, String> = ('try_1: {
        crate::unions::Union2::U1((match crate::port_from_file(&__handle_2, String::from("ports.txt"), &String::from("-1")) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => break 'try_1 crate::unions::Union2::U2(__m) }))
    });
    if matches!(guarded, crate::unions::Union2::U1(_)) {
        let mut guarded_3 = match &guarded { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("3. guarded: {}", guarded_3));
    } else {
        let mut guarded_4 = match &guarded { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("3. guarded: rejected — {}", guarded_4));
    };
    let mut mixed: crate::unions::Union2<i32, crate::unions::Union2<String, i32>> = ('try_2: {
        crate::unions::Union2::U1((match crate::strict_port(&String::from("")) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => break 'try_2 crate::unions::Union2::U2(__m) }))
    });
    if matches!(mixed, crate::unions::Union2::U1(_)) {
        let mut mixed_5 = match &mixed { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("3. mixed: {}", mixed_5));
    } else {
        let mut mixed_6 = match &mixed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut why = mixed_6;
        if matches!(why, crate::unions::Union2::U1(_)) {
            let mut why_7 = match &why { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::core_console::println(&__handle_2, &format!("3. mixed: message {}", why_7));
        } else {
            let mut why_8 = match &why { crate::unions::Union2::U2(__v) => *__v, _ => unreachable!() };
            crate::core_console::println(&__handle_2, &format!("3. mixed: length {}", why_8));
        };
    };
    let mut outer: crate::unions::Union2<i32, String> = ('try_3: {
        let mut inner: crate::unions::Union2<i32, String> = ('try_4: {
            crate::unions::Union2::U1((match crate::parse_port(&String::from("nope")) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => break 'try_4 crate::unions::Union2::U2(__m) }))
        });
        crate::unions::Union2::U1(if matches!(inner, crate::unions::Union2::U1(_)) {
            let mut inner_9 = match &inner { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            let mut got: i32 = inner_9;
            got
        } else {
            let mut inner_10 = match &inner { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(&__handle_2, &format!("3. inner caught: {}", inner_10));
            (match crate::parse_port(&String::from("also nope")) { std::ops::ControlFlow::Continue(__v) => __v, std::ops::ControlFlow::Break(__m) => break 'try_3 crate::unions::Union2::U2(__m) })
        })
    });
    if matches!(outer, crate::unions::Union2::U1(_)) {
        let mut outer_11 = match &outer { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("3. outer: {}", outer_11));
    } else {
        let mut outer_12 = match &outer { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("3. outer caught: {}", outer_12));
    };
}
