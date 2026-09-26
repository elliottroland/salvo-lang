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
#[path = "core/nonempty.rs"]
pub mod core_nonempty;
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

pub trait Counter {
    fn bump(&mut self, n: i32);
    fn total(&mut self, out: crate::scheduler::SalvoReply);
}

pub trait __Has_Counter {
    fn __get_Counter(&mut self) -> &mut dyn Counter;
}

pub struct __Stub_Counter {
    addr: usize,
}

impl __Stub_Counter {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Counter for __Stub_Counter {
    fn bump(&mut self, n: i32) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Counter::Bump(n)));
    }
    fn total(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Counter::Total(out)));
    }
}

pub enum __Msg_Counter {
    Bump(i32),
    Total(crate::scheduler::SalvoReply),
}

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

impl Counter for Counting {

    fn bump(&mut self, n: i32) {
        self.sum = self.sum + n;
    }

    fn total(&mut self, out: crate::scheduler::SalvoReply) {
        (out).send(Box::new(self.sum));
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
            crate::__Msg_Counter::Bump(n) => crate::Counter::bump(&mut self.handler, n),
            crate::__Msg_Counter::Total(out) => crate::Counter::total(&mut self.handler, out),
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
}

pub trait Ledger {
    fn report(&mut self, label: String, out: crate::scheduler::SalvoReply);
    fn reported(&mut self, label: String, out: crate::scheduler::SalvoReply, total: i32);
}

pub trait __Has_Ledger {
    fn __get_Ledger(&mut self) -> &mut dyn Ledger;
}

pub struct __Stub_Ledger {
    addr: usize,
}

impl __Stub_Ledger {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Ledger for __Stub_Ledger {
    fn report(&mut self, label: String, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Ledger::Report(label, out)));
    }
    fn reported(&mut self, label: String, out: crate::scheduler::SalvoReply, total: i32) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Ledger::Reported(label, out, total)));
    }
}

pub enum __Msg_Ledger {
    Report(String, crate::scheduler::SalvoReply),
    Reported(String, crate::scheduler::SalvoReply, i32),
}

pub struct Bookkeeping {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Bookkeeping>,
}

impl Bookkeeping {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 4,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_Bookkeeping<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Counter + ?Sized> __Has_Counter for __Deps_Bookkeeping<'a, __P> {
    fn __get_Counter(&mut self) -> &mut dyn Counter {
        __Has_Counter::__get_Counter(&mut *self.__p)
    }
}

pub trait __Impl_Bookkeeping {

    fn report<__Fx: __Has_Counter>(&mut self, __fx: &mut __Fx, label: String, out: crate::scheduler::SalvoReply);

    fn reported<__Fx: __Has_Counter>(&mut self, __fx: &mut __Fx, label: String, out: crate::scheduler::SalvoReply, total: i32);
}

impl __Impl_Bookkeeping for Bookkeeping {

    fn report<__Fx: __Has_Counter>(&mut self, __fx: &mut __Fx, label: String, out: crate::scheduler::SalvoReply) {
        __Has_Counter::__get_Counter(&mut *__fx).total({ let (__r, __s) = crate::scheduler::salvo_mint(self.__addr.expect("a parking handler runs as an actor")); self.__parked.insert(__s, __Cont_Bookkeeping::Reported(label, out)); __r });
    }

    fn reported<__Fx: __Has_Counter>(&mut self, __fx: &mut __Fx, label: String, out: crate::scheduler::SalvoReply, total: i32) {
        (out).send(Box::new(format!("{}={}", label, total)));
    }
}

pub enum __Cont_Bookkeeping {
    Report(String),
    Reported(String, crate::scheduler::SalvoReply),
}

pub struct __Prov_Bookkeeping<__D0> {
    pub __d0: __D0,
}

impl<__D0: Counter> __Has_Counter for __Prov_Bookkeeping<__D0> {
    fn __get_Counter(&mut self) -> &mut dyn Counter {
        &mut self.__d0
    }
}

