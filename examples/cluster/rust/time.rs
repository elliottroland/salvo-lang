use crate::core_actor::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Duration {
    pub nanos: i64,
}

impl crate::wire::__Wire for Duration {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.nanos, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            nanos: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Instant {
    pub nanos: i64,
}

impl crate::wire::__Wire for Instant {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.nanos, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            nanos: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tick {
    pub nanos: i64,
}

impl crate::wire::__Wire for Tick {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.nanos, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            nanos: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn nanos(n: i64) -> Duration {
    return Duration { nanos: n };
}

pub fn micros(n: i64) -> Duration {
    return Duration { nanos: n * 1000i64 };
}

pub fn millis(n: i64) -> Duration {
    return Duration { nanos: n * 1000000i64 };
}

pub fn seconds(n: i64) -> Duration {
    return Duration { nanos: n * 1000000000i64 };
}

pub fn minutes(n: i64) -> Duration {
    return Duration { nanos: n * 60000000000i64 };
}

pub fn hours(n: i64) -> Duration {
    return Duration { nanos: n * 3600000000000i64 };
}

pub fn to_nanos(d: &Duration) -> i64 {
    return d.nanos;
}

pub fn to_micros(d: &Duration) -> i64 {
    return d.nanos / 1000i64;
}

pub fn to_millis(d: &Duration) -> i64 {
    return d.nanos / 1000000i64;
}

pub fn to_seconds(d: &Duration) -> i64 {
    return d.nanos / 1000000000i64;
}

pub fn plus(d1: &Duration, d2: &Duration) -> Duration {
    return Duration { nanos: d1.nanos + d2.nanos };
}

pub fn minus(d1: &Duration, d2: &Duration) -> Duration {
    return Duration { nanos: d1.nanos - d2.nanos };
}

pub fn times(d: &Duration, n: i64) -> Duration {
    return Duration { nanos: d.nanos * n };
}

pub fn abs(d: Duration) -> Duration {
    if d.nanos < ((0) as i64) {
        return Duration { nanos: 0i64 - d.nanos };
    }
    return d;
}

pub fn to_str__6(d: &Duration) -> String {
    if d.nanos < ((0) as i64) {
        let mut positive = Duration { nanos: 0i64 - d.nanos };
        return format!("-{}", to_str__6(&positive));
    }
    if d.nanos == ((0) as i64) {
        return "0s".to_string();
    }
    if d.nanos % ((1000000000) as i64) == ((0) as i64) {
        return format!("{}s", d.nanos / 1000000000i64);
    }
    if d.nanos % ((1000000) as i64) == ((0) as i64) {
        return format!("{}ms", d.nanos / 1000000i64);
    }
    if d.nanos % ((1000) as i64) == ((0) as i64) {
        return format!("{}us", d.nanos / 1000i64);
    }
    return format!("{}ns", d.nanos);
}

pub fn epoch_nano(n: i64) -> Instant {
    return Instant { nanos: n };
}

pub fn epoch_milli(n: i64) -> Instant {
    return Instant { nanos: n * 1000000i64 };
}

pub fn epoch_second(n: i64) -> Instant {
    return Instant { nanos: n * 1000000000i64 };
}

pub fn to_epoch_nano(at: &Instant) -> i64 {
    return at.nanos;
}

pub fn to_epoch_milli(at: &Instant) -> i64 {
    return at.nanos / 1000000i64;
}

pub fn to_epoch_second(at: &Instant) -> i64 {
    return at.nanos / 1000000000i64;
}

pub fn between(start: &Instant, end: &Instant) -> Duration {
    return Duration { nanos: end.nanos - start.nanos };
}

pub fn between__2(start: &Tick, end: &Tick) -> Duration {
    return Duration { nanos: end.nanos - start.nanos };
}

pub fn plus__2(at: &Instant, d: &Duration) -> Instant {
    return Instant { nanos: at.nanos + d.nanos };
}

pub fn minus__2(at: &Instant, d: &Duration) -> Instant {
    return Instant { nanos: at.nanos - d.nanos };
}

pub fn plus__3(at: &Tick, d: &Duration) -> Tick {
    return Tick { nanos: at.nanos + d.nanos };
}

pub fn minus__3(at: &Tick, d: &Duration) -> Tick {
    return Tick { nanos: at.nanos - d.nanos };
}

pub trait __Stateless_Ticker: Send + Sync {
    fn tick(&self) -> Tick;
}

pub trait __Stateful_Ticker: Send {
    fn tick(&mut self) -> Tick;
}

pub struct Ticker {
    inner: __Inner_Ticker,
}

pub enum __Inner_Ticker {
    Shared(std::sync::Arc<dyn __Stateless_Ticker>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Ticker>>),
}

impl Clone for Ticker {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Ticker::Shared(h) => __Inner_Ticker::Shared(h.clone()),
            __Inner_Ticker::Locked(h) => __Inner_Ticker::Locked(h.clone()),
        } }
    }
}

