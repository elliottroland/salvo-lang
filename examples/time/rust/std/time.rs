use crate::std_core_list::add_platform;
use crate::std_runtime_timers::after_nanos;
use crate::std_core_list::get_platform;
use crate::std_core_compare::mix_hash;
use crate::std_runtime::now_nanos;
use crate::std_core_list::remove_at_platform;
use crate::std_core_list::size_platform;


#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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

pub fn nanos(mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: n };
}

pub fn micros(mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_mul(n, 1000i64) };
}

pub fn millis(mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_mul(n, 1000000i64) };
}

pub fn seconds(mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_mul(n, 1000000000i64) };
}

pub fn minutes(mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_mul(n, 60000000000i64) };
}

pub fn hours(mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_mul(n, 3600000000000i64) };
}

pub fn to_nanos(d: &crate::std_time::Duration) -> i64 {
    return d.nanos;
}

pub fn to_micros(d: &crate::std_time::Duration) -> i64 {
    return i64::wrapping_div(d.nanos, 1000i64);
}

pub fn to_millis(d: &crate::std_time::Duration) -> i64 {
    return i64::wrapping_div(d.nanos, 1000000i64);
}

pub fn to_seconds(d: &crate::std_time::Duration) -> i64 {
    return i64::wrapping_div(d.nanos, 1000000000i64);
}

pub fn plus__Duration_Duration(d1: &crate::std_time::Duration, d2: &crate::std_time::Duration) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_add(d1.nanos, d2.nanos) };
}

pub fn minus__Duration_Duration(d1: &crate::std_time::Duration, d2: &crate::std_time::Duration) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_sub(d1.nanos, d2.nanos) };
}

pub fn times(d: &crate::std_time::Duration, mut n: i64) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_mul(d.nanos, n) };
}

pub fn abs(mut d: crate::std_time::Duration) -> crate::std_time::Duration {
    if (d.nanos < 0i64) {
        return crate::std_time::Duration { nanos: i64::wrapping_sub(0i64, d.nanos) };
    };
    return d;
}

pub fn to_str(d: &crate::std_time::Duration) -> String {
    if (d.nanos < 0i64) {
        let mut positive: crate::std_time::Duration = crate::std_time::Duration { nanos: i64::wrapping_sub(0i64, d.nanos) };
        return format!("-{}", crate::std_time::to_str(&positive));
    };
    if (d.nanos == 0i64) {
        return String::from("0s");
    };
    if (i64::wrapping_rem(d.nanos, 1000000000i64) == 0i64) {
        return format!("{}s", i64::wrapping_div(d.nanos, 1000000000i64));
    };
    if (i64::wrapping_rem(d.nanos, 1000000i64) == 0i64) {
        return format!("{}ms", i64::wrapping_div(d.nanos, 1000000i64));
    };
    if (i64::wrapping_rem(d.nanos, 1000i64) == 0i64) {
        return format!("{}us", i64::wrapping_div(d.nanos, 1000i64));
    };
    return format!("{}ns", d.nanos);
}

pub fn epoch_nano(mut n: i64) -> crate::std_time::Instant {
    return crate::std_time::Instant { nanos: n };
}

pub fn epoch_milli(mut n: i64) -> crate::std_time::Instant {
    return crate::std_time::Instant { nanos: i64::wrapping_mul(n, 1000000i64) };
}

pub fn epoch_second(mut n: i64) -> crate::std_time::Instant {
    return crate::std_time::Instant { nanos: i64::wrapping_mul(n, 1000000000i64) };
}

pub fn to_epoch_nano(at: &crate::std_time::Instant) -> i64 {
    return at.nanos;
}

pub fn to_epoch_milli(at: &crate::std_time::Instant) -> i64 {
    return i64::wrapping_div(at.nanos, 1000000i64);
}

pub fn to_epoch_second(at: &crate::std_time::Instant) -> i64 {
    return i64::wrapping_div(at.nanos, 1000000000i64);
}

