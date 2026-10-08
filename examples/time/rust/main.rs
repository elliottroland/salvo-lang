#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/actor.rs"]
pub mod core_actor;
#[path = "core/basic.rs"]
pub mod core_basic;
#[path = "core/buffer.rs"]
pub mod core_buffer;
#[path = "core/bytes.rs"]
pub mod core_bytes;
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
#[path = "core/range.rs"]
pub mod core_range;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "runtime/routing.rs"]
pub mod runtime_routing;
#[path = "runtime/timers.rs"]
pub mod runtime_timers;
#[path = "time.rs"]
pub mod time;
#[path = "platform/core/buffer.rs"]
pub mod platform_core_buffer;
#[path = "platform/core/bytes.rs"]
pub mod platform_core_bytes;
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
#[path = "platform/runtime/routing.rs"]
pub mod platform_runtime_routing;
#[path = "platform/runtime.rs"]
pub mod platform_runtime;
#[path = "platform/time.rs"]
pub mod platform_time;

use crate::time::DefaultClock;
use crate::time::DefaultTicker;
use crate::time::Duration;
use crate::__Msg_Session::Expire;
use crate::time::Fired;
use crate::core_actor::Idle;
use crate::time::Instant;
use crate::__Msg_Sleeper::Nap;
use crate::__Msg_Session::Open;
use crate::time::Tick;
use crate::time::Ticker;
use crate::time::Timer;
use crate::__Msg_Sleeper::Woke;
use crate::time::between__Tick_Tick;
use crate::time::cmp__Duration_Duration;
use crate::time::elapsed;
use crate::time::epoch_milli;
use crate::time::millis;
use crate::time::minus__Duration_Duration;
use crate::time::minutes;
use crate::time::nanos;
use crate::core_actor::on_idle;
use crate::time::plus__Duration_Duration;
use crate::time::plus__Instant_Duration;
use crate::core_actor::pool;
use crate::core_console::println;
use crate::time::seconds;
use crate::time::times;
use crate::time::to_epoch_second;
use crate::time::to_millis;
use crate::time::to_str;


pub fn verdict(started: &crate::time::Tick, at: &crate::time::Tick, budget: &crate::time::Duration) -> String {
    let mut took: crate::time::Duration = crate::time::between__Tick_Tick(started, at);
    if (crate::time::cmp__Duration_Duration(&took, budget) > 0i32) {
        return format!("late by {}", crate::time::to_str(&crate::time::minus__Duration_Duration(&took, budget)));
    };
    return format!("in time, {} to spare", crate::time::to_str(&crate::time::minus__Duration_Duration(budget, &took)));
}

pub fn overdue(ticker: &crate::time::Ticker, started: &crate::time::Tick, budget: &crate::time::Duration) -> bool {
    return (crate::time::cmp__Duration_Duration(&crate::time::elapsed(ticker, started), budget) > 0i32);
}

pub struct SteppingTicker {
    step: crate::time::Duration,
    at: i64,
}

impl SteppingTicker {
    pub fn new(step: crate::time::Duration) -> Self {
        Self {
            step: step.clone(),
            at: 0i64
        }
    }
}

impl crate::time::__Stateful_Ticker for SteppingTicker {
    fn tick(&mut self) -> crate::time::Tick {
        self.at = i64::wrapping_add(self.at, self.step.nanos);
        return crate::time::Tick { nanos: self.at };
    }
}

pub trait __Stateless_Session: Send + Sync {
    fn open(&self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply);
    fn expire(&self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply, f: crate::time::Fired);
}

pub trait __Stateful_Session: Send {
    fn open(&mut self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply);
    fn expire(&mut self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply, f: crate::time::Fired);
}

pub struct Session {
    inner: __Inner_Session,
}

pub enum __Inner_Session {
    Shared(std::sync::Arc<dyn __Stateless_Session>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Session>>),
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Session::Shared(h) => __Inner_Session::Shared(h.clone()),
            __Inner_Session::Locked(h) => __Inner_Session::Locked(h.clone()),
        } }
    }
}

impl Session {
    pub fn shared<__H: __Stateless_Session + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Session::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Session>) -> Self {
        Self { inner: __Inner_Session::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Session + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Session::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Session>>) -> Self {
        Self { inner: __Inner_Session::Locked(inner) }
    }
    pub fn open(&self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Session::Shared(h) => h.open(started, budget, out),
            __Inner_Session::Locked(h) => h.lock().unwrap().open(started, budget, out),
        }
    }
    pub fn expire(&self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply, f: crate::time::Fired) {
        match &self.inner {
            __Inner_Session::Shared(h) => h.expire(started, budget, out, f),
            __Inner_Session::Locked(h) => h.lock().unwrap().expire(started, budget, out, f),
        }
    }
}

