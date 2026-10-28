#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "std/core/actor.rs"]
pub mod std_core_actor;
#[path = "std/core/basic.rs"]
pub mod std_core_basic;
#[path = "std/core/buffer.rs"]
pub mod std_core_buffer;
#[path = "std/core/bytes.rs"]
pub mod std_core_bytes;
#[path = "std/core/checked.rs"]
pub mod std_core_checked;
#[path = "std/core/compare.rs"]
pub mod std_core_compare;
#[path = "std/core/console.rs"]
pub mod std_core_console;
#[path = "std/core/deque.rs"]
pub mod std_core_deque;
#[path = "std/core/index.rs"]
pub mod std_core_index;
#[path = "std/core/iterator.rs"]
pub mod std_core_iterator;
#[path = "std/core/list.rs"]
pub mod std_core_list;
#[path = "std/core/map.rs"]
pub mod std_core_map;
#[path = "std/core/range.rs"]
pub mod std_core_range;
#[path = "std/core/set.rs"]
pub mod std_core_set;
#[path = "std/core/sorted.rs"]
pub mod std_core_sorted;
#[path = "std/core/string.rs"]
pub mod std_core_string;
#[path = "std/runtime.rs"]
pub mod std_runtime;
#[path = "std/runtime/routing.rs"]
pub mod std_runtime_routing;
#[path = "std/runtime/timers.rs"]
pub mod std_runtime_timers;
#[path = "std/time.rs"]
pub mod std_time;
#[path = "platform/std/core/buffer.rs"]
pub mod platform_std_core_buffer;
#[path = "platform/std/core/bytes.rs"]
pub mod platform_std_core_bytes;
#[path = "platform/std/core/console.rs"]
pub mod platform_std_core_console;
#[path = "platform/std/core/deque.rs"]
pub mod platform_std_core_deque;
#[path = "platform/std/core/list.rs"]
pub mod platform_std_core_list;
#[path = "platform/std/core/map.rs"]
pub mod platform_std_core_map;
#[path = "platform/std/core/set.rs"]
pub mod platform_std_core_set;
#[path = "platform/std/core/sorted.rs"]
pub mod platform_std_core_sorted;
#[path = "platform/std/core/string.rs"]
pub mod platform_std_core_string;
#[path = "platform/std/runtime/routing.rs"]
pub mod platform_std_runtime_routing;
#[path = "platform/std/runtime.rs"]
pub mod platform_std_runtime;
#[path = "platform/std/time.rs"]
pub mod platform_std_time;

use crate::std_time::DefaultClock;
use crate::std_time::DefaultTicker;
use crate::std_time::Duration;
use crate::std_time::Fired;
use crate::std_core_actor::Idle;
use crate::std_time::Instant;
use crate::std_time::Tick;
use crate::std_time::Ticker;
use crate::std_time::Timer;
use crate::std_time::between__Tick_Tick;
use crate::std_time::cmp__Duration_Duration;
use crate::std_time::elapsed;
use crate::std_time::epoch_milli;
use crate::std_time::millis;
use crate::std_time::minus__Duration_Duration;
use crate::std_time::minutes;
use crate::std_time::nanos;
use crate::std_core_actor::on_idle;
use crate::std_time::plus__Duration_Duration;
use crate::std_time::plus__Instant_Duration;
use crate::std_core_actor::pool;
use crate::std_core_console::println;
use crate::std_time::seconds;
use crate::std_time::times;
use crate::std_time::to_epoch_second;
use crate::std_time::to_millis;
use crate::std_time::to_str;


pub fn verdict(started: &crate::std_time::Tick, at: &crate::std_time::Tick, budget: &crate::std_time::Duration) -> String {
    let mut took: crate::std_time::Duration = crate::std_time::between__Tick_Tick(started, at);
    if (crate::std_time::cmp__Duration_Duration(&took, budget) > 0i32) {
        return format!("late by {}", crate::std_time::to_str(&crate::std_time::minus__Duration_Duration(&took, budget)));
    };
    return format!("in time, {} to spare", crate::std_time::to_str(&crate::std_time::minus__Duration_Duration(budget, &took)));
}

pub fn overdue(ticker: &crate::std_time::Ticker, started: &crate::std_time::Tick, budget: &crate::std_time::Duration) -> bool {
    return (crate::std_time::cmp__Duration_Duration(&crate::std_time::elapsed(ticker, started), budget) > 0i32);
}

pub struct SteppingTicker {
    step: crate::std_time::Duration,
    at: i64,
}

