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
    if __module_use_1().is_virtual() {
        return __module_use_1().random_bits();
    }
    return __module_use_0().secure_bits();
}

pub fn now_nanos() -> i64 {
    if __module_use_1().is_virtual() {
        return __module_use_1().virtual_now();
    }
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

/// [platform-type] The host's `Body`.
pub use crate::platform_runtime::Body;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Body>(); };

pub fn body_of_platform(f: Box<dyn FnMut(i32, i64, Dyn) + Send + 'static>) -> Body {
    crate::platform_runtime::body_of(f)
}

pub fn activate_platform(b: Body, kind: i32, slot: i64, value: Dyn) -> Ran {
    crate::platform_runtime::activate(b, kind, slot, value)
}

pub fn drop_body_platform(b: Body) {
    crate::platform_runtime::drop_body(b)
}

pub fn granted_platform(addr: i32, pool: i32, from: i64) {
    crate::platform_runtime::granted(addr, pool, from)
}

pub fn flush_frames_platform() {
    crate::platform_runtime::flush_frames()
}

pub fn exit_process_platform(code: i32) -> ! {
    crate::platform_runtime::exit_process(code)
}


pub struct Ran {
    pub body: Body,
    pub fault: Option<String>,
}

impl std::fmt::Debug for Ran {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ran")
            .field("body", &"<fn>")
            .field("fault", &self.fault)
            .finish()
    }
}

pub fn drop_ran(r: Ran) {
    let __destructured1 = r;
    let mut body = __destructured1.body;
    let mut fault = __destructured1.fault;
    drop_body_platform(body);
}

/// [platform-type] The host's `Slot`.
pub use crate::platform_runtime::Slot;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Slot<i32>>(); };

pub fn slot_of_platform<T: Send + 'static>(v: T) -> Slot<T> {
    crate::platform_runtime::slot_of(v)
}

pub fn slot_empty_platform<T: Send + 'static>() -> Slot<T> {
    crate::platform_runtime::slot_empty()
}

pub fn slot_take_platform<T: Send + 'static>(s: &mut Slot<T>) -> Option<T> {
    crate::platform_runtime::slot_take(s)
}

pub fn slot_put_platform<T: Send + 'static>(s: &mut Slot<T>, v: T) {
    crate::platform_runtime::slot_put(s, v)
}

pub fn drop_slot_platform<T: Send + 'static>(s: Slot<T>) {
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


pub struct Delivered {
    pub msg: Dyn,
    pub from: i64,
}

impl std::fmt::Debug for Delivered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delivered")
            .field("msg", &"<fn>")
            .field("from", &self.from)
            .finish()
    }
}


pub struct Answered {
    pub slot: i64,
    pub value: Dyn,
}

impl std::fmt::Debug for Answered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answered")
            .field("slot", &self.slot)
            .field("value", &"<fn>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reported {
    pub reason: String,
}

impl crate::wire::__Wire for Reported {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.reason, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            reason: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn drop_delivered(d: Delivered) {
    let __destructured2 = d;
    let mut msg = __destructured2.msg;
    let mut from = __destructured2.from;
    drop_dyn_platform(msg);
}

pub fn drop_answered(a: Answered) {
    let __destructured3 = a;
    let mut slot = __destructured3.slot;
    let mut value = __destructured3.value;
    drop_dyn_platform(value);
}

pub fn drop_entry(e: Union3<Delivered, Answered, Reported>) {
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


pub struct ActorRec {
    pub body: Slot<Body>,
    pub pool: i32,
    pub bound: i32,
    pub queue: std::collections::VecDeque<Union3<Delivered, Answered, Reported>>,
    pub slots: std::collections::VecDeque<i64>,
    pub user_len: i32,
    pub gate: Option<i64>,
    pub running: bool,
    pub dead: bool,
    pub exit_reason: String,
    pub blocked: Vec<Parker>,
    pub watchers: Vec<Token>,
    pub owed: i32,
    pub ready: bool,
    pub proxy: bool,
}

impl std::fmt::Debug for ActorRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorRec")
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
            .field("proxy", &self.proxy)
            .finish()
    }
}

pub fn drop_actor_rec(a: ActorRec) {
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
    let mut proxy = __destructured4.proxy;
    drop_slot_platform(body);
    queue.into_iter().for_each(|e| drop_entry(e));
    watchers.into_iter().for_each(|t| drop_token(t));
}


pub struct WaiterRec {
    pub pool: i32,
    pub value: Slot<Dyn>,
    pub filled: bool,
    pub parker: Option<Parker>,
    pub waiting: i32,
    pub waiting_actor: i32,
}

impl std::fmt::Debug for WaiterRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaiterRec")
            .field("pool", &self.pool)
            .field("value", &"<fn>")
            .field("filled", &self.filled)
            .field("parker", &"<fn>")
            .field("waiting", &self.waiting)
            .field("waiting_actor", &self.waiting_actor)
            .finish()
    }
}

pub fn drop_waiter_rec(w: WaiterRec) {
    let __destructured5 = w;
    let mut pool = __destructured5.pool;
    let mut value = __destructured5.value;
    let mut filled = __destructured5.filled;
    let mut parker = __destructured5.parker;
    let mut waiting = __destructured5.waiting;
    let mut waiting_actor = __destructured5.waiting_actor;
    drop_slot_platform(value);
}


pub struct TaskRun {
    pub body: Body,
    pub value: Dyn,
}

impl std::fmt::Debug for TaskRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskRun")
            .field("body", &"<fn>")
            .field("value", &"<fn>")
            .finish()
    }
}

pub fn drop_task_run(t: TaskRun) {
    let __destructured6 = t;
    let mut body = __destructured6.body;
    let mut value = __destructured6.value;
    drop_body_platform(body);
    drop_dyn_platform(value);
}


pub struct PoolRec {
    pub idle: Vec<Parker>,
    pub tasks: std::collections::VecDeque<TaskRun>,
    pub ready: std::collections::VecDeque<i32>,
    pub sink: i32,
    pub owed: i32,
    pub dedicated: bool,
    pub retired: bool,
}

impl std::fmt::Debug for PoolRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoolRec")
            .field("idle", &"<fn>")
            .field("tasks", &"<fn>")
            .field("ready", &self.ready)
            .field("sink", &self.sink)
            .field("owed", &self.owed)
            .field("dedicated", &self.dedicated)
            .field("retired", &self.retired)
            .finish()
    }
}

pub fn drop_pool_rec(p: PoolRec) {
    let __destructured7 = p;
    let mut idle = __destructured7.idle;
    let mut tasks = __destructured7.tasks;
    let mut ready = __destructured7.ready;
    let mut sink = __destructured7.sink;
    let mut owed = __destructured7.owed;
    let mut dedicated = __destructured7.dedicated;
    let mut retired = __destructured7.retired;
    tasks.into_iter().for_each(|t| drop_task_run(t));
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToActor {
    pub addr: i32,
}

impl crate::wire::__Wire for ToActor {
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
pub struct ToWaiter {
    pub wid: i32,
}

impl crate::wire::__Wire for ToWaiter {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.wid, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            wid: crate::wire::__Wire::__dec(r)?,
        })
    }
}


