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

use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_console::*;
use crate::core_fs::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub id: i32,
    pub seat: String,
}

pub fn issue(console: &mut dyn Console, id: i32, seat: String) -> Ticket {
    println(console, &(format!("1. issued #{} for {}", id, seat)));
    return Ticket { id: id, seat: seat };
}

pub fn redeem(console: &mut dyn Console, ticket: Ticket) {
    println(console, &(format!("1. redeemed #{}", ticket.id)));
    drop(ticket);
}

pub fn one_use(console: &mut dyn Console) {
    let mut ticket = issue(console, 1, "12A".to_string());
    redeem(console, ticket);
    println(console, &("2. gone after one use".to_string()));
}

pub fn describe(console: &mut dyn Console, ticket: &Ticket) {
    println(console, &(format!("3. still holding #{} ({})", ticket.id, ticket.seat.clone())));
}

pub fn borrow_then_use(console: &mut dyn Console) {
    let mut ticket = issue(console, 2, "3C".to_string());
    describe(console, &ticket);
    describe(console, &ticket);
    redeem(console, ticket);
}

pub fn read_a_field(console: &mut dyn Console) {
    let mut ticket = issue(console, 3, "1A".to_string());
    let mut seat = &ticket.seat;
    println(console, &(format!("4. read {}, and #{} is still owed", seat.clone(), ticket.id)));
    redeem(console, ticket);
}

pub fn hand_over<T: Clone>(console: &mut dyn Console, value: T, to: impl FnOnce(&mut dyn Console, T)) {
    to(console, value);
}

pub fn generic_handoff(console: &mut dyn Console) {
    let mut ticket = issue(console, 4, "9B".to_string());
    hand_over(console, ticket, |console2: &mut dyn Console, t| redeem(console2, t));
}

pub fn scrap(ticket: Ticket) {
    drop(ticket);
}

pub fn a_queue_of_tickets(console: &mut dyn Console) {
    let mut queue: Vec<Ticket> = vec![];
    queue.push(issue(console, 5, "2B".to_string()));
    queue.push(issue(console, 6, "2C".to_string()));
    println(console, &(format!("6. queued {}", (queue.len() as i32))));
    let mut first = queue.salvo_remove_first();
    match first {
        Some(_) => {
            redeem(console, first.unwrap());
        }
        None => {
        }
    }
    loop {
        let mut __is1 = queue.salvo_remove_first();
        if !(__is1.is_some()) {
            break;
        }
        let mut next = __is1.unwrap();
        redeem(console, next);
    }
    queue.into_iter().for_each(|mut __a0| scrap(__a0));
    println(console, &("6. queue drained".to_string()));
}

pub fn main() {
    let mut console = StdOutConsole::new();
    one_use(&mut console);
    borrow_then_use(&mut console);
    read_a_field(&mut console);
    generic_handoff(&mut console);
    a_queue_of_tickets(&mut console);
}
