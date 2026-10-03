use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;
use crate::unions::*;

/// [mod-use] The module's `use` #0, bound on first use.
fn __module_use_0() -> &'static crate::runtime::RuntimeHost {
    static CELL: std::sync::OnceLock<crate::runtime::RuntimeHost> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
            let runtime_host = crate::runtime::RuntimeHost::shared(crate::runtime::__Platform_HostRuntime::new());
        runtime_host
    })
}

/// [mod-use] The module's `use` #1, bound on first use.
fn __module_use_1() -> &'static crate::runtime::SchedTable {
    static CELL: std::sync::OnceLock<crate::runtime::SchedTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
            let sched_table = crate::runtime::SchedTable::locked({ let mut __h = Scheduler::new(); __h.init(); __h });
        sched_table
    })
}

/// [platform-type] The host's `Parker`.
pub use crate::platform_runtime::Parker;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + Sync>() {} __contract::<Parker>(); };

pub fn this_parker_platform() -> Parker {
    crate::platform_runtime::this_parker()
}

pub fn park_platform(p: &Parker) {
    crate::platform_runtime::park(p)
}

pub fn park_nanos_platform(p: &Parker, nanos: i64) {
    crate::platform_runtime::park_nanos(p, nanos)
}

pub fn unpark_platform(p: &Parker) {
    crate::platform_runtime::unpark(p)
}

pub fn start_thread_platform(body: Box<dyn FnOnce() + Send + 'static>) {
    crate::platform_runtime::start_thread(body)
}

pub fn guarded_platform(body: Box<dyn FnOnce() + Send + 'static>) -> Option<String> {
    crate::platform_runtime::guarded(body)
}

pub trait __Stateless_RuntimeHost: Send + Sync {
    fn secure_bits(&self) -> i64;
    fn report(&self, line: &String);
    fn mono_nanos(&self) -> i64;
}

pub trait __Stateful_RuntimeHost: Send {
    fn secure_bits(&mut self) -> i64;
    fn report(&mut self, line: &String);
    fn mono_nanos(&mut self) -> i64;
}

pub struct RuntimeHost {
    inner: __Inner_RuntimeHost,
}

pub enum __Inner_RuntimeHost {
    Shared(std::sync::Arc<dyn __Stateless_RuntimeHost>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_RuntimeHost>>),
}

impl Clone for RuntimeHost {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_RuntimeHost::Shared(h) => __Inner_RuntimeHost::Shared(h.clone()),
            __Inner_RuntimeHost::Locked(h) => __Inner_RuntimeHost::Locked(h.clone()),
        } }
    }
}

impl RuntimeHost {
    pub fn shared<__H: __Stateless_RuntimeHost + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RuntimeHost::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_RuntimeHost>) -> Self {
        Self { inner: __Inner_RuntimeHost::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_RuntimeHost + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RuntimeHost::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_RuntimeHost>>) -> Self {
        Self { inner: __Inner_RuntimeHost::Locked(inner) }
    }
    pub fn secure_bits(&self) -> i64 {
        match &self.inner {
            __Inner_RuntimeHost::Shared(h) => h.secure_bits(),
            __Inner_RuntimeHost::Locked(h) => h.lock().unwrap().secure_bits(),
        }
    }
    pub fn report(&self, line: &String) {
        match &self.inner {
            __Inner_RuntimeHost::Shared(h) => h.report(line),
            __Inner_RuntimeHost::Locked(h) => h.lock().unwrap().report(line),
        }
    }
    pub fn mono_nanos(&self) -> i64 {
        match &self.inner {
            __Inner_RuntimeHost::Shared(h) => h.mono_nanos(),
            __Inner_RuntimeHost::Locked(h) => h.lock().unwrap().mono_nanos(),
        }
    }
}

/// The adapter a `use` of a platform handler of `RuntimeHost` constructs [platform-abi].
pub struct __Platform_RuntimeHost<T>(pub T);

/// What a `threadsafe platform handler` of `RuntimeHost` implements [platform-abi].
pub trait RuntimeHostPlatformSync: Send + Sync {
    fn secure_bits(&self) -> i64;
    fn report(&self, line: &String);
    fn mono_nanos(&self) -> i64;
}

impl<T: RuntimeHostPlatformSync> __Stateless_RuntimeHost for __Platform_RuntimeHost<T> {
    fn secure_bits(&self) -> i64 {
        self.0.secure_bits()
    }
    fn report(&self, line: &String) {
        self.0.report(line)
    }
    fn mono_nanos(&self) -> i64 {
        self.0.mono_nanos()
    }
}

pub type __Platform_HostRuntime = crate::runtime::__Platform_RuntimeHost<crate::platform_runtime::HostRuntime>;

impl __Platform_HostRuntime {
    pub fn new() -> Self {
        crate::runtime::__Platform_RuntimeHost(crate::platform_runtime::HostRuntime::new())
    }
}

pub fn fresh_bits() -> i64 {
    return __module_use_0().secure_bits();
}

pub fn now_nanos() -> i64 {
    return __module_use_0().mono_nanos();
}

/// [platform-type] The host's `Dyn`.
pub use crate::platform_runtime::Dyn;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Dyn>(); };

pub fn erase_platform<T: Send + 'static>(v: T) -> Dyn {
    crate::platform_runtime::erase(v)
}

pub fn unerase_platform<T: Send + 'static>(d: Dyn) -> T {
    crate::platform_runtime::unerase(d)
}

pub fn drop_dyn_platform(d: Dyn) {
    crate::platform_runtime::drop_dyn(d)
}

/// [platform-type] The host's `RtBody`.
pub use crate::platform_runtime::RtBody;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<RtBody>(); };

pub fn body_of_platform(f: Box<dyn FnMut(i32, i64, Dyn) + Send + 'static>) -> RtBody {
    crate::platform_runtime::body_of(f)
}

pub fn activate_platform(b: RtBody, kind: i32, slot: i64, value: Dyn) -> RtRan {
    crate::platform_runtime::activate(b, kind, slot, value)
}

pub fn drop_body_platform(b: RtBody) {
    crate::platform_runtime::drop_body(b)
}

pub fn granted_platform(addr: i32, from: i64) {
    crate::platform_runtime::granted(addr, from)
}

pub fn flush_frames_platform() {
    crate::platform_runtime::flush_frames()
}

pub fn exit_process_platform(code: i32) -> ! {
    crate::platform_runtime::exit_process(code)
}


pub struct RtRan {
    pub body: RtBody,
    pub fault: Option<String>,
}

impl std::fmt::Debug for RtRan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtRan")
            .field("body", &"<fn>")
            .field("fault", &self.fault)
            .finish()
    }
}

pub fn drop_ran(r: RtRan) {
    let __destructured1 = r;
    let mut body = __destructured1.body;
    let mut fault = __destructured1.fault;
    drop_body_platform(body);
}

/// [platform-type] The host's `RtSlot`.
pub use crate::platform_runtime::RtSlot;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<RtSlot<i32>>(); };

pub fn slot_of_platform<T: Send + 'static>(v: T) -> RtSlot<T> {
    crate::platform_runtime::slot_of(v)
}

pub fn slot_empty_platform<T: Send + 'static>() -> RtSlot<T> {
    crate::platform_runtime::slot_empty()
}

pub fn slot_take_platform<T: Send + 'static>(s: &mut RtSlot<T>) -> Option<T> {
    crate::platform_runtime::slot_take(s)
}

pub fn slot_put_platform<T: Send + 'static>(s: &mut RtSlot<T>, v: T) {
    crate::platform_runtime::slot_put(s, v)
}

pub fn drop_slot_platform<T: Send + 'static>(s: RtSlot<T>) {
    crate::platform_runtime::drop_slot(s)
}

pub fn here_pool_platform() -> i32 {
    crate::platform_runtime::here_pool()
}

pub fn here_actor_platform() -> i32 {
    crate::platform_runtime::here_actor()
}

