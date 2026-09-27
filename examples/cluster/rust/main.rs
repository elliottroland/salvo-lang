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
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/bytes.rs"]
pub mod core_bytes;
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
#[path = "core/range.rs"]
pub mod core_range;
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "net.rs"]
pub mod net;
#[path = "time.rs"]
pub mod time;
#[path = "platform/net.rs"]
pub mod platform_net;

use crate::core_actor::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_range::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::net::*;
use crate::seq::*;
use crate::time::*;

pub trait Sequencer {
    fn next(&mut self, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Sequencer {
    fn __get_Sequencer(&mut self) -> &mut dyn Sequencer;
}

pub struct __Stub_Sequencer {
    addr: usize,
}

impl __Stub_Sequencer {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Sequencer for __Stub_Sequencer {
    fn next(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Sequencer::Next(out), crate::__PROTO_Sequencer);
    }
}

pub struct __Lock_Sequencer<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Sequencer<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Sequencer<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Sequencer + Send> Sequencer for __Lock_Sequencer<H> {
    fn next(&mut self, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().next(out)
    }
}

pub enum __Msg_Sequencer {
    Next(crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Sequencer {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Sequencer::Next(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Sequencer::Next(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Sequencer`.
pub const __PROTO_Sequencer: &str = "7a5334482e5247f7";

pub trait Inventory {
    fn reserve(&mut self, sku: String, qty: i32, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Inventory {
    fn __get_Inventory(&mut self) -> &mut dyn Inventory;
}

pub struct __Stub_Inventory {
    addr: usize,
}

impl __Stub_Inventory {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Inventory for __Stub_Inventory {
    fn reserve(&mut self, sku: String, qty: i32, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Inventory::Reserve(sku, qty, out), crate::__PROTO_Inventory);
    }
}

pub struct __Lock_Inventory<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Inventory<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Inventory<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Inventory + Send> Inventory for __Lock_Inventory<H> {
    fn reserve(&mut self, sku: String, qty: i32, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().reserve(sku, qty, out)
    }
}

pub enum __Msg_Inventory {
    Reserve(String, i32, crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Inventory {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Inventory::Reserve(__p0, __p1, __p2) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Inventory::Reserve(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Inventory`.
pub const __PROTO_Inventory: &str = "d3482a697a944808";

pub trait Search {
    fn query(&mut self, word: String, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Search {
    fn __get_Search(&mut self) -> &mut dyn Search;
}

pub struct __Stub_Search {
    addr: usize,
}

impl __Stub_Search {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Search for __Stub_Search {
    fn query(&mut self, word: String, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Search::Query(word, out), crate::__PROTO_Search);
    }
}

pub struct __Lock_Search<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Search<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Search<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Search + Send> Search for __Lock_Search<H> {
    fn query(&mut self, word: String, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().query(word, out)
    }
}

pub enum __Msg_Search {
    Query(String, crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Search {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Search::Query(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Search::Query(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Search`.
pub const __PROTO_Search: &str = "ed817fc30774f01d";

pub trait Lookup {
    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Lookup {
    fn __get_Lookup(&mut self) -> &mut dyn Lookup;
}

pub struct __Stub_Lookup {
    addr: usize,
}

impl __Stub_Lookup {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Lookup for __Stub_Lookup {
    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Lookup::Lookup(key, out), crate::__PROTO_Lookup);
    }
}

pub struct __Lock_Lookup<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Lookup<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Lookup<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Lookup + Send> Lookup for __Lock_Lookup<H> {
    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().lookup(key, out)
    }
}

pub enum __Msg_Lookup {
    Lookup(String, crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Lookup {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Lookup::Lookup(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Lookup::Lookup(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Lookup`.
pub const __PROTO_Lookup: &str = "7c0f441570dc9a6f";

pub struct Sequencing {
    who: String,
    n: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sequencing>,
}

impl Sequencing {
    pub fn new(who: String) -> Self {
        Self {
            who,
            n: 0,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Sequencer for Sequencing {

    fn next(&mut self, out: crate::scheduler::SalvoReply) {
        self.n = self.n + 1;
        crate::scheduler::salvo_reply_wire::<String>(out, format!("{}#{}", self.who.clone(), self.n));
    }
}

pub enum __Cont_Sequencing {
    Next,
}

pub struct __Actor_Sequencing {
    handler: Sequencing,
}

impl __Actor_Sequencing {
    pub fn new(handler: Sequencing) -> Self {
        Self { handler }
    }
}

impl __Actor_Sequencing {
    fn __dispatch(&mut self, msg: crate::__Msg_Sequencer) {
        match msg {
            crate::__Msg_Sequencer::Next(out) => crate::Sequencer::next(&mut self.handler, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Sequencing {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Sequencer>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Sequencing::Next => self.__dispatch(crate::__Msg_Sequencer::Next(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sequencing::Next{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sequencing: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sequencing);
fn __decode_msg_Sequencing(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Sequencer {
            return crate::wire::salvo_decode::<crate::__Msg_Sequencer>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct Stocking {
    shard: String,
    served: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Stocking>,
}

impl Stocking {
    pub fn new(shard: String) -> Self {
        Self {
            shard,
            served: 0,
            __mailbox_capacity: 32,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Inventory for Stocking {

    fn reserve(&mut self, sku: String, qty: i32, out: crate::scheduler::SalvoReply) {
        self.served = self.served + qty;
        crate::scheduler::salvo_reply_wire::<String>(out, format!("{}:{}", self.shard.clone(), self.served));
    }
}

pub enum __Cont_Stocking {
    Reserve(String, i32),
}

pub struct __Actor_Stocking {
    handler: Stocking,
}

impl __Actor_Stocking {
    pub fn new(handler: Stocking) -> Self {
        Self { handler }
    }
}

impl __Actor_Stocking {
    fn __dispatch(&mut self, msg: crate::__Msg_Inventory) {
        match msg {
            crate::__Msg_Inventory::Reserve(sku, qty, out) => crate::Inventory::reserve(&mut self.handler, sku, qty, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Stocking {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Inventory>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Stocking::Reserve(sku, qty) => self.__dispatch(crate::__Msg_Inventory::Reserve(sku, qty, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Stocking::Reserve{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Stocking: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Stocking);
fn __decode_msg_Stocking(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Inventory {
            return crate::wire::salvo_decode::<crate::__Msg_Inventory>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct Indexing {
    words: Vec<String>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Indexing>,
}

impl Indexing {
    pub fn new(words: Vec<String>) -> Self {
        Self {
            words,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Search for Indexing {

    fn query(&mut self, word: String, out: crate::scheduler::SalvoReply) {
        let mut n = 0;
        for w in &self.words {
            if ((&w[..] == &word[..])) {
                n = n + 1;
            }
        }
        crate::scheduler::salvo_reply_wire::<i32>(out, n);
    }
}

pub enum __Cont_Indexing {
    Query(String),
}

pub struct __Actor_Indexing {
    handler: Indexing,
}

impl __Actor_Indexing {
    pub fn new(handler: Indexing) -> Self {
        Self { handler }
    }
}

impl __Actor_Indexing {
    fn __dispatch(&mut self, msg: crate::__Msg_Search) {
        match msg {
            crate::__Msg_Search::Query(word, out) => crate::Search::query(&mut self.handler, word, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Indexing {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Search>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Indexing::Query(word) => self.__dispatch(crate::__Msg_Search::Query(word, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Indexing::Query{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Indexing: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Indexing);
fn __decode_msg_Indexing(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Search {
            return crate::wire::salvo_decode::<crate::__Msg_Search>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct Looking {
    who: String,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Looking>,
}

impl Looking {
    pub fn new(who: String) -> Self {
        Self {
            who,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Lookup for Looking {

    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<String>(out, format!("{} from {}", key, self.who.clone()));
    }
}

pub enum __Cont_Looking {
    Lookup(String),
}

pub struct __Actor_Looking {
    handler: Looking,
}

impl __Actor_Looking {
    pub fn new(handler: Looking) -> Self {
        Self { handler }
    }
}

impl __Actor_Looking {
    fn __dispatch(&mut self, msg: crate::__Msg_Lookup) {
        match msg {
            crate::__Msg_Lookup::Lookup(key, out) => crate::Lookup::lookup(&mut self.handler, key, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Looking {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Lookup>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Looking::Lookup(key) => self.__dispatch(crate::__Msg_Lookup::Lookup(key, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Looking::Lookup{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Looking: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Looking);
fn __decode_msg_Looking(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Lookup {
            return crate::wire::salvo_decode::<crate::__Msg_Lookup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct SlowLooking {
    who: String,
    timer: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_SlowLooking>,
}

impl SlowLooking {
    pub fn new(who: String, timer: usize) -> Self {
        Self {
            who,
            timer,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Lookup for SlowLooking {

    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.timer.clone(), crate::time::__Msg_Timer::After(millis(150i64), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_SlowLooking::Answer(key, out)); __r }), crate::time::__PROTO_Timer);
    }
}

impl SlowLooking {

    fn answer(&mut self, key: String, out: crate::scheduler::SalvoReply, fired: Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, format!("{} from {}", key, self.who.clone()));
    }
}

pub enum __Cont_SlowLooking {
    Lookup(String),
    Answer(String, crate::scheduler::SalvoReply),
}

pub enum __Priv_SlowLooking {
    Answer(String, crate::scheduler::SalvoReply, Fired),
}

pub struct __Actor_SlowLooking {
    handler: SlowLooking,
}

impl __Actor_SlowLooking {
    pub fn new(handler: SlowLooking) -> Self {
        Self { handler }
    }
}

impl __Actor_SlowLooking {
    fn __dispatch_Lookup(&mut self, msg: crate::__Msg_Lookup) {
        match msg {
            crate::__Msg_Lookup::Lookup(key, out) => crate::Lookup::lookup(&mut self.handler, key, out),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_SlowLooking) {
        match msg {
            __Priv_SlowLooking::Answer(key, out, fired) => self.handler.answer(key, out, fired),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_SlowLooking {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::__Msg_Lookup>() {
            Ok(__m) => return self.__dispatch_Lookup(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_SlowLooking>() {
            Ok(__m) => return self.__dispatch_priv(*__m),
            Err(__m) => __m,
        };
        let _ = msg;
        unreachable!("a message of one of this actor's protocols")
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_SlowLooking::Lookup(key) => self.__dispatch_Lookup(crate::__Msg_Lookup::Lookup(key, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_SlowLooking::Answer(key, out) => self.__dispatch_priv(__Priv_SlowLooking::Answer(key, out, *value.downcast::<Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_SlowLooking::Lookup{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_SlowLooking::Answer{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Fired>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_SlowLooking: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_SlowLooking);
fn __decode_msg_SlowLooking(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Lookup {
            return crate::wire::salvo_decode::<crate::__Msg_Lookup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct Scattering {
    group: usize,
    gather: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Scattering>,
}

impl Scattering {
    pub fn new(group: usize, gather: usize) -> Self {
        Self {
            group,
            gather,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Search for Scattering {

    fn query(&mut self, word: String, out: crate::scheduler::SalvoReply) {
        let mut members = {
            let (mut ms, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.group.clone(), crate::net::__Msg_ActorGroup::Members(ms), crate::net::__PROTO_ActorGroup);
            *crate::scheduler::salvo_wait(__wid).downcast::<Vec<usize>>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(self.gather.clone(), crate::__Msg_Gather::Scatter(word, members, out), crate::__PROTO_Gather);
    }
}

pub enum __Cont_Scattering {
    Query(String),
}

pub struct __Actor_Scattering {
    handler: Scattering,
}

impl __Actor_Scattering {
    pub fn new(handler: Scattering) -> Self {
        Self { handler }
    }
}

impl __Actor_Scattering {
    fn __dispatch(&mut self, msg: crate::__Msg_Search) {
        match msg {
            crate::__Msg_Search::Query(word, out) => crate::Search::query(&mut self.handler, word, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Scattering {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Search>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Scattering::Query(word) => self.__dispatch(crate::__Msg_Search::Query(word, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Scattering::Query{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Scattering: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Scattering);
fn __decode_msg_Scattering(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Search {
            return crate::wire::salvo_decode::<crate::__Msg_Search>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub trait Gather {
    fn scatter(&mut self, word: String, members: Vec<usize>, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Gather {
    fn __get_Gather(&mut self) -> &mut dyn Gather;
}

pub struct __Stub_Gather {
    addr: usize,
}

impl __Stub_Gather {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Gather for __Stub_Gather {
    fn scatter(&mut self, word: String, members: Vec<usize>, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Gather::Scatter(word, members, out), crate::__PROTO_Gather);
    }
}

pub enum __Msg_Gather {
    Scatter(String, Vec<usize>, crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Gather {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Gather::Scatter(__p0, __p1, __p2) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Gather::Scatter(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Gather`.
pub const __PROTO_Gather: &str = "98712ca205da344c";

pub struct Gathering {
    pending: Vec<crate::scheduler::SalvoReply>,
    left: i32,
    total: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Gathering>,
}

impl Gathering {
    pub fn new() -> Self {
        Self {
            pending: vec![],
            left: 0,
            total: 0,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Gather for Gathering {

    fn scatter(&mut self, word: String, members: Vec<usize>, out: crate::scheduler::SalvoReply) {
        self.pending.push(out);
        self.left = (members.len() as i32);
        self.total = 0;
        for m in &members {
            crate::scheduler::salvo_send_wire(m.clone(), crate::__Msg_Search::Query(word.clone(), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Gathering::Partial); __r }), crate::__PROTO_Search);
        }
    }
}

impl Gathering {

    fn partial(&mut self, n: i32) {
        self.total = self.total + n;
        self.left = self.left - 1;
        if self.left == 0 {
            let mut out = self.pending.salvo_remove_first();
            match out {
                Some(_) => {
                    crate::scheduler::salvo_reply_wire::<i32>(out.unwrap(), self.total.clone());
                }
                None => {
                }
            }
        }
    }
}

pub enum __Cont_Gathering {
    Scatter(String, Vec<usize>),
    Partial,
}

pub enum __Priv_Gathering {
    Partial(i32),
}

pub struct __Actor_Gathering {
    handler: Gathering,
}

impl __Actor_Gathering {
    pub fn new(handler: Gathering) -> Self {
        Self { handler }
    }
}

impl __Actor_Gathering {
    fn __dispatch_Gather(&mut self, msg: crate::__Msg_Gather) {
        match msg {
            crate::__Msg_Gather::Scatter(word, members, out) => crate::Gather::scatter(&mut self.handler, word, members, out),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_Gathering) {
        match msg {
            __Priv_Gathering::Partial(n) => self.handler.partial(n),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Gathering {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::__Msg_Gather>() {
            Ok(__m) => return self.__dispatch_Gather(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_Gathering>() {
            Ok(__m) => return self.__dispatch_priv(*__m),
            Err(__m) => __m,
        };
        let _ = msg;
        unreachable!("a message of one of this actor's protocols")
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Gathering::Scatter(word, members) => self.__dispatch_Gather(crate::__Msg_Gather::Scatter(word, members, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Gathering::Partial => self.__dispatch_priv(__Priv_Gathering::Partial(*value.downcast::<i32>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Gathering::Scatter{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Gathering::Partial{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Gathering: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Gathering);
fn __decode_msg_Gathering(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Gather {
            return crate::wire::salvo_decode::<crate::__Msg_Gather>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct Hedging {
    group: usize,
    racer: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Hedging>,
}

impl Hedging {
    pub fn new(group: usize, racer: usize) -> Self {
        Self {
            group,
            racer,
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Lookup for Hedging {

    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply) {
        let mut members = {
            let (mut ms, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.group.clone(), crate::net::__Msg_ActorGroup::Members(ms), crate::net::__PROTO_ActorGroup);
            *crate::scheduler::salvo_wait(__wid).downcast::<Vec<usize>>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(self.racer.clone(), crate::__Msg_Race::Race(key, members, out), crate::__PROTO_Race);
    }
}

pub enum __Cont_Hedging {
    Lookup(String),
}

pub struct __Actor_Hedging {
    handler: Hedging,
}

impl __Actor_Hedging {
    pub fn new(handler: Hedging) -> Self {
        Self { handler }
    }
}

impl __Actor_Hedging {
    fn __dispatch(&mut self, msg: crate::__Msg_Lookup) {
        match msg {
            crate::__Msg_Lookup::Lookup(key, out) => crate::Lookup::lookup(&mut self.handler, key, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Hedging {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Lookup>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Hedging::Lookup(key) => self.__dispatch(crate::__Msg_Lookup::Lookup(key, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Hedging::Lookup{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Hedging: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Hedging);
fn __decode_msg_Hedging(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Lookup {
            return crate::wire::salvo_decode::<crate::__Msg_Lookup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub trait Race {
    fn race(&mut self, key: String, members: Vec<usize>, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Race {
    fn __get_Race(&mut self) -> &mut dyn Race;
}

pub struct __Stub_Race {
    addr: usize,
}

impl __Stub_Race {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Race for __Stub_Race {
    fn race(&mut self, key: String, members: Vec<usize>, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Race::Race(key, members, out), crate::__PROTO_Race);
    }
}

pub enum __Msg_Race {
    Race(String, Vec<usize>, crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Race {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Race::Race(__p0, __p1, __p2) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Race::Race(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Race`.
pub const __PROTO_Race: &str = "5e0ec4d63d5f53dd";

pub struct Racing {
    pending: Vec<crate::scheduler::SalvoReply>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Racing>,
}

impl Racing {
    pub fn new() -> Self {
        Self {
            pending: vec![],
            __mailbox_capacity: 16,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Race for Racing {

    fn race(&mut self, key: String, members: Vec<usize>, out: crate::scheduler::SalvoReply) {
        self.pending.push(out);
        for m in &members {
            crate::scheduler::salvo_send_wire(m.clone(), crate::__Msg_Lookup::Lookup(key.clone(), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Racing::First); __r }), crate::__PROTO_Lookup);
        }
    }
}

impl Racing {

    fn first(&mut self, answer: String) {
        let mut out = self.pending.salvo_remove_first();
        match out {
            Some(_) => {
                crate::scheduler::salvo_reply_wire::<String>(out.unwrap(), answer);
            }
            None => {
                drop(answer);
            }
        }
    }
}

pub enum __Cont_Racing {
    Race(String, Vec<usize>),
    First,
}

pub enum __Priv_Racing {
    First(String),
}

pub struct __Actor_Racing {
    handler: Racing,
}

impl __Actor_Racing {
    pub fn new(handler: Racing) -> Self {
        Self { handler }
    }
}

impl __Actor_Racing {
    fn __dispatch_Race(&mut self, msg: crate::__Msg_Race) {
        match msg {
            crate::__Msg_Race::Race(key, members, out) => crate::Race::race(&mut self.handler, key, members, out),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_Racing) {
        match msg {
            __Priv_Racing::First(answer) => self.handler.first(answer),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Racing {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::__Msg_Race>() {
            Ok(__m) => return self.__dispatch_Race(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_Racing>() {
            Ok(__m) => return self.__dispatch_priv(*__m),
            Err(__m) => __m,
        };
        let _ = msg;
        unreachable!("a message of one of this actor's protocols")
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Racing::Race(key, members) => self.__dispatch_Race(crate::__Msg_Race::Race(key, members, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Racing::First => self.__dispatch_priv(__Priv_Racing::First(*value.downcast::<String>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Racing::Race{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Racing::First{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Racing: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Racing);
fn __decode_msg_Racing(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Race {
            return crate::wire::salvo_decode::<crate::__Msg_Race>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

#[derive(Clone)]
pub struct LastHost {
    nodes: usize,
    me: NodeEndpoint,
}

impl LastHost {
    pub fn new(nodes: usize, me: NodeEndpoint) -> Self {
        Self {
            nodes,
            me,
        }
    }
}

impl Leader for LastHost {

    fn leader(&mut self) -> Option<NodeId> {
        let mut peers = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<Node>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.nodes.clone(), crate::net::__Msg_NodeGroup::Members(out), crate::net::__PROTO_NodeGroup);
            *crate::scheduler::salvo_wait(__wid).downcast::<Vec<Node>>().expect("the awaited answer")
        };
        let mut best_host = self.me.host.clone();
        let mut best = NodeId { id: crate::scheduler::salvo_here_node() as i64 };
        for n in &peers {
            if ((Ord::cmp(&n.at.host[..], &best_host[..]) as i32) > 0) {
                best_host = n.at.host.clone();
                best = n.id.clone();
            }
        }
        return Some(best);
    }
}

pub fn fresh_id<__Fx: __Has_Sequencer>(__fx: &mut __Fx) -> String {
    return {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        __Has_Sequencer::__get_Sequencer(&mut *__fx).next(out);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
}

pub fn checkout<__Fx: __Has_Inventory + __Has_Console>(__fx: &mut __Fx, skus: &Vec<String>) {
    let mut shards: Vec<String> = vec![];
    for sku in skus {
        let mut answer = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            __Has_Inventory::__get_Inventory(&mut *__fx).reserve(sku.clone(), 1, out);
            *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
        };
        let mut parts = answer.split(&":".to_string()[..]).map(|__p| __p.to_string()).collect::<Vec<String>>();
        shards.push(parts.get((0) as i64 as usize).expect("salvo: value is absent at main:217:26").clone());
        println(&mut *__fx, &(format!("  {}: {} reserved on its shard so far", sku.clone(), parts.get((1) as i64 as usize).expect("salvo: value is absent at main:218:30"))));
    }
    println(&mut *__fx, &(format!("  apple and apple on one shard: {}", (&shards.get((0) as i64 as usize).expect("salvo: value is absent at main:220:51")[..] == &shards.get((2) as i64 as usize).expect("salvo: value is absent at main:220:68")[..]))));
    println(&mut *__fx, &(format!("  apple and fig on one shard: {}", (&shards.get((0) as i64 as usize).expect("salvo: value is absent at main:221:49")[..] == &shards.get((3) as i64 as usize).expect("salvo: value is absent at main:221:66")[..]))));
    println(&mut *__fx, &(format!("  apple and pear on one shard: {}", (&shards.get((0) as i64 as usize).expect("salvo: value is absent at main:222:50")[..] == &shards.get((1) as i64 as usize).expect("salvo: value is absent at main:222:67")[..]))));
}

pub fn count<__Fx: __Has_Search>(__fx: &mut __Fx, word: String) -> i32 {
    return {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        __Has_Search::__get_Search(&mut *__fx).query(word, out);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
}

pub fn find<__Fx: __Has_Lookup>(__fx: &mut __Fx, key: String) -> String {
    return {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        __Has_Lookup::__get_Lookup(&mut *__fx).lookup(key, out);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
}

pub fn two_ids<__Fx: __Has_Leader + __Has_Console>(__fx: &mut __Fx, __hs: &__Hs_leader, seq: usize) {
    let mut __fx2 = __Fx_two_ids_1 { __outer: &mut *__fx, __h: Elected::new(__hs.leader.clone()) };
    let mut __fx3 = __Fx_two_ids_2 { __outer: &mut __fx2, __h: __Route_Sequencer::new(seq) };
    { let __a1 = &(format!("  {} {}", fresh_id(&mut __fx3), fresh_id(&mut __fx3))); println(&mut __fx3, __a1) };
}

pub fn shop<__Fx: __Has_Console>(__fx: &mut __Fx, stock: usize) {
    let mut __fx2 = __Fx_shop_3 { __outer: &mut *__fx, __h: Sharded::new() };
    let mut __fx3 = __Fx_shop_4 { __outer: &mut __fx2, __h: __Route_Inventory::new(stock) };
    checkout(&mut __fx3, &(vec!["apple".to_string(), "pear".to_string(), "apple".to_string(), "fig".to_string(), "pear".to_string()]));
}

pub trait Boot {
    fn boot(&mut self, done: crate::scheduler::SalvoReply);
    fn stop(&mut self, done: crate::scheduler::SalvoReply);
}

pub trait __Has_Boot {
    fn __get_Boot(&mut self) -> &mut dyn Boot;
}

pub struct __Stub_Boot {
    addr: usize,
}

impl __Stub_Boot {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Boot for __Stub_Boot {
    fn boot(&mut self, done: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Boot::Boot(done), crate::__PROTO_Boot);
    }
    fn stop(&mut self, done: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Boot::Stop(done), crate::__PROTO_Boot);
    }
}

pub enum __Msg_Boot {
    Boot(crate::scheduler::SalvoReply),
    Stop(crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Boot {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Boot::Boot(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_Boot::Stop(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Boot::Boot(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_Boot::Stop(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Boot`.
pub const __PROTO_Boot: &str = "4b15e647d0ca92a7";

pub struct Booting {
    at: NodeEndpoint,
    all: Vec<NodeEndpoint>,
    net: usize,
    nodes: Option<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Booting>,
}

impl Booting {
    pub fn new(at: NodeEndpoint, all: Vec<NodeEndpoint>, net: usize) -> Self {
        Self {
            at,
            all,
            net,
            nodes: None,
            __mailbox_capacity: 2,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_Booting<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Transport + ?Sized> __Has_Transport for __Deps_Booting<'a, __P> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__p)
    }
}

pub trait __Impl_Booting {

    fn boot<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, done: crate::scheduler::SalvoReply);

    fn stop<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, done: crate::scheduler::SalvoReply);
}

impl __Impl_Booting for Booting {

    fn boot<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, done: crate::scheduler::SalvoReply) {
        let mut p = crate::scheduler::salvo_pool(((1) as usize));
        let mut _connected = connect__3(&mut *__fx, self.at.clone(), ({ let __h = Sending::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sending::new(__h, __Prov_Sending { __d0: MemTransport::new(self.at.clone(), self.net.clone()) })), __DECODE_Sending) }), ({ let __h = Receiving::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Receiving::new(__h)), __DECODE_Receiving) }));
        let mut group = node_group(({ let __h = StaticNodeGroup::new("cluster".to_string(), self.all.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_StaticNodeGroup::new(__h, __Prov_StaticNodeGroup { __d0: MemTransport::new(self.at.clone(), self.net.clone()) })), __DECODE_StaticNodeGroup); (__a, __a) }));
        self.nodes = Some(group.clone());
        let mut seq = crate::net::open_group(Protocol { name: "Sequencer".to_string(), hash: crate::__PROTO_Sequencer.to_string() }, group.clone());
        let mut mine = ({ let __h = Sequencing::new("b".to_string()); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sequencing::new(__h)), __DECODE_Sequencing) });
        join(&seq, mine.clone());
        let mut stock = crate::net::open_group(Protocol { name: "Inventory".to_string(), hash: crate::__PROTO_Inventory.to_string() }, group.clone());
        join(&stock, ({ let __h = Stocking::new("shard-b".to_string()); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Stocking::new(__h)), __DECODE_Stocking) }));
        let mut index = crate::net::open_group(Protocol { name: "Search".to_string(), hash: crate::__PROTO_Search.to_string() }, group.clone());
        join(&index, ({ let __h = Indexing::new(vec!["salvo".to_string(), "actors".to_string(), "salvo".to_string(), "nodes".to_string()]); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Indexing::new(__h)), __DECODE_Indexing) }));
        let mut looks = crate::net::open_group(Protocol { name: "Lookup".to_string(), hash: crate::__PROTO_Lookup.to_string() }, group);
        let mut timer = ({ let __h = DefaultTimer::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_DefaultTimer::new(__h)), __DECODE_DefaultTimer) });
        join(&looks, ({ let __h = SlowLooking::new("b (slow)".to_string(), timer); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_SlowLooking::new(__h)), __DECODE_SlowLooking) }));
        crate::scheduler::salvo_reply_wire::<usize>(done, mine);
    }

    fn stop<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, done: crate::scheduler::SalvoReply) {
        let mut group = &self.nodes;
        if !(group.is_none()) {
            crate::scheduler::salvo_send_wire(group.as_ref().unwrap().clone(), crate::net::__Msg_NodeGroup::Leave, crate::net::__PROTO_NodeGroup);
        }
        crate::scheduler::salvo_reply_wire::<bool>(done, true);
    }
}

pub enum __Cont_Booting {
    Boot,
    Stop,
}

pub struct __Prov_Booting<__D0> {
    pub __d0: __D0,
}

impl<__D0: Transport> __Has_Transport for __Prov_Booting<__D0> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        &mut self.__d0
    }
}

pub struct __Actor_Booting<__D0> {
    handler: Booting,
    prov: __Prov_Booting<__D0>,
}

impl<__D0> __Actor_Booting<__D0> {
    pub fn new(handler: Booting, prov: __Prov_Booting<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Transport> __Actor_Booting<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Boot) {
        let mut __deps = __Deps_Booting{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Boot::Boot(done) => __Impl_Booting::boot(&mut self.handler, &mut __deps, done),
            crate::__Msg_Boot::Stop(done) => __Impl_Booting::stop(&mut self.handler, &mut __deps, done),
        }
    }
}

impl<__D0: Transport + Send + 'static> crate::scheduler::SalvoActor for __Actor_Booting<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Boot>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Booting::Boot => self.__dispatch(crate::__Msg_Boot::Boot(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Booting::Stop => self.__dispatch(crate::__Msg_Boot::Stop(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Booting::Boot{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Booting::Stop{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Booting: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Booting);
fn __decode_msg_Booting(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Boot {
            return crate::wire::salvo_decode::<crate::__Msg_Boot>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn settle(timer: &usize) {
    let mut _f = {
        let (mut f, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Fired>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(timer.clone(), crate::time::__Msg_Timer::After(millis(400i64), f), crate::time::__PROTO_Timer);
        *crate::scheduler::salvo_wait(__wid).downcast::<Fired>().expect("the awaited answer")
    };
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("ActorChanges".to_string(), crate::net::__PROTO_ActorChanges.to_string()), ("ActorGroup".to_string(), crate::net::__PROTO_ActorGroup.to_string()), ("Boot".to_string(), crate::__PROTO_Boot.to_string()), ("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Gather".to_string(), crate::__PROTO_Gather.to_string()), ("Inbound".to_string(), crate::net::__PROTO_Inbound.to_string()), ("Inventory".to_string(), crate::__PROTO_Inventory.to_string()), ("Lookup".to_string(), crate::__PROTO_Lookup.to_string()), ("MemNet".to_string(), crate::net::__PROTO_MemNet.to_string()), ("NodeChanges".to_string(), crate::net::__PROTO_NodeChanges.to_string()), ("NodeGroup".to_string(), crate::net::__PROTO_NodeGroup.to_string()), ("Outbound".to_string(), crate::net::__PROTO_Outbound.to_string()), ("PeerEvents".to_string(), crate::net::__PROTO_PeerEvents.to_string()), ("Race".to_string(), crate::__PROTO_Race.to_string()), ("Search".to_string(), crate::__PROTO_Search.to_string()), ("Sequencer".to_string(), crate::__PROTO_Sequencer.to_string()), ("Timer".to_string(), crate::time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::time::__PROTO_TimerCtl.to_string())]);
    let mut __fx = __Fx_main_5 { __h: StdOutConsole::new() };
    let mut a = NodeEndpoint { host: "a".to_string(), port: 1 };
    let mut b = NodeEndpoint { host: "b".to_string(), port: 1 };
    let mut all = vec![a.clone(), b.clone()];
    let mut network = ({ let __h = MemNetwork::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, Box::new(__Actor_MemNetwork::new(__h)), __DECODE_MemNetwork) });
    let mut __bind = MemTransport::new(a.clone(), network.clone());
    let __handle = crate::net::__Mon_Transport::new(Box::new(__bind.clone()));
    let mut __fx2 = __Fx_main_6 { __outer: &mut __fx, __h: __bind };
    let mut p = crate::scheduler::salvo_pool(((2) as usize));
    let mut _connected = connect__2(&mut __fx2, &crate::net::__Hs_transport { transport: __handle.clone() }, a.clone(), p.clone());
    let mut nodes = node_group(({ let __h = StaticNodeGroup::new("cluster".to_string(), all.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_StaticNodeGroup::new(__h, __Prov_StaticNodeGroup { __d0: __handle.clone() })), __DECODE_StaticNodeGroup); (__a, __a) }));
    let mut seq = crate::net::open_group(Protocol { name: "Sequencer".to_string(), hash: crate::__PROTO_Sequencer.to_string() }, nodes.clone());
    join(&seq, ({ let __h = Sequencing::new("a".to_string()); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sequencing::new(__h)), __DECODE_Sequencing) }));
    let mut stock = crate::net::open_group(Protocol { name: "Inventory".to_string(), hash: crate::__PROTO_Inventory.to_string() }, nodes.clone());
    join(&stock, ({ let __h = Stocking::new("shard-a".to_string()); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Stocking::new(__h)), __DECODE_Stocking) }));
    let mut index = crate::net::open_group(Protocol { name: "Search".to_string(), hash: crate::__PROTO_Search.to_string() }, nodes.clone());
    join(&index, ({ let __h = Indexing::new(vec!["salvo".to_string(), "is".to_string(), "salvo".to_string()]); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Indexing::new(__h)), __DECODE_Indexing) }));
    let mut looks = crate::net::open_group(Protocol { name: "Lookup".to_string(), hash: crate::__PROTO_Lookup.to_string() }, nodes.clone());
    join(&looks, ({ let __h = Looking::new("a".to_string()); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Looking::new(__h)), __DECODE_Looking) }));
    let mut pb = crate::scheduler::salvo_pool_at((NodeId { id: crate::scheduler::salvo_new_node() as i64 }).id as u64, (1) as usize);
    let mut booter = ({ let __h = Booting::new(b.clone(), all.clone(), network.clone()); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(pb, __cap as usize, Box::new(__Actor_Booting::new(__h, __Prov_Booting { __d0: MemTransport::new(b.clone(), network.clone()) })), __DECODE_Booting) });
    let mut remote_seq = {
        let (mut done, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(booter, crate::__Msg_Boot::Boot(done), crate::__PROTO_Boot);
        *crate::scheduler::salvo_wait(__wid).downcast::<usize>().expect("the awaited answer")
    };
    let mut timer = ({ let __h = DefaultTimer::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, Box::new(__Actor_DefaultTimer::new(__h)), __DECODE_DefaultTimer) });
    settle(&(timer.clone()));
    let mut members = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<Node>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(nodes, crate::net::__Msg_NodeGroup::Members(out), crate::net::__PROTO_NodeGroup);
        *crate::scheduler::salvo_wait(__wid).downcast::<Vec<Node>>().expect("the awaited answer")
    };
    println(&mut __fx2, &(format!("nodes: {}, sequencers: {}", (members.len() as i32) + 1, ({
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(seq, crate::net::__Msg_ActorGroup::Members(out), crate::net::__PROTO_ActorGroup);
        *crate::scheduler::salvo_wait(__wid).downcast::<Vec<usize>>().expect("the awaited answer")
    }.len() as i32))));
    println(&mut __fx2, &(format!("singleton (b's sequencer is remote: {}):", !eq__2(&(NodeId { id: crate::scheduler::salvo_addr_identity((remote_seq).clone()).node as i64 }), &(NodeId { id: crate::scheduler::salvo_here_node() as i64 })))));
    let mut __bind2 = LastHost::new(nodes.clone(), a.clone());
    let __handle2 = crate::net::__Mon_Leader::new(Box::new(__bind2.clone()));
    let mut __fx3 = __Fx_main_7 { __outer: &mut __fx2, __h: __bind2 };
    two_ids(&mut __fx3, &crate::__Hs_leader { leader: __handle2.clone() }, seq.clone());
    println(&mut __fx3, &("sharded:".to_string()));
    shop(&mut __fx3, stock);
    println(&mut __fx3, &("scatter:".to_string()));
    let mut __fx4 = __Fx_main_8 { __outer: &mut __fx3, __h: Scattering::new(index.clone(), ({ let __h = Gathering::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Gathering::new(__h)), __DECODE_Gathering) })) };
    { let __a1 = &(format!("  salvo: {}, actors: {}, none: {}", count(&mut __fx4, "salvo".to_string()), count(&mut __fx4, "actors".to_string()), count(&mut __fx4, "none".to_string()))); println(&mut __fx4, __a1) };
    println(&mut __fx4, &("hedge:".to_string()));
    let mut __fx5 = __Fx_main_9 { __outer: &mut __fx4, __h: Hedging::new(looks.clone(), ({ let __h = Racing::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Racing::new(__h)), __DECODE_Racing) })) };
    { let __a2 = &(format!("  {}", find(&mut __fx5, "k1".to_string()))); println(&mut __fx5, __a2) };
    let mut _stopped = {
        let (mut done, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<bool>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(booter, crate::__Msg_Boot::Stop(done), crate::__PROTO_Boot);
        *crate::scheduler::salvo_wait(__wid).downcast::<bool>().expect("the awaited answer")
    };
    settle(&(timer.clone()));
    println(&mut __fx5, &("after b left:".to_string()));
    println(&mut __fx5, &(format!("  sequencers: {}", ({
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(seq, crate::net::__Msg_ActorGroup::Members(out), crate::net::__PROTO_ActorGroup);
        *crate::scheduler::salvo_wait(__wid).downcast::<Vec<usize>>().expect("the awaited answer")
    }.len() as i32))));
    two_ids(&mut __fx5, &crate::__Hs_leader { leader: __handle2.clone() }, seq);
}

pub struct __Route_Inventory {
    group: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Inventory>,
}

impl __Route_Inventory {
    pub fn new(group: usize) -> Self {
        Self {
            group,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps___Route_Inventory<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Pick + ?Sized> __Has_Pick for __Deps___Route_Inventory<'a, __P> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        __Has_Pick::__get_Pick(&mut *self.__p)
    }
}

pub trait __Impl___Route_Inventory {

    fn reserve<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, sku: String, qty: i32, out: crate::scheduler::SalvoReply);
}

impl __Impl___Route_Inventory for __Route_Inventory {

    fn reserve<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, sku: String, qty: i32, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to__2(&mut *__fx, &self.group, crate::scheduler::salvo_key_hash(&crate::wire::salvo_encode(&sku)));
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Inventory::Reserve(sku, qty, out), crate::__PROTO_Inventory);
    }
}

pub enum __Cont___Route_Inventory {
    Reserve(String, i32),
}

pub struct __Prov___Route_Inventory<__D0> {
    pub __d0: __D0,
}

impl<__D0: Pick> __Has_Pick for __Prov___Route_Inventory<__D0> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        &mut self.__d0
    }
}

pub struct __Actor___Route_Inventory<__D0> {
    handler: __Route_Inventory,
    prov: __Prov___Route_Inventory<__D0>,
}

impl<__D0> __Actor___Route_Inventory<__D0> {
    pub fn new(handler: __Route_Inventory, prov: __Prov___Route_Inventory<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Pick> __Actor___Route_Inventory<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Inventory) {
        let mut __deps = __Deps___Route_Inventory{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Inventory::Reserve(sku, qty, out) => __Impl___Route_Inventory::reserve(&mut self.handler, &mut __deps, sku, qty, out),
        }
    }
}

impl<__D0: Pick + Send + 'static> crate::scheduler::SalvoActor for __Actor___Route_Inventory<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Inventory>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont___Route_Inventory::Reserve(sku, qty) => self.__dispatch(crate::__Msg_Inventory::Reserve(sku, qty, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont___Route_Inventory::Reserve{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE___Route_Inventory: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg___Route_Inventory);
fn __decode_msg___Route_Inventory(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Inventory {
            return crate::wire::salvo_decode::<crate::__Msg_Inventory>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct __Route_Lookup {
    group: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Lookup>,
}

impl __Route_Lookup {
    pub fn new(group: usize) -> Self {
        Self {
            group,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps___Route_Lookup<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Pick + ?Sized> __Has_Pick for __Deps___Route_Lookup<'a, __P> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        __Has_Pick::__get_Pick(&mut *self.__p)
    }
}

pub trait __Impl___Route_Lookup {

    fn lookup<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, key: String, out: crate::scheduler::SalvoReply);
}

impl __Impl___Route_Lookup for __Route_Lookup {

    fn lookup<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, key: String, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to(&mut *__fx, &self.group);
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Lookup::Lookup(key, out), crate::__PROTO_Lookup);
    }
}

pub enum __Cont___Route_Lookup {
    Lookup(String),
}

pub struct __Prov___Route_Lookup<__D0> {
    pub __d0: __D0,
}

impl<__D0: Pick> __Has_Pick for __Prov___Route_Lookup<__D0> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        &mut self.__d0
    }
}

pub struct __Actor___Route_Lookup<__D0> {
    handler: __Route_Lookup,
    prov: __Prov___Route_Lookup<__D0>,
}

impl<__D0> __Actor___Route_Lookup<__D0> {
    pub fn new(handler: __Route_Lookup, prov: __Prov___Route_Lookup<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Pick> __Actor___Route_Lookup<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Lookup) {
        let mut __deps = __Deps___Route_Lookup{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Lookup::Lookup(key, out) => __Impl___Route_Lookup::lookup(&mut self.handler, &mut __deps, key, out),
        }
    }
}

impl<__D0: Pick + Send + 'static> crate::scheduler::SalvoActor for __Actor___Route_Lookup<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Lookup>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont___Route_Lookup::Lookup(key) => self.__dispatch(crate::__Msg_Lookup::Lookup(key, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont___Route_Lookup::Lookup{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE___Route_Lookup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg___Route_Lookup);
fn __decode_msg___Route_Lookup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Lookup {
            return crate::wire::salvo_decode::<crate::__Msg_Lookup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct __Route_Search {
    group: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Search>,
}

impl __Route_Search {
    pub fn new(group: usize) -> Self {
        Self {
            group,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps___Route_Search<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Pick + ?Sized> __Has_Pick for __Deps___Route_Search<'a, __P> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        __Has_Pick::__get_Pick(&mut *self.__p)
    }
}

pub trait __Impl___Route_Search {

    fn query<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, word: String, out: crate::scheduler::SalvoReply);
}

impl __Impl___Route_Search for __Route_Search {

    fn query<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, word: String, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to(&mut *__fx, &self.group);
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Search::Query(word, out), crate::__PROTO_Search);
    }
}

pub enum __Cont___Route_Search {
    Query(String),
}

pub struct __Prov___Route_Search<__D0> {
    pub __d0: __D0,
}

impl<__D0: Pick> __Has_Pick for __Prov___Route_Search<__D0> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        &mut self.__d0
    }
}

pub struct __Actor___Route_Search<__D0> {
    handler: __Route_Search,
    prov: __Prov___Route_Search<__D0>,
}

impl<__D0> __Actor___Route_Search<__D0> {
    pub fn new(handler: __Route_Search, prov: __Prov___Route_Search<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Pick> __Actor___Route_Search<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Search) {
        let mut __deps = __Deps___Route_Search{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Search::Query(word, out) => __Impl___Route_Search::query(&mut self.handler, &mut __deps, word, out),
        }
    }
}

impl<__D0: Pick + Send + 'static> crate::scheduler::SalvoActor for __Actor___Route_Search<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Search>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont___Route_Search::Query(word) => self.__dispatch(crate::__Msg_Search::Query(word, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont___Route_Search::Query{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE___Route_Search: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg___Route_Search);
fn __decode_msg___Route_Search(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Search {
            return crate::wire::salvo_decode::<crate::__Msg_Search>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct __Route_Sequencer {
    group: usize,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Sequencer>,
}

impl __Route_Sequencer {
    pub fn new(group: usize) -> Self {
        Self {
            group,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps___Route_Sequencer<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Pick + ?Sized> __Has_Pick for __Deps___Route_Sequencer<'a, __P> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        __Has_Pick::__get_Pick(&mut *self.__p)
    }
}

pub trait __Impl___Route_Sequencer {

    fn next<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, out: crate::scheduler::SalvoReply);
}

impl __Impl___Route_Sequencer for __Route_Sequencer {

    fn next<__Fx: __Has_Pick>(&mut self, __fx: &mut __Fx, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to(&mut *__fx, &self.group);
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Sequencer::Next(out), crate::__PROTO_Sequencer);
    }
}

pub enum __Cont___Route_Sequencer {
    Next,
}

pub struct __Prov___Route_Sequencer<__D0> {
    pub __d0: __D0,
}

impl<__D0: Pick> __Has_Pick for __Prov___Route_Sequencer<__D0> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        &mut self.__d0
    }
}

pub struct __Actor___Route_Sequencer<__D0> {
    handler: __Route_Sequencer,
    prov: __Prov___Route_Sequencer<__D0>,
}

impl<__D0> __Actor___Route_Sequencer<__D0> {
    pub fn new(handler: __Route_Sequencer, prov: __Prov___Route_Sequencer<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Pick> __Actor___Route_Sequencer<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Sequencer) {
        let mut __deps = __Deps___Route_Sequencer{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Sequencer::Next(out) => __Impl___Route_Sequencer::next(&mut self.handler, &mut __deps, out),
        }
    }
}

impl<__D0: Pick + Send + 'static> crate::scheduler::SalvoActor for __Actor___Route_Sequencer<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Sequencer>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont___Route_Sequencer::Next => self.__dispatch(crate::__Msg_Sequencer::Next(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont___Route_Sequencer::Next{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE___Route_Sequencer: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg___Route_Sequencer);
fn __decode_msg___Route_Sequencer(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Sequencer {
            return crate::wire::salvo_decode::<crate::__Msg_Sequencer>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct __Hs_leader {
    pub leader: crate::net::__Mon_Leader,
}

pub trait __Prov_Console_Leader: __Has_Console + __Has_Leader {}
impl<T: __Has_Console + __Has_Leader + ?Sized> __Prov_Console_Leader for T {}

pub struct __Fx_two_ids_1<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Leader,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_two_ids_1<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Leader for __Fx_two_ids_1<'a, __H> {
    fn __get_Leader(&mut self) -> &mut dyn Leader {
        __Has_Leader::__get_Leader(&mut *self.__outer)
    }
}

impl<'a, __H: Pick> __Has_Pick for __Fx_two_ids_1<'a, __H> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        &mut self.__h
    }
}

pub trait __Prov_Console_Leader_Pick: __Has_Console + __Has_Leader + __Has_Pick {}
impl<T: __Has_Console + __Has_Leader + __Has_Pick + ?Sized> __Prov_Console_Leader_Pick for T {}

pub struct __Fx_two_ids_2<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Leader_Pick,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_two_ids_2<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Leader for __Fx_two_ids_2<'a, __H> {
    fn __get_Leader(&mut self) -> &mut dyn Leader {
        __Has_Leader::__get_Leader(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Pick for __Fx_two_ids_2<'a, __H> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        __Has_Pick::__get_Pick(&mut *self.__outer)
    }
}

impl<'a, __H: __Impl___Route_Sequencer> Sequencer for __Fx_two_ids_2<'a, __H> {
    fn next(&mut self, out: crate::scheduler::SalvoReply) {
        let Self { __outer, __h } = self;
        let mut __deps = __Deps___Route_Sequencer{ __p: &mut **__outer };
        __Impl___Route_Sequencer::next(__h, &mut __deps, out)
    }
}

impl<'a, __H: __Impl___Route_Sequencer> __Has_Sequencer for __Fx_two_ids_2<'a, __H> {
    fn __get_Sequencer(&mut self) -> &mut dyn Sequencer {
        self
    }
}

pub struct __Fx_shop_3<'a, __H> {
    __outer: &'a mut dyn __Has_Console,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_shop_3<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Pick> __Has_Pick for __Fx_shop_3<'a, __H> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        &mut self.__h
    }
}

pub trait __Prov_Console_Pick: __Has_Console + __Has_Pick {}
impl<T: __Has_Console + __Has_Pick + ?Sized> __Prov_Console_Pick for T {}

pub struct __Fx_shop_4<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Pick,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_shop_4<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Pick for __Fx_shop_4<'a, __H> {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        __Has_Pick::__get_Pick(&mut *self.__outer)
    }
}

impl<'a, __H: __Impl___Route_Inventory> Inventory for __Fx_shop_4<'a, __H> {
    fn reserve(&mut self, sku: String, qty: i32, out: crate::scheduler::SalvoReply) {
        let Self { __outer, __h } = self;
        let mut __deps = __Deps___Route_Inventory{ __p: &mut **__outer };
        __Impl___Route_Inventory::reserve(__h, &mut __deps, sku, qty, out)
    }
}

impl<'a, __H: __Impl___Route_Inventory> __Has_Inventory for __Fx_shop_4<'a, __H> {
    fn __get_Inventory(&mut self) -> &mut dyn Inventory {
        self
    }
}

pub struct __Fx_main_5<__H> {
    __h: __H,
}

impl<__H: Console> __Has_Console for __Fx_main_5<__H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        &mut self.__h
    }
}

pub struct __Fx_main_6<'a, __H> {
    __outer: &'a mut dyn __Has_Console,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_6<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Transport> __Has_Transport for __Fx_main_6<'a, __H> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        &mut self.__h
    }
}

pub trait __Prov_Console_Transport: __Has_Console + __Has_Transport {}
impl<T: __Has_Console + __Has_Transport + ?Sized> __Prov_Console_Transport for T {}

pub struct __Fx_main_7<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Transport,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_7<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Transport for __Fx_main_7<'a, __H> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__outer)
    }
}

impl<'a, __H: Leader> __Has_Leader for __Fx_main_7<'a, __H> {
    fn __get_Leader(&mut self) -> &mut dyn Leader {
        &mut self.__h
    }
}

pub trait __Prov_Console_Leader_Transport: __Has_Console + __Has_Leader + __Has_Transport {}
impl<T: __Has_Console + __Has_Leader + __Has_Transport + ?Sized> __Prov_Console_Leader_Transport for T {}

pub struct __Fx_main_8<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Leader_Transport,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_8<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Leader for __Fx_main_8<'a, __H> {
    fn __get_Leader(&mut self) -> &mut dyn Leader {
        __Has_Leader::__get_Leader(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Transport for __Fx_main_8<'a, __H> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__outer)
    }
}

impl<'a, __H: Search> __Has_Search for __Fx_main_8<'a, __H> {
    fn __get_Search(&mut self) -> &mut dyn Search {
        &mut self.__h
    }
}

pub trait __Prov_Console_Leader_Search_Transport: __Has_Console + __Has_Leader + __Has_Search + __Has_Transport {}
impl<T: __Has_Console + __Has_Leader + __Has_Search + __Has_Transport + ?Sized> __Prov_Console_Leader_Search_Transport for T {}

pub struct __Fx_main_9<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Leader_Search_Transport,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_9<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Leader for __Fx_main_9<'a, __H> {
    fn __get_Leader(&mut self) -> &mut dyn Leader {
        __Has_Leader::__get_Leader(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Search for __Fx_main_9<'a, __H> {
    fn __get_Search(&mut self) -> &mut dyn Search {
        __Has_Search::__get_Search(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Transport for __Fx_main_9<'a, __H> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__outer)
    }
}

impl<'a, __H: Lookup> __Has_Lookup for __Fx_main_9<'a, __H> {
    fn __get_Lookup(&mut self) -> &mut dyn Lookup {
        &mut self.__h
    }
}