impl SteppingTicker {
    pub fn new(step: crate::std_time::Duration) -> Self {
        Self {
            step: step.clone(),
            at: 0i64
        }
    }
}

impl crate::std_time::__Stateful_Ticker for SteppingTicker {
    fn tick(&mut self) -> crate::std_time::Tick {
        self.at = i64::wrapping_add(self.at, self.step.nanos);
        return crate::std_time::Tick { nanos: self.at };
    }
}

pub trait __Stateless_Session: Send + Sync {
    fn open(&self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply);
    fn expire(&self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired);
}

pub trait __Stateful_Session: Send {
    fn open(&mut self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply);
    fn expire(&mut self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired);
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
    pub fn open(&self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Session::Shared(h) => h.open(started, budget, out),
            __Inner_Session::Locked(h) => h.lock().unwrap().open(started, budget, out),
        }
    }
    pub fn expire(&self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired) {
        match &self.inner {
            __Inner_Session::Shared(h) => h.expire(started, budget, out, f),
            __Inner_Session::Locked(h) => h.lock().unwrap().expire(started, budget, out, f),
        }
    }
}

pub struct Sessions {
    __dep0: crate::std_time::Timer,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sessions>,
}

impl Sessions {
    pub fn new(__dep0: crate::std_time::Timer) -> Self {
        Self {
            __dep0,
            __mailbox_capacity: 8i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::__Stateful_Session for Sessions {
    fn open(&mut self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply) {
        self.__dep0.after(crate::std_time::plus__Duration_Duration(&budget, &crate::std_time::seconds(1i64)), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Sessions::Expire(started, budget, out)); __r });
    }
    fn expire(&mut self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, crate::verdict(&started, &f.at, &budget));
    }
}

pub struct __Actor_Sessions {
    handler: Sessions,
}