pub fn set_here_platform(pool: i32, actor: i32) {
    crate::platform_runtime::set_here(pool, actor)
}

pub fn main_pool() -> i32 {
    return 0;
}

pub fn no_frame() -> i32 {
    return -1;
}

pub fn task_frame() -> i32 {
    return -2;
}


pub struct RtDelivered {
    pub msg: Dyn,
    pub from: i64,
}

impl std::fmt::Debug for RtDelivered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtDelivered")
            .field("msg", &"<fn>")
            .field("from", &self.from)
            .finish()
    }
}


pub struct RtAnswered {
    pub slot: i64,
    pub value: Dyn,
}

impl std::fmt::Debug for RtAnswered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtAnswered")
            .field("slot", &self.slot)
            .field("value", &"<fn>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtReported {
    pub reason: String,
}

impl crate::wire::__Wire for RtReported {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.reason, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            reason: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn drop_delivered(d: RtDelivered) {
    let __destructured2 = d;
    let mut msg = __destructured2.msg;
    let mut from = __destructured2.from;
    drop_dyn_platform(msg);
}

pub fn drop_answered(a: RtAnswered) {
    let __destructured3 = a;
    let mut slot = __destructured3.slot;
    let mut value = __destructured3.value;
    drop_dyn_platform(value);
}

pub fn drop_entry(e: Union3<RtDelivered, RtAnswered, RtReported>) {
    if matches!(e, Union3::U1(_)) {
        let mut d = match e { Union3::U1(__v) => __v, _ => unreachable!() };
        drop_delivered(d);
    } else if matches!(e, Union3::U2(_)) {
        let mut a = match e { Union3::U2(__v) => __v, _ => unreachable!() };
        drop_answered(a);
    } else {
        drop(e.u3().clone());
    }
}


pub struct RtActorRec {
    pub body: RtSlot<RtBody>,
    pub pool: i32,
    pub bound: i32,
    pub queue: std::collections::VecDeque<Union3<RtDelivered, RtAnswered, RtReported>>,
    pub slots: std::collections::VecDeque<i64>,
    pub user_len: i32,
    pub gate: Option<i64>,
    pub running: bool,
    pub dead: bool,
    pub exit_reason: String,
    pub blocked: Vec<Parker>,
    pub watchers: Vec<RtToken>,
    pub owed: i32,
    pub ready: bool,
}

impl std::fmt::Debug for RtActorRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtActorRec")
            .field("body", &"<fn>")
            .field("pool", &self.pool)
            .field("bound", &self.bound)
            .field("queue", &"<fn>")
            .field("slots", &self.slots)
            .field("user_len", &self.user_len)
            .field("gate", &self.gate)
            .field("running", &self.running)
            .field("dead", &self.dead)
            .field("exit_reason", &self.exit_reason)
            .field("blocked", &"<fn>")
            .field("watchers", &"<fn>")
            .field("owed", &self.owed)
            .field("ready", &self.ready)
            .finish()
    }
}

pub fn drop_actor_rec(a: RtActorRec) {
    let __destructured4 = a;
    let mut body = __destructured4.body;
    let mut pool = __destructured4.pool;
    let mut bound = __destructured4.bound;
    let mut queue = __destructured4.queue;
    let mut slots = __destructured4.slots;
    let mut user_len = __destructured4.user_len;
    let mut gate = __destructured4.gate;
    let mut running = __destructured4.running;
    let mut dead = __destructured4.dead;
    let mut exit_reason = __destructured4.exit_reason;
    let mut blocked = __destructured4.blocked;
    let mut watchers = __destructured4.watchers;
    let mut owed = __destructured4.owed;
    let mut ready = __destructured4.ready;
    drop_slot_platform(body);
    queue.into_iter().for_each(|e| drop_entry(e));
    watchers.into_iter().for_each(|t| drop_token(t));
}


pub struct RtWaiterRec {
    pub pool: i32,
    pub value: RtSlot<Dyn>,
    pub filled: bool,
    pub parker: Option<Parker>,
    pub waiting: i32,
    pub waiting_actor: i32,
}

impl std::fmt::Debug for RtWaiterRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtWaiterRec")
            .field("pool", &self.pool)
            .field("value", &"<fn>")
            .field("filled", &self.filled)
            .field("parker", &"<fn>")
            .field("waiting", &self.waiting)
            .field("waiting_actor", &self.waiting_actor)
            .finish()
    }
}

pub fn drop_waiter_rec(w: RtWaiterRec) {
    let __destructured5 = w;
    let mut pool = __destructured5.pool;
    let mut value = __destructured5.value;
    let mut filled = __destructured5.filled;
    let mut parker = __destructured5.parker;
    let mut waiting = __destructured5.waiting;
    let mut waiting_actor = __destructured5.waiting_actor;
    drop_slot_platform(value);
}


pub struct RtTaskRun {
    pub body: RtBody,
    pub value: Dyn,
}

impl std::fmt::Debug for RtTaskRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtTaskRun")
            .field("body", &"<fn>")
            .field("value", &"<fn>")
            .finish()
    }
}

pub fn drop_task_run(t: RtTaskRun) {
    let __destructured6 = t;
    let mut body = __destructured6.body;
    let mut value = __destructured6.value;
    drop_body_platform(body);
    drop_dyn_platform(value);
}


pub struct RtPoolRec {
    pub idle: Vec<Parker>,
    pub tasks: std::collections::VecDeque<RtTaskRun>,
    pub ready: std::collections::VecDeque<i32>,
    pub sink: i32,
    pub owed: i32,
    pub deferred: i32,
}

impl std::fmt::Debug for RtPoolRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtPoolRec")
            .field("idle", &"<fn>")
            .field("tasks", &"<fn>")
            .field("ready", &self.ready)
            .field("sink", &self.sink)
            .field("owed", &self.owed)
            .field("deferred", &self.deferred)
            .finish()
    }
}

pub fn drop_pool_rec(p: RtPoolRec) {
    let __destructured7 = p;
    let mut idle = __destructured7.idle;
    let mut tasks = __destructured7.tasks;
    let mut ready = __destructured7.ready;
    let mut sink = __destructured7.sink;
    let mut owed = __destructured7.owed;
    let mut deferred = __destructured7.deferred;
    tasks.into_iter().for_each(|t| drop_task_run(t));
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtToActor {
    pub addr: i32,
}

impl crate::wire::__Wire for RtToActor {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.addr, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            addr: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtToWaiter {
    pub wid: i32,
}

impl crate::wire::__Wire for RtToWaiter {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.wid, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            wid: crate::wire::__Wire::__dec(r)?,
        })
    }
}


pub struct RtToTask {
    pub pool: i32,
    pub body: RtBody,
}

impl std::fmt::Debug for RtToTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtToTask")
            .field("pool", &self.pool)
            .field("body", &"<fn>")
            .finish()
    }
}

pub fn drop_to_task(t: RtToTask) {
    let __destructured8 = t;
    let mut pool = __destructured8.pool;
    let mut body = __destructured8.body;
    drop_body_platform(body);
}


pub struct RtToken {
    pub target: Union3<RtToActor, RtToWaiter, RtToTask>,
    pub slot: i64,
    pub tracked: bool,
}

impl std::fmt::Debug for RtToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtToken")
            .field("target", &"<fn>")
            .field("slot", &self.slot)
            .field("tracked", &self.tracked)
            .finish()
    }
}

pub fn drop_token(t: RtToken) {
    answer(t, erase_platform(0));
}


pub struct RtWaiterMint {
    pub token: RtToken,
    pub wid: i32,
}

impl std::fmt::Debug for RtWaiterMint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtWaiterMint")
            .field("token", &"<fn>")
            .field("wid", &self.wid)
            .finish()
    }
}

pub fn drop_waiter_mint(m: RtWaiterMint) {
    let __destructured9 = m;
    let mut token = __destructured9.token;
    let mut wid = __destructured9.wid;
    drop_token(token);
}