pub struct ToTask {
    pub pool: i32,
    pub body: Body,
}

impl std::fmt::Debug for ToTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToTask")
            .field("pool", &self.pool)
            .field("body", &"<fn>")
            .finish()
    }
}

pub fn drop_to_task(t: ToTask) {
    let __destructured8 = t;
    let mut pool = __destructured8.pool;
    let mut body = __destructured8.body;
    drop_body_platform(body);
}


pub struct Token {
    pub target: Union3<ToActor, ToWaiter, ToTask>,
    pub slot: i64,
    pub tracked: bool,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("target", &"<fn>")
            .field("slot", &self.slot)
            .field("tracked", &self.tracked)
            .finish()
    }
}

pub fn drop_token(t: Token) {
    answer(t, erase_platform(0));
}


pub struct WaiterMint {
    pub token: Token,
    pub wid: i32,
}

impl std::fmt::Debug for WaiterMint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaiterMint")
            .field("token", &"<fn>")
            .field("wid", &self.wid)
            .finish()
    }
}

pub fn drop_waiter_mint(m: WaiterMint) {
    let __destructured9 = m;
    let mut token = __destructured9.token;
    let mut wid = __destructured9.wid;
    drop_token(token);
}


pub struct RunActor {
    pub addr: i32,
    pub pool: i32,
    pub kind: i32,
    pub slot: i64,
    pub value: Dyn,
    pub body: Body,
}

impl std::fmt::Debug for RunActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunActor")
            .field("addr", &self.addr)
            .field("pool", &self.pool)
            .field("kind", &self.kind)
            .field("slot", &self.slot)
            .field("value", &"<fn>")
            .field("body", &"<fn>")
            .finish()
    }
}


pub struct RunTask {
    pub pool: i32,
    pub body: Body,
    pub value: Dyn,
}

impl std::fmt::Debug for RunTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunTask")
            .field("pool", &self.pool)
            .field("body", &"<fn>")
            .field("value", &"<fn>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Retire {
}

impl crate::wire::__Wire for Retire {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub fn drop_run_actor(a: RunActor) {
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

pub fn drop_run_task(t: RunTask) {
    let __destructured11 = t;
    let mut pool = __destructured11.pool;
    let mut body = __destructured11.body;
    let mut value = __destructured11.value;
    drop_body_platform(body);
    drop_dyn_platform(value);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sent {
}

impl crate::wire::__Wire for Sent {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dead {
}

impl crate::wire::__Wire for Dead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}


pub struct Full {
    pub msg: Dyn,
}

impl std::fmt::Debug for Full {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Full")
            .field("msg", &"<fn>")
            .finish()
    }
}


pub struct Remote {
    pub msg: Dyn,
}

impl std::fmt::Debug for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Remote")
            .field("msg", &"<fn>")
            .finish()
    }
}

pub fn drop_remote(r: Remote) {
    let __destructured12 = r;
    let mut msg = __destructured12.msg;
    drop_dyn_platform(msg);
}

pub fn drop_full(f: Full) {
    let __destructured13 = f;
    let mut msg = __destructured13.msg;
    drop_dyn_platform(msg);
}


pub struct IdleHook {
    pub pool: i32,
    pub token: Token,
}

impl std::fmt::Debug for IdleHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdleHook")
            .field("pool", &self.pool)
            .field("token", &"<fn>")
            .finish()
    }
}

pub fn drop_idle_hook(h: IdleHook) {
    let __destructured14 = h;
    let mut pool = __destructured14.pool;
    let mut token = __destructured14.token;
    drop_token(token);
}


pub struct Got {
    pub value: Dyn,
}

