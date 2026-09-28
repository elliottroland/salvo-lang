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

pub struct __Handle_Sequencer {
    inner: std::sync::Arc<std::sync::Mutex<dyn Sequencer + Send>>,
}

impl Clone for __Handle_Sequencer {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Sequencer {
    pub fn new<__H: Sequencer + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Sequencer + Send>>) -> Self {
        Self { inner }
    }
}

impl Sequencer for __Handle_Sequencer {
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

pub struct __Handle_Inventory {
    inner: std::sync::Arc<std::sync::Mutex<dyn Inventory + Send>>,
}

impl Clone for __Handle_Inventory {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Inventory {
    pub fn new<__H: Inventory + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Inventory + Send>>) -> Self {
        Self { inner }
    }
}

impl Inventory for __Handle_Inventory {
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

pub struct __Handle_Search {
    inner: std::sync::Arc<std::sync::Mutex<dyn Search + Send>>,
}

impl Clone for __Handle_Search {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Search {
    pub fn new<__H: Search + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Search + Send>>) -> Self {
        Self { inner }
    }
}

impl Search for __Handle_Search {
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

pub struct __Handle_Lookup {
    inner: std::sync::Arc<std::sync::Mutex<dyn Lookup + Send>>,
}

impl Clone for __Handle_Lookup {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Lookup {
    pub fn new<__H: Lookup + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Lookup + Send>>) -> Self {
        Self { inner }
    }
}

impl Lookup for __Handle_Lookup {
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

pub struct __Handle_Gather {
    inner: std::sync::Arc<std::sync::Mutex<dyn Gather + Send>>,
}

impl Clone for __Handle_Gather {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Gather {
    pub fn new<__H: Gather + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Gather + Send>>) -> Self {
        Self { inner }
    }
}

impl Gather for __Handle_Gather {
    fn scatter(&mut self, word: String, members: Vec<usize>, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().scatter(word, members, out)
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

pub struct __Handle_Race {
    inner: std::sync::Arc<std::sync::Mutex<dyn Race + Send>>,
}

impl Clone for __Handle_Race {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Race {
    pub fn new<__H: Race + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Race + Send>>) -> Self {
        Self { inner }
    }
}

impl Race for __Handle_Race {
    fn race(&mut self, key: String, members: Vec<usize>, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().race(key, members, out)
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

pub fn fresh_id(sequencer: &mut crate::__Handle_Sequencer) -> String {
    return {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        sequencer.next(out);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
}

pub fn checkout(inventory: &mut crate::__Handle_Inventory, console: &mut crate::core_console::__Handle_Console, skus: &Vec<String>) {
    let mut shards: Vec<String> = vec![];
    for sku in skus {
        let mut answer = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            inventory.reserve(sku.clone(), 1, out);
            *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
        };
        let mut parts = answer.split(&":".to_string()[..]).map(|__p| __p.to_string()).collect::<Vec<String>>();
        shards.push(parts.get((0) as i64 as usize).expect("salvo: value is absent at main:217:26").clone());
        println(console, &(format!("  {}: {} reserved on its shard so far", sku.clone(), parts.get((1) as i64 as usize).expect("salvo: value is absent at main:218:30"))));
    }
    println(console, &(format!("  apple and apple on one shard: {}", (&shards.get((0) as i64 as usize).expect("salvo: value is absent at main:220:51")[..] == &shards.get((2) as i64 as usize).expect("salvo: value is absent at main:220:68")[..]))));
    println(console, &(format!("  apple and fig on one shard: {}", (&shards.get((0) as i64 as usize).expect("salvo: value is absent at main:221:49")[..] == &shards.get((3) as i64 as usize).expect("salvo: value is absent at main:221:66")[..]))));
    println(console, &(format!("  apple and pear on one shard: {}", (&shards.get((0) as i64 as usize).expect("salvo: value is absent at main:222:50")[..] == &shards.get((1) as i64 as usize).expect("salvo: value is absent at main:222:67")[..]))));
}

pub fn count(search: &mut crate::__Handle_Search, word: String) -> i32 {
    return {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<i32>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        search.query(word, out);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
}

pub fn find(lookup: &mut crate::__Handle_Lookup, key: String) -> String {
    return {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        lookup.lookup(key, out);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
}

pub fn two_ids(leader: &mut crate::net::__Handle_Leader, console: &mut crate::core_console::__Handle_Console, seq: usize) {
    let mut pick = crate::net::__Handle_Pick::new(Elected::new(leader.clone()));
    let mut sequencer = crate::__Handle_Sequencer::new(__Route_Sequencer::new(seq, pick.clone()));
    println(console, &(format!("  {} {}", fresh_id(&mut sequencer), fresh_id(&mut sequencer))));
}

pub fn shop(console: &mut crate::core_console::__Handle_Console, stock: usize) {
    let mut pick = crate::net::__Handle_Pick::new(Sharded::new());
    let mut inventory = crate::__Handle_Inventory::new(__Route_Inventory::new(stock, pick.clone()));
    checkout(&mut inventory, console, &(vec!["apple".to_string(), "pear".to_string(), "apple".to_string(), "fig".to_string(), "pear".to_string()]));
}

pub trait Boot {
    fn boot(&mut self, done: crate::scheduler::SalvoReply);
    fn stop(&mut self, done: crate::scheduler::SalvoReply);
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

pub struct __Handle_Boot {
    inner: std::sync::Arc<std::sync::Mutex<dyn Boot + Send>>,
}

impl Clone for __Handle_Boot {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Boot {
    pub fn new<__H: Boot + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Boot + Send>>) -> Self {
        Self { inner }
    }
}

impl Boot for __Handle_Boot {
    fn boot(&mut self, done: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().boot(done)
    }
    fn stop(&mut self, done: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().stop(done)
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
    __dep_Transport: crate::net::__Handle_Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Booting>,
}

impl Booting {
    pub fn new(at: NodeEndpoint, all: Vec<NodeEndpoint>, net: usize, __dep_Transport: crate::net::__Handle_Transport) -> Self {
        Self {
            at,
            all,
            net,
            nodes: None,
            __dep_Transport,
            __mailbox_capacity: 2,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Boot for Booting {

    fn boot(&mut self, done: crate::scheduler::SalvoReply) {
        let mut p = crate::scheduler::salvo_pool(((1) as usize));
        let mut _connected = connect__3(&mut self.__dep_Transport, self.at.clone(), ({ let __h = Sending::new(crate::net::__Handle_Transport::new(MemTransport::new(self.at.clone(), self.net.clone()))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sending::new(__h)), __DECODE_Sending); __a }), ({ let __h = Receiving::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Receiving::new(__h)), __DECODE_Receiving); __a }));
        let mut group = ({ let __h = StaticNodeGroup::new("cluster".to_string(), self.all.clone(), crate::net::__Handle_Transport::new(MemTransport::new(self.at.clone(), self.net.clone()))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_StaticNodeGroup::new(__h)), __DECODE_StaticNodeGroup); crate::scheduler::salvo_send(__a, Box::new(__Priv_StaticNodeGroup::Init)); __a });
        self.nodes = Some(group.clone());
        let mut seq = crate::net::open_group(Protocol { name: "Sequencer".to_string(), hash: crate::__PROTO_Sequencer.to_string() }, group.clone());
        let mut mine = { let __spawned = ({ let __h = Sequencing::new("b".to_string()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sequencing::new(__h)), __DECODE_Sequencing); __a }); crate::scheduler::salvo_send_wire((seq).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
        let mut stock = crate::net::open_group(Protocol { name: "Inventory".to_string(), hash: crate::__PROTO_Inventory.to_string() }, group.clone());
        { let __spawned = ({ let __h = Stocking::new("shard-b".to_string()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Stocking::new(__h)), __DECODE_Stocking); __a }); crate::scheduler::salvo_send_wire((stock).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
        let mut index = crate::net::open_group(Protocol { name: "Search".to_string(), hash: crate::__PROTO_Search.to_string() }, group.clone());
        { let __spawned = ({ let __h = Indexing::new(vec!["salvo".to_string(), "actors".to_string(), "salvo".to_string(), "nodes".to_string()]); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Indexing::new(__h)), __DECODE_Indexing); __a }); crate::scheduler::salvo_send_wire((index).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
        let mut looks = crate::net::open_group(Protocol { name: "Lookup".to_string(), hash: crate::__PROTO_Lookup.to_string() }, group);
        let mut timer = ({ let __h = DefaultTimer::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_DefaultTimer::new(__h)), __DECODE_DefaultTimer); __a });
        { let __spawned = ({ let __h = SlowLooking::new("b (slow)".to_string(), timer); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_SlowLooking::new(__h)), __DECODE_SlowLooking); __a }); crate::scheduler::salvo_send_wire((looks).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
        crate::scheduler::salvo_reply_wire::<usize>(done, mine);
    }

    fn stop(&mut self, done: crate::scheduler::SalvoReply) {
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

pub struct __Actor_Booting {
    handler: Booting,
}

impl __Actor_Booting {
    pub fn new(handler: Booting) -> Self {
        Self { handler }
    }
}

impl __Actor_Booting {
    fn __dispatch(&mut self, msg: crate::__Msg_Boot) {
        match msg {
            crate::__Msg_Boot::Boot(done) => crate::Boot::boot(&mut self.handler, done),
            crate::__Msg_Boot::Stop(done) => crate::Boot::stop(&mut self.handler, done),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Booting {
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
    crate::scheduler::salvo_set_protocols(vec![("ActorChanges".to_string(), crate::net::__PROTO_ActorChanges.to_string()), ("ActorGroup".to_string(), crate::net::__PROTO_ActorGroup.to_string()), ("Boot".to_string(), crate::__PROTO_Boot.to_string()), ("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Gather".to_string(), crate::__PROTO_Gather.to_string()), ("Inbound".to_string(), crate::net::__PROTO_Inbound.to_string()), ("Inventory".to_string(), crate::__PROTO_Inventory.to_string()), ("Lookup".to_string(), crate::__PROTO_Lookup.to_string()), ("MemNet".to_string(), crate::net::__PROTO_MemNet.to_string()), ("NodeChanges".to_string(), crate::net::__PROTO_NodeChanges.to_string()), ("NodeGroup".to_string(), crate::net::__PROTO_NodeGroup.to_string()), ("Outbound".to_string(), crate::net::__PROTO_Outbound.to_string()), ("Race".to_string(), crate::__PROTO_Race.to_string()), ("Search".to_string(), crate::__PROTO_Search.to_string()), ("Sequencer".to_string(), crate::__PROTO_Sequencer.to_string()), ("Timer".to_string(), crate::time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::time::__PROTO_TimerCtl.to_string())]);
    let mut console = crate::core_console::__Handle_Console::new(StdOutConsole::new());
    let mut a = NodeEndpoint { host: "a".to_string(), port: 1 };
    let mut b = NodeEndpoint { host: "b".to_string(), port: 1 };
    let mut all = vec![a.clone(), b.clone()];
    let mut network = ({ let __h = MemNetwork::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, Box::new(__Actor_MemNetwork::new(__h)), __DECODE_MemNetwork); __a });
    let mut transport = crate::net::__Handle_Transport::new(MemTransport::new(a.clone(), network.clone()));
    let mut p = crate::scheduler::salvo_pool(((2) as usize));
    let mut _connected = connect__2(&mut transport, a.clone(), p.clone());
    let mut nodes = ({ let __h = StaticNodeGroup::new("cluster".to_string(), all.clone(), transport.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_StaticNodeGroup::new(__h)), __DECODE_StaticNodeGroup); crate::scheduler::salvo_send(__a, Box::new(__Priv_StaticNodeGroup::Init)); __a });
    let mut seq = crate::net::open_group(Protocol { name: "Sequencer".to_string(), hash: crate::__PROTO_Sequencer.to_string() }, nodes.clone());
    { let __spawned = ({ let __h = Sequencing::new("a".to_string()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sequencing::new(__h)), __DECODE_Sequencing); __a }); crate::scheduler::salvo_send_wire((seq).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
    let mut stock = crate::net::open_group(Protocol { name: "Inventory".to_string(), hash: crate::__PROTO_Inventory.to_string() }, nodes.clone());
    { let __spawned = ({ let __h = Stocking::new("shard-a".to_string()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Stocking::new(__h)), __DECODE_Stocking); __a }); crate::scheduler::salvo_send_wire((stock).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
    let mut index = crate::net::open_group(Protocol { name: "Search".to_string(), hash: crate::__PROTO_Search.to_string() }, nodes.clone());
    { let __spawned = ({ let __h = Indexing::new(vec!["salvo".to_string(), "is".to_string(), "salvo".to_string()]); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Indexing::new(__h)), __DECODE_Indexing); __a }); crate::scheduler::salvo_send_wire((index).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
    let mut looks = crate::net::open_group(Protocol { name: "Lookup".to_string(), hash: crate::__PROTO_Lookup.to_string() }, nodes.clone());
    { let __spawned = ({ let __h = Looking::new("a".to_string()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Looking::new(__h)), __DECODE_Looking); __a }); crate::scheduler::salvo_send_wire((looks).clone(), crate::net::__Msg_ActorGroup::Join(__spawned), crate::net::__PROTO_ActorGroup); __spawned };
    let mut pb = crate::scheduler::salvo_pool_at((NodeId { id: crate::scheduler::salvo_new_node() as i64 }).id as u64, (1) as usize);
    let mut booter = ({ let __h = Booting::new(b.clone(), all.clone(), network.clone(), crate::net::__Handle_Transport::new(MemTransport::new(b.clone(), network.clone()))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(pb, __cap as usize, Box::new(__Actor_Booting::new(__h)), __DECODE_Booting); __a });
    let mut remote_seq = {
        let (mut done, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(booter, crate::__Msg_Boot::Boot(done), crate::__PROTO_Boot);
        *crate::scheduler::salvo_wait(__wid).downcast::<usize>().expect("the awaited answer")
    };
    let mut timer = ({ let __h = DefaultTimer::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, Box::new(__Actor_DefaultTimer::new(__h)), __DECODE_DefaultTimer); __a });
    settle(&(timer.clone()));
    let mut members = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<Node>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(nodes, crate::net::__Msg_NodeGroup::Members(out), crate::net::__PROTO_NodeGroup);
        *crate::scheduler::salvo_wait(__wid).downcast::<Vec<Node>>().expect("the awaited answer")
    };
    println(&mut console, &(format!("nodes: {}, sequencers: {}", (members.len() as i32) + 1, ({
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(seq, crate::net::__Msg_ActorGroup::Members(out), crate::net::__PROTO_ActorGroup);
        *crate::scheduler::salvo_wait(__wid).downcast::<Vec<usize>>().expect("the awaited answer")
    }.len() as i32))));
    println(&mut console, &(format!("singleton (b's sequencer is remote: {}):", !eq__2(&(NodeId { id: crate::scheduler::salvo_addr_identity((remote_seq).clone()).node as i64 }), &(NodeId { id: crate::scheduler::salvo_here_node() as i64 })))));
    let mut leader = crate::net::__Handle_Leader::new(LastHost::new(nodes.clone(), a.clone()));
    two_ids(&mut leader, &mut console, seq.clone());
    println(&mut console, &("sharded:".to_string()));
    shop(&mut console, stock);
    println(&mut console, &("scatter:".to_string()));
    let mut search = crate::__Handle_Search::new(Scattering::new(index.clone(), ({ let __h = Gathering::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Gathering::new(__h)), __DECODE_Gathering); __a })));
    println(&mut console, &(format!("  salvo: {}, actors: {}, none: {}", count(&mut search, "salvo".to_string()), count(&mut search, "actors".to_string()), count(&mut search, "none".to_string()))));
    println(&mut console, &("hedge:".to_string()));
    let mut lookup = crate::__Handle_Lookup::new(Hedging::new(looks.clone(), ({ let __h = Racing::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Racing::new(__h)), __DECODE_Racing); __a })));
    println(&mut console, &(format!("  {}", find(&mut lookup, "k1".to_string()))));
    let mut _stopped = {
        let (mut done, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<bool>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(booter, crate::__Msg_Boot::Stop(done), crate::__PROTO_Boot);
        *crate::scheduler::salvo_wait(__wid).downcast::<bool>().expect("the awaited answer")
    };
    settle(&(timer.clone()));
    println(&mut console, &("after b left:".to_string()));
    println(&mut console, &(format!("  sequencers: {}", ({
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(seq, crate::net::__Msg_ActorGroup::Members(out), crate::net::__PROTO_ActorGroup);
        *crate::scheduler::salvo_wait(__wid).downcast::<Vec<usize>>().expect("the awaited answer")
    }.len() as i32))));
    two_ids(&mut leader, &mut console, seq);
}

pub struct __Route_Inventory {
    group: usize,
    __dep_Pick: crate::net::__Handle_Pick,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Inventory>,
}

impl __Route_Inventory {
    pub fn new(group: usize, __dep_Pick: crate::net::__Handle_Pick) -> Self {
        Self {
            group,
            __dep_Pick,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Inventory for __Route_Inventory {

    fn reserve(&mut self, sku: String, qty: i32, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to__2(&mut self.__dep_Pick, &self.group, crate::scheduler::salvo_key_hash(&crate::wire::salvo_encode(&sku)));
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Inventory::Reserve(sku, qty, out), crate::__PROTO_Inventory);
    }
}

pub enum __Cont___Route_Inventory {
    Reserve(String, i32),
}

pub struct __Actor___Route_Inventory {
    handler: __Route_Inventory,
}

impl __Actor___Route_Inventory {
    pub fn new(handler: __Route_Inventory) -> Self {
        Self { handler }
    }
}

impl __Actor___Route_Inventory {
    fn __dispatch(&mut self, msg: crate::__Msg_Inventory) {
        match msg {
            crate::__Msg_Inventory::Reserve(sku, qty, out) => crate::Inventory::reserve(&mut self.handler, sku, qty, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor___Route_Inventory {
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
    __dep_Pick: crate::net::__Handle_Pick,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Lookup>,
}

impl __Route_Lookup {
    pub fn new(group: usize, __dep_Pick: crate::net::__Handle_Pick) -> Self {
        Self {
            group,
            __dep_Pick,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Lookup for __Route_Lookup {

    fn lookup(&mut self, key: String, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to(&mut self.__dep_Pick, &self.group);
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Lookup::Lookup(key, out), crate::__PROTO_Lookup);
    }
}

pub enum __Cont___Route_Lookup {
    Lookup(String),
}

pub struct __Actor___Route_Lookup {
    handler: __Route_Lookup,
}

impl __Actor___Route_Lookup {
    pub fn new(handler: __Route_Lookup) -> Self {
        Self { handler }
    }
}

impl __Actor___Route_Lookup {
    fn __dispatch(&mut self, msg: crate::__Msg_Lookup) {
        match msg {
            crate::__Msg_Lookup::Lookup(key, out) => crate::Lookup::lookup(&mut self.handler, key, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor___Route_Lookup {
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
    __dep_Pick: crate::net::__Handle_Pick,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Search>,
}

impl __Route_Search {
    pub fn new(group: usize, __dep_Pick: crate::net::__Handle_Pick) -> Self {
        Self {
            group,
            __dep_Pick,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Search for __Route_Search {

    fn query(&mut self, word: String, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to(&mut self.__dep_Pick, &self.group);
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Search::Query(word, out), crate::__PROTO_Search);
    }
}

pub enum __Cont___Route_Search {
    Query(String),
}

pub struct __Actor___Route_Search {
    handler: __Route_Search,
}

impl __Actor___Route_Search {
    pub fn new(handler: __Route_Search) -> Self {
        Self { handler }
    }
}

impl __Actor___Route_Search {
    fn __dispatch(&mut self, msg: crate::__Msg_Search) {
        match msg {
            crate::__Msg_Search::Query(word, out) => crate::Search::query(&mut self.handler, word, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor___Route_Search {
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
    __dep_Pick: crate::net::__Handle_Pick,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont___Route_Sequencer>,
}

impl __Route_Sequencer {
    pub fn new(group: usize, __dep_Pick: crate::net::__Handle_Pick) -> Self {
        Self {
            group,
            __dep_Pick,
            __mailbox_capacity: 1,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Sequencer for __Route_Sequencer {

    fn next(&mut self, out: crate::scheduler::SalvoReply) {
        let mut __target = route_to(&mut self.__dep_Pick, &self.group);
        crate::scheduler::salvo_send_wire(__target, crate::__Msg_Sequencer::Next(out), crate::__PROTO_Sequencer);
    }
}

pub enum __Cont___Route_Sequencer {
    Next,
}

pub struct __Actor___Route_Sequencer {
    handler: __Route_Sequencer,
}

impl __Actor___Route_Sequencer {
    pub fn new(handler: __Route_Sequencer) -> Self {
        Self { handler }
    }
}

impl __Actor___Route_Sequencer {
    fn __dispatch(&mut self, msg: crate::__Msg_Sequencer) {
        match msg {
            crate::__Msg_Sequencer::Next(out) => crate::Sequencer::next(&mut self.handler, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor___Route_Sequencer {
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
