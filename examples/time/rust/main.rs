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
#[path = "time.rs"]
pub mod time;

use crate::core_actor::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_string::*;
use crate::time::*;

pub fn verdict(started: &Tick, at: &Tick, budget: &Duration) -> String {
    let mut took = between__2(started, at);
    if cmp__2(&took, budget) > 0 {
        return format!("late by {}", to_str__5(&minus(&took, budget)));
    }
    return format!("in time, {} to spare", to_str__5(&minus(budget, &took)));
}

pub fn overdue(ticker: &mut crate::time::__Handle_Ticker, started: &Tick, budget: &Duration) -> bool {
    return cmp__2(&(elapsed(ticker, started)), budget) > 0;
}

pub struct SteppingTicker {
    step: Duration,
    at: i64,
}

impl SteppingTicker {
    pub fn new(step: Duration) -> Self {
        Self {
            step,
            at: 0i64,
        }
    }
}

impl Ticker for SteppingTicker {

    fn tick(&mut self) -> Tick {
        self.at = self.at + self.step.nanos;
        return Tick { nanos: self.at };
    }
}

pub trait Session {
    fn open(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply);
    fn expire(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired);
}

pub struct __Stub_Session {
    addr: usize,
}

impl __Stub_Session {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Session for __Stub_Session {
    fn open(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Session::Open(started, budget, out), crate::__PROTO_Session);
    }
    fn expire(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Session::Expire(started, budget, out, f), crate::__PROTO_Session);
    }
}

pub struct __Handle_Session {
    inner: std::sync::Arc<std::sync::Mutex<dyn Session + Send>>,
}

impl Clone for __Handle_Session {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Session {
    pub fn new<__H: Session + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Session + Send>>) -> Self {
        Self { inner }
    }
}

impl Session for __Handle_Session {
    fn open(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().open(started, budget, out)
    }
    fn expire(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired) {
        self.inner.lock().unwrap().expire(started, budget, out, f)
    }
}