impl std::fmt::Debug for Got {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Got")
            .field("value", &"<fn>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sleep {
}

impl crate::wire::__Wire for Sleep {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Again {
}

impl crate::wire::__Wire for Again {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stuck {
    pub report: String,
}

impl crate::wire::__Wire for Stuck {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.report, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            report: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn drop_got(g: Got) {
    let __destructured15 = g;
    let mut value = __destructured15.value;
    drop_dyn_platform(value);
}

pub trait __Stateless_SchedTable: Send + Sync {
    fn new_pool(&self, sink: i32, dedicated: bool) -> i32;
    fn new_actor(&self, pool: i32, bound: i32, body: Body) -> i32;
    fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union4<Sent, Dead, Full, Remote>;
    fn set_proxy(&self, addr: i32);
    fn enqueue_remote(&self, addr: i32, msg: Dyn, from: i64) -> bool;
    fn kill(&self, addr: i32, reason: String);
    fn mint_actor(&self, addr: i32, gated: bool) -> Token;
    fn mint_task(&self, pool: i32, body: Body) -> Token;
    fn mint_waiter(&self, pool: i32) -> WaiterMint;
    fn deliver(&self, t: Token, value: Dyn);
    fn watch_actor(&self, addr: i32, t: Token);
    fn idle_hook(&self, pool: i32, t: Token);
    fn next_work(&self, pool: i32, idle: Parker) -> Option<Union3<RunActor, RunTask, Retire>>;
    fn retired_workers(&self) -> i32;
    fn wait_step(&self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<Got, RunActor, RunTask, Sleep, Again, Stuck>;
    fn finish(&self, addr: i32, body: Body, fault: Option<String>);
    fn task_done(&self, pool: i32, fault: Option<String>);
    fn pool_of_actor(&self, addr: i32) -> i32;
    fn pool_of_waiter(&self, wid: i32) -> i32;
    fn external(&self, delta: i32);
    fn room(&self, addr: i32) -> i32;
    fn is_dead(&self, addr: i32) -> bool;
    fn queued(&self, addr: i32) -> i32;
    fn go_virtual(&self, seed: i64);
    fn is_virtual(&self) -> bool;
    fn virtual_now(&self) -> i64;
    fn advance_to(&self, at: i64);
    fn random_bits(&self) -> i64;
    fn clock_hook(&self, t: Token);
    fn virtual_work(&self, own: i32) -> Option<Union2<RunActor, RunTask>>;
}

pub trait __Stateful_SchedTable: Send {
    fn new_pool(&mut self, sink: i32, dedicated: bool) -> i32;
    fn new_actor(&mut self, pool: i32, bound: i32, body: Body) -> i32;
    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union4<Sent, Dead, Full, Remote>;
    fn set_proxy(&mut self, addr: i32);
    fn enqueue_remote(&mut self, addr: i32, msg: Dyn, from: i64) -> bool;
    fn kill(&mut self, addr: i32, reason: String);
    fn mint_actor(&mut self, addr: i32, gated: bool) -> Token;
    fn mint_task(&mut self, pool: i32, body: Body) -> Token;
    fn mint_waiter(&mut self, pool: i32) -> WaiterMint;
    fn deliver(&mut self, t: Token, value: Dyn);
    fn watch_actor(&mut self, addr: i32, t: Token);
    fn idle_hook(&mut self, pool: i32, t: Token);
    fn next_work(&mut self, pool: i32, idle: Parker) -> Option<Union3<RunActor, RunTask, Retire>>;
    fn retired_workers(&mut self) -> i32;
    fn wait_step(&mut self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<Got, RunActor, RunTask, Sleep, Again, Stuck>;
    fn finish(&mut self, addr: i32, body: Body, fault: Option<String>);
    fn task_done(&mut self, pool: i32, fault: Option<String>);
    fn pool_of_actor(&mut self, addr: i32) -> i32;
    fn pool_of_waiter(&mut self, wid: i32) -> i32;
    fn external(&mut self, delta: i32);
    fn room(&mut self, addr: i32) -> i32;
    fn is_dead(&mut self, addr: i32) -> bool;
    fn queued(&mut self, addr: i32) -> i32;
    fn go_virtual(&mut self, seed: i64);
    fn is_virtual(&mut self) -> bool;
    fn virtual_now(&mut self) -> i64;
    fn advance_to(&mut self, at: i64);
    fn random_bits(&mut self) -> i64;
    fn clock_hook(&mut self, t: Token);
    fn virtual_work(&mut self, own: i32) -> Option<Union2<RunActor, RunTask>>;
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
    pub fn new_pool(&self, sink: i32, dedicated: bool) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_pool(sink, dedicated),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_pool(sink, dedicated),
        }
    }
    pub fn new_actor(&self, pool: i32, bound: i32, body: Body) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_actor(pool, bound, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_actor(pool, bound, body),
        }
    }
    pub fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union4<Sent, Dead, Full, Remote> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.enqueue(addr, msg, waiter),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().enqueue(addr, msg, waiter),
        }
    }
    pub fn set_proxy(&self, addr: i32) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.set_proxy(addr),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().set_proxy(addr),
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
    pub fn mint_actor(&self, addr: i32, gated: bool) -> Token {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_actor(addr, gated),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_actor(addr, gated),
        }
    }
    pub fn mint_task(&self, pool: i32, body: Body) -> Token {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_task(pool, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_task(pool, body),
        }
    }
    pub fn mint_waiter(&self, pool: i32) -> WaiterMint {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_waiter(pool),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_waiter(pool),
        }
    }
    pub fn deliver(&self, t: Token, value: Dyn) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.deliver(t, value),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().deliver(t, value),
        }
    }
    pub fn watch_actor(&self, addr: i32, t: Token) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.watch_actor(addr, t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().watch_actor(addr, t),
        }
    }
    pub fn idle_hook(&self, pool: i32, t: Token) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.idle_hook(pool, t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().idle_hook(pool, t),
        }
    }
    pub fn next_work(&self, pool: i32, idle: Parker) -> Option<Union3<RunActor, RunTask, Retire>> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.next_work(pool, idle),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().next_work(pool, idle),
        }
    }
    pub fn retired_workers(&self) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.retired_workers(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().retired_workers(),
        }
    }
    pub fn wait_step(&self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<Got, RunActor, RunTask, Sleep, Again, Stuck> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.wait_step(wid, pool, own, frame, me),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().wait_step(wid, pool, own, frame, me),
        }
    }
    pub fn finish(&self, addr: i32, body: Body, fault: Option<String>) {
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
    pub fn pool_of_waiter(&self, wid: i32) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.pool_of_waiter(wid),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().pool_of_waiter(wid),
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
    pub fn go_virtual(&self, seed: i64) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.go_virtual(seed),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().go_virtual(seed),
        }
    }
    pub fn is_virtual(&self) -> bool {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.is_virtual(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().is_virtual(),
        }
    }
    pub fn virtual_now(&self) -> i64 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.virtual_now(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().virtual_now(),
        }
    }
    pub fn advance_to(&self, at: i64) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.advance_to(at),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().advance_to(at),
        }
    }
    pub fn random_bits(&self) -> i64 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.random_bits(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().random_bits(),
        }
    }
    pub fn clock_hook(&self, t: Token) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.clock_hook(t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().clock_hook(t),
        }
    }
    pub fn virtual_work(&self, own: i32) -> Option<Union2<RunActor, RunTask>> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.virtual_work(own),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().virtual_work(own),
        }
    }
}

pub struct Scheduler {
    actors: Vec<ActorRec>,
    waiters: Vec<WaiterRec>,
    pools: Vec<PoolRec>,
    idle_hooks: Vec<IdleHook>,
    next_slot: i64,
    active: i32,
    parked_frames: i32,
    main_waits: i32,
    externals: i32,
    retired: i32,
    virtual_mode: bool,
    vnow: i64,
    rng: i64,
    clock_hooks: Vec<Token>,
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
            retired: 0,
            virtual_mode: false,
            vnow: 0i64,
            rng: 1i64,
            clock_hooks: vec![],
        }
    }
}

impl crate::runtime::__Stateful_SchedTable for Scheduler {

    fn new_pool(&mut self, sink: i32, dedicated: bool) -> i32 {
        self.pools.push(PoolRec { idle: vec![], tasks: std::collections::VecDeque::<TaskRun>::new(), ready: std::collections::VecDeque::<i32>::new(), sink: sink, owed: 0, dedicated: dedicated, retired: false });
        return (self.pools.len() as i32) - 1;
    }

    fn new_actor(&mut self, pool: i32, bound: i32, body: Body) -> i32 {
        self.actors.push(ActorRec { body: slot_of_platform(body), pool: pool, bound: bound, queue: std::collections::VecDeque::<Union3<Delivered, Answered, Reported>>::new(), slots: std::collections::VecDeque::<i64>::new(), user_len: 0, gate: None, running: false, dead: false, exit_reason: "".to_string(), blocked: vec![], watchers: vec![], owed: 0, ready: false, proxy: false });
        return (self.actors.len() as i32) - 1;
    }

    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union4<Sent, Dead, Full, Remote> {
        if addr < 0 || addr >= (self.actors.len() as i32) {
            drop_dyn_platform(msg);
            return Union4::<Sent, Dead, Full, Remote>::U2(Dead {  });
        }
        let __h0 = (addr) as usize;
        self.actors.get(__h0).expect("salvo: value is absent at runtime:497:17");
        if self.actors[__h0].dead {
            drop_dyn_platform(msg);
            return Union4::<Sent, Dead, Full, Remote>::U2(Dead {  });
        }
        if self.actors[__h0].proxy {
            return Union4::<Sent, Dead, Full, Remote>::U4(Remote { msg: msg });
        }
        if self.actors[__h0].user_len >= self.actors[__h0].bound {
            self.actors[__h0].blocked.push(waiter);
            return Union4::<Sent, Dead, Full, Remote>::U3(Full { msg: msg });
        }
        let mut e: Union3<Delivered, Answered, Reported> = Union3::<Delivered, Answered, Reported>::U1(Delivered { msg: msg, from: ((-1) as i64) });
        self.actors[__h0].queue.push_back(e);
        self.actors[__h0].slots.push_back(((-1) as i64));
        self.actors[__h0].user_len = self.actors[__h0].user_len + 1;
        if mark_ready(&mut self.actors[__h0], &mut self.pools, addr.clone()) {
            let mut pool = self.actors[__h0].pool;
            wake_pool(&mut self.pools, pool);
        }
        return Union4::<Sent, Dead, Full, Remote>::U1(Sent {  });
    }

