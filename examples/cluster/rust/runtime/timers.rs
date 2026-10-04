use crate::core_actor::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::runtime::*;
use crate::seq::*;
use crate::time::*;

/// [mod-use] The module's `use` #0, bound on first use.
fn __module_use_0() -> &'static crate::runtime_timers::DeadlineTable {
    static CELL: std::sync::OnceLock<crate::runtime_timers::DeadlineTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
            let deadline_table = crate::runtime_timers::DeadlineTable::locked(Deadlines::new());
        deadline_table
    })
}

/// [mod-use] The module's `use` #1, bound on first use.
fn __module_use_1() -> &'static crate::runtime_timers::Wheel {
    static CELL: std::sync::OnceLock<crate::runtime_timers::Wheel> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
            let mut wheel = crate::runtime_timers::Wheel::shared(__Stub_Wheel::new(({ let __h = Wheeling::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_thread(), __cap as usize, std::boxed::Box::new(__Actor_Wheeling::new(__h)), __DECODE_Wheeling); __a })));
        wheel
    })
}

pub trait __Stateless_DeadlineTable: Send + Sync {
    fn register(&self, at: i64, done: crate::scheduler::SalvoReply) -> bool;
    fn wheel_parker(&self, p: Parker);
    fn waker(&self) -> Option<Parker>;
    fn take_due(&self, now: i64) -> Vec<crate::scheduler::SalvoReply>;
    fn next_deadline(&self) -> Option<i64>;
    fn clear(&self);
}

pub trait __Stateful_DeadlineTable: Send {
    fn register(&mut self, at: i64, done: crate::scheduler::SalvoReply) -> bool;
    fn wheel_parker(&mut self, p: Parker);
    fn waker(&mut self) -> Option<Parker>;
    fn take_due(&mut self, now: i64) -> Vec<crate::scheduler::SalvoReply>;
    fn next_deadline(&mut self) -> Option<i64>;
    fn clear(&mut self);
}

pub struct DeadlineTable {
    inner: __Inner_DeadlineTable,
}

pub enum __Inner_DeadlineTable {
    Shared(std::sync::Arc<dyn __Stateless_DeadlineTable>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_DeadlineTable>>),
}

impl Clone for DeadlineTable {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_DeadlineTable::Shared(h) => __Inner_DeadlineTable::Shared(h.clone()),
            __Inner_DeadlineTable::Locked(h) => __Inner_DeadlineTable::Locked(h.clone()),
        } }
    }
}

impl DeadlineTable {
    pub fn shared<__H: __Stateless_DeadlineTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_DeadlineTable::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_DeadlineTable>) -> Self {
        Self { inner: __Inner_DeadlineTable::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_DeadlineTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_DeadlineTable::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_DeadlineTable>>) -> Self {
        Self { inner: __Inner_DeadlineTable::Locked(inner) }
    }
    pub fn register(&self, at: i64, done: crate::scheduler::SalvoReply) -> bool {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.register(at, done),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().register(at, done),
        }
    }
    pub fn wheel_parker(&self, p: Parker) {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.wheel_parker(p),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().wheel_parker(p),
        }
    }
    pub fn waker(&self) -> Option<Parker> {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.waker(),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().waker(),
        }
    }
    pub fn take_due(&self, now: i64) -> Vec<crate::scheduler::SalvoReply> {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.take_due(now),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().take_due(now),
        }
    }
    pub fn next_deadline(&self) -> Option<i64> {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.next_deadline(),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().next_deadline(),
        }
    }
    pub fn clear(&self) {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.clear(),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().clear(),
        }
    }
}

pub struct Deadlines {
    ats: Vec<i64>,
    dones: Vec<crate::scheduler::SalvoReply>,
    running: bool,
    parker: Option<Parker>,
    forgotten: Vec<crate::scheduler::SalvoReply>,
}

impl Deadlines {
    pub fn new() -> Self {
        Self {
            ats: vec![],
            dones: vec![],
            running: false,
            parker: None,
            forgotten: vec![],
        }
    }
}

impl crate::runtime_timers::__Stateful_DeadlineTable for Deadlines {

    fn register(&mut self, at: i64, done: crate::scheduler::SalvoReply) -> bool {
        self.ats.push(at);
        self.dones.push(done);
        if self.running {
            return false;
        }
        self.running = true;
        return true;
    }

    fn wheel_parker(&mut self, p: Parker) {
        self.parker = Some(p.clone());
    }

    fn waker(&mut self) -> Option<Parker> {
        if self.running {
            return self.parker.clone();
        }
        return None;
    }

    fn take_due(&mut self, now: i64) -> Vec<crate::scheduler::SalvoReply> {
        let mut due: Vec<crate::scheduler::SalvoReply> = vec![];
        let mut i = 0;
        while i < (self.ats.len() as i32) {
            if *self.ats.get((i) as i64 as usize).expect("salvo: value is absent at runtime.timers:72:16") <= now {
                let mut _at = self.ats.salvo_remove_at(i);
                let mut __is1 = self.dones.salvo_remove_at(i);
                if __is1.is_some() {
                    let mut r = __is1.unwrap();
                    due.push(r);
                }
            } else {
                i = i + 1;
            }
        }
        return due;
    }