pub struct RtRunActor {
    pub addr: i32,
    pub pool: i32,
    pub kind: i32,
    pub slot: i64,
    pub value: Dyn,
    pub body: RtBody,
}

impl std::fmt::Debug for RtRunActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtRunActor")
            .field("addr", &self.addr)
            .field("pool", &self.pool)
            .field("kind", &self.kind)
            .field("slot", &self.slot)
            .field("value", &"<fn>")
            .field("body", &"<fn>")
            .finish()
    }
}


pub struct RtRunTask {
    pub pool: i32,
    pub body: RtBody,
    pub value: Dyn,
}

impl std::fmt::Debug for RtRunTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtRunTask")
            .field("pool", &self.pool)
            .field("body", &"<fn>")
            .field("value", &"<fn>")
            .finish()
    }
}

pub fn drop_run_actor(a: RtRunActor) {
    let __destructured10 = a;
    let mut addr = __destructured10.addr;
    let mut pool = __destructured10.pool;
    let mut kind = __destructured10.kind;
    let mut slot = __destructured10.slot;
    let mut value = __destructured10.value;
    let mut body = __destructured10.body;
    drop_dyn_platform(value);
    drop_body_platform(body);
}

pub fn drop_run_task(t: RtRunTask) {
    let __destructured11 = t;
    let mut pool = __destructured11.pool;
    let mut body = __destructured11.body;
    let mut value = __destructured11.value;
    drop_body_platform(body);
    drop_dyn_platform(value);
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtSent {
}

impl crate::wire::__Wire for RtSent {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtDead {
}

impl crate::wire::__Wire for RtDead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}


pub struct RtFull {
    pub msg: Dyn,
}

impl std::fmt::Debug for RtFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtFull")
            .field("msg", &"<fn>")
            .finish()
    }
}

pub fn drop_full(f: RtFull) {
    let __destructured12 = f;
    let mut msg = __destructured12.msg;
    drop_dyn_platform(msg);
}


pub struct RtIdleHook {
    pub pool: i32,
    pub token: RtToken,
}

impl std::fmt::Debug for RtIdleHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtIdleHook")
            .field("pool", &self.pool)
            .field("token", &"<fn>")
            .finish()
    }
}

pub fn drop_idle_hook(h: RtIdleHook) {
    let __destructured13 = h;
    let mut pool = __destructured13.pool;
    let mut token = __destructured13.token;
    drop_token(token);
}


pub struct RtGot {
    pub value: Dyn,
}

impl std::fmt::Debug for RtGot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtGot")
            .field("value", &"<fn>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtSleep {
}

impl crate::wire::__Wire for RtSleep {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtAgain {
}

impl crate::wire::__Wire for RtAgain {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtStuck {
    pub report: String,
}

impl crate::wire::__Wire for RtStuck {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.report, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            report: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn drop_got(g: RtGot) {
    let __destructured14 = g;
    let mut value = __destructured14.value;
    drop_dyn_platform(value);
}

pub trait __Stateless_SchedTable: Send + Sync {
    fn new_pool(&self, sink: i32) -> i32;
    fn new_actor(&self, pool: i32, bound: i32, body: RtBody) -> i32;
    fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<RtSent, RtDead, RtFull>;
    fn enqueue_remote(&self, addr: i32, msg: Dyn, from: i64) -> bool;
    fn kill(&self, addr: i32, reason: String);
    fn mint_actor(&self, addr: i32, gated: bool) -> RtToken;
    fn mint_task(&self, pool: i32, body: RtBody) -> RtToken;
    fn mint_waiter(&self, pool: i32) -> RtWaiterMint;
    fn deliver(&self, t: RtToken, value: Dyn);
    fn watch_actor(&self, addr: i32, t: RtToken);
    fn idle_hook(&self, pool: i32, t: RtToken);
    fn next_work(&self, pool: i32, idle: Parker) -> Option<Union2<RtRunActor, RtRunTask>>;
    fn wait_step(&self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>;
    fn finish(&self, addr: i32, body: RtBody, fault: Option<String>);
    fn task_done(&self, pool: i32, fault: Option<String>);
    fn pool_of_actor(&self, addr: i32) -> i32;
    fn external(&self, delta: i32);
    fn room(&self, addr: i32) -> i32;
    fn is_dead(&self, addr: i32) -> bool;
    fn queued(&self, addr: i32) -> i32;
}

pub trait __Stateful_SchedTable: Send {
    fn new_pool(&mut self, sink: i32) -> i32;
    fn new_actor(&mut self, pool: i32, bound: i32, body: RtBody) -> i32;
    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<RtSent, RtDead, RtFull>;
    fn enqueue_remote(&mut self, addr: i32, msg: Dyn, from: i64) -> bool;
    fn kill(&mut self, addr: i32, reason: String);
    fn mint_actor(&mut self, addr: i32, gated: bool) -> RtToken;
    fn mint_task(&mut self, pool: i32, body: RtBody) -> RtToken;
    fn mint_waiter(&mut self, pool: i32) -> RtWaiterMint;
    fn deliver(&mut self, t: RtToken, value: Dyn);
    fn watch_actor(&mut self, addr: i32, t: RtToken);
    fn idle_hook(&mut self, pool: i32, t: RtToken);
    fn next_work(&mut self, pool: i32, idle: Parker) -> Option<Union2<RtRunActor, RtRunTask>>;
    fn wait_step(&mut self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>;
    fn finish(&mut self, addr: i32, body: RtBody, fault: Option<String>);
    fn task_done(&mut self, pool: i32, fault: Option<String>);
    fn pool_of_actor(&mut self, addr: i32) -> i32;
    fn external(&mut self, delta: i32);
    fn room(&mut self, addr: i32) -> i32;
    fn is_dead(&mut self, addr: i32) -> bool;
    fn queued(&mut self, addr: i32) -> i32;
}

pub struct SchedTable {
    inner: __Inner_SchedTable,
}

pub enum __Inner_SchedTable {
    Shared(std::sync::Arc<dyn __Stateless_SchedTable>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_SchedTable>>),
}

impl Clone for SchedTable {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_SchedTable::Shared(h) => __Inner_SchedTable::Shared(h.clone()),
            __Inner_SchedTable::Locked(h) => __Inner_SchedTable::Locked(h.clone()),
        } }
    }
}

impl SchedTable {
    pub fn shared<__H: __Stateless_SchedTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_SchedTable::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_SchedTable>) -> Self {
        Self { inner: __Inner_SchedTable::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_SchedTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_SchedTable::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_SchedTable>>) -> Self {
        Self { inner: __Inner_SchedTable::Locked(inner) }
    }
    pub fn new_pool(&self, sink: i32) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_pool(sink),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_pool(sink),
        }
    }
    pub fn new_actor(&self, pool: i32, bound: i32, body: RtBody) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_actor(pool, bound, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_actor(pool, bound, body),
        }
    }
    pub fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<RtSent, RtDead, RtFull> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.enqueue(addr, msg, waiter),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().enqueue(addr, msg, waiter),
        }
    }
    pub fn enqueue_remote(&self, addr: i32, msg: Dyn, from: i64) -> bool {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.enqueue_remote(addr, msg, from),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().enqueue_remote(addr, msg, from),
        }
    }
    pub fn kill(&self, addr: i32, reason: String) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.kill(addr, reason),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().kill(addr, reason),
        }
    }
    pub fn mint_actor(&self, addr: i32, gated: bool) -> RtToken {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_actor(addr, gated),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_actor(addr, gated),
        }
    }
    pub fn mint_task(&self, pool: i32, body: RtBody) -> RtToken {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_task(pool, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_task(pool, body),
        }
    }
    pub fn mint_waiter(&self, pool: i32) -> RtWaiterMint {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_waiter(pool),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_waiter(pool),
        }
    }
    pub fn deliver(&self, t: RtToken, value: Dyn) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.deliver(t, value),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().deliver(t, value),
        }
    }
    pub fn watch_actor(&self, addr: i32, t: RtToken) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.watch_actor(addr, t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().watch_actor(addr, t),
        }
    }
    pub fn idle_hook(&self, pool: i32, t: RtToken) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.idle_hook(pool, t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().idle_hook(pool, t),
        }
    }
    pub fn next_work(&self, pool: i32, idle: Parker) -> Option<Union2<RtRunActor, RtRunTask>> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.next_work(pool, idle),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().next_work(pool, idle),
        }
    }
    pub fn wait_step(&self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.wait_step(wid, pool, own, frame, me),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().wait_step(wid, pool, own, frame, me),
        }
    }
    pub fn finish(&self, addr: i32, body: RtBody, fault: Option<String>) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.finish(addr, body, fault),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().finish(addr, body, fault),
        }
    }
    pub fn task_done(&self, pool: i32, fault: Option<String>) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.task_done(pool, fault),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().task_done(pool, fault),
        }
    }
    pub fn pool_of_actor(&self, addr: i32) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.pool_of_actor(addr),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().pool_of_actor(addr),
        }
    }
    pub fn external(&self, delta: i32) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.external(delta),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().external(delta),
        }
    }
    pub fn room(&self, addr: i32) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.room(addr),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().room(addr),
        }
    }
    pub fn is_dead(&self, addr: i32) -> bool {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.is_dead(addr),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().is_dead(addr),
        }
    }
    pub fn queued(&self, addr: i32) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.queued(addr),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().queued(addr),
        }
    }
}