    fn enqueue_remote(&mut self, addr: i32, msg: Dyn, from: i64) -> bool {
        if addr < 0 || addr >= (self.actors.len() as i32) {
            drop_dyn_platform(msg);
            return false;
        }
        let __h1 = (addr) as usize;
        self.actors.get(__h1).expect("salvo: value is absent at runtime:525:17");
        if self.actors[__h1].dead {
            drop_dyn_platform(msg);
            return false;
        }
        let mut e: Union3<Delivered, Answered, Reported> = Union3::<Delivered, Answered, Reported>::U1(Delivered { msg: msg, from: from });
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
        self.actors.get(__h2).expect("salvo: value is absent at runtime:542:17");
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
        let mut watchers: Vec<Token> = vec![];
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

    fn mint_actor(&mut self, addr: i32, gated: bool) -> Token {
        self.next_slot = self.next_slot + ((1) as i64);
        let mut slot = self.next_slot.clone();
        let __h3 = (addr) as usize;
        self.actors.get(__h3).expect("salvo: value is absent at runtime:564:17");
        if gated {
            self.actors[__h3].gate = Some(slot.clone());
        }
        self.actors[__h3].owed = self.actors[__h3].owed + 1;
        return Token { target: Union3::<ToActor, ToWaiter, ToTask>::U1(ToActor { addr: addr }), slot: slot, tracked: true };
    }

    fn mint_task(&mut self, pool: i32, body: Body) -> Token {
        self.next_slot = self.next_slot + ((1) as i64);
        let __h4 = (pool) as usize;
        self.pools.get(__h4).expect("salvo: value is absent at runtime:574:17");
        self.pools[__h4].owed = self.pools[__h4].owed + 1;
        return Token { target: Union3::<ToActor, ToWaiter, ToTask>::U3(ToTask { pool: pool, body: body }), slot: self.next_slot.clone(), tracked: true };
    }

    fn mint_waiter(&mut self, pool: i32) -> WaiterMint {
        let __h5 = (pool) as usize;
        self.pools.get(__h5).expect("salvo: value is absent at runtime:580:17");
        self.pools[__h5].owed = self.pools[__h5].owed + 1;
        self.waiters.push(WaiterRec { pool: pool, value: slot_empty_platform(), filled: false, parker: None, waiting: 0, waiting_actor: -1 });
        let mut wid = (self.waiters.len() as i32) - 1;
        self.next_slot = self.next_slot + ((1) as i64);
        let mut t = Token { target: Union3::<ToActor, ToWaiter, ToTask>::U2(ToWaiter { wid: wid.clone() }), slot: self.next_slot.clone(), tracked: true };
        return WaiterMint { token: t, wid: wid };
    }

    fn deliver(&mut self, t: Token, value: Dyn) {
        deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, value);
    }