pub enum __Msg_Session {
    Open(crate::time::Tick, crate::time::Duration, crate::scheduler::SalvoReply),
    Expire(crate::time::Tick, crate::time::Duration, crate::scheduler::SalvoReply, crate::time::Fired),
}

impl crate::wire::__Wire for __Msg_Session {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Session::Open(__p0, __p1, __p2) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
            __Msg_Session::Expire(__p0, __p1, __p2, __p3) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
                crate::wire::__Wire::__enc(__p3, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Session::Open(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_Session::Expire(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Session`.
pub const __PROTO_Session: &str = "7ed70285b7e6568c";

pub struct __Stub_Session {
    addr: usize,
}

impl __Stub_Session {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Session for __Stub_Session {
    fn open(&self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Session::Open(started, budget, out), crate::__PROTO_Session);
    }
    fn expire(&self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply, f: crate::time::Fired) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Session::Expire(started, budget, out, f), crate::__PROTO_Session);
    }
}

pub struct Sessions {
    __dep0: crate::time::Timer,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sessions>,
}

impl Sessions {
    pub fn new(__dep0: crate::time::Timer) -> Self {
        Self {
            __dep0,
            __mailbox_capacity: 8i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::__Stateful_Session for Sessions {
    fn open(&mut self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply) {
        self.__dep0.after(crate::time::plus__Duration_Duration(&budget, &crate::time::seconds(1i64)), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Sessions::Expire(started.clone(), budget.clone(), out)); __r });
    }
    fn expire(&mut self, started: crate::time::Tick, budget: crate::time::Duration, out: crate::scheduler::SalvoReply, f: crate::time::Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, crate::verdict(&started, &f.at, &budget));
    }
}

pub enum __Cont_Sessions {
    Open(crate::time::Tick, crate::time::Duration),
    Expire(crate::time::Tick, crate::time::Duration, crate::scheduler::SalvoReply),
}

pub struct __Actor_Sessions {
    handler: Sessions,
}

impl __Actor_Sessions {
    pub fn new(handler: Sessions) -> Self {
        Self { handler }
    }
    fn __dispatch(&mut self, msg: crate::__Msg_Session) {
        match msg {
            crate::__Msg_Session::Open(started, budget, out) => crate::__Stateful_Session::open(&mut self.handler, started, budget, out),
            crate::__Msg_Session::Expire(started, budget, out, f) => crate::__Stateful_Session::expire(&mut self.handler, started, budget, out, f),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Sessions {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Session>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Sessions::Open(started, budget) => self.__dispatch(crate::__Msg_Session::Open(started, budget, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Sessions::Expire(started, budget, out) => self.__dispatch(crate::__Msg_Session::Expire(started, budget, out, *value.downcast::<crate::time::Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sessions::Open{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Sessions::Expire{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::time::Fired>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sessions: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sessions);
fn __decode_msg_Sessions(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::__PROTO_Session {
        return crate::wire::salvo_decode::<crate::__Msg_Session>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

#[derive(Clone)]
pub struct TestTicker {
    timer: usize,
}

impl TestTicker {
    pub fn new(timer: usize) -> Self {
        Self {
            timer
        }
    }
}

impl crate::time::__Stateless_Ticker for TestTicker {
    fn tick(&self) -> crate::time::Tick {
        let mut fired: crate::time::Fired = {
            let (mut answer, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::time::Fired>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.timer, crate::time::__Msg_Timer::After(crate::time::nanos(0i64), answer), crate::time::__PROTO_Timer);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::time::Fired>().expect("the awaited answer")
        };
        return fired.at.clone();
    }
}

pub trait __Stateless_Sleeper: Send + Sync {
    fn nap(&self, wait: crate::time::Duration, out: crate::scheduler::SalvoReply);
    fn woke(&self, started: crate::time::Tick, out: crate::scheduler::SalvoReply, f: crate::time::Fired);
}

pub trait __Stateful_Sleeper: Send {
    fn nap(&mut self, wait: crate::time::Duration, out: crate::scheduler::SalvoReply);
    fn woke(&mut self, started: crate::time::Tick, out: crate::scheduler::SalvoReply, f: crate::time::Fired);
}

pub struct Sleeper {
    inner: __Inner_Sleeper,
}

pub enum __Inner_Sleeper {
    Shared(std::sync::Arc<dyn __Stateless_Sleeper>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Sleeper>>),
}

impl Clone for Sleeper {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Sleeper::Shared(h) => __Inner_Sleeper::Shared(h.clone()),
            __Inner_Sleeper::Locked(h) => __Inner_Sleeper::Locked(h.clone()),
        } }
    }
}

impl Sleeper {
    pub fn shared<__H: __Stateless_Sleeper + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Sleeper::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Sleeper>) -> Self {
        Self { inner: __Inner_Sleeper::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Sleeper + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Sleeper::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Sleeper>>) -> Self {
        Self { inner: __Inner_Sleeper::Locked(inner) }
    }
    pub fn nap(&self, wait: crate::time::Duration, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sleeper::Shared(h) => h.nap(wait, out),
            __Inner_Sleeper::Locked(h) => h.lock().unwrap().nap(wait, out),
        }
    }
    pub fn woke(&self, started: crate::time::Tick, out: crate::scheduler::SalvoReply, f: crate::time::Fired) {
        match &self.inner {
            __Inner_Sleeper::Shared(h) => h.woke(started, out, f),
            __Inner_Sleeper::Locked(h) => h.lock().unwrap().woke(started, out, f),
        }
    }
}

pub enum __Msg_Sleeper {
    Nap(crate::time::Duration, crate::scheduler::SalvoReply),
    Woke(crate::time::Tick, crate::scheduler::SalvoReply, crate::time::Fired),
}

impl crate::wire::__Wire for __Msg_Sleeper {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Sleeper::Nap(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
            __Msg_Sleeper::Woke(__p0, __p1, __p2) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Sleeper::Nap(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_Sleeper::Woke(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Sleeper`.
pub const __PROTO_Sleeper: &str = "2385b53950dcbd9c";

pub struct __Stub_Sleeper {
    addr: usize,
}

impl __Stub_Sleeper {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Sleeper for __Stub_Sleeper {
    fn nap(&self, wait: crate::time::Duration, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Sleeper::Nap(wait, out), crate::__PROTO_Sleeper);
    }
    fn woke(&self, started: crate::time::Tick, out: crate::scheduler::SalvoReply, f: crate::time::Fired) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Sleeper::Woke(started, out, f), crate::__PROTO_Sleeper);
    }
}

pub struct Napping {
    __dep0: crate::time::Timer,
    __dep1: crate::time::Ticker,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Napping>,
}

impl Napping {
    pub fn new(__dep0: crate::time::Timer, __dep1: crate::time::Ticker) -> Self {
        Self {
            __dep0,
            __dep1,
            __mailbox_capacity: 8i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::__Stateful_Sleeper for Napping {
    fn nap(&mut self, wait: crate::time::Duration, out: crate::scheduler::SalvoReply) {
        self.__dep0.after(wait, { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Napping::Woke(self.__dep1.tick(), out)); __r });
    }
    fn woke(&mut self, started: crate::time::Tick, out: crate::scheduler::SalvoReply, f: crate::time::Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, format!("napped {}", crate::time::to_str(&crate::time::elapsed(&self.__dep1, &started))));
    }
}

pub enum __Cont_Napping {
    Nap(crate::time::Duration),
    Woke(crate::time::Tick, crate::scheduler::SalvoReply),
}

pub struct __Actor_Napping {
    handler: Napping,
}

impl __Actor_Napping {
    pub fn new(handler: Napping) -> Self {
        Self { handler }
    }
    fn __dispatch(&mut self, msg: crate::__Msg_Sleeper) {
        match msg {
            crate::__Msg_Sleeper::Nap(wait, out) => crate::__Stateful_Sleeper::nap(&mut self.handler, wait, out),
            crate::__Msg_Sleeper::Woke(started, out, f) => crate::__Stateful_Sleeper::woke(&mut self.handler, started, out, f),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Napping {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Sleeper>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Napping::Nap(wait) => self.__dispatch(crate::__Msg_Sleeper::Nap(wait, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Napping::Woke(started, out) => self.__dispatch(crate::__Msg_Sleeper::Woke(started, out, *value.downcast::<crate::time::Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Napping::Nap{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Napping::Woke{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::time::Fired>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Napping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Napping);
fn __decode_msg_Napping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::__PROTO_Sleeper {
        return crate::wire::salvo_decode::<crate::__Msg_Sleeper>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Session".to_string(), crate::__PROTO_Session.to_string()), ("Sleeper".to_string(), crate::__PROTO_Sleeper.to_string()), ("Timer".to_string(), crate::time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::time::__PROTO_TimerCtl.to_string()), ("Wheel".to_string(), crate::runtime_timers::__PROTO_Wheel.to_string())]);
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut budget: crate::time::Duration = crate::time::millis(1500i64);
    crate::core_console::println(&__handle_2, &format!("budget {}, doubled {}, in millis {}", crate::time::to_str(&budget), crate::time::to_str(&crate::time::times(&budget, 2i64)), crate::time::to_millis(&budget)));
    let mut stamp: crate::time::Instant = crate::time::epoch_milli(1700000000000i64);
    crate::core_console::println(&__handle_2, &format!("stamp {}s, a minute later {}s", crate::time::to_epoch_second(&stamp), crate::time::to_epoch_second(&crate::time::plus__Instant_Duration(&stamp, &crate::time::minutes(1i64)))));
    let mut __use_3: crate::time::DefaultClock = crate::time::DefaultClock::new();
    let __handle_4 = crate::time::Clock::locked(__use_3);
    let mut __use_5: crate::time::DefaultTicker = crate::time::DefaultTicker::new();
    let __handle_6 = crate::time::Ticker::shared(__use_5);
    crate::core_console::println(&__handle_2, &format!("wall clock is set: {}", (crate::time::to_epoch_second(&__handle_4.now()) > 1600000000i64)));
    let mut __use_7: crate::SteppingTicker = crate::SteppingTicker::new(crate::time::millis(500i64));
    let __handle_8 = crate::time::Ticker::locked(__use_7);
    let mut started: crate::time::Tick = __handle_8.tick();
    crate::core_console::println(&__handle_2, &format!("overdue after one more read: {}", crate::overdue(&__handle_8, &started, &budget)));
    crate::core_console::println(&__handle_2, &format!("overdue after three: {} {} {}", crate::overdue(&__handle_8, &started, &budget), crate::overdue(&__handle_8, &started, &budget), crate::overdue(&__handle_8, &started, &budget)));
    crate::core_console::println(&__handle_2, &crate::verdict(&crate::time::Tick { nanos: 0i64 }, &crate::time::Tick { nanos: 1000000000i64 }, &budget));
    crate::core_console::println(&__handle_2, &crate::verdict(&crate::time::Tick { nanos: 0i64 }, &crate::time::Tick { nanos: 2000000000i64 }, &budget));
    let mut p: usize = crate::core_actor::pool(1i32);
    let mut __destructured_9: (usize, usize) = ({ let __h = crate::time::ManualTime::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, std::boxed::Box::new(crate::time::__Actor_ManualTime::new(__h)), crate::time::__DECODE_ManualTime); (__a, __a) });
    let mut timer: usize = __destructured_9.0;
    let mut ctl: usize = __destructured_9.1;
    let mut sessions: usize = ({ let __h = crate::Sessions::new(crate::time::Timer::shared(crate::time::__Stub_Timer::new(timer))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, std::boxed::Box::new(crate::__Actor_Sessions::new(__h)), crate::__DECODE_Sessions); __a });
    let mut outcome: String = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(sessions, crate::__Msg_Session::Open(crate::time::Tick { nanos: 0i64 }, budget.clone(), answer), crate::__PROTO_Session);
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_actor::Idle>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::core_actor::on_idle(p, settled);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::core_actor::Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(ctl, crate::time::__Msg_TimerCtl::Advance(crate::time::millis(2500i64)), crate::time::__PROTO_TimerCtl);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    crate::core_console::println(&__handle_2, &format!("session: {}", outcome));
    let mut sleeper: usize = ({ let __h = crate::Napping::new(crate::time::Timer::shared(crate::time::__Stub_Timer::new(timer)), crate::time::Ticker::shared(crate::TestTicker::new(timer))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_thread(), __cap as usize, std::boxed::Box::new(crate::__Actor_Napping::new(__h)), crate::__DECODE_Napping); __a });
    let mut napped: String = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(sleeper, crate::__Msg_Sleeper::Nap(crate::time::seconds(2i64), answer), crate::__PROTO_Sleeper);
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_actor::Idle>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::core_actor::on_idle(p, settled);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::core_actor::Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(ctl, crate::time::__Msg_TimerCtl::Advance(crate::time::seconds(2i64)), crate::time::__PROTO_TimerCtl);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    crate::core_console::println(&__handle_2, &napped);
}
