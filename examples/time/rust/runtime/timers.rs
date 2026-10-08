use crate::runtime::Dyn;
use crate::time::Fired;
use crate::runtime::Parker;
use crate::time::Tick;
use crate::core_list::add_platform;
use crate::runtime::body_of_platform;
use crate::core_list::drain;
use crate::runtime::drop_dyn_platform;
use crate::core_list::get_platform;
use crate::runtime::main_pool;
use crate::runtime::mint_task_on;
use crate::runtime::now_nanos;
use crate::runtime::on_clock;
use crate::runtime::park_nanos_platform;
use crate::core_list::remove_at_platform;
use crate::core_list::remove_first_platform;
use crate::runtime::set_virtual_now;
use crate::core_list::size_platform;
use crate::runtime::this_parker_platform;
use crate::runtime::unpark_platform;
use crate::runtime::virtual_runtime;


pub fn __module_use0() -> &'static std::sync::Arc<std::sync::Mutex<crate::runtime_timers::Deadlines>> {
    static CELL: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<crate::runtime_timers::Deadlines>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Arc::new(std::sync::Mutex::new(crate::runtime_timers::Deadlines::new())))
}

pub fn __module_use0_0() -> &'static crate::runtime_timers::DeadlineTable {
    static CELL: std::sync::OnceLock<crate::runtime_timers::DeadlineTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| crate::runtime_timers::DeadlineTable::share_locked(crate::runtime_timers::__module_use0().clone()))
}

pub fn __module_use1() -> &'static crate::runtime_timers::Wheel {
    static CELL: std::sync::OnceLock<crate::runtime_timers::Wheel> = std::sync::OnceLock::new();
    CELL.get_or_init(|| crate::runtime_timers::Wheel::shared(crate::runtime_timers::__Stub_Wheel::new(({ let __h = crate::runtime_timers::Wheeling::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_thread(), __cap as usize, std::boxed::Box::new(crate::runtime_timers::__Actor_Wheeling::new(__h)), crate::runtime_timers::__DECODE_Wheeling); __a }))))
}

pub trait __Stateless_DeadlineTable: Send + Sync {
    fn register(&self, at: i64, done: crate::scheduler::SalvoReply) -> bool;
    fn wheel_parker(&self, p: crate::runtime::Parker);
    fn waker(&self) -> Option<crate::runtime::Parker>;
    fn take_due(&self, now: i64) -> Vec<crate::scheduler::SalvoReply>;
    fn next_deadline(&self) -> Option<i64>;
    fn clear(&self);
}