    fn watch_actor(&mut self, addr: i32, t: Token) {
        let mut watch = untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        let __h6 = (addr) as usize;
        self.actors.get(__h6).expect("salvo: value is absent at runtime:597:17");
        if self.actors[__h6].dead {
            let mut reason = self.actors[__h6].exit_reason.clone();
            deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, watch, erase_platform(Exit { reason: reason }));
            return;
        }
        self.actors[__h6].watchers.push(watch);
    }

    fn idle_hook(&mut self, pool: i32, t: Token) {
        let mut hook = untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        self.idle_hooks.push(IdleHook { pool: pool, token: hook });
        wake_all_pools(&mut self.pools);
    }

    fn next_work(&mut self, pool: i32, idle: Parker) -> Option<Union3<RunActor, RunTask, Retire>> {
        if self.pools.get((pool) as i64 as usize).expect("salvo: value is absent at runtime:613:12").retired {
            self.retired = self.retired + 1;
            return Some(Union3::<RunActor, RunTask, Retire>::U3(Retire {  }));
        }
        let mut w = take_work(&mut self.actors, &mut self.pools, pool.clone(), -1);
        if matches!(w, Some(Union2::U1(_))) {
            let mut ra = match w { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Some(Union3::<RunActor, RunTask, Retire>::U1(ra));
        }
        if matches!(w, Some(Union2::U2(_))) {
            let mut rt = match w { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Some(Union3::<RunActor, RunTask, Retire>::U2(rt));
        }
        let __h7 = (pool) as usize;
        self.pools.get(__h7).expect("salvo: value is absent at runtime:626:17");
        self.pools[__h7].idle.push(idle);
        if self.active == self.parked_frames && quiet(&mut self.actors, &mut self.waiters, &mut self.pools, self.externals) {
            if !((self.idle_hooks.len() as i32) == 0) && self.active == 0 {
                fire_idle(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.idle_hooks);
            }
            wake_waiters(&mut self.waiters);
        }
        return None;
    }

    fn retired_workers(&mut self) -> i32 {
        return self.retired.clone();
    }

    fn wait_step(&mut self, wid: i32, pool: i32, own: i32, frame: i32, me: Parker) -> Union6<Got, RunActor, RunTask, Sleep, Again, Stuck> {
        let __h8 = (wid) as usize;
        self.waiters.get(__h8).expect("salvo: value is absent at runtime:647:17");
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
            return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U1(Got { value: v });
        }
        let mut work = take_for(&mut self.actors, &mut self.pools, pool.clone(), own.clone(), self.virtual_mode.clone());
        if matches!(work, Some(Union2::U1(_))) {
            let mut ra = match work { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U2(ra);
        }
        if matches!(work, Some(Union2::U2(_))) {
            let mut rt = match work { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U3(rt);
        }
        let mut q = quiet(&mut self.actors, &mut self.waiters, &mut self.pools, self.externals);
        if self.virtual_mode && (self.clock_hooks.len() as i32) > 0 && q {
            fire_clock(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.clock_hooks);
            return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U5(Again {  });
        }
        if !((self.idle_hooks.len() as i32) == 0) && self.active == 0 && q {
            fire_idle(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.idle_hooks);
            return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U5(Again {  });
        }
        if self.virtual_mode || self.active == self.parked_frames && self.main_waits > 0 && q {
            return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U6(Stuck { report: deadlock_report(&mut self.actors, &mut self.waiters, own) });
        }
        let __h9 = (wid) as usize;
        self.waiters.get(__h9).expect("salvo: value is absent at runtime:704:22");
        self.waiters[__h9].parker = Some(me.clone());
        let __h10 = (pool) as usize;
        self.pools.get(__h10).expect("salvo: value is absent at runtime:706:17");
        self.pools[__h10].idle.push(me);
        return Union6::<Got, RunActor, RunTask, Sleep, Again, Stuck>::U4(Sleep {  });
    }

    fn finish(&mut self, addr: i32, body: Body, fault: Option<String>) {
        self.active = self.active - 1;
        let __h11 = (addr) as usize;
        self.actors.get(__h11).expect("salvo: value is absent at runtime:713:17");
        self.actors[__h11].running = false;
        if fault.is_none() {
            slot_put_platform(&mut self.actors[__h11].body, body);
            let mut _again = mark_ready(&mut self.actors[__h11], &mut self.pools, addr.clone());
            return;
        }
        drop_body_platform(body);
        let mut reason = fault.as_ref().unwrap().clone();
        self.actors[__h11].dead = true;
        self.actors[__h11].exit_reason = reason.clone();
        self.actors[__h11].gate = None;
        self.actors[__h11].user_len = 0;
        while ((self.actors[__h11].queue.len() as i32) > 0) {
            drop_entry(self.actors[__h11].queue.pop_front().expect("salvo: value is absent at runtime:729:24"));
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
        let mut watchers: Vec<Token> = vec![];
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
        retire_if_done(&mut self.actors, &mut self.pools, pool.clone());
        wake_all_pools(&mut self.pools);
    }

    fn task_done(&mut self, pool: i32, fault: Option<String>) {
        self.active = self.active - 1;
        if fault.is_some() {
            let mut reason = fault.as_ref().unwrap().clone();
            report_fault(&mut self.actors, &mut self.pools, pool.clone(), reason.clone());
        }
        retire_if_done(&mut self.actors, &mut self.pools, pool);
    }

    fn pool_of_actor(&mut self, addr: i32) -> i32 {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:764:21").pool;
    }

    fn set_proxy(&mut self, addr: i32) {
        let __h12 = (addr) as usize;
        self.actors.get(__h12).expect("salvo: value is absent at runtime:768:17");
        self.actors[__h12].proxy = true;
    }

    fn pool_of_waiter(&mut self, wid: i32) -> i32 {
        return self.waiters.get((wid) as i64 as usize).expect("salvo: value is absent at runtime:773:21").pool;
    }

    fn room(&mut self, addr: i32) -> i32 {
        let mut a = self.actors.get((addr) as i64 as usize).unwrap();
        return a.bound - a.user_len;
    }

    fn is_dead(&mut self, addr: i32) -> bool {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:782:21").dead;
    }

    fn queued(&mut self, addr: i32) -> i32 {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:786:21").user_len;
    }

    fn external(&mut self, delta: i32) {
        self.externals = self.externals + delta;
        if self.externals < 0 {
            self.externals = 0;
        }
        wake_all_pools(&mut self.pools);
    }

    fn go_virtual(&mut self, seed: i64) {
        self.virtual_mode = true;
        self.vnow = 0i64;
        self.rng = seed_of(seed);
        reset_all(&mut self.actors, &mut self.pools, &mut self.idle_hooks, &mut self.clock_hooks);
        self.active = 0;
        self.parked_frames = 0;
        self.main_waits = 0;
        self.externals = 0;
    }

    fn is_virtual(&mut self) -> bool {
        return self.virtual_mode.clone();
    }

    fn virtual_now(&mut self) -> i64 {
        return self.vnow.clone();
    }

    fn advance_to(&mut self, at: i64) {
        if at > self.vnow {
            self.vnow = at;
        }
    }

    fn random_bits(&mut self) -> i64 {
        let mut hi = lehmer(self.rng.clone());
        let mut lo = lehmer(hi.clone());
        self.rng = lo.clone();
        return hi * ((2147483647) as i64) + lo;
    }

    fn clock_hook(&mut self, t: Token) {
        let mut hook = untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        self.clock_hooks.push(hook);
    }

    fn virtual_work(&mut self, own: i32) -> Option<Union2<RunActor, RunTask>> {
        let mut w = take_for(&mut self.actors, &mut self.pools, 0, own, true);
        if matches!(w, Some(Union2::U1(_))) {
            let mut ra = match w { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Some(Union2::<RunActor, RunTask>::U1(ra));
        }
        if matches!(w, Some(Union2::U2(_))) {
            let mut rt = match w { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = self.active + 1;
            return Some(Union2::<RunActor, RunTask>::U2(rt));
        }
        return None;
    }
}

impl Scheduler {

    fn init(&mut self) {
        self.pools.push(PoolRec { idle: vec![], tasks: std::collections::VecDeque::<TaskRun>::new(), ready: std::collections::VecDeque::<i32>::new(), sink: -1, owed: 0, dedicated: false, retired: false });
    }
}

pub enum __Priv_Scheduler {
    Init,
}

pub fn lehmer(x: i64) -> i64 {
    return x * ((48271) as i64) % ((2147483647) as i64);
}

pub fn seed_of(seed: i64) -> i64 {
    let mut s = seed % ((2147483646) as i64);
    if s < ((0) as i64) {
        s = ((0) as i64) - s;
    }
    return s + ((1) as i64);
}

pub fn reset_all(actors: &mut Vec<ActorRec>, pools: &mut Vec<PoolRec>, idle_hooks: &mut Vec<IdleHook>, clock_hooks: &mut Vec<Token>) {
    let mut k = 0;
    while k < (actors.len() as i32) {
        let __h13 = (k) as usize;
        actors.get(__h13).expect("salvo: value is absent at runtime:872:17");
        k = k + 1;
        actors[__h13].dead = true;
        actors[__h13].running = false;
        actors[__h13].ready = false;
        actors[__h13].gate = None;
        actors[__h13].user_len = 0;
        actors[__h13].owed = 0;
        while ((actors[__h13].queue.len() as i32) > 0) {
            drop_entry(actors[__h13].queue.pop_front().expect("salvo: value is absent at runtime:881:24"));
        }
        while actors[__h13].slots.pop_front().is_some() {
        }
        loop {
            let mut __is7 = actors[__h13].watchers.salvo_remove_first();
            if !(__is7.is_some()) {
                break;
            }
            let mut t = __is7.unwrap();
            drop_token(t);
        }
        while actors[__h13].blocked.salvo_remove_first().is_some() {
        }
        let mut __is8 = slot_take_platform(&mut actors[__h13].body);
        if __is8.is_some() {
            let mut b = __is8.unwrap();
            drop_body_platform(b);
        }
    }
    let mut i = 0;
    while i < (pools.len() as i32) {
        let __h14 = (i) as usize;
        pools.get(__h14).expect("salvo: value is absent at runtime:896:17");
        loop {
            let mut __is9 = pools[__h14].tasks.pop_front();
            if !(__is9.is_some()) {
                break;
            }
            let mut t = __is9.unwrap();
            drop_task_run(t);
        }
        while pools[__h14].ready.pop_front().is_some() {
        }
        pools[__h14].owed = 0;
        if i > 0 {
            pools[__h14].retired = true;
        }
        i = i + 1;
    }
    loop {
        let mut __is10 = idle_hooks.salvo_remove_first();
        if !(__is10.is_some()) {
            break;
        }
        let mut h = __is10.unwrap();
        drop_idle_hook(h);
    }
    loop {
        let mut __is11 = clock_hooks.salvo_remove_first();
        if !(__is11.is_some()) {
            break;
        }
        let mut t = __is11.unwrap();
        drop_token(t);
    }
}

pub fn fire_clock(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, hooks: &mut Vec<Token>) {
    let mut __is12 = hooks.salvo_remove_first();
    if __is12.is_some() {
        let mut t = __is12.unwrap();
        deliver_to(actors, waiters, pools, t, erase_platform(0));
    }
}

pub fn take_for(actors: &mut Vec<ActorRec>, pools: &mut Vec<PoolRec>, pool: i32, exclude: i32, any: bool) -> Option<Union2<RunActor, RunTask>> {
    if !any {
        return take_work(actors, pools, pool, exclude);
    }
    let mut i = 0;
    while i < (pools.len() as i32) {
        let mut w = take_work(actors, pools, i.clone(), exclude.clone());
        if matches!(w, Some(Union2::U1(_))) {
            let mut ra = match w { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            return Some(Union2::<RunActor, RunTask>::U1(ra));
        }
        if matches!(w, Some(Union2::U2(_))) {
            let mut rt = match w { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            return Some(Union2::<RunActor, RunTask>::U2(rt));
        }
        i = i + 1;
    }
    return None;
}

pub fn untrack(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, t: Token) -> Token {
    let __destructured16 = t;
    let mut target = __destructured16.target;
    let mut slot = __destructured16.slot;
    let mut tracked = __destructured16.tracked;
    if tracked {
        release(actors, waiters, pools, &target);
    }
    return Token { target: target, slot: slot, tracked: false };
}

pub fn release(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, target: &Union3<ToActor, ToWaiter, ToTask>) {
    if matches!(target, Union3::U1(_)) {
        let mut to = target.u1().clone();
        let __h15 = (to.addr) as usize;
        actors.get(__h15).expect("salvo: value is absent at runtime:960:17");
        if actors[__h15].owed > 0 {
            actors[__h15].owed = actors[__h15].owed - 1;
        }
    } else if matches!(target, Union3::U2(_)) {
        let mut tw = target.u2().clone();
        let mut pool = waiters.get((tw.wid) as i64 as usize).expect("salvo: value is absent at runtime:965:25").pool;
        let __h16 = (pool) as usize;
        pools.get(__h16).expect("salvo: value is absent at runtime:966:17");
        if pools[__h16].owed > 0 {
            pools[__h16].owed = pools[__h16].owed - 1;
        }
    } else {
        let __h17 = (target.u3().clone().pool) as usize;
        pools.get(__h17).expect("salvo: value is absent at runtime:971:17");
        if pools[__h17].owed > 0 {
            pools[__h17].owed = pools[__h17].owed - 1;
        }
    }
}

pub fn deliver_to(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, t: Token, value: Dyn) {
    let __destructured17 = t;
    let mut target = __destructured17.target;
    let mut slot = __destructured17.slot;
    let mut tracked = __destructured17.tracked;
    if tracked {
        release(actors, waiters, pools, &target);
    }
    if matches!(target, Union3::U1(_)) {
        let mut to = target.u1().clone();
        let __h18 = (to.addr) as usize;
        actors.get(__h18).expect("salvo: value is absent at runtime:987:17");
        if actors[__h18].dead {
            drop_dyn_platform(value);
            return;
        }
        actors[__h18].slots.push_back(slot.clone());
        let mut e: Union3<Delivered, Answered, Reported> = Union3::<Delivered, Answered, Reported>::U2(Answered { slot: slot, value: value });
        actors[__h18].queue.push_back(e);
        if mark_ready(&mut actors[__h18], pools, to.addr) {
            let mut pool = actors[__h18].pool;
            wake_pool(pools, pool);
        }
    } else if matches!(target, Union3::U2(_)) {
        let mut tw = target.u2().clone();
        let __h19 = (tw.wid) as usize;
        waiters.get(__h19).expect("salvo: value is absent at runtime:1000:17");
        slot_put_platform(&mut waiters[__h19].value, value);
        waiters[__h19].filled = true;
        if waiters[__h19].parker.is_some() {
            let mut p = waiters[__h19].parker.as_ref().unwrap().clone();
            unpark_platform(&(p.clone()));
        }
    } else {
        let __destructured18 = (match target { Union3::U3(__v) => __v, _ => unreachable!() });
        let mut pool = __destructured18.pool;
        let mut body = __destructured18.body;
        let __h20 = (pool) as usize;
        pools.get(__h20).expect("salvo: value is absent at runtime:1008:17");
        pools[__h20].tasks.push_back(TaskRun { body: body, value: value });
        wake_pool(pools, pool.clone());
    }
}

pub fn report_fault(actors: &mut Vec<ActorRec>, pools: &mut Vec<PoolRec>, pool: i32, reason: String) {
    let mut sink = pools.get((pool) as i64 as usize).expect("salvo: value is absent at runtime:1019:21").sink;
    if sink >= 0 {
        let __h21 = (sink) as usize;
        actors.get(__h21).expect("salvo: value is absent at runtime:1021:17");
        if !actors[__h21].dead {
            let mut e: Union3<Delivered, Answered, Reported> = Union3::<Delivered, Answered, Reported>::U3(Reported { reason: reason });
            actors[__h21].queue.push_back(e);
            actors[__h21].slots.push_back(((-1) as i64));
            if mark_ready(&mut actors[__h21], pools, sink.clone()) {
                let mut sink_pool = actors[__h21].pool;
                wake_pool(pools, sink_pool);
            }
            return;
        }
    }
    __module_use_0().report(&(format!("salvo: an uncaught fault on pool {}: {}", pool, reason)));
}

pub fn quiet(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, externals: i32) -> bool {
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

pub fn fire_idle(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, hooks: &mut Vec<IdleHook>) {
    loop {
        let mut __is13 = hooks.salvo_remove_first();
        if !(__is13.is_some()) {
            break;
        }
        let mut h = __is13.unwrap();
        let __destructured19 = h;
        let mut pool = __destructured19.pool;
        let mut token = __destructured19.token;
        let mut gates = 0;
        let mut tokens = pools.get((pool) as i64 as usize).expect("salvo: value is absent at runtime:1071:27").owed;
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

pub fn deadlock_report(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, own: i32) -> String {
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

pub fn take_work(actors: &mut Vec<ActorRec>, pools: &mut Vec<PoolRec>, pool: i32, exclude: i32) -> Option<Union2<RunActor, RunTask>> {
    let __h22 = (pool) as usize;
    pools.get(__h22).expect("salvo: value is absent at runtime:1120:13");
    let mut task = pools[__h22].tasks.pop_front();
    if task.is_some() {
        let mut t = task.unwrap();
        let __destructured20 = t;
        let mut body = __destructured20.body;
        let mut value = __destructured20.value;
        return Some(Union2::<RunActor, RunTask>::U2(RunTask { pool: pool, body: body, value: value }));
    }
    loop {
        let mut __is14 = pools[__h22].ready.pop_front();
        if !(__is14.is_some()) {
            break;
        }
        let mut i = __is14.unwrap();
        let __h23 = (i) as usize;
        actors.get(__h23).expect("salvo: value is absent at runtime:1127:17");
        actors[__h23].ready = false;
        if i != exclude && !actors[__h23].running && !actors[__h23].dead {
            let mut at = deliverable(&actors[__h23].slots, &actors[__h23].gate);
            if at.is_some() {
                let mut k = at.unwrap();
                let mut _slot = actors[__h23].slots.remove((k.clone()) as i64 as usize);
                let mut e = actors[__h23].queue.remove((k) as i64 as usize).expect("salvo: value is absent at runtime:1133:25");
                actors[__h23].running = true;
                let mut body = slot_take_platform(&mut actors[__h23].body).expect("salvo: value is absent at runtime:1135:28");
                return work_of(&mut actors[__h23], i.clone(), e, body);
            }
        }
    }
    return None;
}

pub fn mark_ready(a: &mut ActorRec, pools: &mut Vec<PoolRec>, addr: i32) -> bool {
    if a.ready || a.running || a.dead {
        return false;
    }
    if deliverable(&a.slots, &a.gate).is_none() {
        return false;
    }
    a.ready = true;
    let __h24 = (a.pool) as usize;
    pools.get(__h24).expect("salvo: value is absent at runtime:1154:13");
    pools[__h24].ready.push_back(addr);
    return true;
}

pub fn work_of(a: &mut ActorRec, addr: i32, e: Union3<Delivered, Answered, Reported>, body: Body) -> Option<Union2<RunActor, RunTask>> {
    if matches!(e, Union3::U1(_)) {
        let mut d = match e { Union3::U1(__v) => __v, _ => unreachable!() };
        a.user_len = a.user_len - 1;
        let mut woken = a.blocked.salvo_remove_first();
        if woken.is_some() {
            let mut b = woken.as_ref().unwrap().clone();
            unpark_platform(&b);
        }
        let __destructured21 = d;
        let mut msg = __destructured21.msg;
        let mut from = __destructured21.from;
        if from >= ((0) as i64) {
            granted_platform(addr.clone(), a.pool, from);
        }
        return Some(Union2::<RunActor, RunTask>::U1(RunActor { addr: addr, pool: a.pool, kind: 0, slot: 0i64, value: msg, body: body }));
    }
    if matches!(e, Union3::U3(_)) {
        let mut r = e.u3().clone();
        return Some(Union2::<RunActor, RunTask>::U1(RunActor { addr: addr, pool: a.pool, kind: 2, slot: 0i64, value: erase_platform(r.reason.clone()), body: body }));
    }
    let __destructured22 = (match e { Union3::U2(__v) => __v, _ => unreachable!() });
    let mut slot = __destructured22.slot;
    let mut value = __destructured22.value;
    let mut opens = false;
    if a.gate.is_some() {
        let mut g = a.gate.unwrap();
        opens = g == slot;
    }
    if opens {
        a.gate = None;
    }
    return Some(Union2::<RunActor, RunTask>::U1(RunActor { addr: addr, pool: a.pool, kind: 1, slot: slot, value: value, body: body }));
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
        if *slots.get((i) as i64 as usize).expect("salvo: value is absent at runtime:1202:12") == gate.unwrap() {
            return Some(i.clone());
        }
        i = i + 1;
    }
    return None;
}

pub fn wake_pool(pools: &mut Vec<PoolRec>, pool: i32) {
    let __h25 = (pool) as usize;
    pools.get(__h25).expect("salvo: value is absent at runtime:1217:13");
    let mut __is15 = pools[__h25].idle.salvo_remove_first();
    if __is15.is_some() {
        let mut w = __is15.as_ref().unwrap().clone();
        unpark_platform(&w);
    }
}

pub fn wake_every(pools: &mut Vec<PoolRec>, pool: i32) {
    let __h26 = (pool) as usize;
    pools.get(__h26).expect("salvo: value is absent at runtime:1225:13");
    loop {
        let mut __is16 = pools[__h26].idle.salvo_remove_first();
        if !(__is16.is_some()) {
            break;
        }
        let mut w = __is16.as_ref().unwrap().clone();
        unpark_platform(&w);
    }
}

pub fn wake_waiters(waiters: &mut Vec<WaiterRec>) {
    for w in &*waiters {
        if w.waiting > 0 && (w.parker.is_some()) {
            let mut p = w.parker.as_ref().unwrap().clone();
            unpark_platform(&(p.clone()));
        }
    }
}

pub fn retire_if_done(actors: &mut Vec<ActorRec>, pools: &mut Vec<PoolRec>, pool: i32) {
    let __h27 = (pool) as usize;
    pools.get(__h27).expect("salvo: value is absent at runtime:1247:13");
    if !pools[__h27].dedicated || pools[__h27].retired || (pools[__h27].tasks.len() as i32) > 0 || pools[__h27].owed > 0 {
        return;
    }
    for a in &*actors {
        if a.pool == pool && !a.dead {
            return;
        }
    }
    pools[__h27].retired = true;
    wake_every(pools, pool);
}

pub fn wake_all_pools(pools: &mut Vec<PoolRec>) {
    let mut i = 0;
    while i < (pools.len() as i32) {
        wake_every(pools, i.clone());
        i = i + 1;
    }
}

pub fn new_pool_of(n: i32, sink: i32) -> i32 {
    return start_pool(n, sink, false);
}

pub fn new_dedicated_pool() -> i32 {
    return start_pool(1, -1, true);
}

pub fn retired_worker_count() -> i32 {
    return __module_use_1().retired_workers();
}

pub fn start_pool(n: i32, sink: i32, dedicated: bool) -> i32 {
    let mut id = __module_use_1().new_pool(sink, dedicated);
    if __module_use_1().is_virtual() {
        return id;
    }
    let mut i = 0;
    while i < n {
        start_thread_platform(Box::new({ let mut id = id.clone(); move || {
    serve_pool(id.clone());
} }));
        i = i + 1;
    }
    return id;
}

pub fn spawn_body(pool: i32, bound: i32, body: Body) -> i32 {
    return __module_use_1().new_actor(pool, bound, body);
}

pub fn send_dyn(addr: i32, msg: Dyn) {
    let mut back = send_or_back(addr, msg);
    if back.is_some() {
        let mut d = back.unwrap();
        drop_dyn_platform(d);
    }
}

pub fn send_or_back(addr: i32, msg: Dyn) -> Option<Dyn> {
    let mut r = __module_use_1().enqueue(addr.clone(), msg, this_parker_platform());
    while matches!(r, Union4::U3(_)) {
        let mut full = match r { Union4::U3(__v) => __v, _ => unreachable!() };
        if __module_use_1().is_virtual() {
            make_room(addr.clone());
            let __destructured23 = full;
            let mut again = __destructured23.msg;
            r = __module_use_1().enqueue(addr.clone(), again, this_parker_platform());
            continue;
        }
        if here_pool_platform() == main_pool() && here_actor_platform() == no_frame() && __module_use_1().pool_of_actor(addr.clone()) == main_pool() {
            __module_use_0().report(&(format!("salvo: deadlock: the main pool's actor {} has a full mailbox and the only thread that could drain it is the one sending: the main pool has one worker, `main` itself, and it serves work only inside a `waitfor` — send fewer messages before waiting, raise the handler's `mailbox` capacity, or place the actor on a pool of its own", addr)));
            exit_process_platform(1);
        }
        park_platform(&(this_parker_platform()));
        let __destructured24 = full;
        let mut back = __destructured24.msg;
        r = __module_use_1().enqueue(addr.clone(), back, this_parker_platform());
    }
    if matches!(r, Union4::U3(_)) {
        let mut full = match r { Union4::U3(__v) => __v, _ => unreachable!() };
        drop_full(full);
        return None;
    }
    if matches!(r, Union4::U4(_)) {
        let mut remote = match r { Union4::U4(__v) => __v, _ => unreachable!() };
        let __destructured25 = remote;
        let mut back = __destructured25.msg;
        return Some(back);
    }
    return None;
}

pub fn make_room(addr: i32) {
    let mut w = __module_use_1().virtual_work(here_actor_platform());
    if matches!(w, Some(Union2::U1(_))) {
        let mut ra = match w { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
        run_actor(ra);
        return;
    }
    if matches!(w, Some(Union2::U2(_))) {
        let mut rt = match w { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
        run_task(rt);
        return;
    }
    __module_use_0().report(&(format!("salvo: deadlock: actor {} has a full mailbox and nothing can run to drain it: an actor test runs every pool on one thread", addr)));
    exit_process_platform(1);
}

pub fn mark_proxy(addr: i32) {
    __module_use_1().set_proxy(addr);
}

pub fn mint(addr: i32, gated: bool) -> Token {
    return __module_use_1().mint_actor(addr, gated);
}

pub fn mint_task_on(pool: i32, body: Body) -> Token {
    return __module_use_1().mint_task(pool, body);
}

pub fn waiter() -> WaiterMint {
    return __module_use_1().mint_waiter(here_pool_platform());
}

pub fn answer(t: Token, value: Dyn) {
    __module_use_1().deliver(t, value);
}

pub fn watch(addr: i32, t: Token) {
    __module_use_1().watch_actor(addr, t);
}

pub fn on_idle(pool: i32, t: Token) {
    __module_use_1().idle_hook(pool, t);
}

pub fn external_begin() {
    if __module_use_1().is_virtual() {
        __module_use_0().report(&("salvo: an actor test opened a host thread (a platform handler that reads or listens on a thread of its own): the virtual runtime runs everything on one thread, so its work would not be deterministic — use an in-memory fake (`MemTransport`, `MemFs`)".to_string()));
        exit_process_platform(1);
    }
    __module_use_1().external(1);
}

pub fn enter_virtual(seed: i64) {
    __module_use_1().go_virtual(seed);
}

pub fn virtual_runtime() -> bool {
    return __module_use_1().is_virtual();
}

pub fn set_virtual_now(at: i64) {
    __module_use_1().advance_to(at);
}

pub fn on_clock(t: Token) {
    __module_use_1().clock_hook(t);
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
            let __destructured26 = g;
            let mut value = __destructured26.value;
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

pub fn run_actor(ra: RunActor) {
    flush_frames_platform();
    let __destructured27 = ra;
    let mut addr = __destructured27.addr;
    let mut pool = __destructured27.pool;
    let mut kind = __destructured27.kind;
    let mut slot = __destructured27.slot;
    let mut value = __destructured27.value;
    let mut body = __destructured27.body;
    let mut saved_pool = here_pool_platform();
    let mut saved_actor = here_actor_platform();
    set_here_platform(pool, addr.clone());
    let mut ran = activate_platform(body, kind, slot, value);
    set_here_platform(saved_pool, saved_actor);
    let __destructured28 = ran;
    let mut back = __destructured28.body;
    let mut fault = __destructured28.fault;
    __module_use_1().finish(addr, back, fault);
}

pub fn run_task(rt: RunTask) {
    let __destructured29 = rt;
    let mut pool = __destructured29.pool;
    let mut body = __destructured29.body;
    let mut value = __destructured29.value;
    let mut saved_pool = here_pool_platform();
    let mut saved_actor = here_actor_platform();
    set_here_platform(pool.clone(), task_frame());
    let mut ran = activate_platform(body, 1, 0i64, value);
    set_here_platform(saved_pool, saved_actor);
    let __destructured30 = ran;
    let mut done = __destructured30.body;
    let mut fault = __destructured30.fault;
    drop_body_platform(done);
    __module_use_1().task_done(pool, fault);
}

pub fn serve_pool(pool: i32) {
    set_here_platform(pool.clone(), no_frame());
    loop {
        let mut w = __module_use_1().next_work(pool.clone(), this_parker_platform());
        if matches!(w, Some(Union3::U1(_))) {
            let mut ra = match w { Some(Union3::U1(__v)) => __v, _ => unreachable!() };
            run_actor(ra);
        } else if matches!(w, Some(Union3::U2(_))) {
            let mut rt = match w { Some(Union3::U2(__v)) => __v, _ => unreachable!() };
            run_task(rt);
        } else if matches!(w, Some(Union3::U3(_))) {
            return;
        } else {
            park_platform(&(this_parker_platform()));
        }
    }
}

pub fn token_to_actor(addr: i32, slot: i64) -> Token {
    return Token { target: Union3::<ToActor, ToWaiter, ToTask>::U1(ToActor { addr: addr }), slot: slot, tracked: false };
}

pub fn token_to_waiter(wid: i32, slot: i64) -> Token {
    return Token { target: Union3::<ToActor, ToWaiter, ToTask>::U2(ToWaiter { wid: wid }), slot: slot, tracked: false };
}


pub struct Exported {
    pub kind: i32,
    pub id: i32,
    pub slot: i64,
    pub body: Option<Body>,
}

impl std::fmt::Debug for Exported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exported")
            .field("kind", &self.kind)
            .field("id", &self.id)
            .field("slot", &self.slot)
            .field("body", &"<fn>")
            .finish()
    }
}

pub fn export_token(t: Token) -> Exported {
    let __destructured31 = t;
    let mut target = __destructured31.target;
    let mut slot = __destructured31.slot;
    let mut tracked = __destructured31.tracked;
    if matches!(target, Union3::U3(_)) {
        let mut tt = match target { Union3::U3(__v) => __v, _ => unreachable!() };
        let __destructured32 = tt;
        let mut pool = __destructured32.pool;
        let mut body = __destructured32.body;
        return Exported { kind: 2, id: pool, slot: slot, body: Some(body) };
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
    return Exported { kind: kind, id: id, slot: slot, body: None };
}

pub fn drop_exported(e: Exported) {
    let __destructured33 = e;
    let mut kind = __destructured33.kind;
    let mut id = __destructured33.id;
    let mut slot = __destructured33.slot;
    let mut body = __destructured33.body;
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

pub fn actor_pool(addr: i32) -> i32 {
    return __module_use_1().pool_of_actor(addr);
}

pub fn waiter_pool(wid: i32) -> i32 {
    return __module_use_1().pool_of_waiter(wid);
}

pub fn spawn_inert() -> i32 {
    return spawn_body(0, 0, body_of_platform(Box::new(move |kind, slot, value| drop_dyn_platform(value))));
}
