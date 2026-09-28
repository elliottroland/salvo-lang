#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/actor.rs"]
pub mod core_actor;
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
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;

use crate::core_actor::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;

pub trait __Stateless_Counter: Send + Sync {
    fn bump(&self, n: i32);
    fn total(&self, out: crate::scheduler::SalvoReply);
}

pub trait __Stateful_Counter: Send {
    fn bump(&mut self, n: i32);
    fn total(&mut self, out: crate::scheduler::SalvoReply);
}

pub struct __Stub_Counter {
    addr: usize,
}

impl __Stub_Counter {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Counter for __Stub_Counter {
    fn bump(&self, n: i32) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Counter::Bump(n), crate::__PROTO_Counter);
    }
    fn total(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Counter::Total(out), crate::__PROTO_Counter);
    }
}

pub struct Counter {
    inner: __Inner_Counter,
}

pub enum __Inner_Counter {
    Shared(std::sync::Arc<dyn __Stateless_Counter>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Counter>>),
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Counter::Shared(h) => __Inner_Counter::Shared(h.clone()),
            __Inner_Counter::Locked(h) => __Inner_Counter::Locked(h.clone()),
        } }
    }
}

impl Counter {
    pub fn shared<__H: __Stateless_Counter + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Counter::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Counter>) -> Self {
        Self { inner: __Inner_Counter::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Counter + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Counter::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Counter>>) -> Self {
        Self { inner: __Inner_Counter::Locked(inner) }
    }
    pub fn bump(&self, n: i32) {
        match &self.inner {
            __Inner_Counter::Shared(h) => h.bump(n),
            __Inner_Counter::Locked(h) => h.lock().unwrap().bump(n),
        }
    }
    pub fn total(&self, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Counter::Shared(h) => h.total(out),
            __Inner_Counter::Locked(h) => h.lock().unwrap().total(out),
        }
    }
}

