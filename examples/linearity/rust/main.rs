#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
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
#[path = "core/deque.rs"]
pub mod core_deque;
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
#[path = "platform/core/string.rs"]
pub mod platform_core_string;

use crate::core_console::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub id: i32,
    pub seat: String,
}

impl crate::wire::__Wire for Ticket {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.id, out);
        crate::wire::__Wire::__enc(&self.seat, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            id: crate::wire::__Wire::__dec(r)?,
            seat: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn issue(console: &crate::core_console::Console, id: i32, seat: String) -> Ticket {
    println(console, &(format!("1. issued #{} for {}", id, seat)));
    return Ticket { id: id, seat: seat };
}

pub fn redeem(console: &crate::core_console::Console, ticket: Ticket) {
    println(console, &(format!("1. redeemed #{}", ticket.id)));
    drop(ticket);
}

pub fn one_use(console: &crate::core_console::Console) {
    let mut ticket = issue(console, 1, "12A".to_string());
    redeem(console, ticket);
    println(console, &("2. gone after one use".to_string()));
}

pub fn describe(console: &crate::core_console::Console, ticket: &Ticket) {
    println(console, &(format!("3. still holding #{} ({})", ticket.id, ticket.seat.clone())));
}

pub fn borrow_then_use(console: &crate::core_console::Console) {
    let mut ticket = issue(console, 2, "3C".to_string());
    describe(console, &ticket);
    describe(console, &ticket);
    redeem(console, ticket);
}

pub fn read_a_field(console: &crate::core_console::Console) {
    let mut ticket = issue(console, 3, "1A".to_string());
    let mut seat = &ticket.seat;
    println(console, &(format!("4. read {}, and #{} is still owed", seat.clone(), ticket.id)));
    redeem(console, ticket);
}

pub fn hand_over<T: Clone>(console: &crate::core_console::Console, value: T, to: impl FnOnce(&crate::core_console::Console, T)) {
    to(console, value);
}

pub fn generic_handoff(console: &crate::core_console::Console) {
    let mut ticket = issue(console, 4, "9B".to_string());
    hand_over(console, ticket, |console2: &crate::core_console::Console, t| redeem(console2, t));
}

pub fn scrap(ticket: Ticket) {
    drop(ticket);
}

pub fn a_queue_of_tickets(console: &crate::core_console::Console) {
    let mut queue: std::collections::VecDeque<Ticket> = std::collections::VecDeque::<Ticket>::new();
    queue.push_back(issue(console, 5, "2B".to_string()));
    queue.push_back(issue(console, 6, "2C".to_string()));
    println(console, &(format!("6. queued {}", (queue.len() as i32))));
    let mut first = queue.pop_front();
    match first {
        Some(_) => {
            redeem(console, first.unwrap());
        }
        None => {
        }
    }
    loop {
        let mut __is1 = queue.pop_front();
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
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    one_use(&console);
    borrow_then_use(&console);
    read_a_field(&console);
    generic_handoff(&console);
    a_queue_of_tickets(&console);
}