pub trait __Stateful_DeadlineTable: Send {
    fn register(&mut self, at: i64, done: crate::scheduler::SalvoReply) -> bool;
    fn wheel_parker(&mut self, p: crate::runtime::Parker);
    fn waker(&mut self) -> Option<crate::runtime::Parker>;
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
    pub fn wheel_parker(&self, p: crate::runtime::Parker) {
        match &self.inner {
            __Inner_DeadlineTable::Shared(h) => h.wheel_parker(p),
            __Inner_DeadlineTable::Locked(h) => h.lock().unwrap().wheel_parker(p),
        }
    }
    pub fn waker(&self) -> Option<crate::runtime::Parker> {
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
    parker: Option<crate::runtime::Parker>,
    forgotten: Vec<crate::scheduler::SalvoReply>,
}

impl Deadlines {
    pub fn new() -> Self {
        Self {
            ats: vec![],
            dones: vec![],
            running: false,
            parker: None,
            forgotten: vec![]
        }
    }
}

impl crate::runtime_timers::__Stateful_DeadlineTable for Deadlines {
    fn register(&mut self, at: i64, done: crate::scheduler::SalvoReply) -> bool {
        crate::core_list::add_platform::<i64>(&mut self.ats, at);
        crate::core_list::add_platform::<crate::scheduler::SalvoReply>(&mut self.dones, done);
        if self.running {
            return false;
        };
        self.running = true;
        return true;
    }
    fn wheel_parker(&mut self, p: crate::runtime::Parker) {
        self.parker = Some(p);
    }
    fn waker(&mut self) -> Option<crate::runtime::Parker> {
        if self.running {
            return (self.parker).clone();
        };
        return None;
    }
    fn take_due(&mut self, now: i64) -> Vec<crate::scheduler::SalvoReply> {
        let mut due: Vec<crate::scheduler::SalvoReply> = vec![];
        let mut i: i32 = 0i32;
        loop {
            if !((i < crate::core_list::size_platform::<i64>(&self.ats))) {
                break;
            };
            if ({
                let mut __nn_1: Option<i64> = crate::core_list::get_platform::<i64>(&self.ats, i).copied();
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.timers:72:16");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            } <= now) {
                let mut _at: Option<i64> = crate::core_list::remove_at_platform::<i64>(&mut self.ats, i);
                let mut __subject_3: Option<crate::scheduler::SalvoReply> = crate::core_list::remove_at_platform::<crate::scheduler::SalvoReply>(&mut self.dones, i);
                if __subject_3.is_some() {
                    let mut r = __subject_3.unwrap();
                    crate::core_list::add_platform::<crate::scheduler::SalvoReply>(&mut due, r);
                };
            } else {
                i = i32::wrapping_add(i, 1i32);
            };
        }
        return due;
    }
    fn next_deadline(&mut self) -> Option<i64> {
        let mut earliest: Option<i64> = None;
        for mut at in self.ats.iter().copied() {
            if if earliest.is_none() {
                true
            } else {
                let mut earliest_1 = earliest.unwrap();
                (at < earliest_1)
            } {
                earliest = Some(at);
            };
        }
        if earliest.is_none() {
            self.running = false;
        };
        return earliest;
    }
    fn clear(&mut self) {
        loop {
            let mut __subject_1: Option<crate::scheduler::SalvoReply> = crate::core_list::remove_first_platform::<crate::scheduler::SalvoReply>(&mut self.dones);
            if !(__subject_1.is_some()) {
                break;
            };
            let mut r = __subject_1.unwrap();
            crate::core_list::add_platform::<crate::scheduler::SalvoReply>(&mut self.forgotten, r);
        }
        loop {
            if !((crate::core_list::remove_first_platform::<i64>(&mut self.ats)).is_some()) {
                break;
            };
        }
        self.running = false;
    }
}

pub fn answer_all(mut due: Vec<crate::scheduler::SalvoReply>, mut now: i64) {
    crate::core_list::drain::<crate::scheduler::SalvoReply>(due, &mut |mut r| {
        crate::scheduler::salvo_reply_wire::<crate::time::Fired>(r, crate::time::Fired { at: crate::time::Tick { nanos: now } })
    });
}

pub trait __Stateless_Wheel: Send + Sync {
    fn run(&self);
}

pub trait __Stateful_Wheel: Send {
    fn run(&mut self);
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

pub struct Wheeling {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
}

impl Wheeling {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 2i32,
            __addr: None
        }
    }
}

impl crate::runtime_timers::__Stateless_Wheel for Wheeling {
    fn run(&self) {
        crate::runtime_timers::__module_use0_0().wheel_parker(crate::runtime::this_parker_platform());
        loop {
            if !(true) {
                break;
            };
            let mut now: i64 = crate::runtime::now_nanos();
            let mut due: Vec<crate::scheduler::SalvoReply> = crate::runtime_timers::__module_use0_0().take_due(now);
            crate::runtime_timers::answer_all(due, now);
            let mut until: Option<i64> = crate::runtime_timers::__module_use0_0().next_deadline();
            if until.is_none() {
                return;
            };
            let mut until_1 = until.unwrap();
            let mut wait: i64 = i64::wrapping_sub(until_1, crate::runtime::now_nanos());
            if (wait > 0i64) {
                crate::runtime::park_nanos_platform(&crate::runtime::this_parker_platform(), wait);
            };
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
        return crate::wire::salvo_decode::<crate::runtime_timers::__Msg_Wheel>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn after_nanos(mut delay: i64, mut done: crate::scheduler::SalvoReply) {
    let mut wait: i64 = delay;
    if (wait < 0i64) {
        wait = 0i64;
    };
    if crate::runtime::virtual_runtime() {
        if crate::runtime_timers::__module_use0_0().register(i64::wrapping_add(crate::runtime::now_nanos(), wait), done) {
            crate::runtime_timers::arm_clock();
        };
        return;
    };
    if crate::runtime_timers::__module_use0_0().register(i64::wrapping_add(crate::runtime::now_nanos(), wait), done) {
        crate::runtime_timers::__module_use1().run();
        return;
    };
    let mut p: Option<crate::runtime::Parker> = crate::runtime_timers::__module_use0_0().waker();
    if p.is_some() {
        let mut p_1 = p.as_ref().unwrap();
        crate::runtime::unpark_platform(p_1);
    };
}

pub fn arm_clock() {
    crate::runtime::on_clock(crate::runtime::mint_task_on(crate::runtime::main_pool(), crate::runtime::body_of_platform(std::boxed::Box::new(move |mut kind: i32, mut slot: i64, mut value: crate::runtime::Dyn| {
        crate::runtime::drop_dyn_platform(value);
        crate::runtime_timers::advance();
    }))));
}

pub fn advance() {
    let mut until: Option<i64> = crate::runtime_timers::__module_use0_0().next_deadline();
    if until.is_some() {
        let mut at = until.unwrap();
        crate::runtime::set_virtual_now(at);
        let mut now: i64 = crate::runtime::now_nanos();
        let mut due: Vec<crate::scheduler::SalvoReply> = crate::runtime_timers::__module_use0_0().take_due(now);
        crate::runtime_timers::answer_all(due, now);
        if (crate::runtime_timers::__module_use0_0().next_deadline()).is_some() {
            crate::runtime_timers::arm_clock();
        };
    };
}

pub fn reset_timers() {
    crate::runtime_timers::__module_use0_0().clear();
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