pub struct __Actor_Bookkeeping<__D0> {
    handler: Bookkeeping,
    prov: __Prov_Bookkeeping<__D0>,
}

impl<__D0> __Actor_Bookkeeping<__D0> {
    pub fn new(handler: Bookkeeping, prov: __Prov_Bookkeeping<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Counter> __Actor_Bookkeeping<__D0> {
    fn __dispatch(&mut self, msg: crate::__Msg_Ledger) {
        let mut __deps = __Deps_Bookkeeping{ __p: &mut self.prov };
        match msg {
            crate::__Msg_Ledger::Report(label, out) => __Impl_Bookkeeping::report(&mut self.handler, &mut __deps, label, out),
            crate::__Msg_Ledger::Reported(label, out, total) => __Impl_Bookkeeping::reported(&mut self.handler, &mut __deps, label, out, total),
        }
    }
}

impl<__D0: Counter + Send + 'static> crate::scheduler::SalvoActor for __Actor_Bookkeeping<__D0> {
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
}

pub trait Desk {
    fn ticket(&mut self, out: crate::scheduler::SalvoReply);
    fn serve(&mut self, name: String);
    fn close_up(&mut self, reason: String);
}

pub trait __Has_Desk {
    fn __get_Desk(&mut self) -> &mut dyn Desk;
}

pub struct __Stub_Desk {
    addr: usize,
}

impl __Stub_Desk {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Desk for __Stub_Desk {
    fn ticket(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Desk::Ticket(out)));
    }
    fn serve(&mut self, name: String) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Desk::Serve(name)));
    }
    fn close_up(&mut self, reason: String) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Desk::CloseUp(reason)));
    }
}

pub enum __Msg_Desk {
    Ticket(crate::scheduler::SalvoReply),
    Serve(String),
    CloseUp(String),
}

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

impl Desk for Desking {

    fn ticket(&mut self, out: crate::scheduler::SalvoReply) {
        self.waiting.push(out);
    }

    fn serve(&mut self, name: String) {
        let mut next = self.waiting.salvo_remove_first();
        match next {
            Some(_) => {
                (next.unwrap()).send(Box::new(format!("served {}", name)));
            }
            None => {
                drop(name);
            }
        }
    }

    fn close_up(&mut self, reason: String) {
        std::mem::take(&mut self.waiting).into_iter().for_each(|r| (r).send(Box::new(format!("closed: {}", reason))));
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
            crate::__Msg_Desk::Ticket(out) => crate::Desk::ticket(&mut self.handler, out),
            crate::__Msg_Desk::Serve(name) => crate::Desk::serve(&mut self.handler, name),
            crate::__Msg_Desk::CloseUp(reason) => crate::Desk::close_up(&mut self.handler, reason),
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
}

pub trait Fragile {
    fn crash(&mut self);
}

pub trait __Has_Fragile {
    fn __get_Fragile(&mut self) -> &mut dyn Fragile;
}

pub struct __Stub_Fragile {
    addr: usize,
}

impl __Stub_Fragile {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Fragile for __Stub_Fragile {
    fn crash(&mut self) {
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Fragile::Crash));
    }
}

pub enum __Msg_Fragile {
    Crash,
}

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

impl Fragile for Breaking {

    fn crash(&mut self) {
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
            crate::__Msg_Fragile::Crash => crate::Fragile::crash(&mut self.handler),
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

pub fn formatted(label: String, out: crate::scheduler::SalvoReply, total: i32) {
    (out).send(Box::new(format!("{} totalled {}", label, total)));
}

pub fn report_line(counter: usize, label: String, out: crate::scheduler::SalvoReply) {
    crate::scheduler::salvo_send(counter, Box::new(crate::__Msg_Counter::Total(({ let __c0 = label; let __c1 = out; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), Box::new(move |__v| formatted(__c0, __c1, *__v.downcast::<i32>().expect("the awaited answer")))) }))));
}

