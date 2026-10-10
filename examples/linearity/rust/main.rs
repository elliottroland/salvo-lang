#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
#[path = "unions/mod.rs"]
pub mod unions;
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
#[path = "core/deque.rs"]
pub mod core_deque;
#[path = "core/index.rs"]
pub mod core_index;
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
#[path = "platform/core/deque.rs"]
pub mod platform_core_deque;
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
use crate::core_deque::add_last_platform;
use crate::core_deque::drain;
use crate::core_deque::mut_deque_of;
use crate::core_console::println;
use crate::core_deque::remove_first_platform;
use crate::core_deque::size_platform;


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

pub fn issue(console: &crate::core_console::Console, mut id: i32, mut seat: String) -> crate::Ticket {
    crate::core_console::println(console, &format!("1. issued #{} for {}", id, seat));
    return crate::Ticket { id: id, seat: seat.clone() };
}

pub fn redeem(console: &crate::core_console::Console, mut ticket: crate::Ticket) {
    crate::core_console::println(console, &format!("1. redeemed #{}", ticket.id));
    std::mem::drop(ticket);
}

pub fn one_use(console: &crate::core_console::Console) {
    let mut ticket: crate::Ticket = crate::issue(console, 1i32, String::from("12A"));
    crate::redeem(console, ticket);
    crate::core_console::println(console, &String::from("2. gone after one use"));
}

pub fn describe(console: &crate::core_console::Console, ticket: &crate::Ticket) {
    crate::core_console::println(console, &format!("3. still holding #{} ({})", ticket.id, ticket.seat));
}

pub fn borrow_then_use(console: &crate::core_console::Console) {
    let mut ticket: crate::Ticket = crate::issue(console, 2i32, String::from("3C"));
    crate::describe(console, &ticket);
    crate::describe(console, &ticket);
    crate::redeem(console, ticket);
}

pub fn read_a_field(console: &crate::core_console::Console) {
    let mut ticket: crate::Ticket = crate::issue(console, 3i32, String::from("1A"));
    let mut seat = &ticket.seat;
    crate::core_console::println(console, &format!("4. read {}, and #{} is still owed", seat, ticket.id));
    crate::redeem(console, ticket);
}

pub fn hand_over<T>(console: &crate::core_console::Console, mut value: T, to: impl FnOnce(&crate::core_console::Console, T)) {
    to(console, value);
}

pub fn generic_handoff(console: &crate::core_console::Console) {
    let mut ticket: crate::Ticket = crate::issue(console, 4i32, String::from("9B"));
    crate::hand_over(console, ticket, move |mut __leff0: &crate::core_console::Console, mut t: crate::Ticket| {
        crate::redeem(__leff0, t)
    });
}

pub fn scrap(mut ticket: crate::Ticket) {
    std::mem::drop(ticket);
}

pub fn a_queue_of_tickets(console: &crate::core_console::Console) {
    let mut queue: std::collections::VecDeque<crate::Ticket> = crate::core_deque::mut_deque_of::<crate::Ticket>();
    crate::core_deque::add_last_platform::<crate::Ticket>(&mut queue, crate::issue(console, 5i32, String::from("2B")));
    crate::core_deque::add_last_platform::<crate::Ticket>(&mut queue, crate::issue(console, 6i32, String::from("2C")));
    crate::core_console::println(console, &format!("6. queued {}", crate::core_deque::size_platform::<crate::Ticket>(&queue)));
    let mut first: Option<crate::Ticket> = crate::core_deque::remove_first_platform::<crate::Ticket>(&mut queue);
    if first.is_some() {
        let mut first_1 = first.unwrap();
        crate::redeem(console, first_1);
    } else {
    };
    loop {
        let mut __subject_2: Option<crate::Ticket> = crate::core_deque::remove_first_platform::<crate::Ticket>(&mut queue);
        if !(__subject_2.is_some()) {
            break;
        };
        let mut next = __subject_2.unwrap();
        crate::redeem(console, next);
    }
    crate::core_deque::drain::<crate::Ticket>(queue, &mut |__a0: crate::Ticket| crate::scrap(__a0));
    crate::core_console::println(console, &String::from("6. queue drained"));
}

pub fn main() {
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    crate::one_use(&__handle_2);
    crate::borrow_then_use(&__handle_2);
    crate::read_a_field(&__handle_2);
    crate::generic_handoff(&__handle_2);
    crate::a_queue_of_tickets(&__handle_2);
}