pub enum __Msg_Counter {
    Bump(i32),
    Total(crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Counter {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Counter::Bump(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_Counter::Total(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Counter::Bump(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_Counter::Total(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Counter`.
pub const __PROTO_Counter: &str = "f39d50f9ee8f9923";

pub struct Counting {
    sum: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Counting>,
}

impl Counting {
    pub fn new() -> Self {
        Self {
            sum: 0,
            __mailbox_capacity: 8,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::__Stateful_Counter for Counting {

    fn bump(&mut self, n: i32) {
        self.sum = self.sum + n;
    }

    fn total(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<i32>(out, self.sum);
    }
}

pub enum __Cont_Counting {
    Bump,
    Total,
}

pub struct __Actor_Counting {
    handler: Counting,
}

impl __Actor_Counting {
    pub fn new(handler: Counting) -> Self {
        Self { handler }
    }
}

impl __Actor_Counting {
    fn __dispatch(&mut self, msg: crate::__Msg_Counter) {
        match msg {
            crate::__Msg_Counter::Bump(n) => crate::__Stateful_Counter::bump(&mut self.handler, n),
            crate::__Msg_Counter::Total(out) => crate::__Stateful_Counter::total(&mut self.handler, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Counting {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Counter>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Counting::Bump => self.__dispatch(crate::__Msg_Counter::Bump(*value.downcast::<i32>().expect("the awaited answer"))),
            __Cont_Counting::Total => self.__dispatch(crate::__Msg_Counter::Total(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Counting::Bump{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Counting::Total{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Counting: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Counting);
fn __decode_msg_Counting(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Counter {
            return crate::wire::salvo_decode::<crate::__Msg_Counter>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub trait __Stateless_Ledger: Send + Sync {
    fn report(&self, label: String, out: crate::scheduler::SalvoReply);
    fn reported(&self, label: String, out: crate::scheduler::SalvoReply, total: i32);
}

pub trait __Stateful_Ledger: Send {
    fn report(&mut self, label: String, out: crate::scheduler::SalvoReply);
    fn reported(&mut self, label: String, out: crate::scheduler::SalvoReply, total: i32);
}

pub struct __Stub_Ledger {
    addr: usize,
}

impl __Stub_Ledger {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Ledger for __Stub_Ledger {
    fn report(&self, label: String, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Ledger::Report(label, out), crate::__PROTO_Ledger);
    }
    fn reported(&self, label: String, out: crate::scheduler::SalvoReply, total: i32) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Ledger::Reported(label, out, total), crate::__PROTO_Ledger);
    }
}

pub struct Ledger {
    inner: __Inner_Ledger,
}

pub enum __Inner_Ledger {
    Shared(std::sync::Arc<dyn __Stateless_Ledger>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Ledger>>),
}

impl Clone for Ledger {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Ledger::Shared(h) => __Inner_Ledger::Shared(h.clone()),
            __Inner_Ledger::Locked(h) => __Inner_Ledger::Locked(h.clone()),
        } }
    }
}

impl Ledger {
    pub fn shared<__H: __Stateless_Ledger + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Ledger::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Ledger>) -> Self {
        Self { inner: __Inner_Ledger::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Ledger + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Ledger::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Ledger>>) -> Self {
        Self { inner: __Inner_Ledger::Locked(inner) }
    }
    pub fn report(&self, label: String, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Ledger::Shared(h) => h.report(label, out),
            __Inner_Ledger::Locked(h) => h.lock().unwrap().report(label, out),
        }
    }
    pub fn reported(&self, label: String, out: crate::scheduler::SalvoReply, total: i32) {
        match &self.inner {
            __Inner_Ledger::Shared(h) => h.reported(label, out, total),
            __Inner_Ledger::Locked(h) => h.lock().unwrap().reported(label, out, total),
        }
    }
}

pub enum __Msg_Ledger {
    Report(String, crate::scheduler::SalvoReply),
    Reported(String, crate::scheduler::SalvoReply, i32),
}

impl crate::wire::__Wire for __Msg_Ledger {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Ledger::Report(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
            __Msg_Ledger::Reported(__p0, __p1, __p2) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Ledger::Report(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_Ledger::Reported(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Ledger`.
pub const __PROTO_Ledger: &str = "4a99401c8b66babf";

pub struct Bookkeeping {
    __dep_Counter: crate::Counter,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Bookkeeping>,
}

impl Bookkeeping {
    pub fn new(__dep_Counter: crate::Counter) -> Self {
        Self {
            __dep_Counter,
            __mailbox_capacity: 4,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::__Stateful_Ledger for Bookkeeping {

    fn report(&mut self, label: String, out: crate::scheduler::SalvoReply) {
        self.__dep_Counter.total({ let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Bookkeeping::Reported(label, out)); __r });
    }

    fn reported(&mut self, label: String, out: crate::scheduler::SalvoReply, total: i32) {
        crate::scheduler::salvo_reply_wire::<String>(out, format!("{}={}", label, total));
    }
}

pub enum __Cont_Bookkeeping {
    Report(String),
    Reported(String, crate::scheduler::SalvoReply),
}

pub struct __Actor_Bookkeeping {
    handler: Bookkeeping,
}

impl __Actor_Bookkeeping {
    pub fn new(handler: Bookkeeping) -> Self {
        Self { handler }
    }
}

impl __Actor_Bookkeeping {
    fn __dispatch(&mut self, msg: crate::__Msg_Ledger) {
        match msg {
            crate::__Msg_Ledger::Report(label, out) => crate::__Stateful_Ledger::report(&mut self.handler, label, out),
            crate::__Msg_Ledger::Reported(label, out, total) => crate::__Stateful_Ledger::reported(&mut self.handler, label, out, total),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Bookkeeping {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Ledger>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Bookkeeping::Report(label) => self.__dispatch(crate::__Msg_Ledger::Report(label, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Bookkeeping::Reported(label, out) => self.__dispatch(crate::__Msg_Ledger::Reported(label, out, *value.downcast::<i32>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Bookkeeping::Report{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Bookkeeping::Reported{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Bookkeeping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Bookkeeping);
fn __decode_msg_Bookkeeping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Ledger {
            return crate::wire::salvo_decode::<crate::__Msg_Ledger>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub trait __Stateless_Desk: Send + Sync {
    fn ticket(&self, out: crate::scheduler::SalvoReply);
    fn serve(&self, name: String);
    fn close_up(&self, reason: String);
}

pub trait __Stateful_Desk: Send {
    fn ticket(&mut self, out: crate::scheduler::SalvoReply);
    fn serve(&mut self, name: String);
    fn close_up(&mut self, reason: String);
}

pub struct __Stub_Desk {
    addr: usize,
}

impl __Stub_Desk {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Desk for __Stub_Desk {
    fn ticket(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Desk::Ticket(out), crate::__PROTO_Desk);
    }
    fn serve(&self, name: String) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Desk::Serve(name), crate::__PROTO_Desk);
    }
    fn close_up(&self, reason: String) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Desk::CloseUp(reason), crate::__PROTO_Desk);
    }
}

pub struct Desk {
    inner: __Inner_Desk,
}

pub enum __Inner_Desk {
    Shared(std::sync::Arc<dyn __Stateless_Desk>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Desk>>),
}

impl Clone for Desk {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Desk::Shared(h) => __Inner_Desk::Shared(h.clone()),
            __Inner_Desk::Locked(h) => __Inner_Desk::Locked(h.clone()),
        } }
    }
}

impl Desk {
    pub fn shared<__H: __Stateless_Desk + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Desk::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Desk>) -> Self {
        Self { inner: __Inner_Desk::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Desk + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Desk::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Desk>>) -> Self {
        Self { inner: __Inner_Desk::Locked(inner) }
    }
    pub fn ticket(&self, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Desk::Shared(h) => h.ticket(out),
            __Inner_Desk::Locked(h) => h.lock().unwrap().ticket(out),
        }
    }
    pub fn serve(&self, name: String) {
        match &self.inner {
            __Inner_Desk::Shared(h) => h.serve(name),
            __Inner_Desk::Locked(h) => h.lock().unwrap().serve(name),
        }
    }
    pub fn close_up(&self, reason: String) {
        match &self.inner {
            __Inner_Desk::Shared(h) => h.close_up(reason),
            __Inner_Desk::Locked(h) => h.lock().unwrap().close_up(reason),
        }
    }
}

pub enum __Msg_Desk {
    Ticket(crate::scheduler::SalvoReply),
    Serve(String),
    CloseUp(String),
}

impl crate::wire::__Wire for __Msg_Desk {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Desk::Ticket(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_Desk::Serve(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_Desk::CloseUp(__p0) => {
                out.push(2);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Desk::Ticket(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_Desk::Serve(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_Desk::CloseUp(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Desk`.
pub const __PROTO_Desk: &str = "cb180ef2d1bfd753";

pub struct Desking {
    room: i32,
    waiting: Vec<crate::scheduler::SalvoReply>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Desking>,
}

impl Desking {
    pub fn new(room: i32) -> Self {
        Self {
            room,
            waiting: vec![],
            __mailbox_capacity: room,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::__Stateful_Desk for Desking {

    fn ticket(&mut self, out: crate::scheduler::SalvoReply) {
        self.waiting.push(out);
    }

    fn serve(&mut self, name: String) {
        let mut next = self.waiting.salvo_remove_first();
        match next {
            Some(_) => {
                crate::scheduler::salvo_reply_wire::<String>(next.unwrap(), format!("served {}", name));
            }
            None => {
                drop(name);
            }
        }
    }

    fn close_up(&mut self, reason: String) {
        std::mem::take(&mut self.waiting).into_iter().for_each(|r| crate::scheduler::salvo_reply_wire::<String>(r, format!("closed: {}", reason)));
        self.waiting = vec![];
    }
}

pub enum __Cont_Desking {
    Ticket,
    Serve,
    CloseUp,
}

pub struct __Actor_Desking {
    handler: Desking,
}

impl __Actor_Desking {
    pub fn new(handler: Desking) -> Self {
        Self { handler }
    }
}

impl __Actor_Desking {
    fn __dispatch(&mut self, msg: crate::__Msg_Desk) {
        match msg {
            crate::__Msg_Desk::Ticket(out) => crate::__Stateful_Desk::ticket(&mut self.handler, out),
            crate::__Msg_Desk::Serve(name) => crate::__Stateful_Desk::serve(&mut self.handler, name),
            crate::__Msg_Desk::CloseUp(reason) => crate::__Stateful_Desk::close_up(&mut self.handler, reason),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Desking {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Desk>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Desking::Ticket => self.__dispatch(crate::__Msg_Desk::Ticket(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Desking::Serve => self.__dispatch(crate::__Msg_Desk::Serve(*value.downcast::<String>().expect("the awaited answer"))),
            __Cont_Desking::CloseUp => self.__dispatch(crate::__Msg_Desk::CloseUp(*value.downcast::<String>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Desking::Ticket{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Desking::Serve{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Desking::CloseUp{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Desking: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Desking);
fn __decode_msg_Desking(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Desk {
            return crate::wire::salvo_decode::<crate::__Msg_Desk>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub trait __Stateless_Fragile: Send + Sync {
    fn crash(&self);
}

pub trait __Stateful_Fragile: Send {
    fn crash(&mut self);
}

pub struct __Stub_Fragile {
    addr: usize,
}

impl __Stub_Fragile {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Fragile for __Stub_Fragile {
    fn crash(&self) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Fragile::Crash, crate::__PROTO_Fragile);
    }
}

pub struct Fragile {
    inner: __Inner_Fragile,
}

pub enum __Inner_Fragile {
    Shared(std::sync::Arc<dyn __Stateless_Fragile>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Fragile>>),
}

impl Clone for Fragile {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Fragile::Shared(h) => __Inner_Fragile::Shared(h.clone()),
            __Inner_Fragile::Locked(h) => __Inner_Fragile::Locked(h.clone()),
        } }
    }
}

impl Fragile {
    pub fn shared<__H: __Stateless_Fragile + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Fragile::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Fragile>) -> Self {
        Self { inner: __Inner_Fragile::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Fragile + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Fragile::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Fragile>>) -> Self {
        Self { inner: __Inner_Fragile::Locked(inner) }
    }
    pub fn crash(&self) {
        match &self.inner {
            __Inner_Fragile::Shared(h) => h.crash(),
            __Inner_Fragile::Locked(h) => h.lock().unwrap().crash(),
        }
    }
}

pub enum __Msg_Fragile {
    Crash,
}

impl crate::wire::__Wire for __Msg_Fragile {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Fragile::Crash => out.push(0),
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Fragile::Crash),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Fragile`.
pub const __PROTO_Fragile: &str = "a8c912bc262644a0";

pub struct Breaking {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
}

impl Breaking {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 1,
            __addr: None,
        }
    }
}

impl crate::__Stateless_Fragile for Breaking {

    fn crash(&self) {
        let mut empty: Vec<i32> = vec![];
        let mut boom = *empty.get((7) as i64 as usize).expect("salvo: value is absent at main:145:20");
        drop(boom);
    }
}

pub struct __Actor_Breaking {
    handler: Breaking,
}

impl __Actor_Breaking {
    pub fn new(handler: Breaking) -> Self {
        Self { handler }
    }
}

impl __Actor_Breaking {
    fn __dispatch(&mut self, msg: crate::__Msg_Fragile) {
        match msg {
            crate::__Msg_Fragile::Crash => crate::__Stateless_Fragile::crash(&mut self.handler),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Breaking {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Fragile>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, _slot: u64, _value: crate::scheduler::SalvoMsg) {
        unreachable!("this protocol has no continuation targets")
    }
}

pub const __DECODE_Breaking: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Breaking);
fn __decode_msg_Breaking(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Fragile {
            return crate::wire::salvo_decode::<crate::__Msg_Fragile>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn formatted(label: String, out: crate::scheduler::SalvoReply, total: i32) {
    crate::scheduler::salvo_reply_wire::<String>(out, format!("{} totalled {}", label, total));
}

pub fn report_line(counter: usize, label: String, out: crate::scheduler::SalvoReply) {
    crate::scheduler::salvo_send_wire(counter, crate::__Msg_Counter::Total(({ let __c0 = label; let __c1 = out; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), Box::new(move |__v| formatted(__c0, __c1, *__v.downcast::<i32>().expect("the awaited answer"))), (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))) })), crate::__PROTO_Counter);
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Counter".to_string(), crate::__PROTO_Counter.to_string()), ("Desk".to_string(), crate::__PROTO_Desk.to_string()), ("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Fragile".to_string(), crate::__PROTO_Fragile.to_string()), ("Ledger".to_string(), crate::__PROTO_Ledger.to_string())]);
    let console = crate::core_console::Console::shared(StdOutConsole::new());
    let mut workers = crate::scheduler::salvo_pool(((2) as usize));
    let mut counter = ({ let __h = Counting::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Counting::new(__h)), __DECODE_Counting); __a });
    crate::scheduler::salvo_send_wire(counter, crate::__Msg_Counter::Bump(2), crate::__PROTO_Counter);
    crate::scheduler::salvo_send_wire(counter, crate::__Msg_Counter::Bump(3), crate::__PROTO_Counter);
    let mut sum = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(counter, crate::__Msg_Counter::Total(out), crate::__PROTO_Counter);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
    println(&console, &(format!("1. counter total is {}", sum)));
    let mut ledger = ({ let __h = Bookkeeping::new(crate::Counter::shared(__Stub_Counter::new(counter))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Bookkeeping::new(__h)), __DECODE_Bookkeeping); __a });
    let mut line = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(ledger, crate::__Msg_Ledger::Report("counter".to_string(), out), crate::__PROTO_Ledger);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&console, &(format!("3. ledger says {}", line)));
    let mut desk = ({ let __h = Desking::new(8); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Desking::new(__h)), __DECODE_Desking); __a });
    let mut first = {
        let (mut a, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(desk, crate::__Msg_Desk::Ticket(a), crate::__PROTO_Desk);
        let mut second = {
            let (mut b, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(desk, crate::__Msg_Desk::Ticket(b), crate::__PROTO_Desk);
            crate::scheduler::salvo_send_wire(desk, crate::__Msg_Desk::Serve("ada".to_string()), crate::__PROTO_Desk);
            crate::scheduler::salvo_send_wire(desk, crate::__Msg_Desk::CloseUp("end of day".to_string()), crate::__PROTO_Desk);
            *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
        };
        println(&console, &(format!("4. second waiter got: {}", second)));
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&console, &(format!("4. first waiter got: {}", first)));
    let mut fragile = ({ let __h = Breaking::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Breaking::new(__h)), __DECODE_Breaking); __a });
    let mut exit = {
        let (mut gone, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Exit>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_watch(fragile, gone, |__reason| Box::new(Exit { reason: __reason }));
        crate::scheduler::salvo_send_wire(fragile, crate::__Msg_Fragile::Crash, crate::__PROTO_Fragile);
        *crate::scheduler::salvo_wait(__wid).downcast::<Exit>().expect("the awaited answer")
    };
    println(&console, &(format!("5. it died with a reason: {}", (exit.reason.chars().count() as i32) > 0)));
    crate::scheduler::salvo_send_wire(fragile, crate::__Msg_Fragile::Crash, crate::__PROTO_Fragile);
    let counter2 = crate::Counter::locked(Counting::new());
    counter2.bump(4);
    counter2.bump(5);
    let mut inline = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        counter2.total(out);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
    println(&console, &(format!("6. inline total is {}", inline)));
    let mut mine = ({ let __h = Counting::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_current_pool(), __cap as usize, Box::new(__Actor_Counting::new(__h)), __DECODE_Counting); __a });
    crate::scheduler::salvo_send_wire(mine, crate::__Msg_Counter::Bump(6), crate::__PROTO_Counter);
    let mut local = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(mine, crate::__Msg_Counter::Total(out), crate::__PROTO_Counter);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
    println(&console, &(format!("7. the main pool's own actor totalled {}", local)));
    let mut line8 = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        report_line(mine, "the counter".to_string(), out);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&console, &(format!("8. {}", line8)));
    println(&console, &("done".to_string()));
}
