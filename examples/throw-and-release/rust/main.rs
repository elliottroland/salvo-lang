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
#[path = "core/throw.rs"]
pub mod core_throw;

use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_console::*;
use crate::core_fs::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::core_throw::*;
use crate::unions::*;
use std::ops::ControlFlow;

#[derive(Clone, Debug, PartialEq)]
pub struct FileHandle {
    pub name: String,
}

pub fn open_file(console: &mut dyn Console, name: String) -> FileHandle {
    println(console, &(format!("1. open {}", name)));
    return FileHandle { name: name };
}

pub fn close__3(console: &mut dyn Console, handle: FileHandle) {
    println(console, &(format!("1. close {}", handle.name.clone())));
    drop(handle);
}

pub fn read_size(console: &mut dyn Console, name: String, want: i32) -> i32 {
    let mut there_is = (name.chars().count() as i32);
    let mut handle = open_file(console, name);
    if want > there_is {
        println(console, &("1. asked for more than there is".to_string()));
        close__3(console, handle);
        return there_is;
    }
    close__3(console, handle);
    return want;
}

pub fn parse_port(text: &String) -> ControlFlow<String, i32> {
    let mut n = text.parse::<i32>().ok();
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

pub fn port_from_file(console: &mut dyn Console, name: String, text: &String) -> ControlFlow<String, i32> {
    let mut handle = open_file(console, name);
    let mut from = handle.name.clone();
    close__3(console, handle);
    println(console, &(format!("2. reading a port out of {}", from)));
    return ControlFlow::Continue(parse_port(text)?);
}

pub fn strict_port(text: &String) -> ControlFlow<Union2<String, i32>, i32> {
    if ((text.chars().count() as i32) == 0) {
        return ControlFlow::Break(Union2::<String, i32>::U1("empty".to_string()));
    }
    let mut n = text.parse::<i32>().ok();
    if n.is_none() {
        return ControlFlow::Break(Union2::<String, i32>::U2((text.chars().count() as i32)));
    }
    return ControlFlow::Continue(n.unwrap());
}

pub fn report(console: &mut dyn Console, label: &String, config: &String) {
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
    let mut console = StdOutConsole::new();
    let mut small = read_size(&mut console, "notes.txt".to_string(), 3);
    println(&mut console, &(format!("1. read {}", small)));
    let mut clamped = read_size(&mut console, "notes.txt".to_string(), 99);
    println(&mut console, &(format!("1. read {}", clamped)));
    report(&mut console, &("good".to_string()), &("8080".to_string()));
    report(&mut console, &("bad".to_string()), &("http".to_string()));
    let mut guarded = 'try_2: {
        Union2::<i32, String>::U1(match port_from_file(&mut console, "ports.txt".to_string(), &("-1".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_2 Union2::<i32, String>::U2(__m) })
    };
    match guarded {
        Union2::U1(_) => {
            println(&mut console, &(format!("3. guarded: {}", *guarded.u1())));
        }
        Union2::U2(_) => {
            println(&mut console, &(format!("3. guarded: rejected — {}", guarded.u2().clone())));
        }
    }
    let mut mixed = 'try_3: {
        Union2::<i32, Union2<String, i32>>::U1(match strict_port(&("".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_3 Union2::<i32, Union2<String, i32>>::U2(__m) })
    };
    match mixed {
        Union2::U1(_) => {
            println(&mut console, &(format!("3. mixed: {}", *mixed.u1())));
        }
        Union2::U2(_) => {
            let mut why: Union2<String, i32> = mixed.u2().clone();
            match why {
                Union2::U1(_) => {
                    println(&mut console, &(format!("3. mixed: message {}", why.u1().clone())));
                }
                Union2::U2(_) => {
                    println(&mut console, &(format!("3. mixed: length {}", *why.u2())));
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
                println(&mut console, &(format!("3. inner caught: {}", inner.u2().clone())));
                match parse_port(&("also nope".to_string())) { ControlFlow::Continue(__v) => __v, ControlFlow::Break(__m) => break 'try_4 Union2::<i32, String>::U2(__m) }
            }
        })
    };
    match outer {
        Union2::U1(_) => {
            println(&mut console, &(format!("3. outer: {}", *outer.u1())));
        }
        Union2::U2(_) => {
            println(&mut console, &(format!("3. outer caught: {}", outer.u2().clone())));
        }
    }
}