    fn next_deadline(&mut self) -> Option<i64> {
        let mut earliest: Option<i64> = None;
        for at in &self.ats {
            if ((earliest.is_none()) || *at < earliest.unwrap()) {
                earliest = Some(at.clone());
            }
        }
        if earliest.is_none() {
            self.running = false;
        }
        return earliest;
    }

    fn clear(&mut self) {
        loop {
            let mut __is2 = self.dones.salvo_remove_first();
            if !(__is2.is_some()) {
                break;
            }
            let mut r = __is2.unwrap();
            self.forgotten.push(r);
        }
        while self.ats.salvo_remove_first().is_some() {
        }
        self.running = false;
    }
}

pub fn answer_all(mut due: Vec<crate::scheduler::SalvoReply>, now: i64) {
    due.into_iter().for_each(|r| crate::scheduler::salvo_reply_wire::<Fired>(r, Fired { at: Tick { nanos: now.clone() } }));
}

pub trait __Stateless_Wheel: Send + Sync {
    fn run(&self);
}

pub trait __Stateful_Wheel: Send {
    fn run(&mut self);
}

pub struct __Stub_Wheel {
    addr: usize,
}

impl __Stub_Wheel {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Wheel for __Stub_Wheel {
    fn run(&self) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Wheel::Run, crate::runtime_timers::__PROTO_Wheel);
    }
}

pub struct Wheel {
    inner: __Inner_Wheel,
}

pub enum __Inner_Wheel {
    Shared(std::sync::Arc<dyn __Stateless_Wheel>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Wheel>>),
}

impl Clone for Wheel {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Wheel::Shared(h) => __Inner_Wheel::Shared(h.clone()),
            __Inner_Wheel::Locked(h) => __Inner_Wheel::Locked(h.clone()),
        } }
    }
}

impl Wheel {
    pub fn shared<__H: __Stateless_Wheel + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Wheel::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Wheel>) -> Self {
        Self { inner: __Inner_Wheel::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Wheel + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Wheel::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Wheel>>) -> Self {
        Self { inner: __Inner_Wheel::Locked(inner) }
    }
    pub fn run(&self) {
        match &self.inner {
            __Inner_Wheel::Shared(h) => h.run(),
            __Inner_Wheel::Locked(h) => h.lock().unwrap().run(),
        }
    }
}

pub enum __Msg_Wheel {
    Run,
}

impl crate::wire::__Wire for __Msg_Wheel {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Wheel::Run => out.push(0),
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Wheel::Run),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Wheel`.
pub const __PROTO_Wheel: &str = "df3353758a650c52";

pub struct Wheeling {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
}

impl Wheeling {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 2,
            __addr: None,
        }
    }
}

impl crate::runtime_timers::__Stateless_Wheel for Wheeling {

    fn run(&self) {
        __module_use_0().wheel_parker(this_parker_platform());
        loop {
            let mut now = now_nanos();
            let mut due = __module_use_0().take_due(now);
            answer_all(due, now);
            let mut until = __module_use_0().next_deadline();
            if until.is_none() {
                return;
            }
            let mut wait = until.unwrap() - now_nanos();
            if wait > ((0) as i64) {
                park_nanos_platform(&(this_parker_platform()), wait);
            }
        }
    }
}

pub struct __Actor_Wheeling {
    handler: Wheeling,
}

impl __Actor_Wheeling {
    pub fn new(handler: Wheeling) -> Self {
        Self { handler }
    }
}

impl __Actor_Wheeling {
    fn __dispatch(&mut self, msg: crate::runtime_timers::__Msg_Wheel) {
        match msg {
            crate::runtime_timers::__Msg_Wheel::Run => crate::runtime_timers::__Stateless_Wheel::run(&mut self.handler),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Wheeling {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::runtime_timers::__Msg_Wheel>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, _slot: u64, _value: crate::scheduler::SalvoMsg) {
        unreachable!("this protocol has no continuation targets")
    }
}

pub const __DECODE_Wheeling: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Wheeling);
fn __decode_msg_Wheeling(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::runtime_timers::__PROTO_Wheel {
            return crate::wire::salvo_decode::<crate::runtime_timers::__Msg_Wheel>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn after_nanos(delay: i64, done: crate::scheduler::SalvoReply) {
    let mut wait = delay;
    if wait < ((0) as i64) {
        wait = 0i64;
    }
    if virtual_runtime() {
        if __module_use_0().register(now_nanos() + wait, done) {
            arm_clock();
        }
        return;
    }
    if __module_use_0().register(now_nanos() + wait, done) {
        __module_use_1().run();
        return;
    }
    let mut p = __module_use_0().waker();
    if p.is_some() {
        unpark_platform(p.as_ref().unwrap());
    }
}

pub fn arm_clock() {
    on_clock(mint_task_on(main_pool(), body_of_platform(std::boxed::Box::new(move |kind, slot, value| {
    drop_dyn_platform(value);
    advance();
}))));
}

pub fn advance() {
    let mut until = __module_use_0().next_deadline();
    if until.is_some() {
        let mut at = until.unwrap();
        set_virtual_now(at);
        let mut now = now_nanos();
        let mut due = __module_use_0().take_due(now.clone());
        answer_all(due, now);
        if __module_use_0().next_deadline().is_some() {
            arm_clock();
        }
    }
}

pub fn reset_timers() {
    __module_use_0().clear();
}
