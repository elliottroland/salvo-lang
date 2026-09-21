#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
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
#[path = "core/actor.rs"]
pub mod core_actor;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/fs.rs"]
pub mod core_fs;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/nonempty.rs"]
pub mod core_nonempty;
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/seq.rs"]
pub mod core_seq;
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
use crate::core_string::*;
use crate::time::*;

pub fn verdict(started: &Tick, at: &Tick, budget: &Duration) -> String {
    let mut took = between__2(started, at);
    if cmp(&took, budget) > 0 {
        return format!("late by {}", to_str__3(&minus(&took, budget)));
    }
    return format!("in time, {} to spare", to_str__3(&minus(budget, &took)));
}

pub fn overdue<__Fx: __Has_Ticker>(__fx: &mut __Fx, started: &Tick, budget: &Duration) -> bool {
    return cmp(&(elapsed(&mut *__fx, started)), budget) > 0;
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

pub trait __Has_Session {
    fn __get_Session(&mut self) -> &mut dyn Session;
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
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Session::Open(started, budget, out)));
    }
    fn expire(&mut self, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Session::Expire(started, budget, out, f)));
    }
}

pub enum __Msg_Session {
    Open(Tick, Duration, crate::scheduler::SalvoReply),
    Expire(Tick, Duration, crate::scheduler::SalvoReply, Fired),
}

pub struct Sessions {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sessions>,
}

impl Sessions {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 8,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_Sessions<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Timer + ?Sized> __Has_Timer for __Deps_Sessions<'a, __P> {
    fn __get_Timer(&mut self) -> &mut dyn Timer {
        __Has_Timer::__get_Timer(&mut *self.__p)
    }
}

pub trait __Impl_Sessions {

    fn open<__Fx: __Has_Timer>(&mut self, __fx: &mut __Fx, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply);

    fn expire<__Fx: __Has_Timer>(&mut self, __fx: &mut __Fx, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired);
}

impl __Impl_Sessions for Sessions {

    fn open<__Fx: __Has_Timer>(&mut self, __fx: &mut __Fx, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply) {
        __Has_Timer::__get_Timer(&mut *__fx).after(plus(&budget, &(seconds(1i64))), { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Sessions::Expire(started, budget, out)); __r });
    }

    fn expire<__Fx: __Has_Timer>(&mut self, __fx: &mut __Fx, started: Tick, budget: Duration, out: crate::scheduler::SalvoReply, f: Fired) {
        (out).send(Box::new(verdict(&started, &f.at, &budget)));
    }
}

pub enum __Cont_Sessions {
    Open(Tick, Duration),
    Expire(Tick, Duration, crate::scheduler::SalvoReply),
}

pub struct __Prov_Sessions<__D0> {
    pub __d0: __D0,
}

impl<__D0: Timer> __Has_Timer for __Prov_Sessions<__D0> {
    fn __get_Timer(&mut self) -> &mut dyn Timer {
        &mut self.__d0
    }
}

pub struct __Actor_Sessions<__D0> {
    handler: Sessions,
    prov: __Prov_Sessions<__D0>,
}

impl<__D0> __Actor_Sessions<__D0> {
    pub fn new(handler: Sessions, prov: __Prov_Sessions<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Timer> __Actor_Sessions<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Session) {
        let mut __deps = __Deps_Sessions{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Session::Open(started, budget, out) => __Impl_Sessions::open(&mut self.handler, &mut __deps, started, budget, out),
            crate::__Msg_Session::Expire(started, budget, out, f) => __Impl_Sessions::expire(&mut self.handler, &mut __deps, started, budget, out, f),
        }
    }
}

impl<__D0: Timer + Send + 'static> crate::scheduler::SalvoActor for __Actor_Sessions<__D0> {
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
            crate::scheduler::salvo_send(self.timer.clone(), Box::new(crate::time::__Msg_Timer::After(nanos(0i64), answer)));
            *crate::scheduler::salvo_wait(__wid).downcast::<Fired>().expect("the awaited answer")
        };
        return fired.at.clone();
    }
}

pub trait Sleeper {
    fn nap(&mut self, wait: Duration, out: crate::scheduler::SalvoReply);
    fn woke(&mut self, started: Tick, out: crate::scheduler::SalvoReply, f: Fired);
}

pub trait __Has_Sleeper {
    fn __get_Sleeper(&mut self) -> &mut dyn Sleeper;
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
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Sleeper::Nap(wait, out)));
    }
    fn woke(&mut self, started: Tick, out: crate::scheduler::SalvoReply, f: Fired) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Sleeper::Woke(started, out, f)));
    }
}