impl Ticker {
    pub fn shared<__H: __Stateless_Ticker + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Ticker::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Ticker>) -> Self {
        Self { inner: __Inner_Ticker::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Ticker + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Ticker::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Ticker>>) -> Self {
        Self { inner: __Inner_Ticker::Locked(inner) }
    }
    pub fn tick(&self) -> Tick {
        match &self.inner {
            __Inner_Ticker::Shared(h) => h.tick(),
            __Inner_Ticker::Locked(h) => h.lock().unwrap().tick(),
        }
    }
}

pub trait __Stateless_Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn to_instant(&self, at: &Tick) -> Instant;
    fn to_tick(&self, at: &Instant) -> Tick;
}

pub trait __Stateful_Clock: Send {
    fn now(&mut self) -> Instant;
    fn to_instant(&mut self, at: &Tick) -> Instant;
    fn to_tick(&mut self, at: &Instant) -> Tick;
}

pub struct Clock {
    inner: __Inner_Clock,
}

pub enum __Inner_Clock {
    Shared(std::sync::Arc<dyn __Stateless_Clock>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Clock>>),
}

impl Clone for Clock {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Clock::Shared(h) => __Inner_Clock::Shared(h.clone()),
            __Inner_Clock::Locked(h) => __Inner_Clock::Locked(h.clone()),
        } }
    }
}

impl Clock {
    pub fn shared<__H: __Stateless_Clock + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Clock::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Clock>) -> Self {
        Self { inner: __Inner_Clock::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Clock + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Clock::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Clock>>) -> Self {
        Self { inner: __Inner_Clock::Locked(inner) }
    }
    pub fn now(&self) -> Instant {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.now(),
            __Inner_Clock::Locked(h) => h.lock().unwrap().now(),
        }
    }
    pub fn to_instant(&self, at: &Tick) -> Instant {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.to_instant(at),
            __Inner_Clock::Locked(h) => h.lock().unwrap().to_instant(at),
        }
    }
    pub fn to_tick(&self, at: &Instant) -> Tick {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.to_tick(at),
            __Inner_Clock::Locked(h) => h.lock().unwrap().to_tick(at),
        }
    }
}

pub fn elapsed(ticker: &crate::time::Ticker, since: &Tick) -> Duration {
    return between__2(since, &(ticker.tick()));
}

#[derive(Clone)]
pub struct DefaultTicker {
}

impl DefaultTicker {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl crate::time::__Stateless_Ticker for DefaultTicker {

    fn tick(&self) -> Tick {
        return Tick { nanos: crate::hosttime::salvo_mono_nanos() };
    }
}

pub struct DefaultClock {
    base_tick: i64,
    base_epoch: i64,
}

impl DefaultClock {
    pub fn new() -> Self {
        Self {
            base_tick: crate::hosttime::salvo_mono_nanos(),
            base_epoch: crate::hosttime::salvo_epoch_nanos(),
        }
    }
}

impl crate::time::__Stateful_Clock for DefaultClock {

    fn now(&mut self) -> Instant {
        return Instant { nanos: crate::hosttime::salvo_epoch_nanos() };
    }

    fn to_instant(&mut self, at: &Tick) -> Instant {
        return Instant { nanos: self.base_epoch + (at.nanos - self.base_tick) };
    }

    fn to_tick(&mut self, at: &Instant) -> Tick {
        return Tick { nanos: self.base_tick + (at.nanos - self.base_epoch) };
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fired {
    pub at: Tick,
}

impl crate::wire::__Wire for Fired {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub trait __Stateless_Timer: Send + Sync {
    fn after(&self, wait: Duration, done: crate::scheduler::SalvoReply);
}

pub trait __Stateful_Timer: Send {
    fn after(&mut self, wait: Duration, done: crate::scheduler::SalvoReply);
}

pub struct __Stub_Timer {
    addr: usize,
}

impl __Stub_Timer {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Timer for __Stub_Timer {
    fn after(&self, wait: Duration, done: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Timer::After(wait, done), crate::time::__PROTO_Timer);
    }
}

pub struct Timer {
    inner: __Inner_Timer,
}

pub enum __Inner_Timer {
    Shared(std::sync::Arc<dyn __Stateless_Timer>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Timer>>),
}

impl Clone for Timer {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Timer::Shared(h) => __Inner_Timer::Shared(h.clone()),
            __Inner_Timer::Locked(h) => __Inner_Timer::Locked(h.clone()),
        } }
    }
}

impl Timer {
    pub fn shared<__H: __Stateless_Timer + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Timer::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Timer>) -> Self {
        Self { inner: __Inner_Timer::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Timer + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Timer::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Timer>>) -> Self {
        Self { inner: __Inner_Timer::Locked(inner) }
    }
    pub fn after(&self, wait: Duration, done: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Timer::Shared(h) => h.after(wait, done),
            __Inner_Timer::Locked(h) => h.lock().unwrap().after(wait, done),
        }
    }
}