pub fn between__Instant_Instant(start: &crate::std_time::Instant, end: &crate::std_time::Instant) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_sub(end.nanos, start.nanos) };
}

pub fn between__Tick_Tick(start: &crate::std_time::Tick, end: &crate::std_time::Tick) -> crate::std_time::Duration {
    return crate::std_time::Duration { nanos: i64::wrapping_sub(end.nanos, start.nanos) };
}

pub fn plus__Instant_Duration(at: &crate::std_time::Instant, d: &crate::std_time::Duration) -> crate::std_time::Instant {
    return crate::std_time::Instant { nanos: i64::wrapping_add(at.nanos, d.nanos) };
}

pub fn minus__Instant_Duration(at: &crate::std_time::Instant, d: &crate::std_time::Duration) -> crate::std_time::Instant {
    return crate::std_time::Instant { nanos: i64::wrapping_sub(at.nanos, d.nanos) };
}

pub fn plus__Tick_Duration(at: &crate::std_time::Tick, d: &crate::std_time::Duration) -> crate::std_time::Tick {
    return crate::std_time::Tick { nanos: i64::wrapping_add(at.nanos, d.nanos) };
}

pub fn minus__Tick_Duration(at: &crate::std_time::Tick, d: &crate::std_time::Duration) -> crate::std_time::Tick {
    return crate::std_time::Tick { nanos: i64::wrapping_sub(at.nanos, d.nanos) };
}

pub trait __Stateless_Ticker: Send + Sync {
    fn tick(&self) -> crate::std_time::Tick;
}

pub trait __Stateful_Ticker: Send {
    fn tick(&mut self) -> crate::std_time::Tick;
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
    pub fn tick(&self) -> crate::std_time::Tick {
        match &self.inner {
            __Inner_Ticker::Shared(h) => h.tick(),
            __Inner_Ticker::Locked(h) => h.lock().unwrap().tick(),
        }
    }
}

pub trait __Stateless_Clock: Send + Sync {
    fn now(&self) -> crate::std_time::Instant;
    fn to_instant(&self, at: &crate::std_time::Tick) -> crate::std_time::Instant;
    fn to_tick(&self, at: &crate::std_time::Instant) -> crate::std_time::Tick;
}

pub trait __Stateful_Clock: Send {
    fn now(&mut self) -> crate::std_time::Instant;
    fn to_instant(&mut self, at: &crate::std_time::Tick) -> crate::std_time::Instant;
    fn to_tick(&mut self, at: &crate::std_time::Instant) -> crate::std_time::Tick;
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
    pub fn now(&self) -> crate::std_time::Instant {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.now(),
            __Inner_Clock::Locked(h) => h.lock().unwrap().now(),
        }
    }
    pub fn to_instant(&self, at: &crate::std_time::Tick) -> crate::std_time::Instant {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.to_instant(at),
            __Inner_Clock::Locked(h) => h.lock().unwrap().to_instant(at),
        }
    }
    pub fn to_tick(&self, at: &crate::std_time::Instant) -> crate::std_time::Tick {
        match &self.inner {
            __Inner_Clock::Shared(h) => h.to_tick(at),
            __Inner_Clock::Locked(h) => h.lock().unwrap().to_tick(at),
        }
    }
}

