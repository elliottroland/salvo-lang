use crate::core_actor::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_fs::*;
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Instant {
    pub nanos: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tick {
    pub nanos: i64,
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

pub fn to_str__4(d: &Duration) -> String {
    if d.nanos < ((0) as i64) {
        let mut positive = Duration { nanos: 0i64 - d.nanos };
        return format!("-{}", to_str__4(&positive));
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

pub trait Ticker {
    fn tick(&mut self) -> Tick;
}

pub trait __Has_Ticker {
    fn __get_Ticker(&mut self) -> &mut dyn Ticker;
}

pub trait __Share_Ticker: Ticker + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Ticker>;
}

impl<__H: Ticker + Clone + Send + 'static> __Share_Ticker for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Ticker> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Ticker {
    inner: Box<dyn __Share_Ticker>,
}

impl Clone for __Mon_Ticker {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Ticker {
    pub fn new(inner: Box<dyn __Share_Ticker>) -> Self {
        Self { inner }
    }
}

impl Ticker for __Mon_Ticker {
    fn tick(&mut self) -> Tick {
        self.inner.tick()
    }
}

pub struct __Lock_Ticker<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Ticker<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Ticker<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Ticker + Send> Ticker for __Lock_Ticker<H> {
    fn tick(&mut self) -> Tick {
        self.inner.lock().unwrap().tick()
    }
}

impl __Has_Ticker for __Mon_Ticker {
    fn __get_Ticker(&mut self) -> &mut dyn Ticker {
        self
    }
}

pub trait Clock {
    fn now(&mut self) -> Instant;
    fn to_instant(&mut self, at: &Tick) -> Instant;
    fn to_tick(&mut self, at: &Instant) -> Tick;
}

pub trait __Has_Clock {
    fn __get_Clock(&mut self) -> &mut dyn Clock;
}

pub trait __Share_Clock: Clock + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Clock>;
}

impl<__H: Clock + Clone + Send + 'static> __Share_Clock for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Clock> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Clock {
    inner: Box<dyn __Share_Clock>,
}

impl Clone for __Mon_Clock {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Clock {
    pub fn new(inner: Box<dyn __Share_Clock>) -> Self {
        Self { inner }
    }
}

impl Clock for __Mon_Clock {
    fn now(&mut self) -> Instant {
        self.inner.now()
    }
    fn to_instant(&mut self, at: &Tick) -> Instant {
        self.inner.to_instant(at)
    }
    fn to_tick(&mut self, at: &Instant) -> Tick {
        self.inner.to_tick(at)
    }
}

pub struct __Lock_Clock<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Clock<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Clock<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Clock + Send> Clock for __Lock_Clock<H> {
    fn now(&mut self) -> Instant {
        self.inner.lock().unwrap().now()
    }
    fn to_instant(&mut self, at: &Tick) -> Instant {
        self.inner.lock().unwrap().to_instant(at)
    }
    fn to_tick(&mut self, at: &Instant) -> Tick {
        self.inner.lock().unwrap().to_tick(at)
    }
}

impl __Has_Clock for __Mon_Clock {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        self
    }
}

pub fn elapsed<__Fx: __Has_Ticker>(__fx: &mut __Fx, since: &Tick) -> Duration {
    return between__2(since, &(__Has_Ticker::__get_Ticker(&mut *__fx).tick()));
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

impl Ticker for DefaultTicker {

    fn tick(&mut self) -> Tick {
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

impl Clock for DefaultClock {

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

pub trait Timer {
    fn after(&mut self, wait: Duration, done: crate::scheduler::SalvoReply);
}

pub trait __Has_Timer {
    fn __get_Timer(&mut self) -> &mut dyn Timer;
}

pub struct __Stub_Timer {
    addr: usize,
}

impl __Stub_Timer {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Timer for __Stub_Timer {
    fn after(&mut self, wait: Duration, done: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Timer::After(wait, done)));
    }
}

pub enum __Msg_Timer {
    After(Duration, crate::scheduler::SalvoReply),
}

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

impl Timer for DefaultTimer {

    fn after(&mut self, wait: Duration, done: crate::scheduler::SalvoReply) {
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
            crate::time::__Msg_Timer::After(wait, done) => crate::time::Timer::after(&mut self.handler, wait, done),
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
}

pub trait TimerCtl {
    fn advance(&mut self, by: Duration);
}

pub trait __Has_TimerCtl {
    fn __get_TimerCtl(&mut self) -> &mut dyn TimerCtl;
}

pub struct __Stub_TimerCtl {
    addr: usize,
}

impl __Stub_TimerCtl {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl TimerCtl for __Stub_TimerCtl {
    fn advance(&mut self, by: Duration) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_TimerCtl::Advance(by)));
    }
}

pub enum __Msg_TimerCtl {
    Advance(Duration),
}

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

impl Timer for ManualTime {

    fn after(&mut self, wait: Duration, done: crate::scheduler::SalvoReply) {
        if wait.nanos <= ((0) as i64) {
            (done).send(Box::new(Fired { at: Tick { nanos: self.now } }));
        } else {
            self.deadlines.push(self.now + wait.nanos);
            self.pending.push(done);
        }
    }
}

impl TimerCtl for ManualTime {

    fn advance(&mut self, by: Duration) {
        let mut target = self.now + by.nanos;
        loop {
            let mut __is1 = earliest_due(&self.deadlines, target);
            if !(__is1.is_some()) {
                break;
            }
            let mut at = __is1.unwrap();
            let mut deadline = *self.deadlines.get((at) as i64 as usize).unwrap();
            self.deadlines.salvo_remove_at(at);
            self.now = deadline.clone();
            let mut __is2 = self.pending.salvo_remove_at(at);
            if __is2.is_some() {
                let mut token = __is2.unwrap();
                (token).send(Box::new(Fired { at: Tick { nanos: deadline } }));
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
            crate::time::__Msg_Timer::After(wait, done) => crate::time::Timer::after(&mut self.handler, wait, done),
        }
    }
    fn __dispatch_TimerCtl(&mut self, msg: crate::time::__Msg_TimerCtl) {
        match msg {
            crate::time::__Msg_TimerCtl::Advance(by) => crate::time::TimerCtl::advance(&mut self.handler, by),
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
}

pub fn earliest_due(deadlines: &Vec<i64>, target: i64) -> Option<i32> {
    let mut best = -1;
    let mut best_at = 0i64;
    let mut i = 0;
    while i < (deadlines.len() as i32) {
        let mut at = *deadlines.get((i) as i64 as usize).unwrap();
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

pub fn cmp(a: &Duration, b: &Duration) -> i32 {
    (Ord::cmp(a, b) as i32)
}

pub fn hash(value: &Duration) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq(a: &Duration, b: &Duration) -> bool {
    (a == b)
}

pub fn cmp__2(a: &Instant, b: &Instant) -> i32 {
    (Ord::cmp(a, b) as i32)
}

pub fn hash__2(value: &Instant) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq__2(a: &Instant, b: &Instant) -> bool {
    (a == b)
}

pub fn cmp__3(a: &Tick, b: &Tick) -> i32 {
    (Ord::cmp(a, b) as i32)
}

pub fn hash__3(value: &Tick) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq__3(a: &Tick, b: &Tick) -> bool {
    (a == b)
}