pub enum __Msg_Timer {
    After(Duration, crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_Timer {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Timer::After(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Timer::After(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Timer`.
pub const __PROTO_Timer: &str = "d0432e460a159011";

pub struct DefaultTimer {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_DefaultTimer>,
}

impl DefaultTimer {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::time::__Stateless_Timer for DefaultTimer {

    fn after(&self, wait: Duration, done: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_after((wait).nanos, done, |__at| Box::new(Fired { at: Tick { nanos: __at } }));
    }
}

pub enum __Cont_DefaultTimer {
    After(Duration),
}

pub struct __Actor_DefaultTimer {
    handler: DefaultTimer,
}

impl __Actor_DefaultTimer {
    pub fn new(handler: DefaultTimer) -> Self {
        Self { handler }
    }
}

impl __Actor_DefaultTimer {
    fn __dispatch(&mut self, msg: crate::time::__Msg_Timer) {
        match msg {
            crate::time::__Msg_Timer::After(wait, done) => crate::time::__Stateless_Timer::after(&mut self.handler, wait, done),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_DefaultTimer {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::time::__Msg_Timer>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_DefaultTimer::After(wait) => self.__dispatch(crate::time::__Msg_Timer::After(wait, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_DefaultTimer::After{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_DefaultTimer: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_DefaultTimer);
fn __decode_msg_DefaultTimer(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::time::__PROTO_Timer {
            return crate::wire::salvo_decode::<crate::time::__Msg_Timer>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub trait __Stateless_TimerCtl: Send + Sync {
    fn advance(&self, by: Duration);
}

pub trait __Stateful_TimerCtl: Send {
    fn advance(&mut self, by: Duration);
}

pub struct __Stub_TimerCtl {
    addr: usize,
}

impl __Stub_TimerCtl {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_TimerCtl for __Stub_TimerCtl {
    fn advance(&self, by: Duration) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_TimerCtl::Advance(by), crate::time::__PROTO_TimerCtl);
    }
}

pub struct TimerCtl {
    inner: __Inner_TimerCtl,
}

pub enum __Inner_TimerCtl {
    Shared(std::sync::Arc<dyn __Stateless_TimerCtl>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_TimerCtl>>),
}

impl Clone for TimerCtl {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_TimerCtl::Shared(h) => __Inner_TimerCtl::Shared(h.clone()),
            __Inner_TimerCtl::Locked(h) => __Inner_TimerCtl::Locked(h.clone()),
        } }
    }
}

impl TimerCtl {
    pub fn shared<__H: __Stateless_TimerCtl + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_TimerCtl::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_TimerCtl>) -> Self {
        Self { inner: __Inner_TimerCtl::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_TimerCtl + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_TimerCtl::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_TimerCtl>>) -> Self {
        Self { inner: __Inner_TimerCtl::Locked(inner) }
    }
    pub fn advance(&self, by: Duration) {
        match &self.inner {
            __Inner_TimerCtl::Shared(h) => h.advance(by),
            __Inner_TimerCtl::Locked(h) => h.lock().unwrap().advance(by),
        }
    }
}

pub enum __Msg_TimerCtl {
    Advance(Duration),
}

impl crate::wire::__Wire for __Msg_TimerCtl {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_TimerCtl::Advance(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_TimerCtl::Advance(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `TimerCtl`.
pub const __PROTO_TimerCtl: &str = "93f92d20477305ad";

pub struct ManualTime {
    now: i64,
    deadlines: Vec<i64>,
    pending: Vec<crate::scheduler::SalvoReply>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_ManualTime>,
}

impl ManualTime {
    pub fn new() -> Self {
        Self {
            now: 0i64,
            deadlines: vec![],
            pending: vec![],
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::time::__Stateful_Timer for ManualTime {

    fn after(&mut self, wait: Duration, done: crate::scheduler::SalvoReply) {
        if wait.nanos <= ((0) as i64) {
            crate::scheduler::salvo_reply_wire::<Fired>(done, Fired { at: Tick { nanos: self.now } });
        } else {
            self.deadlines.push(self.now + wait.nanos);
            self.pending.push(done);
        }
    }
}

impl crate::time::__Stateful_TimerCtl for ManualTime {

    fn advance(&mut self, by: Duration) {
        let mut target = self.now + by.nanos;
        loop {
            let mut __is1 = earliest_due(&self.deadlines, target);
            if !(__is1.is_some()) {
                break;
            }
            let mut at = __is1.unwrap();
            let mut deadline = *self.deadlines.get((at) as i64 as usize).expect("salvo: value is absent at time:480:33");
            self.deadlines.salvo_remove_at(at);
            self.now = deadline.clone();
            let mut __is2 = self.pending.salvo_remove_at(at);
            if __is2.is_some() {
                let mut token = __is2.unwrap();
                crate::scheduler::salvo_reply_wire::<Fired>(token, Fired { at: Tick { nanos: deadline } });
            }
        }
        self.now = target;
    }
}

pub enum __Cont_ManualTime {
    After(Duration),
    Advance,
}

pub struct __Actor_ManualTime {
    handler: ManualTime,
}

impl __Actor_ManualTime {
    pub fn new(handler: ManualTime) -> Self {
        Self { handler }
    }
}

impl __Actor_ManualTime {
    fn __dispatch_Timer(&mut self, msg: crate::time::__Msg_Timer) {
        match msg {
            crate::time::__Msg_Timer::After(wait, done) => crate::time::__Stateful_Timer::after(&mut self.handler, wait, done),
        }
    }
    fn __dispatch_TimerCtl(&mut self, msg: crate::time::__Msg_TimerCtl) {
        match msg {
            crate::time::__Msg_TimerCtl::Advance(by) => crate::time::__Stateful_TimerCtl::advance(&mut self.handler, by),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_ManualTime {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::time::__Msg_Timer>() {
            Ok(__m) => return self.__dispatch_Timer(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<crate::time::__Msg_TimerCtl>() {
            Ok(__m) => return self.__dispatch_TimerCtl(*__m),
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
            __Cont_ManualTime::After(wait) => self.__dispatch_Timer(crate::time::__Msg_Timer::After(wait, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_ManualTime::Advance => self.__dispatch_TimerCtl(crate::time::__Msg_TimerCtl::Advance(*value.downcast::<Duration>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ManualTime::After{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ManualTime::Advance{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Duration>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_ManualTime: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_ManualTime);
fn __decode_msg_ManualTime(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::time::__PROTO_Timer {
            return crate::wire::salvo_decode::<crate::time::__Msg_Timer>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
        if proto == crate::time::__PROTO_TimerCtl {
            return crate::wire::salvo_decode::<crate::time::__Msg_TimerCtl>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn earliest_due(deadlines: &Vec<i64>, target: i64) -> Option<i32> {
    let mut best = -1;
    let mut best_at = 0i64;
    let mut i = 0;
    while i < (deadlines.len() as i32) {
        let mut at = *deadlines.get((i) as i64 as usize).expect("salvo: value is absent at time:504:23");
        if at <= target && (best < 0 || at < best_at) {
            best = i.clone();
            best_at = at.clone();
        }
        i = i + 1;
    }
    if best < 0 {
        return None;
    }
    return Some(best);
}

pub fn cmp__2(a: &Duration, b: &Duration) -> i32 {
    let mut c__c1 = (Ord::cmp(&(a.nanos), &(b.nanos)) as i32);
    if c__c1 != 0 {
        return c__c1;
    }
    return 0;
}

pub fn hash__4(value: &Duration) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.nanos), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__4(a: &Duration, b: &Duration) -> bool {
    if !((a.nanos) == (b.nanos)) {
        return false;
    }
    return true;
}

pub fn cmp__3(a: &Instant, b: &Instant) -> i32 {
    let mut c__c1 = (Ord::cmp(&(a.nanos), &(b.nanos)) as i32);
    if c__c1 != 0 {
        return c__c1;
    }
    return 0;
}

pub fn hash__5(value: &Instant) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.nanos), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__5(a: &Instant, b: &Instant) -> bool {
    if !((a.nanos) == (b.nanos)) {
        return false;
    }
    return true;
}

pub fn cmp__4(a: &Tick, b: &Tick) -> i32 {
    let mut c__c1 = (Ord::cmp(&(a.nanos), &(b.nanos)) as i32);
    if c__c1 != 0 {
        return c__c1;
    }
    return 0;
}

pub fn hash__6(value: &Tick) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.nanos), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__6(a: &Tick, b: &Tick) -> bool {
    if !((a.nanos) == (b.nanos)) {
        return false;
    }
    return true;
}