pub fn main() {
    let mut __fx = __Fx_main_1 { __h: StdOutConsole::new() };
    let mut workers = crate::scheduler::salvo_pool(((2) as usize));
    let mut counter = ({ let __h = Counting::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Counting::new(__h))) });
    crate::scheduler::salvo_send(counter, Box::new(crate::__Msg_Counter::Bump(2)));
    crate::scheduler::salvo_send(counter, Box::new(crate::__Msg_Counter::Bump(3)));
    let mut sum = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_send(counter, Box::new(crate::__Msg_Counter::Total(out)));
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
    println(&mut __fx, &(format!("1. counter total is {}", sum)));
    let mut ledger = ({ let __h = Bookkeeping::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Bookkeeping::new(__h, __Prov_Bookkeeping { __d0: __Stub_Counter::new(counter) }))) });
    let mut line = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_send(ledger, Box::new(crate::__Msg_Ledger::Report("counter".to_string(), out)));
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut __fx, &(format!("3. ledger says {}", line)));
    let mut desk = ({ let __h = Desking::new(8); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Desking::new(__h))) });
    let mut first = {
        let (mut a, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_send(desk, Box::new(crate::__Msg_Desk::Ticket(a)));
        let mut second = {
            let (mut b, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_send(desk, Box::new(crate::__Msg_Desk::Ticket(b)));
            crate::scheduler::salvo_send(desk, Box::new(crate::__Msg_Desk::Serve("ada".to_string())));
            crate::scheduler::salvo_send(desk, Box::new(crate::__Msg_Desk::CloseUp("end of day".to_string())));
            *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
        };
        println(&mut __fx, &(format!("4. second waiter got: {}", second)));
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut __fx, &(format!("4. first waiter got: {}", first)));
    let mut fragile = ({ let __h = Breaking::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(workers, __cap as usize, Box::new(__Actor_Breaking::new(__h))) });
    let mut exit = {
        let (mut gone, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_watch(fragile, gone, |__reason| Box::new(Exit { reason: __reason }));
        crate::scheduler::salvo_send(fragile, Box::new(crate::__Msg_Fragile::Crash));
        *crate::scheduler::salvo_wait(__wid).downcast::<Exit>().expect("the awaited answer")
    };
    println(&mut __fx, &(format!("5. it died with a reason: {}", (exit.reason.chars().count() as i32) > 0)));
    crate::scheduler::salvo_send(fragile, Box::new(crate::__Msg_Fragile::Crash));
    let mut __fx2 = __Fx_main_2 { __outer: &mut __fx, __h: Counting::new() };
    __Has_Counter::__get_Counter(&mut __fx2).bump(4);
    __Has_Counter::__get_Counter(&mut __fx2).bump(5);
    let mut inline = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        __Has_Counter::__get_Counter(&mut __fx2).total(out);
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
    println(&mut __fx2, &(format!("6. inline total is {}", inline)));
    let mut mine = ({ let __h = Counting::new(); let __cap = __h.__mailbox_capacity; crate::scheduler::salvo_spawn(crate::scheduler::salvo_current_pool(), __cap as usize, Box::new(__Actor_Counting::new(__h))) });
    crate::scheduler::salvo_send(mine, Box::new(crate::__Msg_Counter::Bump(6)));
    let mut local = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_send(mine, Box::new(crate::__Msg_Counter::Total(out)));
        *crate::scheduler::salvo_wait(__wid).downcast::<i32>().expect("the awaited answer")
    };
    println(&mut __fx2, &(format!("7. the main pool's own actor totalled {}", local)));
    let mut line8 = {
        let (mut out, __wid) = crate::scheduler::salvo_waiter();
        report_line(mine, "the counter".to_string(), out);
        *crate::scheduler::salvo_wait(__wid).downcast::<String>().expect("the awaited answer")
    };
    println(&mut __fx2, &(format!("8. {}", line8)));
    println(&mut __fx2, &("done".to_string()));
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

impl<'a, __H: Counter> __Has_Counter for __Fx_main_2<'a, __H> {
    fn __get_Counter(&mut self) -> &mut dyn Counter {
        &mut self.__h
    }
}
