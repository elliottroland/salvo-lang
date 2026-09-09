#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/string.rs"]
pub mod core_string;

use crate::core_array::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_string::*;

pub fn NonEmpty_qualifies<T: Clone + 'static>(list: &Vec<T>) -> bool {
    return (list.len() as i32) > 0;
}

pub fn head(list: &Vec<i32>) -> i32 {
    let mut first = list.get((0) as usize).cloned();
    return first.unwrap();
}

pub fn celsius(degrees: i32) -> i32 {
    return degrees;
}

pub fn describe(temp: i32) -> String {
    return format!("{} (no unit)", temp);
}

pub fn describe__Celsius(temp: &i32) -> String {
    return format!("{}°C", *temp);
}

pub fn sum(list: &Vec<i32>) -> i32 {
    let mut total = 0;
    for n in list {
        total = total + *n;
    }
    return total;
}

pub fn compact(list: &mut Vec<i32>) {
}

#[derive(Clone, Debug)]
pub struct Request {
    pub path: String,
    pub touches: i32,
}

pub fn authenticate(mut request: Request) -> Request {
    return request;
}

pub fn freshen(mut request: Request) -> Request {
    return request;
}

pub fn touch(request: &mut Request) {
    request.touches = request.touches + 1;
}

pub fn handle(request: &Request) -> String {
    return format!("plain {}", request.path.clone());
}

pub fn handle__Authenticated(request: &Request) -> String {
    return format!("authenticated {}", request.path.clone());
}

pub fn handle__Fresh(request: &Request) -> String {
    return format!("fresh {}", request.path.clone());
}

pub fn main() {
    let mut console = StdOutConsole::new();
    let mut xs: Vec<i32> = vec![];
    xs.push(3);
    println(&mut console, &(format!("1. head after add: {}", head(&xs))));
    let mut maybe_empty = vec![7, 8];
    if NonEmpty_qualifies(&maybe_empty) {
        println(&mut console, &(format!("2. checked at run time, head is {}", head(&maybe_empty))));
    }
    let mut plain = 21;
    let mut warm = celsius(21);
    println(&mut console, &(format!("2. {} vs {}", describe(plain), describe__Celsius(&warm))));
    if true {
        println(&mut console, &(format!("2. widened: {}", describe(warm))));
    }
    println(&mut console, &(format!("3. sum {}, head still {}", sum(&xs), head(&xs))));
    compact(&mut xs);
    xs.push(9);
    println(&mut console, &(format!("3. after compact and add, head is {}", head(&xs))));
    let mut session = authenticate(Request { path: "/orders".to_string(), touches: 0 });
    let mut fresh = freshen(Request { path: "/health".to_string(), touches: 0 });
    println(&mut console, &(format!("4. before: {} / {}", handle__Authenticated(&session), handle__Fresh(&fresh))));
    touch(&mut session);
    touch(&mut fresh);
    println(&mut console, &(format!("4. after:  {} / {}", handle__Authenticated(&session), handle(&fresh))));
}
