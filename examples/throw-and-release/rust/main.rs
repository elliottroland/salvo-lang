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
#[path = "core/console.rs"]
pub mod core_console;
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

use crate::unions::*;
use std::ops::ControlFlow;
use crate::core_console::ConsolePlatformSync as _;
use crate::core_console::__Stateful_Console as _;
use crate::core_console::__Stateless_Console as _;
use crate::core_console::println;

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

pub fn open_file(console: &crate::core_console::Console, name: String) -> FileHandle {
    println(console, &(format!("1. open {}", name)));
    return FileHandle { name: name };
}

pub fn close(console: &crate::core_console::Console, handle: FileHandle) {
    println(console, &(format!("1. close {}", handle.name.clone())));
    drop(handle);
}

pub fn read_size(console: &crate::core_console::Console, name: String, want: i32) -> i32 {
    let mut there_is = crate::core_string::size_platform(&name);
    let mut handle = open_file(console, name);
    if want > there_is {
        println(console, &("1. asked for more than there is".to_string()));
        close(console, handle);
        return there_is;
    }
    close(console, handle);
    return want;
}

pub fn parse_port(text: &String) -> ControlFlow<String, i32> {
    let mut n = crate::core_string::parse_int_platform(text);
    if n.is_none() {
        return ControlFlow::Break(format!("not a number: {}", text.clone()));
    }
    if n.unwrap() < 1 {
        return ControlFlow::Break("port must be positive".to_string());
    }
    return ControlFlow::Continue(n.unwrap());
}

pub fn port_of(config: &String) -> ControlFlow<String, i32> {
    let mut port = parse_port(config)?;
    return ControlFlow::Continue(port * 1);
}

pub fn port_from_file(console: &crate::core_console::Console, name: String, text: &String) -> ControlFlow<String, i32> {
    let mut handle = open_file(console, name);
    let mut from = handle.name.clone();
    close(console, handle);
    println(console, &(format!("2. reading a port out of {}", from)));
    return ControlFlow::Continue(parse_port(text)?);
}

pub fn strict_port(text: &String) -> ControlFlow<Union2<String, i32>, i32> {
    if crate::core_string::size_platform(text) == 0 {
        return ControlFlow::Break(Union2::<String, i32>::U1("empty".to_string()));
    }
    let mut n = crate::core_string::parse_int_platform(text);
    if n.is_none() {
        return ControlFlow::Break(Union2::<String, i32>::U2(crate::core_string::size_platform(text)));
    }
    return ControlFlow::Continue(n.unwrap());
}

pub fn report(console: &crate::core_console::Console, label: &String, config: &String) {
    let mut outcome = 'try_1: {
        Union2::<i32, String>::U1(match port_of(config) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_1 Union2::<i32, String>::U2(__m) })
    };
    match outcome {
        Union2::U1(_) => {
            println(console, &(format!("3. {}: port {}", label.clone(), *outcome.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("3. {}: rejected — {}", label.clone(), outcome.u2().clone())));
        }
    }
}

pub fn main() {
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let mut small = read_size(&console, "notes.txt".to_string(), 3);
    println(&console, &(format!("1. read {}", small)));
    let mut clamped = read_size(&console, "notes.txt".to_string(), 99);
    println(&console, &(format!("1. read {}", clamped)));
    report(&console, &("good".to_string()), &("8080".to_string()));
    report(&console, &("bad".to_string()), &("http".to_string()));
    let mut guarded = 'try_2: {
        Union2::<i32, String>::U1(match port_from_file(&console, "ports.txt".to_string(), &("-1".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_2 Union2::<i32, String>::U2(__m) })
    };
    match guarded {
        Union2::U1(_) => {
            println(&console, &(format!("3. guarded: {}", *guarded.u1())));
        }
        Union2::U2(_) => {
            println(&console, &(format!("3. guarded: rejected — {}", guarded.u2().clone())));
        }
    }
    let mut mixed = 'try_3: {
        Union2::<i32, Union2<String, i32>>::U1(match strict_port(&("".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_3 Union2::<i32, Union2<String, i32>>::U2(__m) })
    };
    match mixed {
        Union2::U1(_) => {
            println(&console, &(format!("3. mixed: {}", *mixed.u1())));
        }
        Union2::U2(_) => {
            let mut why: Union2<String, i32> = mixed.u2().clone();
            match why {
                Union2::U1(_) => {
                    println(&console, &(format!("3. mixed: message {}", why.u1().clone())));
                }
                Union2::U2(_) => {
                    println(&console, &(format!("3. mixed: length {}", *why.u2())));
                }
            }
        }
    }
    let mut outer = 'try_4: {
        let mut inner = 'try_5: {
            Union2::<i32, String>::U1(match parse_port(&("nope".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_5 Union2::<i32, String>::U2(__m) })
        };
        Union2::<i32, String>::U1(match inner {
            Union2::U1(_) => {
                let mut got: i32 = *inner.u1();
                got
            }
            Union2::U2(_) => {
                println(&console, &(format!("3. inner caught: {}", inner.u2().clone())));
                match parse_port(&("also nope".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_4 Union2::<i32, String>::U2(__m) }
            }
        })
    };
    match outer {
        Union2::U1(_) => {
            println(&console, &(format!("3. outer: {}", *outer.u1())));
        }
        Union2::U2(_) => {
            println(&console, &(format!("3. outer caught: {}", outer.u2().clone())));
        }
    }
}