pub enum __Msg_Sleeper {
    Nap(Duration, crate::scheduler::SalvoReply),
    Woke(Tick, crate::scheduler::SalvoReply, Fired),
}

pub struct Napping {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Napping>,
}

impl Napping {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 8,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_Napping<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Timer + ?Sized> __Has_Timer for __Deps_Napping<'a, __P> {
    fn __get_Timer(&mut self) -> &mut dyn Timer {
        __Has_Timer::__get_Timer(&mut *self.__p)
    }
}

impl<'a, __P: __Has_Ticker + ?Sized> __Has_Ticker for __Deps_Napping<'a, __P> {
    fn __get_Ticker(&mut self) -> &mut dyn Ticker {
        __Has_Ticker::__get_Ticker(&mut *self.__p)
    }
}

pub trait __Impl_Napping {

    fn nap<__Fx: __Has_Timer + __Has_Ticker>(&mut self, __fx: &mut __Fx, wait: Duration, out: crate::scheduler::SalvoReply);

    fn woke<__Fx: __Has_Timer + __Has_Ticker>(&mut self, __fx: &mut __Fx, started: Tick, out: crate::scheduler::SalvoReply, f: Fired);
}

impl __Impl_Napping for Napping {

    fn nap<__Fx: __Has_Timer + __Has_Ticker>(&mut self, __fx: &mut __Fx, wait: Duration, out: crate::scheduler::SalvoReply) {
        { let __a1 = { let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Napping::Woke(__Has_Ticker::__get_Ticker(&mut *__fx).tick(), out)); __r }; __Has_Timer::__get_Timer(&mut *__fx).after(wait, __a1) };
    }

    fn woke<__Fx: __Has_Timer + __Has_Ticker>(&mut self, __fx: &mut __Fx, started: Tick, out: crate::scheduler::SalvoReply, f: Fired) {
        (out).send(Box::new(format!("napped {}", to_str__3(&elapsed(&mut *__fx, &started)))));
    }
}

pub enum __Cont_Napping {
    Nap(Duration),
    Woke(Tick, crate::scheduler::SalvoReply),
}

pub struct __Prov_Napping<__D0, __D1> {
    pub __d0: __D0,
    pub __d1: __D1,
}

impl<__D0: Timer, __D1: Ticker> __Has_Timer for __Prov_Napping<__D0, __D1> {
    fn __get_Timer(&mut self) -> &mut dyn Timer {
        &mut self.__d0
    }
}

impl<__D0: Timer, __D1: Ticker> __Has_Ticker for __Prov_Napping<__D0, __D1> {
    fn __get_Ticker(&mut self) -> &mut dyn Ticker {
        &mut self.__d1
    }
}

pub struct __Actor_Napping<__D0, __D1> {
    handler: Napping,
    prov: __Prov_Napping<__D0, __D1>,
}