pub struct Scheduler {
    actors: Vec<RtActorRec>,
    waiters: Vec<RtWaiterRec>,
    pools: Vec<RtPoolRec>,
    idle_hooks: Vec<RtIdleHook>,
    next_slot: i64,
    active: i32,
    parked_frames: i32,
    main_waits: i32,
    externals: i32,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            actors: vec![],
            waiters: vec![],
            pools: vec![],
            idle_hooks: vec![],
            next_slot: 0i64,
            active: 0,
            parked_frames: 0,
            main_waits: 0,
            externals: 0,
        }
    }
}

impl crate::runtime::__Stateful_SchedTable for Scheduler {

    fn new_pool(&mut self, sink: i32) -> i32 {
        self.pools.push(RtPoolRec { idle: vec![], tasks: std::collections::VecDeque::<RtTaskRun>::new(), ready: std::collections::VecDeque::<i32>::new(), sink: sink, owed: 0, deferred: 0 });
        return (self.pools.len() as i32) - 1;
    }

    fn new_actor(&mut self, pool: i32, bound: i32, body: RtBody) -> i32 {
        self.actors.push(RtActorRec { body: slot_of_platform(body), pool: pool, bound: bound, queue: std::collections::VecDeque::<Union3<RtDelivered, RtAnswered, RtReported>>::new(), slots: std::collections::VecDeque::<i64>::new(), user_len: 0, gate: None, running: false, dead: false, exit_reason: "".to_string(), blocked: vec![], watchers: vec![], owed: 0, ready: false });
        return (self.actors.len() as i32) - 1;
    }

    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<RtSent, RtDead, RtFull> {
        if addr < 0 || addr >= (self.actors.len() as i32) {
            drop_dyn_platform(msg);
            return Union3::<RtSent, RtDead, RtFull>::U2(RtDead {  });
        }
        let __h0 = (addr) as usize;
        self.actors.get(__h0).expect("salvo: value is absent at runtime:452:17");
        if self.actors[__h0].dead {
            drop_dyn_platform(msg);
            return Union3::<RtSent, RtDead, RtFull>::U2(RtDead {  });
        }
        if self.actors[__h0].user_len >= self.actors[__h0].bound {
            self.actors[__h0].blocked.push(waiter);
            return Union3::<RtSent, RtDead, RtFull>::U3(RtFull { msg: msg });
        }
        let mut e: Union3<RtDelivered, RtAnswered, RtReported> = Union3::<RtDelivered, RtAnswered, RtReported>::U1(RtDelivered { msg: msg, from: ((-1) as i64) });
        self.actors[__h0].queue.push_back(e);
        self.actors[__h0].slots.push_back(((-1) as i64));
        self.actors[__h0].user_len = self.actors[__h0].user_len + 1;
        if mark_ready(&mut self.actors[__h0], &mut self.pools, addr.clone()) {
            let mut pool = self.actors[__h0].pool;
            wake_for(&mut self.pools, pool);
        }
        return Union3::<RtSent, RtDead, RtFull>::U1(RtSent {  });
    }

    fn enqueue_remote(&mut self, addr: i32, msg: Dyn, from: i64) -> bool {
        if addr < 0 || addr >= (self.actors.len() as i32) {
            drop_dyn_platform(msg);
            return false;
        }
        let __h1 = (addr) as usize;
        self.actors.get(__h1).expect("salvo: value is absent at runtime:477:17");
        if self.actors[__h1].dead {
            drop_dyn_platform(msg);
            return false;
        }
        let mut e: Union3<RtDelivered, RtAnswered, RtReported> = Union3::<RtDelivered, RtAnswered, RtReported>::U1(RtDelivered { msg: msg, from: from });
        self.actors[__h1].queue.push_back(e);
        self.actors[__h1].slots.push_back(((-1) as i64));
        self.actors[__h1].user_len = self.actors[__h1].user_len + 1;
        if mark_ready(&mut self.actors[__h1], &mut self.pools, addr.clone()) {
            let mut pool = self.actors[__h1].pool;
            wake_pool(&mut self.pools, pool);
        }
        return true;
    }