pub fn elapsed(ticker: &crate::std_time::Ticker, since: &crate::std_time::Tick) -> crate::std_time::Duration {
    return crate::std_time::between__Tick_Tick(since, &ticker.tick());
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

impl crate::std_time::__Stateless_Ticker for DefaultTicker {
    fn tick(&self) -> crate::std_time::Tick {
        return crate::std_time::Tick { nanos: crate::std_time::monotonic_nanos() };
    }
}

pub struct DefaultClock {
    base_tick: i64,
    base_epoch: i64,
}

impl DefaultClock {
    pub fn new() -> Self {
        Self {
            base_tick: crate::std_time::monotonic_nanos(),
            base_epoch: crate::std_time::epoch_nanos_platform()
        }
    }
}

impl crate::std_time::__Stateful_Clock for DefaultClock {
    fn now(&mut self) -> crate::std_time::Instant {
        return crate::std_time::Instant { nanos: crate::std_time::epoch_nanos_platform() };
    }
    fn to_instant(&mut self, at: &crate::std_time::Tick) -> crate::std_time::Instant {
        return crate::std_time::Instant { nanos: i64::wrapping_add(self.base_epoch, i64::wrapping_sub(at.nanos, self.base_tick)) };
    }
    fn to_tick(&mut self, at: &crate::std_time::Instant) -> crate::std_time::Tick {
        return crate::std_time::Tick { nanos: i64::wrapping_add(self.base_tick, i64::wrapping_sub(at.nanos, self.base_epoch)) };
    }
}

pub fn monotonic_nanos() -> i64 {
    return crate::std_runtime::now_nanos();
}

pub fn epoch_nanos_platform() -> i64 {
    crate::platform_std_time::epoch_nanos()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fired {
    pub at: crate::std_time::Tick,
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
    fn after(&self, wait: crate::std_time::Duration, done: crate::scheduler::SalvoReply);
}

pub trait __Stateful_Timer: Send {
    fn after(&mut self, wait: crate::std_time::Duration, done: crate::scheduler::SalvoReply);
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
    pub fn after(&self, wait: crate::std_time::Duration, done: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Timer::Shared(h) => h.after(wait, done),
            __Inner_Timer::Locked(h) => h.lock().unwrap().after(wait, done),
        }
    }
}

pub struct DefaultTimer {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_DefaultTimer>,
}

impl DefaultTimer {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::std_time::__Stateless_Timer for DefaultTimer {
    fn after(&self, wait: crate::std_time::Duration, done: crate::scheduler::SalvoReply) {
        crate::std_runtime_timers::after_nanos(wait.nanos, done);
    }
}

pub struct __Actor_DefaultTimer {
    handler: DefaultTimer,
}

impl __Actor_DefaultTimer {
    pub fn new(handler: DefaultTimer) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_DefaultTimer {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::std_time::__Msg_Timer>().expect("message of this protocol");
        crate::std_time::__dispatch_DefaultTimer_Timer(&mut self.handler, msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_DefaultTimer::After(wait) => crate::std_time::__dispatch_DefaultTimer_Timer(&mut self.handler, crate::std_time::__Msg_Timer::After(wait, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_DefaultTimer::After{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_DefaultTimer: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_DefaultTimer);
fn __decode_msg_DefaultTimer(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_time::__PROTO_Timer {
        return crate::wire::salvo_decode::<crate::std_time::__Msg_Timer>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub trait __Stateless_TimerCtl: Send + Sync {
    fn advance(&self, by: crate::std_time::Duration);
}

pub trait __Stateful_TimerCtl: Send {
    fn advance(&mut self, by: crate::std_time::Duration);
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
    pub fn advance(&self, by: crate::std_time::Duration) {
        match &self.inner {
            __Inner_TimerCtl::Shared(h) => h.advance(by),
            __Inner_TimerCtl::Locked(h) => h.lock().unwrap().advance(by),
        }
    }
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
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::std_time::__Stateful_Timer for ManualTime {
    fn after(&mut self, wait: crate::std_time::Duration, done: crate::scheduler::SalvoReply) {
        if (wait.nanos <= 0i64) {
            crate::scheduler::salvo_reply_wire::<crate::std_time::Fired>(done, crate::std_time::Fired { at: crate::std_time::Tick { nanos: self.now } });
        } else {
            crate::std_core_list::add_platform::<i64>(&mut self.deadlines, i64::wrapping_add(self.now, wait.nanos));
            crate::std_core_list::add_platform::<crate::scheduler::SalvoReply>(&mut self.pending, done);
        };
    }
}

impl crate::std_time::__Stateful_TimerCtl for ManualTime {
    fn advance(&mut self, by: crate::std_time::Duration) {
        let mut target: i64 = i64::wrapping_add(self.now, by.nanos);
        loop {
            let mut __subject_1: Option<i32> = crate::std_time::earliest_due(&self.deadlines, target);
            if !(__subject_1.is_some()) {
                break;
            };
            let mut at = __subject_1.unwrap();
            let mut deadline: i64 = {
                let mut __pick_2: Option<i64> = crate::std_core_list::get_platform::<i64>(&self.deadlines, at).copied();
                if __pick_2.is_some() {
                    let mut __picked_3 = __pick_2.unwrap();
                    __picked_3
                } else {
                    panic!("salvo: unreachable at std.time:494:55")
                }
            };
            crate::std_core_list::remove_at_platform::<i64>(&mut self.deadlines, at);
            self.now = deadline;
            let mut __subject_5: Option<crate::scheduler::SalvoReply> = crate::std_core_list::remove_at_platform::<crate::scheduler::SalvoReply>(&mut self.pending, at);
            if __subject_5.is_some() {
                let mut token = __subject_5.unwrap();
                crate::scheduler::salvo_reply_wire::<crate::std_time::Fired>(token, crate::std_time::Fired { at: crate::std_time::Tick { nanos: deadline } });
            };
        }
        self.now = target;
    }
}

pub struct __Actor_ManualTime {
    handler: ManualTime,
}

impl __Actor_ManualTime {
    pub fn new(handler: ManualTime) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_ManualTime {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::std_time::__Msg_Timer>() {
            Ok(__m) => return crate::std_time::__dispatch_ManualTime_Timer(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<crate::std_time::__Msg_TimerCtl>() {
            Ok(__m) => return crate::std_time::__dispatch_ManualTime_TimerCtl(&mut self.handler, *__m),
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
            __Cont_ManualTime::After(wait) => crate::std_time::__dispatch_ManualTime_Timer(&mut self.handler, crate::std_time::__Msg_Timer::After(wait, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_ManualTime::Advance => crate::std_time::__dispatch_ManualTime_TimerCtl(&mut self.handler, crate::std_time::__Msg_TimerCtl::Advance(*value.downcast::<crate::std_time::Duration>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ManualTime::After{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ManualTime::Advance{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_time::Duration>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_ManualTime: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_ManualTime);
fn __decode_msg_ManualTime(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_time::__PROTO_Timer {
        return crate::wire::salvo_decode::<crate::std_time::__Msg_Timer>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    if proto == crate::std_time::__PROTO_TimerCtl {
        return crate::wire::salvo_decode::<crate::std_time::__Msg_TimerCtl>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn earliest_due(deadlines: &Vec<i64>, mut target: i64) -> Option<i32> {
    let mut best: i32 = i32::wrapping_neg(1i32);
    let mut best_at: i64 = 0i64;
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::std_core_list::size_platform::<i64>(deadlines))) {
            break;
        };
        let mut at: i64 = {
            let mut __pick_1: Option<i64> = crate::std_core_list::get_platform::<i64>(deadlines, i).copied();
            if __pick_1.is_some() {
                let mut __picked_2 = __pick_1.unwrap();
                __picked_2
            } else {
                panic!("salvo: unreachable at std.time:519:44")
            }
        };
        if ((at <= target) && ((best < 0i32) || (at < best_at))) {
            best = i;
            best_at = at;
        };
        i = i32::wrapping_add(i, 1i32);
    }
    if (best < 0i32) {
        return None;
    };
    return Some(best);
}

pub fn cmp__Duration_Duration(a: &crate::std_time::Duration, b: &crate::std_time::Duration) -> i32 {
    let mut c__c1: i32 = (Ord::cmp(&(a.nanos), &(b.nanos)) as i32);
    if (c__c1 != 0i32) {
        return c__c1;
    };
    return 0i32;
}

pub fn hash__Duration(value: &crate::std_time::Duration) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.nanos), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__Duration_Duration(a: &crate::std_time::Duration, b: &crate::std_time::Duration) -> bool {
    if !(((a.nanos) == (b.nanos))) {
        return false;
    };
    return true;
}

pub fn cmp__Instant_Instant(a: &crate::std_time::Instant, b: &crate::std_time::Instant) -> i32 {
    let mut c__c1: i32 = (Ord::cmp(&(a.nanos), &(b.nanos)) as i32);
    if (c__c1 != 0i32) {
        return c__c1;
    };
    return 0i32;
}

pub fn hash__Instant(value: &crate::std_time::Instant) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.nanos), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__Instant_Instant(a: &crate::std_time::Instant, b: &crate::std_time::Instant) -> bool {
    if !(((a.nanos) == (b.nanos))) {
        return false;
    };
    return true;
}

pub fn cmp__Tick_Tick(a: &crate::std_time::Tick, b: &crate::std_time::Tick) -> i32 {
    let mut c__c1: i32 = (Ord::cmp(&(a.nanos), &(b.nanos)) as i32);
    if (c__c1 != 0i32) {
        return c__c1;
    };
    return 0i32;
}

pub fn hash__Tick(value: &crate::std_time::Tick) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.nanos), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__Tick_Tick(a: &crate::std_time::Tick, b: &crate::std_time::Tick) -> bool {
    if !(((a.nanos) == (b.nanos))) {
        return false;
    };
    return true;
}

pub enum __Msg_Timer {
    After(crate::std_time::Duration, crate::scheduler::SalvoReply),
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

/// [actor-protocol-hash] The canonical hash of `Timer`.
pub const __PROTO_Timer: &str = "d0432e460a159011";

pub enum __Msg_TimerCtl {
    Advance(crate::std_time::Duration),
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

/// [actor-protocol-hash] The canonical hash of `TimerCtl`.
pub const __PROTO_TimerCtl: &str = "93f92d20477305ad";

pub enum __Cont_DefaultTimer {
    After(crate::std_time::Duration),
}

pub enum __Cont_ManualTime {
    After(crate::std_time::Duration),
    Advance,
}

#[derive(Clone)]
pub struct __Stub_Timer {
    addr: usize,
}

impl __Stub_Timer {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_time::__Stateless_Timer for __Stub_Timer {
    fn after(&self, wait: crate::std_time::Duration, done: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_time::__Msg_Timer::After(wait, done), crate::std_time::__PROTO_Timer);
    }
}

#[derive(Clone)]
pub struct __Stub_TimerCtl {
    addr: usize,
}

impl __Stub_TimerCtl {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_time::__Stateless_TimerCtl for __Stub_TimerCtl {
    fn advance(&self, by: crate::std_time::Duration) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_time::__Msg_TimerCtl::Advance(by), crate::std_time::__PROTO_TimerCtl);
    }
}

pub fn __dispatch_DefaultTimer_Timer(__handler: &mut crate::std_time::DefaultTimer, mut __msg: crate::std_time::__Msg_Timer) {
    if matches!(__msg, crate::std_time::__Msg_Timer::After(..)) {
        let crate::std_time::__Msg_Timer::After(wait, done) = __msg else { unreachable!() };
        crate::std_time::__Stateless_Timer::after(&mut *__handler, wait, done);
    };
}

pub fn __dispatch_ManualTime_Timer(__handler: &mut crate::std_time::ManualTime, mut __msg: crate::std_time::__Msg_Timer) {
    if matches!(__msg, crate::std_time::__Msg_Timer::After(..)) {
        let crate::std_time::__Msg_Timer::After(wait, done) = __msg else { unreachable!() };
        crate::std_time::__Stateful_Timer::after(&mut *__handler, wait, done);
    };
}

pub fn __dispatch_ManualTime_TimerCtl(__handler: &mut crate::std_time::ManualTime, mut __msg: crate::std_time::__Msg_TimerCtl) {
    if matches!(__msg, crate::std_time::__Msg_TimerCtl::Advance(..)) {
        let crate::std_time::__Msg_TimerCtl::Advance(by) = __msg else { unreachable!() };
        crate::std_time::__Stateful_TimerCtl::advance(&mut *__handler, by);
    };
}