pub enum __Msg_Session {
    Open(Tick, Duration, crate::scheduler::SalvoReply),
    Expire(Tick, Duration, crate::scheduler::SalvoReply, Fired),
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

pub struct Sessions {
    __dep_Timer: crate::time::__Handle_Timer,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sessions>,
}

impl Sessions {
    pub fn new(__dep_Timer: crate::time::__Handle_Timer) -> Self {
        Self {
            __dep_Timer,
            __mailbox_capacity: 8,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Session for Sessions {

    fn open(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply) {
        self.__dep_Timer.after(plus(&budget, &(seconds(1i64))), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Sessions::Expire(started, budget, out)); __r });
    }

    fn expire(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, verdict(&started, &f.at, &budget));
    }
}

pub enum __Cont_Sessions {
    Open(Tick, Duration),
    Expire(Tick, Duration, crate::scheduler::SalvoReply),
}

pub struct __Actor_Sessions {
    handler: Sessions,
}

impl __Actor_Sessions {
    pub fn new(handler: Sessions) -> Self {
        Self { handler }
    }
}

impl __Actor_Sessions {
    fn __dispatch(&mut self, msg: crate::__Msg_Session) {
        match msg {
            crate::__Msg_Session::Open(started, budget, out) => crate::Session::open(&mut self.handler, started, budget, out),
            crate::__Msg_Session::Expire(started, budget, out, f) => crate::Session::expire(&mut self.handler, started, budget, out, f),
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
            __Cont_Sessions::Expire(started, budget, out) => self.__dispatch(crate::__Msg_Session::Expire(started, budget, out, *value.downcast::<Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sessions::Open{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Sessions::Expire{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Fired>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sessions: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sessions);
fn __decode_msg_Sessions(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Session {
            return crate::wire::salvo_decode::<crate::__Msg_Session>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
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
            timer,
        }
    }
}

impl Ticker for TestTicker {

    fn tick(&mut self) -> Tick {
        let mut fired = {
            let (mut answer, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Fired>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.timer.clone(), crate::time::__Msg_Timer::After(nanos(0i64), answer), crate::time::__PROTO_Timer);
            *crate::scheduler::salvo_wait(__wid).downcast::<Fired>().expect("the awaited answer")
        };
        return fired.at.clone();
    }
}

pub trait Sleeper {
    fn nap(&mut self, wait: Duration, out: crate::scheduler::SalvoReply);
    fn woke(&mut self, started: Tick, out: crate::scheduler::SalvoReply, f: Fired);
}

pub struct __Stub_Sleeper {
    addr: usize,
}

impl __Stub_Sleeper {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Sleeper for __Stub_Sleeper {
    fn nap(&mut self, wait: Duration, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Sleeper::Nap(wait, out), crate::__PROTO_Sleeper);
    }
    fn woke(&mut self, started: Tick, out: crate::scheduler::SalvoReply, f: Fired) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Sleeper::Woke(started, out, f), crate::__PROTO_Sleeper);
    }
}

pub struct __Handle_Sleeper {
    inner: std::sync::Arc<std::sync::Mutex<dyn Sleeper + Send>>,
}

impl Clone for __Handle_Sleeper {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Sleeper {
    pub fn new<__H: Sleeper + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Sleeper + Send>>) -> Self {
        Self { inner }
    }
}

impl Sleeper for __Handle_Sleeper {
    fn nap(&mut self, wait: Duration, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().nap(wait, out)
    }
    fn woke(&mut self, started: Tick, out: crate::scheduler::SalvoReply, f: Fired) {
        self.inner.lock().unwrap().woke(started, out, f)
    }
}

pub enum __Msg_Sleeper {
    Nap(Duration, crate::scheduler::SalvoReply),
    Woke(Tick, crate::scheduler::SalvoReply, Fired),
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

pub struct Napping {
    __dep_Timer: crate::time::__Handle_Timer,
    __dep_Ticker: crate::time::__Handle_Ticker,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Napping>,
}

impl Napping {
    pub fn new(__dep_Timer: crate::time::__Handle_Timer, __dep_Ticker: crate::time::__Handle_Ticker) -> Self {
        Self {
            __dep_Timer,
            __dep_Ticker,
            __mailbox_capacity: 8,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Sleeper for Napping {

    fn nap(&mut self, wait: Duration, out: crate::scheduler::SalvoReply) {
        self.__dep_Timer.after(wait, { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Napping::Woke(self.__dep_Ticker.tick(), out)); __r });
    }

    fn woke(&mut self, started: Tick, out: crate::scheduler::SalvoReply, f: Fired) {
        crate::scheduler::salvo_reply_wire::<String>(out, format!("napped {}", to_str__5(&elapsed(&mut self.__dep_Ticker, &started))));
    }
}

pub enum __Cont_Napping {
    Nap(Duration),
    Woke(Tick, crate::scheduler::SalvoReply),
}

pub struct __Actor_Napping {
    handler: Napping,
}

impl __Actor_Napping {
    pub fn new(handler: Napping) -> Self {
        Self { handler }
    }
}

impl __Actor_Napping {
    fn __dispatch(&mut self, msg: crate::__Msg_Sleeper) {
        match msg {
            crate::__Msg_Sleeper::Nap(wait, out) => crate::Sleeper::nap(&mut self.handler, wait, out),
            crate::__Msg_Sleeper::Woke(started, out, f) => crate::Sleeper::woke(&mut self.handler, started, out, f),
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
            __Cont_Napping::Woke(started, out) => self.__dispatch(crate::__Msg_Sleeper::Woke(started, out, *value.downcast::<Fired>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Napping::Nap{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_Napping::Woke{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Fired>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Napping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Napping);
fn __decode_msg_Napping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::__PROTO_Sleeper {
            return crate::wire::salvo_decode::<crate::__Msg_Sleeper>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Session".to_string(), crate::__PROTO_Session.to_string()), ("Sleeper".to_string(), crate::__PROTO_Sleeper.to_string()), ("Timer".to_string(), crate::time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::time::__PROTO_TimerCtl.to_string())]);
    let mut console = crate::core_console::__Handle_Console::new(StdOutConsole::new());
    let mut budget = millis(1500i64);
    println(&mut console, &(format!("budget {}, doubled {}, in millis {}", to_str__5(&budget), to_str__5(&times(&budget, 2i64)), to_millis(&budget))));
    let mut stamp = epoch_milli(1700000000000i64);
    println(&mut console, &(format!("stamp {}s, a minute later {}s", to_epoch_second(&stamp), to_epoch_second(&(plus__2(&stamp, &(minutes(1i64))))))));
    let mut clock = crate::time::__Handle_Clock::new(DefaultClock::new());
    let mut ticker = crate::time::__Handle_Ticker::new(DefaultTicker::new());
    println(&mut console, &(format!("wall clock is set: {}", to_epoch_second(&(clock.now())) > ((1600000000) as i64))));
    let mut ticker2 = crate::time::__Handle_Ticker::new(SteppingTicker::new(millis(500i64)));
    let mut started = ticker2.tick();
    println(&mut console, &(format!("overdue after one more read: {}", overdue(&mut ticker2, &started, &budget))));
    println(&mut console, &(format!("overdue after three: {} {} {}", overdue(&mut ticker2, &started, &budget), overdue(&mut ticker2, &started, &budget), overdue(&mut ticker2, &started, &budget))));
    println(&mut console, &(verdict(&(Tick { nanos: 0i64 }), &(Tick { nanos: 1000000000i64 }), &budget)));
    println(&mut console, &(verdict(&(Tick { nanos: 0i64 }), &(Tick { nanos: 2000000000i64 }), &budget)));
    let mut p = crate::scheduler::salvo_pool(((1) as usize));
    let (mut timer, mut ctl) = ({ let __h = ManualTime::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_ManualTime::new(__h)), __DECODE_ManualTime); (__a, __a) });
    let mut sessions = ({ let __h = Sessions::new(crate::time::__Handle_Timer::new(__Stub_Timer::new(timer))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sessions::new(__h)), __DECODE_Sessions); __a });
    let mut outcome = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(sessions, crate::__Msg_Session::Open(Tick { nanos: 0i64 }, budget, answer), crate::__PROTO_Session);
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Idle>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_on_idle(p, settled, |__gates, __tokens| Box::new(Idle { parked_gates: __gates, parked_tokens: __tokens }));
            *crate::scheduler::salvo_wait(__wid).downcast::<Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(ctl, crate::time::__Msg_TimerCtl::Advance(millis(2500i64)), crate::time::__PROTO_TimerCtl);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut console, &(format!("session: {}", outcome)));
    let mut sleeper = ({ let __h = Napping::new(crate::time::__Handle_Timer::new(__Stub_Timer::new(timer)), crate::time::__Handle_Ticker::new(TestTicker::new(timer))); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_thread(), __cap as usize, Box::new(__Actor_Napping::new(__h)), __DECODE_Napping); __a });
    let mut napped = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        crate::scheduler::salvo_send_wire(sleeper, crate::__Msg_Sleeper::Nap(seconds(2i64), answer), crate::__PROTO_Sleeper);
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Idle>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_on_idle(p, settled, |__gates, __tokens| Box::new(Idle { parked_gates: __gates, parked_tokens: __tokens }));
            *crate::scheduler::salvo_wait(__wid).downcast::<Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send_wire(ctl, crate::time::__Msg_TimerCtl::Advance(seconds(2i64)), crate::time::__PROTO_TimerCtl);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut console, &napped);
}