    fn kill(&mut self, addr: i32, reason: String) {
        let __h2 = (addr) as usize;
        self.actors.get(__h2).expect("salvo: value is absent at runtime:494:17");
        if self.actors[__h2].dead {
            return;
        }
        self.actors[__h2].dead = true;
        self.actors[__h2].exit_reason = reason.clone();
        loop {
            let mut __is1 = self.actors[__h2].blocked.salvo_remove_first();
            if !(__is1.is_some()) {
                break;
            }
            let mut b = __is1.as_ref().unwrap().clone();
            unpark_platform(&b);
        }
        let mut watchers: Vec<RtToken> = vec![];
        loop {
            let mut __is2 = self.actors[__h2].watchers.salvo_remove_first();
            if !(__is2.is_some()) {
                break;
            }
            let mut t = __is2.unwrap();
            watchers.push(t);
        }
        loop {
            let mut __is3 = watchers.salvo_remove_first();
            if !(__is3.is_some()) {
                break;
            }
            let mut t = __is3.unwrap();
            deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, erase_platform(Exit { reason: reason.clone() }));
        }
        watchers.into_iter().for_each(|t| drop_token(t));
    }

    fn mint_actor(&mut self, addr: i32, gated: bool) -> RtToken {
        self.next_slot = self.next_slot + ((1) as i64);
        let mut slot = self.next_slot.clone();
        let __h3 = (addr) as usize;
        self.actors.get(__h3).expect("salvo: value is absent at runtime:516:17");
        if gated {
            self.actors[__h3].gate = Some(slot.clone());
        }
        self.actors[__h3].owed = self.actors[__h3].owed + 1;
        return RtToken { target: Union3::<RtToActor, RtToWaiter, RtToTask>::U1(RtToActor { addr: addr }), slot: slot, tracked: true };
    }

    fn mint_task(&mut self, pool: i32, body: RtBody) -> RtToken {
        self.next_slot = self.next_slot + ((1) as i64);
        let __h4 = (pool) as usize;
        self.pools.get(__h4).expect("salvo: value is absent at runtime:526:17");
        self.pools[__h4].owed = self.pools[__h4].owed + 1;
        return RtToken { target: Union3::<RtToActor, RtToWaiter, RtToTask>::U3(RtToTask { pool: pool, body: body }), slot: self.next_slot.clone(), tracked: true };
    }

    fn mint_waiter(&mut self, pool: i32) -> RtWaiterMint {
        let __h5 = (pool) as usize;
        self.pools.get(__h5).expect("salvo: value is absent at runtime:532:17");
        self.pools[__h5].owed = self.pools[__h5].owed + 1;
        self.waiters.push(RtWaiterRec { pool: pool, value: slot_empty_platform(), filled: false, parker: None, waiting: 0, waiting_actor: -1 });
        let mut wid = (self.waiters.len() as i32) - 1;
        self.next_slot = self.next_slot + ((1) as i64);
        let mut t = RtToken { target: Union3::<RtToActor, RtToWaiter, RtToTask>::U2(RtToWaiter { wid: wid.clone() }), slot: self.next_slot.clone(), tracked: true };
        return RtWaiterMint { token: t, wid: wid };
    }

    fn deliver(&mut self, t: RtToken, value: Dyn) {
        deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, value);
    }

    fn watch_actor(&mut self, addr: i32, t: RtToken) {
        let mut watch = untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        let __h6 = (addr) as usize;
        self.actors.get(__h6).expect("salvo: value is absent at runtime:549:17");
        if self.actors[__h6].dead {
            let mut reason = self.actors[__h6].exit_reason.clone();
            deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, watch, erase_platform(Exit { reason: reason }));
            return;
        }
        self.actors[__h6].watchers.push(watch);
    }

    fn idle_hook(&mut self, pool: i32, t: RtToken) {
        let mut hook = untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        self.idle_hooks.push(RtIdleHook { pool: pool, token: hook });
        wake_all_pools(&mut self.pools);
    }

    fn next_work(&mut self, pool: i32, idle: Parker) -> Option<Union2<RtRunActor, RtRunTask>> {
        let mut w = take_work(&mut self.actors, &mut self.pools, pool.clone(), -1);
        if w.is_none() {
            let __h7 = (pool) as usize;
            self.pools.get(__h7).expect("salvo: value is absent at runtime:567:21");
            self.pools[__h7].idle.push(idle);
            if self.active == self.parked_frames && quiet(&mut self.actors, &mut self.waiters, &mut self.pools, self.externals) {
                if !((self.idle_hooks.len() as i32) == 0) && self.active == 0 {
                    fire_idle(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.idle_hooks);
                }
                wake_waiters(&mut self.waiters);
            }
            return None;
        }
        self.active = self.active + 1;
        return w;
    }

    fn wait_step(&mut self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck> {
        let __h8 = (wid) as usize;
        self.waiters.get(__h8).expect("salvo: value is absent at runtime:587:17");
        if self.waiters[__h8].waiting == 0 {
            self.waiters[__h8].waiting = frame.clone();
            self.waiters[__h8].waiting_actor = own.clone();
            if frame == 1 {
                self.parked_frames = self.parked_frames + 1;
            } else {
                self.main_waits = self.main_waits + 1;
            }
        }
        let mut got = slot_take_platform(&mut self.waiters[__h8].value);
        if got.is_some() {
            let mut v = got.unwrap();
            self.waiters[__h8].filled = false;
            self.waiters[__h8].parker = None;
            if self.waiters[__h8].waiting == 1 {
                self.parked_frames = self.parked_frames - 1;
            } else {
                self.main_waits = self.main_waits - 1;
            }
            self.waiters[__h8].waiting = 0;
            self.waiters[__h8].waiting_actor = -1;
            wake_pool(&mut self.pools, pool.clone());
            return Union6::<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>::U1(RtGot { value: v });
        }
        let mut work = take_work(&mut self.actors, &mut self.pools, pool.clone(), own.clone());
        if matches!(work, Some(Union2::U1(_))) {
            let mut ra = match work { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Union6::<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>::U2(ra);
        }
        if matches!(work, Some(Union2::U2(_))) {
            let mut rt = match work { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Union6::<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>::U3(rt);
        }
        let mut q = quiet(&mut self.actors, &mut self.waiters, &mut self.pools, self.externals);
        if !((self.idle_hooks.len() as i32) == 0) && self.active == 0 && q {
            fire_idle(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.idle_hooks);
            return Union6::<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>::U5(RtAgain {  });
        }
        if self.active == self.parked_frames && self.main_waits > 0 && q {
            return Union6::<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>::U6(RtStuck { report: deadlock_report(&mut self.actors, &mut self.waiters, own) });
        }
        let __h9 = (wid) as usize;
        self.waiters.get(__h9).expect("salvo: value is absent at runtime:635:22");
        self.waiters[__h9].parker = Some(me.clone());
        let __h10 = (pool) as usize;
        self.pools.get(__h10).expect("salvo: value is absent at runtime:637:17");
        self.pools[__h10].idle.push(me);
        return Union6::<RtGot, RtRunActor, RtRunTask, RtSleep, RtAgain, RtStuck>::U4(RtSleep {  });
    }

    fn finish(&mut self, addr: i32, body: RtBody, fault: Option<String>) {
        self.active = self.active - 1;
        let __h11 = (addr) as usize;
        self.actors.get(__h11).expect("salvo: value is absent at runtime:644:17");
        self.actors[__h11].running = false;
        if fault.is_none() {
            slot_put_platform(&mut self.actors[__h11].body, body);
            let mut again = mark_ready(&mut self.actors[__h11], &mut self.pools, addr.clone());
            let mut pool = self.actors[__h11].pool;
            let __h12 = (pool) as usize;
            self.pools.get(__h12).expect("salvo: value is absent at runtime:654:21");
            let mut extra = self.pools[__h12].deferred - 1;
            if again {
                extra = extra + 1;
            }
            self.pools[__h12].deferred = 0;
            while extra > 0 {
                wake_pool(&mut self.pools, pool.clone());
                extra = extra - 1;
            }
            return;
        }
        drop_body_platform(body);
        let mut reason = fault.as_ref().unwrap().clone();
        self.actors[__h11].dead = true;
        self.actors[__h11].exit_reason = reason.clone();
        self.actors[__h11].gate = None;
        self.actors[__h11].user_len = 0;
        while ((self.actors[__h11].queue.len() as i32) > 0) {
            drop_entry(self.actors[__h11].queue.pop_front().expect("salvo: value is absent at runtime:673:24"));
        }
        while ((self.actors[__h11].slots.len() as i32) > 0) {
            let mut _s = self.actors[__h11].slots.pop_front();
        }
        loop {
            let mut __is4 = self.actors[__h11].blocked.salvo_remove_first();
            if !(__is4.is_some()) {
                break;
            }
            let mut b = __is4.as_ref().unwrap().clone();
            unpark_platform(&b);
        }
        let mut pool = self.actors[__h11].pool;
        let mut watchers: Vec<RtToken> = vec![];
        loop {
            let mut __is5 = self.actors[__h11].watchers.salvo_remove_first();
            if !(__is5.is_some()) {
                break;
            }
            let mut t = __is5.unwrap();
            watchers.push(t);
        }
        if ((watchers.len() as i32) == 0) {
            report_fault(&mut self.actors, &mut self.pools, pool.clone(), reason.clone());
        }
        loop {
            let mut __is6 = watchers.salvo_remove_first();
            if !(__is6.is_some()) {
                break;
            }
            let mut t = __is6.unwrap();
            deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, erase_platform(Exit { reason: reason.clone() }));
        }
        watchers.into_iter().for_each(|t| drop_token(t));
        wake_all_pools(&mut self.pools);
    }

    fn task_done(&mut self, pool: i32, fault: Option<String>) {
        self.active = self.active - 1;
        if fault.is_some() {
            let mut reason = fault.as_ref().unwrap().clone();
            report_fault(&mut self.actors, &mut self.pools, pool, reason.clone());
        }
    }

    fn pool_of_actor(&mut self, addr: i32) -> i32 {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:706:21").pool;
    }

    fn room(&mut self, addr: i32) -> i32 {
        let mut a = self.actors.get((addr) as i64 as usize).unwrap();
        return a.bound - a.user_len;
    }

    fn is_dead(&mut self, addr: i32) -> bool {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:715:21").dead;
    }

    fn queued(&mut self, addr: i32) -> i32 {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:719:21").user_len;
    }

    fn external(&mut self, delta: i32) {
        self.externals = self.externals + delta;
        if self.externals < 0 {
            self.externals = 0;
        }
        wake_all_pools(&mut self.pools);
    }
}

impl Scheduler {

    fn init(&mut self) {
        self.pools.push(RtPoolRec { idle: vec![], tasks: std::collections::VecDeque::<RtTaskRun>::new(), ready: std::collections::VecDeque::<i32>::new(), sink: -1, owed: 0, deferred: 0 });
    }
}

pub enum __Priv_Scheduler {
    Init,
}

pub fn untrack(actors: &mut Vec<RtActorRec>, waiters: &mut Vec<RtWaiterRec>, pools: &mut Vec<RtPoolRec>, t: RtToken) -> RtToken {
    let __destructured15 = t;
    let mut target = __destructured15.target;
    let mut slot = __destructured15.slot;
    let mut tracked = __destructured15.tracked;
    if tracked {
        release(actors, waiters, pools, &target);
    }
    return RtToken { target: target, slot: slot, tracked: false };
}

pub fn release(actors: &mut Vec<RtActorRec>, waiters: &mut Vec<RtWaiterRec>, pools: &mut Vec<RtPoolRec>, target: &Union3<RtToActor, RtToWaiter, RtToTask>) {
    if matches!(target, Union3::U1(_)) {
        let mut to = target.u1().clone();
        let __h13 = (to.addr) as usize;
        actors.get(__h13).expect("salvo: value is absent at runtime:746:17");
        if actors[__h13].owed > 0 {
            actors[__h13].owed = actors[__h13].owed - 1;
        }
    } else if matches!(target, Union3::U2(_)) {
        let mut tw = target.u2().clone();
        let mut pool = waiters.get((tw.wid) as i64 as usize).expect("salvo: value is absent at runtime:751:25").pool;
        let __h14 = (pool) as usize;
        pools.get(__h14).expect("salvo: value is absent at runtime:752:17");
        if pools[__h14].owed > 0 {
            pools[__h14].owed = pools[__h14].owed - 1;
        }
    } else {
        let __h15 = (target.u3().clone().pool) as usize;
        pools.get(__h15).expect("salvo: value is absent at runtime:757:17");
        if pools[__h15].owed > 0 {
            pools[__h15].owed = pools[__h15].owed - 1;
        }
    }
}

pub fn deliver_to(actors: &mut Vec<RtActorRec>, waiters: &mut Vec<RtWaiterRec>, pools: &mut Vec<RtPoolRec>, t: RtToken, value: Dyn) {
    let __destructured16 = t;
    let mut target = __destructured16.target;
    let mut slot = __destructured16.slot;
    let mut tracked = __destructured16.tracked;
    if tracked {
        release(actors, waiters, pools, &target);
    }
    if matches!(target, Union3::U1(_)) {
        let mut to = target.u1().clone();
        let __h16 = (to.addr) as usize;
        actors.get(__h16).expect("salvo: value is absent at runtime:773:17");
        if actors[__h16].dead {
            drop_dyn_platform(value);
            return;
        }
        actors[__h16].slots.push_back(slot.clone());
        let mut e: Union3<RtDelivered, RtAnswered, RtReported> = Union3::<RtDelivered, RtAnswered, RtReported>::U2(RtAnswered { slot: slot, value: value });
        actors[__h16].queue.push_back(e);
        if mark_ready(&mut actors[__h16], pools, to.addr) {
            let mut pool = actors[__h16].pool;
            wake_for(pools, pool);
        }
    } else if matches!(target, Union3::U2(_)) {
        let mut tw = target.u2().clone();
        let __h17 = (tw.wid) as usize;
        waiters.get(__h17).expect("salvo: value is absent at runtime:786:17");
        slot_put_platform(&mut waiters[__h17].value, value);
        waiters[__h17].filled = true;
        if waiters[__h17].parker.is_some() {
            let mut p = waiters[__h17].parker.as_ref().unwrap().clone();
            unpark_platform(&(p.clone()));
        }
    } else {
        let __destructured17 = (match target { Union3::U3(__v) => __v, _ => unreachable!() });
        let mut pool = __destructured17.pool;
        let mut body = __destructured17.body;
        let __h18 = (pool) as usize;
        pools.get(__h18).expect("salvo: value is absent at runtime:794:17");
        pools[__h18].tasks.push_back(RtTaskRun { body: body, value: value });
        wake_pool(pools, pool.clone());
    }
}

pub fn report_fault(actors: &mut Vec<RtActorRec>, pools: &mut Vec<RtPoolRec>, pool: i32, reason: String) {
    let mut sink = pools.get((pool) as i64 as usize).expect("salvo: value is absent at runtime:805:21").sink;
    if sink >= 0 {
        let __h19 = (sink) as usize;
        actors.get(__h19).expect("salvo: value is absent at runtime:807:17");
        if !actors[__h19].dead {
            let mut e: Union3<RtDelivered, RtAnswered, RtReported> = Union3::<RtDelivered, RtAnswered, RtReported>::U3(RtReported { reason: reason });
            actors[__h19].queue.push_back(e);
            actors[__h19].slots.push_back(((-1) as i64));
            if mark_ready(&mut actors[__h19], pools, sink.clone()) {
                let mut sink_pool = actors[__h19].pool;
                wake_pool(pools, sink_pool);
            }
            return;
        }
    }
    __module_use_0().report(&(format!("salvo: an uncaught fault on pool {}: {}", pool, reason)));
}

pub fn quiet(actors: &mut Vec<RtActorRec>, waiters: &mut Vec<RtWaiterRec>, pools: &mut Vec<RtPoolRec>, externals: i32) -> bool {
    if externals > 0 {
        return false;
    }
    for p in &*pools {
        if ((p.tasks.len() as i32) > 0) {
            return false;
        }
    }
    for a in &*actors {
        if !a.running && !a.dead && !(deliverable(&a.slots, &a.gate).is_none()) {
            return false;
        }
    }
    for w in &*waiters {
        if w.waiting > 0 && w.filled {
            return false;
        }
    }
    return true;
}

pub fn fire_idle(actors: &mut Vec<RtActorRec>, waiters: &mut Vec<RtWaiterRec>, pools: &mut Vec<RtPoolRec>, hooks: &mut Vec<RtIdleHook>) {
    loop {
        let mut __is7 = hooks.salvo_remove_first();
        if !(__is7.is_some()) {
            break;
        }
        let mut h = __is7.unwrap();
        let __destructured18 = h;
        let mut pool = __destructured18.pool;
        let mut token = __destructured18.token;
        let mut gates = 0;
        let mut tokens = pools.get((pool) as i64 as usize).expect("salvo: value is absent at runtime:857:27").owed;
        for a in &*actors {
            if a.pool == pool {
                tokens = tokens + a.owed;
                if !(a.gate.is_none()) && !a.dead {
                    gates = gates + 1;
                }
            }
        }
        deliver_to(actors, waiters, pools, token, erase_platform(Idle { parked_gates: gates, parked_tokens: tokens }));
    }
}

pub fn deadlock_report(actors: &mut Vec<RtActorRec>, waiters: &mut Vec<RtWaiterRec>, own: i32) -> String {
    let mut occupied: Vec<String> = vec![];
    for w in &*waiters {
        if w.waiting > 0 && w.waiting_actor >= 0 {
            occupied.push(format!("actor {}", w.waiting_actor));
        }
    }
    let mut gated: Vec<String> = vec![];
    let mut i = 0;
    for a in &*actors {
        if !(a.gate.is_none()) && !a.dead {
            gated.push(format!("actor {}", i));
        }
        i = i + 1;
    }
    let mut who = if own >= 0 {
        format!("actor {}", own)
    } else {
        "main".to_string()
    };
    let mut clauses: Vec<String> = vec![];
    if ((occupied.len() as i32) > 0) {
        clauses.push(format!("parked in a wait: {}", occupied.join(&", ".to_string()[..])));
    }
    if ((gated.len() as i32) > 0) {
        clauses.push(format!("parked gates: {}", gated.join(&", ".to_string()[..])));
    }
    let mut detail = if ((clauses.len() as i32) == 0) {
        "".to_string()
    } else {
        format!(" ({})", clauses.join(&"; ".to_string()[..]))
    };
    return format!("salvo: deadlock: nothing can run while {} waits{}", who, detail);
}

pub fn take_work(actors: &mut Vec<RtActorRec>, pools: &mut Vec<RtPoolRec>, pool: i32, exclude: i32) -> Option<Union2<RtRunActor, RtRunTask>> {
    let __h20 = (pool) as usize;
    pools.get(__h20).expect("salvo: value is absent at runtime:906:13");
    let mut task = pools[__h20].tasks.pop_front();
    if task.is_some() {
        let mut t = task.unwrap();
        let __destructured19 = t;
        let mut body = __destructured19.body;
        let mut value = __destructured19.value;
        return Some(Union2::<RtRunActor, RtRunTask>::U2(RtRunTask { pool: pool, body: body, value: value }));
    }
    loop {
        let mut __is8 = pools[__h20].ready.pop_front();
        if !(__is8.is_some()) {
            break;
        }
        let mut i = __is8.unwrap();
        let __h21 = (i) as usize;
        actors.get(__h21).expect("salvo: value is absent at runtime:913:17");
        actors[__h21].ready = false;
        if i != exclude && !actors[__h21].running && !actors[__h21].dead {
            let mut at = deliverable(&actors[__h21].slots, &actors[__h21].gate);
            if at.is_some() {
                let mut k = at.unwrap();
                let mut _slot = actors[__h21].slots.remove((k.clone()) as i64 as usize);
                let mut e = actors[__h21].queue.remove((k) as i64 as usize).expect("salvo: value is absent at runtime:919:25");
                actors[__h21].running = true;
                let mut body = slot_take_platform(&mut actors[__h21].body).expect("salvo: value is absent at runtime:921:28");
                return work_of(&mut actors[__h21], i.clone(), e, body);
            }
        }
    }
    return None;
}

pub fn mark_ready(a: &mut RtActorRec, pools: &mut Vec<RtPoolRec>, addr: i32) -> bool {
    if a.ready || a.running || a.dead {
        return false;
    }
    if deliverable(&a.slots, &a.gate).is_none() {
        return false;
    }
    a.ready = true;
    let __h22 = (a.pool) as usize;
    pools.get(__h22).expect("salvo: value is absent at runtime:940:13");
    pools[__h22].ready.push_back(addr);
    return true;
}

pub fn work_of(a: &mut RtActorRec, addr: i32, e: Union3<RtDelivered, RtAnswered, RtReported>, body: RtBody) -> Option<Union2<RtRunActor, RtRunTask>> {
    if matches!(e, Union3::U1(_)) {
        let mut d = match e { Union3::U1(__v) => __v, _ => unreachable!() };
        a.user_len = a.user_len - 1;
        let mut woken = a.blocked.salvo_remove_first();
        if woken.is_some() {
            let mut b = woken.as_ref().unwrap().clone();
            unpark_platform(&b);
        }
        let __destructured20 = d;
        let mut msg = __destructured20.msg;
        let mut from = __destructured20.from;
        if from >= ((0) as i64) {
            granted_platform(addr.clone(), from);
        }
        return Some(Union2::<RtRunActor, RtRunTask>::U1(RtRunActor { addr: addr, pool: a.pool, kind: 0, slot: 0i64, value: msg, body: body }));
    }
    if matches!(e, Union3::U3(_)) {
        let mut r = e.u3().clone();
        return Some(Union2::<RtRunActor, RtRunTask>::U1(RtRunActor { addr: addr, pool: a.pool, kind: 2, slot: 0i64, value: erase_platform(r.reason.clone()), body: body }));
    }
    let __destructured21 = (match e { Union3::U2(__v) => __v, _ => unreachable!() });
    let mut slot = __destructured21.slot;
    let mut value = __destructured21.value;
    let mut opens = false;
    if a.gate.is_some() {
        let mut g = a.gate.unwrap();
        opens = g == slot;
    }
    if opens {
        a.gate = None;
    }
    return Some(Union2::<RtRunActor, RtRunTask>::U1(RtRunActor { addr: addr, pool: a.pool, kind: 1, slot: slot, value: value, body: body }));
}

pub fn deliverable(slots: &std::collections::VecDeque<i64>, gate: &Option<i64>) -> Option<i32> {
    if ((slots.len() as i32) == 0) {
        return None;
    }
    if gate.is_none() {
        return Some(0);
    }
    let mut i = 0;
    while i < (slots.len() as i32) {
        if *slots.get((i) as i64 as usize).expect("salvo: value is absent at runtime:988:12") == gate.unwrap() {
            return Some(i.clone());
        }
        i = i + 1;
    }
    return None;
}

pub fn wake_pool(pools: &mut Vec<RtPoolRec>, pool: i32) {
    let __h23 = (pool) as usize;
    pools.get(__h23).expect("salvo: value is absent at runtime:1003:13");
    let mut __is9 = pools[__h23].idle.salvo_remove_first();
    if __is9.is_some() {
        let mut w = __is9.as_ref().unwrap().clone();
        unpark_platform(&w);
    }
}

pub fn wake_every(pools: &mut Vec<RtPoolRec>, pool: i32) {
    let __h24 = (pool) as usize;
    pools.get(__h24).expect("salvo: value is absent at runtime:1011:13");
    loop {
        let mut __is10 = pools[__h24].idle.salvo_remove_first();
        if !(__is10.is_some()) {
            break;
        }
        let mut w = __is10.as_ref().unwrap().clone();
        unpark_platform(&w);
    }
}

pub fn wake_for(pools: &mut Vec<RtPoolRec>, pool: i32) {
    if here_pool_platform() == pool && here_actor_platform() >= 0 {
        let __h25 = (pool) as usize;
        pools.get(__h25).expect("salvo: value is absent at runtime:1022:17");
        pools[__h25].deferred = pools[__h25].deferred + 1;
        return;
    }
    wake_pool(pools, pool);
}

pub fn wake_waiters(waiters: &mut Vec<RtWaiterRec>) {
    for w in &*waiters {
        if w.waiting > 0 && (w.parker.is_some()) {
            let mut p = w.parker.as_ref().unwrap().clone();
            unpark_platform(&(p.clone()));
        }
    }
}

pub fn wake_all_pools(pools: &mut Vec<RtPoolRec>) {
    let mut i = 0;
    while i < (pools.len() as i32) {
        wake_every(pools, i.clone());
        i = i + 1;
    }
}

pub fn new_pool_of(n: i32, sink: i32) -> i32 {
    let mut id = __module_use_1().new_pool(sink);
    let mut i = 0;
    while i < n {
        start_thread_platform(Box::new({ let mut id = id.clone(); move || {
    serve_pool(id.clone());
} }));
        i = i + 1;
    }
    return id;
}

pub fn spawn_body(pool: i32, bound: i32, body: RtBody) -> i32 {
    return __module_use_1().new_actor(pool, bound, body);
}

pub fn send_dyn(addr: i32, msg: Dyn) {
    let mut r = __module_use_1().enqueue(addr.clone(), msg, this_parker_platform());
    while matches!(r, Union3::U3(_)) {
        let mut full = match r { Union3::U3(__v) => __v, _ => unreachable!() };
        if here_pool_platform() == main_pool() && here_actor_platform() == no_frame() && __module_use_1().pool_of_actor(addr.clone()) == main_pool() {
            __module_use_0().report(&(format!("salvo: deadlock: the main pool's actor {} has a full mailbox and the only thread that could drain it is the one sending: the main pool has one worker, `main` itself, and it serves work only inside a `waitfor` — send fewer messages before waiting, raise the handler's `mailbox` capacity, or place the actor on a pool of its own", addr)));
            exit_process_platform(1);
        }
        park_platform(&(this_parker_platform()));
        let __destructured22 = full;
        let mut back = __destructured22.msg;
        r = __module_use_1().enqueue(addr.clone(), back, this_parker_platform());
    }
    if matches!(r, Union3::U3(_)) {
        let mut full = match r { Union3::U3(__v) => __v, _ => unreachable!() };
        drop_full(full);
    }
}

pub fn mint(addr: i32, gated: bool) -> RtToken {
    return __module_use_1().mint_actor(addr, gated);
}

pub fn mint_task_on(pool: i32, body: RtBody) -> RtToken {
    return __module_use_1().mint_task(pool, body);
}

pub fn waiter() -> RtWaiterMint {
    return __module_use_1().mint_waiter(here_pool_platform());
}

pub fn answer(t: RtToken, value: Dyn) {
    __module_use_1().deliver(t, value);
}

pub fn watch(addr: i32, t: RtToken) {
    __module_use_1().watch_actor(addr, t);
}

pub fn on_idle(pool: i32, t: RtToken) {
    __module_use_1().idle_hook(pool, t);
}

pub fn external_begin() {
    __module_use_1().external(1);
}

pub fn external_end() {
    __module_use_1().external(-1);
}

pub fn await_answer(wid: i32) -> Dyn {
    let mut pool = here_pool_platform();
    let mut own = here_actor_platform();
    let mut frame = if own == no_frame() {
        2
    } else {
        1
    };
    loop {
        let mut step = __module_use_1().wait_step(wid.clone(), pool.clone(), own.clone(), frame.clone(), this_parker_platform());
        if matches!(step, Union6::U1(_)) {
            let mut g = match step { Union6::U1(__v) => __v, _ => unreachable!() };
            let __destructured23 = g;
            let mut value = __destructured23.value;
            return value;
        }
        if matches!(step, Union6::U2(_)) {
            let mut ra = match step { Union6::U2(__v) => __v, _ => unreachable!() };
            run_actor(ra);
        } else if matches!(step, Union6::U3(_)) {
            let mut rt = match step { Union6::U3(__v) => __v, _ => unreachable!() };
            run_task(rt);
        } else if matches!(step, Union6::U6(_)) {
            let mut s = step.u6().clone();
            __module_use_0().report(&(s.report.clone()));
            exit_process_platform(1);
        } else if matches!(step, Union6::U4(_)) {
            park_platform(&(this_parker_platform()));
        }
    }
    return unerase_platform(erase_platform(0));
}

pub fn run_actor(ra: RtRunActor) {
    flush_frames_platform();
    let __destructured24 = ra;
    let mut addr = __destructured24.addr;
    let mut pool = __destructured24.pool;
    let mut kind = __destructured24.kind;
    let mut slot = __destructured24.slot;
    let mut value = __destructured24.value;
    let mut body = __destructured24.body;
    let mut saved_pool = here_pool_platform();
    let mut saved_actor = here_actor_platform();
    set_here_platform(pool, addr.clone());
    let mut ran = activate_platform(body, kind, slot, value);
    set_here_platform(saved_pool, saved_actor);
    let __destructured25 = ran;
    let mut back = __destructured25.body;
    let mut fault = __destructured25.fault;
    __module_use_1().finish(addr, back, fault);
}

pub fn run_task(rt: RtRunTask) {
    let __destructured26 = rt;
    let mut pool = __destructured26.pool;
    let mut body = __destructured26.body;
    let mut value = __destructured26.value;
    let mut saved_pool = here_pool_platform();
    let mut saved_actor = here_actor_platform();
    set_here_platform(pool.clone(), task_frame());
    let mut ran = activate_platform(body, 1, 0i64, value);
    set_here_platform(saved_pool, saved_actor);
    let __destructured27 = ran;
    let mut done = __destructured27.body;
    let mut fault = __destructured27.fault;
    drop_body_platform(done);
    __module_use_1().task_done(pool, fault);
}

pub fn serve_pool(pool: i32) {
    set_here_platform(pool.clone(), no_frame());
    loop {
        let mut w = __module_use_1().next_work(pool.clone(), this_parker_platform());
        if matches!(w, Some(Union2::U1(_))) {
            let mut ra = match w { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            run_actor(ra);
        } else if matches!(w, Some(Union2::U2(_))) {
            let mut rt = match w { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            run_task(rt);
        } else {
            park_platform(&(this_parker_platform()));
        }
    }
}

pub fn token_to_actor(addr: i32, slot: i64) -> RtToken {
    return RtToken { target: Union3::<RtToActor, RtToWaiter, RtToTask>::U1(RtToActor { addr: addr }), slot: slot, tracked: false };
}

pub fn token_to_waiter(wid: i32, slot: i64) -> RtToken {
    return RtToken { target: Union3::<RtToActor, RtToWaiter, RtToTask>::U2(RtToWaiter { wid: wid }), slot: slot, tracked: false };
}


pub struct RtExported {
    pub kind: i32,
    pub id: i32,
    pub slot: i64,
    pub body: Option<RtBody>,
}

impl std::fmt::Debug for RtExported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtExported")
            .field("kind", &self.kind)
            .field("id", &self.id)
            .field("slot", &self.slot)
            .field("body", &"<fn>")
            .finish()
    }
}

pub fn export_token(t: RtToken) -> RtExported {
    let __destructured28 = t;
    let mut target = __destructured28.target;
    let mut slot = __destructured28.slot;
    let mut tracked = __destructured28.tracked;
    if matches!(target, Union3::U3(_)) {
        let mut tt = match target { Union3::U3(__v) => __v, _ => unreachable!() };
        let __destructured29 = tt;
        let mut pool = __destructured29.pool;
        let mut body = __destructured29.body;
        return RtExported { kind: 2, id: pool, slot: slot, body: Some(body) };
    }
    let mut kind = 0;
    let mut id = 0;
    if matches!(target, Union3::U1(_)) {
        let mut to = target.u1().clone();
        id = to.addr;
    } else if matches!(target, Union3::U2(_)) {
        let mut tw = target.u2().clone();
        kind = 1;
        id = tw.wid;
    }
    return RtExported { kind: kind, id: id, slot: slot, body: None };
}

pub fn drop_exported(e: RtExported) {
    let __destructured30 = e;
    let mut kind = __destructured30.kind;
    let mut id = __destructured30.id;
    let mut slot = __destructured30.slot;
    let mut body = __destructured30.body;
    if body.is_some() {
        let mut b = body.unwrap();
        drop_body_platform(b);
    }
}

pub fn deliver_remote(addr: i32, msg: Dyn, from: i64) -> bool {
    return __module_use_1().enqueue_remote(addr, msg, from);
}

pub fn kill_actor(addr: i32, reason: String) {
    __module_use_1().kill(addr, reason);
}

pub fn mailbox_room(addr: i32) -> i32 {
    return __module_use_1().room(addr);
}

pub fn mailbox_queued(addr: i32) -> i32 {
    return __module_use_1().queued(addr);
}

pub fn current_pool() -> i32 {
    return here_pool_platform();
}

pub fn mailbox_dead(addr: i32) -> bool {
    return __module_use_1().is_dead(addr);
}

pub fn identity_bits() -> i64 {
    return fresh_bits();
}