impl<__D0, __D1> __Actor_Napping<__D0, __D1> {
    pub fn new(handler: Napping, prov: __Prov_Napping<__D0, __D1>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Timer, __D1: Ticker> __Actor_Napping<__D0, __D1> {
    fn __dispatch(&mut self, msg: crate::__Msg_Sleeper) {
        let mut __deps = __Deps_Napping{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Sleeper::Nap(wait, out) => __Impl_Napping::nap(&mut self.handler, &mut __deps, wait, out),
            crate::__Msg_Sleeper::Woke(started, out, f) => __Impl_Napping::woke(&mut self.handler, &mut __deps, started, out, f),
        }
    }
}

impl<__D0: Timer + Send + 'static, __D1: Ticker + Send + 'static> crate::scheduler::SalvoActor for __Actor_Napping<__D0, __D1> {
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
}

pub fn main() {
    let mut __fx = __Fx_main_1 { __h: StdOutConsole::new() };
    let mut budget = millis(1500i64);
    println(&mut __fx, &(format!("budget {}, doubled {}, in millis {}", to_str__3(&budget), to_str__3(&times(&budget, 2i64)), to_millis(&budget))));
    let mut stamp = epoch_milli(1700000000000i64);
    println(&mut __fx, &(format!("stamp {}s, a minute later {}s", to_epoch_second(&stamp), to_epoch_second(&(plus__2(&stamp, &(minutes(1i64))))))));
    let mut __fx2 = __Fx_main_2 { __outer: &mut __fx, __h: crate::time::__Lock_Clock::new(DefaultClock::new()) };
    let mut __fx3 = __Fx_main_3 { __outer: &mut __fx2, __h: DefaultTicker::new() };
    { let __a1 = &(format!("wall clock is set: {}", to_epoch_second(&(__Has_Clock::__get_Clock(&mut __fx3).now())) > ((1600000000) as i64))); println(&mut __fx3, __a1) };
    let mut __fx4 = __Fx_main_4 { __outer: &mut __fx3, __h: crate::time::__Lock_Ticker::new(SteppingTicker::new(millis(500i64))) };
    let mut started = __Has_Ticker::__get_Ticker(&mut __fx4).tick();
    { let __a2 = &(format!("overdue after one more read: {}", overdue(&mut __fx4, &started, &budget))); println(&mut __fx4, __a2) };
    { let __a3 = &(format!("overdue after three: {} {} {}", overdue(&mut __fx4, &started, &budget), overdue(&mut __fx4, &started, &budget), overdue(&mut __fx4, &started, &budget))); println(&mut __fx4, __a3) };
    println(&mut __fx4, &(verdict(&(Tick { nanos: 0i64 }), &(Tick { nanos: 1000000000i64 }), &budget)));
    println(&mut __fx4, &(verdict(&(Tick { nanos: 0i64 }), &(Tick { nanos: 2000000000i64 }), &budget)));
    let mut p = crate::scheduler::salvo_pool(((1) as usize));
    let (mut timer, mut ctl) = ({ let __h = ManualTime::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_ManualTime::new(__h))); (__a, __a) });
    let mut sessions = ({ let __h = Sessions::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(p, __cap as usize, Box::new(__Actor_Sessions::new(__h, __Prov_Sessions { __d0: __Stub_Timer::new(timer) }))) });
    let mut outcome = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_send(sessions, Box::new(crate::__Msg_Session::Open(Tick { nanos: 0i64 }, budget, answer)));
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_on_idle(p, settled, |__gates, __tokens| Box::new(Idle { parked_gates: __gates, parked_tokens: __tokens }));
            *crate::scheduler::salvo_wait(__wid).downcast::<Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send(ctl, Box::new(crate::time::__Msg_TimerCtl::Advance(millis(2500i64))));
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut __fx4, &(format!("session: {}", outcome)));
    let mut sleeper = ({ let __h = Napping::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(crate::scheduler::salvo_thread(), __cap as usize, Box::new(__Actor_Napping::new(__h, __Prov_Napping { __d0: __Stub_Timer::new(timer), __d1: TestTicker::new(timer) }))) });
    let mut napped = {
        let (mut answer, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_send(sleeper, Box::new(crate::__Msg_Sleeper::Nap(seconds(2i64), answer)));
        {
            let (mut settled, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_on_idle(p, settled, |__gates, __tokens| Box::new(Idle { parked_gates: __gates, parked_tokens: __tokens }));
            *crate::scheduler::salvo_wait(__wid).downcast::<Idle>().expect("the awaited answer")
        };
        crate::scheduler::salvo_send(ctl, Box::new(crate::time::__Msg_TimerCtl::Advance(seconds(2i64))));
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut __fx4, &napped);
}

pub struct __Fx_main_1<__H> {
    __h: __H,
}

impl<__H: Console> __Has_Console for __Fx_main_1<__H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        &mut self.__h
    }
}

pub struct __Fx_main_2<'a, __H> {
    __outer: &'a mut dyn __Has_Console,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_2<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Clock> __Has_Clock for __Fx_main_2<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        &mut self.__h
    }
}

pub trait __Prov_Clock_Console: __Has_Clock + __Has_Console {}
impl<T: __Has_Clock + __Has_Console + ?Sized> __Prov_Clock_Console for T {}

pub struct __Fx_main_3<'a, __H> {
    __outer: &'a mut dyn __Prov_Clock_Console,
    __h: __H,
}

impl<'a, __H> __Has_Clock for __Fx_main_3<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_3<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Ticker> __Has_Ticker for __Fx_main_3<'a, __H> {
    fn __get_Ticker(&mut self) -> &mut dyn Ticker {
        &mut self.__h
    }
}

pub trait __Prov_Clock_Console_Ticker: __Has_Clock + __Has_Console + __Has_Ticker {}
impl<T: __Has_Clock + __Has_Console + __Has_Ticker + ?Sized> __Prov_Clock_Console_Ticker for T {}

pub struct __Fx_main_4<'a, __H> {
    __outer: &'a mut dyn __Prov_Clock_Console_Ticker,
    __h: __H,
}

impl<'a, __H> __Has_Clock for __Fx_main_4<'a, __H> {
    fn __get_Clock(&mut self) -> &mut dyn Clock {
        __Has_Clock::__get_Clock(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_Console for __Fx_main_4<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: Ticker> __Has_Ticker for __Fx_main_4<'a, __H> {
    fn __get_Ticker(&mut self) -> &mut dyn Ticker {
        &mut self.__h
    }
}