impl __Actor_Sessions {
    pub fn new(handler: Sessions) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Sessions {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Session>().expect("message of this protocol");
        crate::__dispatch_Sessions_Session(&mut self.handler, msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Sessions::Open(started, budget) => crate::__dispatch_Sessions_Session(&mut self.handler, crate::__Msg_Session::Open(started, budget, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Sessions::Expire(started, budget, out) => crate::__dispatch_Sessions_Session(&mut self.handler, crate::__Msg_Session::Expire(started, budget, out, *value.downcast::<crate::std_time::Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sessions::Open{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Sessions::Expire{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_time::Fired>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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

impl crate::std_time::__Stateless_Ticker for TestTicker {
    fn tick(&self) -> crate::std_time::Tick {
        let mut fired: crate::std_time::Fired = {
            let (mut answer, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_time::Fired>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.timer, crate::std_time::__Msg_Timer::After(crate::std_time::nanos(0i64), answer), crate::std_time::__PROTO_Timer);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::std_time::Fired>().expect("the awaited answer")
        };
        return fired.at.clone();
    }
}

pub trait __Stateless_Sleeper: Send + Sync {
    fn nap(&self, wait: crate::std_time::Duration, out: crate::scheduler::SalvoReply);
    fn woke(&self, started: crate::std_time::Tick, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired);
}

pub trait __Stateful_Sleeper: Send {
    fn nap(&mut self, wait: crate::std_time::Duration, out: crate::scheduler::SalvoReply);
    fn woke(&mut self, started: crate::std_time::Tick, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired);
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
    pub fn nap(&self, wait: crate::std_time::Duration, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Sleeper::Shared(h) => h.nap(wait, out),
            __Inner_Sleeper::Locked(h) => h.lock().unwrap().nap(wait, out),
        }
    }
    pub fn woke(&self, started: crate::std_time::Tick, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired) {
        match &self.inner {
            __Inner_Sleeper::Shared(h) => h.woke(started, out, f),
            __Inner_Sleeper::Locked(h) => h.lock().unwrap().woke(started, out, f),
        }
    }
}

pub struct Napping {
    __dep0: crate::std_time::Timer,
    __dep1: crate::std_time::Ticker,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Napping>,
}

impl Napping {
    pub fn new(__dep0: crate::std_time::Timer, __dep1: crate::std_time::Ticker) -> Self {
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
    fn nap(&mut self, wait: crate::std_time::Duration, out: crate::scheduler::SalvoReply) {
        self.__dep0.after(wait, { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Napping::Woke(self.__dep1.tick(), out)); __r });
    }
    fn woke(&mut self, started: crate::std_time::Tick, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, format!("napped {}", crate::std_time::to_str(&crate::std_time::elapsed(&self.__dep1, &started))));
    }
}

pub struct __Actor_Napping {
    handler: Napping,
}

impl __Actor_Napping {
    pub fn new(handler: Napping) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Napping {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::__Msg_Sleeper>().expect("message of this protocol");
        crate::__dispatch_Napping_Sleeper(&mut self.handler, msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Napping::Nap(wait) => crate::__dispatch_Napping_Sleeper(&mut self.handler, crate::__Msg_Sleeper::Nap(wait, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_Napping::Woke(started, out) => crate::__dispatch_Napping_Sleeper(&mut self.handler, crate::__Msg_Sleeper::Woke(started, out, *value.downcast::<crate::std_time::Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Napping::Nap{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Napping::Woke{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_time::Fired>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::std_core_actor::__PROTO_Faults.to_string()), ("Session".to_string(), crate::__PROTO_Session.to_string()), ("Sleeper".to_string(), crate::__PROTO_Sleeper.to_string()), ("Timer".to_string(), crate::std_time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::std_time::__PROTO_TimerCtl.to_string()), ("Wheel".to_string(), crate::std_runtime_timers::__PROTO_Wheel.to_string())]);
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut budget: crate::std_time::Duration = crate::std_time::millis(1500i64);
    crate::std_core_console::println(&__handle_2, &format!("budget {}, doubled {}, in millis {}", crate::std_time::to_str(&budget), crate::std_time::to_str(&crate::std_time::times(&budget, 2i64)), crate::std_time::to_millis(&budget)));
    let mut stamp: crate::std_time::Instant = crate::std_time::epoch_milli(1700000000000i64);
    crate::std_core_console::println(&__handle_2, &format!("stamp {}s, a minute later {}s", crate::std_time::to_epoch_second(&stamp), crate::std_time::to_epoch_second(&crate::std_time::plus__Instant_Duration(&stamp, &crate::std_time::minutes(1i64)))));
    let mut __use_3: crate::std_time::DefaultClock = crate::std_time::DefaultClock::new();
    let __handle_4 = crate::std_time::Clock::locked(__use_3);
    let mut __use_5: crate::std_time::DefaultTicker = crate::std_time::DefaultTicker::new();
    let __handle_6 = crate::std_time::Ticker::shared(__use_5);
    crate::std_core_console::println(&__handle_2, &format!("wall clock is set: {}", (crate::std_time::to_epoch_second(&__handle_4.now()) > 1600000000i64)));
    let mut __use_7: crate::SteppingTicker = crate::SteppingTicker::new(crate::std_time::millis(500i64));
    let __handle_8 = crate::std_time::Ticker::locked(__use_7);
    let mut started: crate::std_time::Tick = __handle_8.tick();
    crate::std_core_console::println(&__handle_2, &format!("overdue after one more read: {}", crate::overdue(&__handle_8, &started, &budget)));
    crate::std_core_console::println(&__handle_2, &format!("overdue after three: {} {} {}", crate::overdue(&__handle_8, &started, &budget), crate::overdue(&__handle_8, &started, &budget), crate::overdue(&__handle_8, &started, &budget)));
    crate::std_core_console::println(&__handle_2, &crate::verdict(&crate::std_time::Tick { nanos: 0i64 }, &crate::std_time::Tick { nanos: 1000000000i64 }, &budget));
    crate::std_core_console::println(&__handle_2, &crate::verdict(&crate::std_time::Tick { nanos: 0i64 }, &crate::std_time::Tick { nanos: 2000000000i64 }, &budget));
    let mut p: usize = crate::std_core_actor::pool(1i32);
    let mut __destructured_9: (usize, usize) = ({ let __h = crate::std_time::ManualTime::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, std::boxed::Box::new(crate::std_time::__Actor_ManualTime::new(__h)), crate::std_time::__DECODE_ManualTime); (__a, __a) });
    let mut timer: usize = __destructured_9.0;
    let mut ctl: usize = __destructured_9.1;
    let mut sessions: usize = ({ let __h = crate::Sessions::new(crate::std_time::Timer::shared(crate::std_time::__Stub_Timer::new(timer))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, std::boxed::Box::new(crate::__Actor_Sessions::new(__h)), crate::__DECODE_Sessions); __a });
    let mut outcome: String = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(sessions, crate::__Msg_Session::Open(crate::std_time::Tick { nanos: 0i64 }, budget, answer), crate::__PROTO_Session);
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_actor::Idle>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::std_core_actor::on_idle(p, settled);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::std_core_actor::Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(ctl, crate::std_time::__Msg_TimerCtl::Advance(crate::std_time::millis(2500i64)), crate::std_time::__PROTO_TimerCtl);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    crate::std_core_console::println(&__handle_2, &format!("session: {}", outcome));
    let mut sleeper: usize = ({ let __h = crate::Napping::new(crate::std_time::Timer::shared(crate::std_time::__Stub_Timer::new(timer)), crate::std_time::Ticker::shared(crate::TestTicker::new(timer))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_thread(), __cap as usize, std::boxed::Box::new(crate::__Actor_Napping::new(__h)), crate::__DECODE_Napping); __a });
    let mut napped: String = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(sleeper, crate::__Msg_Sleeper::Nap(crate::std_time::seconds(2i64), answer), crate::__PROTO_Sleeper);
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_actor::Idle>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::std_core_actor::on_idle(p, settled);
            *crate::scheduler::salvo_wait(__wid).downcast::<crate::std_core_actor::Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(ctl, crate::std_time::__Msg_TimerCtl::Advance(crate::std_time::seconds(2i64)), crate::std_time::__PROTO_TimerCtl);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    crate::std_core_console::println(&__handle_2, &napped);
}

pub enum __Msg_Session {
    Open(crate::std_time::Tick, crate::std_time::Duration, crate::scheduler::SalvoReply),
    Expire(crate::std_time::Tick, crate::std_time::Duration, crate::scheduler::SalvoReply, crate::std_time::Fired),
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

/// [actor-protocol-hash] The canonical hash of `Session`.
pub const __PROTO_Session: &str = "7ed70285b7e6568c";

pub enum __Msg_Sleeper {
    Nap(crate::std_time::Duration, crate::scheduler::SalvoReply),
    Woke(crate::std_time::Tick, crate::scheduler::SalvoReply, crate::std_time::Fired),
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

/// [actor-protocol-hash] The canonical hash of `Sleeper`.
pub const __PROTO_Sleeper: &str = "2385b53950dcbd9c";

pub enum __Cont_Sessions {
    Open(crate::std_time::Tick, crate::std_time::Duration),
    Expire(crate::std_time::Tick, crate::std_time::Duration, crate::scheduler::SalvoReply),
}

pub enum __Cont_Napping {
    Nap(crate::std_time::Duration),
    Woke(crate::std_time::Tick, crate::scheduler::SalvoReply),
}

#[derive(Clone)]
pub struct __Stub_Session {
    addr: usize,
}

impl __Stub_Session {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::__Stateless_Session for __Stub_Session {
    fn open(&self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::__Msg_Session::Open(started, budget, out), crate::__PROTO_Session);
    }
    fn expire(&self, started: crate::std_time::Tick, budget: crate::std_time::Duration, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired) {
        crate::scheduler::salvo_send_wire(self.addr, crate::__Msg_Session::Expire(started, budget, out, f), crate::__PROTO_Session);
    }
}

#[derive(Clone)]
pub struct __Stub_Sleeper {
    addr: usize,
}

impl __Stub_Sleeper {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::__Stateless_Sleeper for __Stub_Sleeper {
    fn nap(&self, wait: crate::std_time::Duration, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::__Msg_Sleeper::Nap(wait, out), crate::__PROTO_Sleeper);
    }
    fn woke(&self, started: crate::std_time::Tick, out: crate::scheduler::SalvoReply, f: crate::std_time::Fired) {
        crate::scheduler::salvo_send_wire(self.addr, crate::__Msg_Sleeper::Woke(started, out, f), crate::__PROTO_Sleeper);
    }
}

pub fn __dispatch_Sessions_Session(__handler: &mut crate::Sessions, mut __msg: crate::__Msg_Session) {
    if matches!(__msg, crate::__Msg_Session::Open(..)) {
        let crate::__Msg_Session::Open(started, budget, out) = __msg else { unreachable!() };
        crate::__Stateful_Session::open(&mut *__handler, started, budget, out);
    } else {
        let crate::__Msg_Session::Expire(started, budget, out, f) = __msg else { unreachable!() };
        crate::__Stateful_Session::expire(&mut *__handler, started, budget, out, f);
    };
}

pub fn __dispatch_Napping_Sleeper(__handler: &mut crate::Napping, mut __msg: crate::__Msg_Sleeper) {
    if matches!(__msg, crate::__Msg_Sleeper::Nap(..)) {
        let crate::__Msg_Sleeper::Nap(wait, out) = __msg else { unreachable!() };
        crate::__Stateful_Sleeper::nap(&mut *__handler, wait, out);
    } else {
        let crate::__Msg_Sleeper::Woke(started, out, f) = __msg else { unreachable!() };
        crate::__Stateful_Sleeper::woke(&mut *__handler, started, out, f);
    };
}
