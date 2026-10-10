use crate::core_actor::Exit;
use crate::core_actor::Idle;
use crate::core_deque::add_last_platform;
use crate::core_list::add_platform;
use crate::core_list::at__loc;
use crate::core_string::join_platform;
use crate::core_deque::mut_deque_of;
use crate::core_deque::remove_at_platform;


pub fn __module_use0() -> &'static std::sync::Arc<crate::runtime::__Platform_HostRuntime> {
    static CELL: std::sync::OnceLock<std::sync::Arc<crate::runtime::__Platform_HostRuntime>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Arc::new(crate::runtime::__Platform_HostRuntime::new()))
}

pub fn __module_use0_0() -> &'static crate::runtime::RuntimeHost {
    static CELL: std::sync::OnceLock<crate::runtime::RuntimeHost> = std::sync::OnceLock::new();
    CELL.get_or_init(|| crate::runtime::RuntimeHost::share_shared(crate::runtime::__module_use0().clone()))
}

pub fn __module_use1() -> &'static std::sync::Arc<std::sync::Mutex<crate::runtime::Scheduler>> {
    static CELL: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<crate::runtime::Scheduler>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Arc::new(std::sync::Mutex::new(crate::runtime::Scheduler::new())))
}

pub fn __module_use1_0() -> &'static crate::runtime::SchedTable {
    static CELL: std::sync::OnceLock<crate::runtime::SchedTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| crate::runtime::SchedTable::share_locked(crate::runtime::__module_use1().clone()))
}

/// [platform-type] The host's `Parker`.
pub use crate::platform_runtime::Parker;
const _: fn() = || { fn __contract<T: Send + 'static + Clone + Sync>() {} __contract::<Parker>(); };

pub fn this_parker_platform() -> crate::runtime::Parker {
    crate::platform_runtime::this_parker()
}

pub fn park_platform(p: &crate::runtime::Parker) {
    crate::platform_runtime::park(p)
}

pub fn park_nanos_platform(p: &crate::runtime::Parker, mut nanos: i64) {
    crate::platform_runtime::park_nanos(p, nanos)
}

pub fn unpark_platform(p: &crate::runtime::Parker) {
    crate::platform_runtime::unpark(p)
}

pub fn start_thread_platform(body: std::boxed::Box<dyn FnOnce() + Send + 'static>) {
    crate::platform_runtime::start_thread(body)
}

pub fn guarded_platform(body: std::boxed::Box<dyn FnOnce() + Send + 'static>) -> Option<String> {
    crate::platform_runtime::guarded(body)
}

pub fn trap_boundary_platform(body: &mut dyn FnMut() -> Option<String>) -> Option<String> {
    let mut body = body;
    crate::platform_runtime::trap_boundary(&mut body)
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
    if crate::runtime::__module_use1_0().is_virtual() {
        return crate::runtime::__module_use1_0().random_bits();
    };
    return crate::runtime::__module_use0_0().secure_bits();
}

pub fn now_nanos() -> i64 {
    if crate::runtime::__module_use1_0().is_virtual() {
        return crate::runtime::__module_use1_0().virtual_now();
    };
    return crate::runtime::__module_use0_0().mono_nanos();
}

/// [platform-type] The host's `Dyn`.
pub use crate::platform_runtime::Dyn;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Dyn>(); };

pub fn erase_platform<T: Send + 'static>(mut v: T) -> crate::runtime::Dyn {
    crate::platform_runtime::erase(v)
}

pub fn unerase_platform<T: Send + 'static>(mut d: crate::runtime::Dyn) -> T {
    crate::platform_runtime::unerase(d)
}

pub fn drop_dyn_platform(mut d: crate::runtime::Dyn) {
    crate::platform_runtime::drop_dyn(d)
}

/// [platform-type] The host's `Body`.
pub use crate::platform_runtime::Body;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Body>(); };

pub fn body_of_platform(f: std::boxed::Box<dyn FnMut(i32, i64, crate::runtime::Dyn) + Send + 'static>) -> crate::runtime::Body {
    crate::platform_runtime::body_of(f)
}

pub fn activate_platform(mut b: crate::runtime::Body, mut kind: i32, mut slot: i64, mut value: crate::runtime::Dyn) -> crate::runtime::Ran {
    crate::platform_runtime::activate(b, kind, slot, value)
}

pub fn drop_body_platform(mut b: crate::runtime::Body) {
    crate::platform_runtime::drop_body(b)
}

pub fn granted_platform(mut addr: i32, mut pool: i32, mut from: i64) {
    crate::platform_runtime::granted(addr, pool, from)
}

pub fn flush_frames_platform() {
    crate::platform_runtime::flush_frames()
}

pub fn exit_process_platform(mut code: i32) {
    crate::platform_runtime::exit_process(code)
}

pub struct Ran {
    pub body: crate::runtime::Body,
    pub fault: Option<String>,
}

impl std::fmt::Debug for Ran {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ran")
            .field("body", &"<opaque>")
            .field("fault", &self.fault)
            .finish()
    }
}

pub fn drop_ran(mut r: crate::runtime::Ran) {
    let mut __destructured_1: crate::runtime::Ran = r;
    let mut body: crate::runtime::Body = __destructured_1.body;
    let mut fault: Option<String> = __destructured_1.fault;
    crate::runtime::drop_body_platform(body);
}

/// [platform-type] The host's `Slot`.
pub use crate::platform_runtime::Slot;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Slot<i32>>(); };

pub fn slot_of_platform<T: Send + 'static>(mut v: T) -> crate::runtime::Slot<T> {
    crate::platform_runtime::slot_of(v)
}

pub fn slot_empty_platform<T: Send + 'static>() -> crate::runtime::Slot<T> {
    crate::platform_runtime::slot_empty()
}

pub fn slot_take_platform<T: Send + 'static>(s: &mut crate::runtime::Slot<T>) -> Option<T> {
    crate::platform_runtime::slot_take(s)
}

pub fn slot_put_platform<T: Send + 'static>(s: &mut crate::runtime::Slot<T>, mut v: T) {
    crate::platform_runtime::slot_put(s, v)
}

pub fn drop_slot_platform<T: Send + 'static>(mut s: crate::runtime::Slot<T>) {
    crate::platform_runtime::drop_slot(s)
}

pub fn here_pool_platform() -> i32 {
    crate::platform_runtime::here_pool()
}

pub fn here_actor_platform() -> i32 {
    crate::platform_runtime::here_actor()
}

pub fn set_here_platform(mut pool: i32, mut actor: i32) {
    crate::platform_runtime::set_here(pool, actor)
}

pub fn main_pool() -> i32 {
    return 0i32;
}

pub fn no_frame() -> i32 {
    return i32::wrapping_neg(1i32);
}

pub fn task_frame() -> i32 {
    return i32::wrapping_neg(2i32);
}

pub struct Delivered {
    pub msg: crate::runtime::Dyn,
    pub from: i64,
}

impl std::fmt::Debug for Delivered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delivered")
            .field("msg", &"<opaque>")
            .field("from", &self.from)
            .finish()
    }
}

pub struct Answered {
    pub slot: i64,
    pub value: crate::runtime::Dyn,
}

impl std::fmt::Debug for Answered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Answered")
            .field("slot", &self.slot)
            .field("value", &"<opaque>")
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

pub fn drop_delivered(mut d: crate::runtime::Delivered) {
    let mut __destructured_1: crate::runtime::Delivered = d;
    let mut msg: crate::runtime::Dyn = __destructured_1.msg;
    let mut from: i64 = __destructured_1.from;
    crate::runtime::drop_dyn_platform(msg);
}

pub fn drop_answered(mut a: crate::runtime::Answered) {
    let mut __destructured_1: crate::runtime::Answered = a;
    let mut slot: i64 = __destructured_1.slot;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    crate::runtime::drop_dyn_platform(value);
}

pub fn drop_entry(mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>) {
    if matches!(e, crate::unions::Union3::U1(_)) {
        let mut d = match e { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        crate::runtime::drop_delivered(d);
    } else if matches!(e, crate::unions::Union3::U2(_)) {
        let mut a = match e { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        crate::runtime::drop_answered(a);
    } else {
        let mut e_1 = match e { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        std::mem::drop(e_1);
    };
}

pub struct ActorRec {
    pub body: crate::runtime::Slot<crate::runtime::Body>,
    pub pool: i32,
    pub bound: i32,
    pub queue: std::collections::VecDeque<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>,
    pub slots: std::collections::VecDeque<i64>,
    pub user_len: i32,
    pub gate: Option<i64>,
    pub running: bool,
    pub dead: bool,
    pub exit_reason: String,
    pub blocked: Vec<crate::runtime::Parker>,
    pub watchers: Vec<crate::runtime::Token>,
    pub owed: i32,
    pub ready: bool,
    pub proxy: bool,
}

impl std::fmt::Debug for ActorRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorRec")
            .field("body", &"<opaque>")
            .field("pool", &self.pool)
            .field("bound", &self.bound)
            .field("queue", &"<opaque>")
            .field("slots", &self.slots)
            .field("user_len", &self.user_len)
            .field("gate", &self.gate)
            .field("running", &self.running)
            .field("dead", &self.dead)
            .field("exit_reason", &self.exit_reason)
            .field("blocked", &"<opaque>")
            .field("watchers", &"<opaque>")
            .field("owed", &self.owed)
            .field("ready", &self.ready)
            .field("proxy", &self.proxy)
            .finish()
    }
}

pub fn drop_actor_rec(mut a: crate::runtime::ActorRec) {
    let mut __destructured_1: crate::runtime::ActorRec = a;
    let mut body: crate::runtime::Slot<crate::runtime::Body> = __destructured_1.body;
    let mut pool: i32 = __destructured_1.pool;
    let mut bound: i32 = __destructured_1.bound;
    let mut queue: std::collections::VecDeque<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>> = __destructured_1.queue;
    let mut slots: std::collections::VecDeque<i64> = __destructured_1.slots;
    let mut user_len: i32 = __destructured_1.user_len;
    let mut gate: Option<i64> = __destructured_1.gate;
    let mut running: bool = __destructured_1.running;
    let mut dead: bool = __destructured_1.dead;
    let mut exit_reason: String = __destructured_1.exit_reason;
    let mut blocked: Vec<crate::runtime::Parker> = __destructured_1.blocked;
    let mut watchers: Vec<crate::runtime::Token> = __destructured_1.watchers;
    let mut owed: i32 = __destructured_1.owed;
    let mut ready: bool = __destructured_1.ready;
    let mut proxy: bool = __destructured_1.proxy;
    crate::runtime::drop_slot_platform::<crate::runtime::Body>(body);
    crate::core_deque::drain::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(queue, &mut |mut e| {
        crate::runtime::drop_entry(e)
    });
    crate::core_list::drain::<crate::runtime::Token>(watchers, &mut |mut t| {
        crate::runtime::drop_token(t)
    });
}

pub struct WaiterRec {
    pub pool: i32,
    pub value: crate::runtime::Slot<crate::runtime::Dyn>,
    pub filled: bool,
    pub parker: Option<crate::runtime::Parker>,
    pub waiting: i32,
    pub waiting_actor: i32,
    pub slot: i64,
}

impl std::fmt::Debug for WaiterRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaiterRec")
            .field("pool", &self.pool)
            .field("value", &"<opaque>")
            .field("filled", &self.filled)
            .field("parker", &"<opaque>")
            .field("waiting", &self.waiting)
            .field("waiting_actor", &self.waiting_actor)
            .field("slot", &self.slot)
            .finish()
    }
}

pub fn drop_waiter_rec(mut w: crate::runtime::WaiterRec) {
    let mut __destructured_1: crate::runtime::WaiterRec = w;
    let mut pool: i32 = __destructured_1.pool;
    let mut value: crate::runtime::Slot<crate::runtime::Dyn> = __destructured_1.value;
    let mut filled: bool = __destructured_1.filled;
    let mut parker: Option<crate::runtime::Parker> = __destructured_1.parker;
    let mut waiting: i32 = __destructured_1.waiting;
    let mut waiting_actor: i32 = __destructured_1.waiting_actor;
    let mut slot: i64 = __destructured_1.slot;
    crate::runtime::drop_slot_platform::<crate::runtime::Dyn>(value);
}

pub struct TaskRun {
    pub body: crate::runtime::Body,
    pub value: crate::runtime::Dyn,
}

impl std::fmt::Debug for TaskRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskRun")
            .field("body", &"<opaque>")
            .field("value", &"<opaque>")
            .finish()
    }
}

pub fn drop_task_run(mut t: crate::runtime::TaskRun) {
    let mut __destructured_1: crate::runtime::TaskRun = t;
    let mut body: crate::runtime::Body = __destructured_1.body;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    crate::runtime::drop_body_platform(body);
    crate::runtime::drop_dyn_platform(value);
}

pub struct PoolRec {
    pub idle: Vec<crate::runtime::Parker>,
    pub tasks: std::collections::VecDeque<crate::runtime::TaskRun>,
    pub ready: std::collections::VecDeque<i32>,
    pub sink: i32,
    pub owed: i32,
    pub dedicated: bool,
    pub retired: bool,
}

impl std::fmt::Debug for PoolRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoolRec")
            .field("idle", &"<opaque>")
            .field("tasks", &"<opaque>")
            .field("ready", &self.ready)
            .field("sink", &self.sink)
            .field("owed", &self.owed)
            .field("dedicated", &self.dedicated)
            .field("retired", &self.retired)
            .finish()
    }
}

pub fn drop_pool_rec(mut p: crate::runtime::PoolRec) {
    let mut __destructured_1: crate::runtime::PoolRec = p;
    let mut idle: Vec<crate::runtime::Parker> = __destructured_1.idle;
    let mut tasks: std::collections::VecDeque<crate::runtime::TaskRun> = __destructured_1.tasks;
    let mut ready: std::collections::VecDeque<i32> = __destructured_1.ready;
    let mut sink: i32 = __destructured_1.sink;
    let mut owed: i32 = __destructured_1.owed;
    let mut dedicated: bool = __destructured_1.dedicated;
    let mut retired: bool = __destructured_1.retired;
    crate::core_deque::drain::<crate::runtime::TaskRun>(tasks, &mut |mut t| {
        crate::runtime::drop_task_run(t)
    });
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
    pub body: crate::runtime::Body,
}

impl std::fmt::Debug for ToTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToTask")
            .field("pool", &self.pool)
            .field("body", &"<opaque>")
            .finish()
    }
}

pub fn drop_to_task(mut t: crate::runtime::ToTask) {
    let mut __destructured_1: crate::runtime::ToTask = t;
    let mut pool: i32 = __destructured_1.pool;
    let mut body: crate::runtime::Body = __destructured_1.body;
    crate::runtime::drop_body_platform(body);
}

pub struct Token {
    pub target: crate::unions::Union3<crate::runtime::ToActor, crate::runtime::ToWaiter, crate::runtime::ToTask>,
    pub slot: i64,
    pub tracked: bool,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("target", &"<opaque>")
            .field("slot", &self.slot)
            .field("tracked", &self.tracked)
            .finish()
    }
}

pub fn drop_token(mut t: crate::runtime::Token) {
    crate::runtime::answer(t, crate::runtime::erase_platform::<i32>(0i32));
}

pub struct WaiterMint {
    pub token: crate::runtime::Token,
    pub wid: i32,
}

impl std::fmt::Debug for WaiterMint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaiterMint")
            .field("token", &"<opaque>")
            .field("wid", &self.wid)
            .finish()
    }
}

pub fn drop_waiter_mint(mut m: crate::runtime::WaiterMint) {
    let mut __destructured_1: crate::runtime::WaiterMint = m;
    let mut token: crate::runtime::Token = __destructured_1.token;
    let mut wid: i32 = __destructured_1.wid;
    crate::runtime::drop_token(token);
}

pub struct RunActor {
    pub addr: i32,
    pub pool: i32,
    pub kind: i32,
    pub slot: i64,
    pub value: crate::runtime::Dyn,
    pub body: crate::runtime::Body,
}

impl std::fmt::Debug for RunActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunActor")
            .field("addr", &self.addr)
            .field("pool", &self.pool)
            .field("kind", &self.kind)
            .field("slot", &self.slot)
            .field("value", &"<opaque>")
            .field("body", &"<opaque>")
            .finish()
    }
}

pub struct RunTask {
    pub pool: i32,
    pub body: crate::runtime::Body,
    pub value: crate::runtime::Dyn,
}

impl std::fmt::Debug for RunTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunTask")
            .field("pool", &self.pool)
            .field("body", &"<opaque>")
            .field("value", &"<opaque>")
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

pub fn drop_run_actor(mut a: crate::runtime::RunActor) {
    let mut __destructured_1: crate::runtime::RunActor = a;
    let mut addr: i32 = __destructured_1.addr;
    let mut pool: i32 = __destructured_1.pool;
    let mut kind: i32 = __destructured_1.kind;
    let mut slot: i64 = __destructured_1.slot;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    let mut body: crate::runtime::Body = __destructured_1.body;
    crate::runtime::drop_dyn_platform(value);
    crate::runtime::drop_body_platform(body);
}

pub fn drop_run_task(mut t: crate::runtime::RunTask) {
    let mut __destructured_1: crate::runtime::RunTask = t;
    let mut pool: i32 = __destructured_1.pool;
    let mut body: crate::runtime::Body = __destructured_1.body;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    crate::runtime::drop_body_platform(body);
    crate::runtime::drop_dyn_platform(value);
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
    pub msg: crate::runtime::Dyn,
}

impl std::fmt::Debug for Full {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Full")
            .field("msg", &"<opaque>")
            .finish()
    }
}

pub struct Remote {
    pub msg: crate::runtime::Dyn,
}

impl std::fmt::Debug for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Remote")
            .field("msg", &"<opaque>")
            .finish()
    }
}

pub fn drop_remote(mut r: crate::runtime::Remote) {
    let mut __destructured_1: crate::runtime::Remote = r;
    let mut msg: crate::runtime::Dyn = __destructured_1.msg;
    crate::runtime::drop_dyn_platform(msg);
}

pub fn drop_full(mut f: crate::runtime::Full) {
    let mut __destructured_1: crate::runtime::Full = f;
    let mut msg: crate::runtime::Dyn = __destructured_1.msg;
    crate::runtime::drop_dyn_platform(msg);
}

pub struct IdleHook {
    pub pool: i32,
    pub token: crate::runtime::Token,
}

impl std::fmt::Debug for IdleHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdleHook")
            .field("pool", &self.pool)
            .field("token", &"<opaque>")
            .finish()
    }
}

pub fn drop_idle_hook(mut h: crate::runtime::IdleHook) {
    let mut __destructured_1: crate::runtime::IdleHook = h;
    let mut pool: i32 = __destructured_1.pool;
    let mut token: crate::runtime::Token = __destructured_1.token;
    crate::runtime::drop_token(token);
}

pub struct Got {
    pub value: crate::runtime::Dyn,
}

impl std::fmt::Debug for Got {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Got")
            .field("value", &"<opaque>")
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

pub fn drop_got(mut g: crate::runtime::Got) {
    let mut __destructured_1: crate::runtime::Got = g;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    crate::runtime::drop_dyn_platform(value);
}

pub trait __Stateless_SchedTable: Send + Sync {
    fn new_pool(&self, sink: i32, dedicated: bool) -> i32;
    fn new_actor(&self, pool: i32, bound: i32, body: crate::runtime::Body) -> i32;
    fn enqueue(&self, addr: i32, msg: crate::runtime::Dyn, waiter: crate::runtime::Parker) -> crate::unions::Union4<crate::runtime::Sent, crate::runtime::Dead, crate::runtime::Full, crate::runtime::Remote>;
    fn set_proxy(&self, addr: i32);
    fn enqueue_remote(&self, addr: i32, msg: crate::runtime::Dyn, from: i64) -> bool;
    fn kill(&self, addr: i32, reason: String);
    fn mint_actor(&self, addr: i32, gated: bool) -> crate::runtime::Token;
    fn mint_task(&self, pool: i32, body: crate::runtime::Body) -> crate::runtime::Token;
    fn mint_waiter(&self, pool: i32) -> crate::runtime::WaiterMint;
    fn deliver(&self, t: crate::runtime::Token, value: crate::runtime::Dyn);
    fn watch_actor(&self, addr: i32, t: crate::runtime::Token);
    fn idle_hook(&self, pool: i32, t: crate::runtime::Token);
    fn next_work(&self, pool: i32, idle: crate::runtime::Parker) -> Option<crate::unions::Union3<crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Retire>>;
    fn retired_workers(&self) -> i32;
    fn waiter_records(&self) -> i32;
    fn wait_step(&self, wid: i32, pool: i32, own: i32, frame: i32, me: crate::runtime::Parker) -> crate::unions::Union6<crate::runtime::Got, crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Sleep, crate::runtime::Again, crate::runtime::Stuck>;
    fn finish(&self, addr: i32, body: crate::runtime::Body, fault: Option<String>);
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
    fn random_unit(&self) -> f64;
    fn clock_hook(&self, t: crate::runtime::Token);
    fn virtual_work(&self, own: i32) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>>;
}

pub trait __Stateful_SchedTable: Send {
    fn new_pool(&mut self, sink: i32, dedicated: bool) -> i32;
    fn new_actor(&mut self, pool: i32, bound: i32, body: crate::runtime::Body) -> i32;
    fn enqueue(&mut self, addr: i32, msg: crate::runtime::Dyn, waiter: crate::runtime::Parker) -> crate::unions::Union4<crate::runtime::Sent, crate::runtime::Dead, crate::runtime::Full, crate::runtime::Remote>;
    fn set_proxy(&mut self, addr: i32);
    fn enqueue_remote(&mut self, addr: i32, msg: crate::runtime::Dyn, from: i64) -> bool;
    fn kill(&mut self, addr: i32, reason: String);
    fn mint_actor(&mut self, addr: i32, gated: bool) -> crate::runtime::Token;
    fn mint_task(&mut self, pool: i32, body: crate::runtime::Body) -> crate::runtime::Token;
    fn mint_waiter(&mut self, pool: i32) -> crate::runtime::WaiterMint;
    fn deliver(&mut self, t: crate::runtime::Token, value: crate::runtime::Dyn);
    fn watch_actor(&mut self, addr: i32, t: crate::runtime::Token);
    fn idle_hook(&mut self, pool: i32, t: crate::runtime::Token);
    fn next_work(&mut self, pool: i32, idle: crate::runtime::Parker) -> Option<crate::unions::Union3<crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Retire>>;
    fn retired_workers(&mut self) -> i32;
    fn waiter_records(&mut self) -> i32;
    fn wait_step(&mut self, wid: i32, pool: i32, own: i32, frame: i32, me: crate::runtime::Parker) -> crate::unions::Union6<crate::runtime::Got, crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Sleep, crate::runtime::Again, crate::runtime::Stuck>;
    fn finish(&mut self, addr: i32, body: crate::runtime::Body, fault: Option<String>);
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
    fn random_unit(&mut self) -> f64;
    fn clock_hook(&mut self, t: crate::runtime::Token);
    fn virtual_work(&mut self, own: i32) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>>;
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
    pub fn new_actor(&self, pool: i32, bound: i32, body: crate::runtime::Body) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_actor(pool, bound, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_actor(pool, bound, body),
        }
    }
    pub fn enqueue(&self, addr: i32, msg: crate::runtime::Dyn, waiter: crate::runtime::Parker) -> crate::unions::Union4<crate::runtime::Sent, crate::runtime::Dead, crate::runtime::Full, crate::runtime::Remote> {
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
    pub fn enqueue_remote(&self, addr: i32, msg: crate::runtime::Dyn, from: i64) -> bool {
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
    pub fn mint_actor(&self, addr: i32, gated: bool) -> crate::runtime::Token {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_actor(addr, gated),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_actor(addr, gated),
        }
    }
    pub fn mint_task(&self, pool: i32, body: crate::runtime::Body) -> crate::runtime::Token {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_task(pool, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_task(pool, body),
        }
    }
    pub fn mint_waiter(&self, pool: i32) -> crate::runtime::WaiterMint {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_waiter(pool),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_waiter(pool),
        }
    }
    pub fn deliver(&self, t: crate::runtime::Token, value: crate::runtime::Dyn) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.deliver(t, value),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().deliver(t, value),
        }
    }
    pub fn watch_actor(&self, addr: i32, t: crate::runtime::Token) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.watch_actor(addr, t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().watch_actor(addr, t),
        }
    }
    pub fn idle_hook(&self, pool: i32, t: crate::runtime::Token) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.idle_hook(pool, t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().idle_hook(pool, t),
        }
    }
    pub fn next_work(&self, pool: i32, idle: crate::runtime::Parker) -> Option<crate::unions::Union3<crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Retire>> {
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
    pub fn waiter_records(&self) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.waiter_records(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().waiter_records(),
        }
    }
    pub fn wait_step(&self, wid: i32, pool: i32, own: i32, frame: i32, me: crate::runtime::Parker) -> crate::unions::Union6<crate::runtime::Got, crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Sleep, crate::runtime::Again, crate::runtime::Stuck> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.wait_step(wid, pool, own, frame, me),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().wait_step(wid, pool, own, frame, me),
        }
    }
    pub fn finish(&self, addr: i32, body: crate::runtime::Body, fault: Option<String>) {
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
    pub fn random_unit(&self) -> f64 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.random_unit(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().random_unit(),
        }
    }
    pub fn clock_hook(&self, t: crate::runtime::Token) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.clock_hook(t),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().clock_hook(t),
        }
    }
    pub fn virtual_work(&self, own: i32) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.virtual_work(own),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().virtual_work(own),
        }
    }
}

pub struct Scheduler {
    actors: Vec<crate::runtime::ActorRec>,
    waiters: Vec<crate::runtime::WaiterRec>,
    free_waiters: Vec<i32>,
    pools: Vec<crate::runtime::PoolRec>,
    idle_hooks: Vec<crate::runtime::IdleHook>,
    next_slot: i64,
    active: i32,
    parked_frames: i32,
    main_waits: i32,
    externals: i32,
    retired: i32,
    virtual_mode: bool,
    vnow: i64,
    rng: i64,
    rng_seeded: bool,
    clock_hooks: Vec<crate::runtime::Token>,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut __s = Self {
            actors: vec![],
            waiters: vec![],
            free_waiters: vec![],
            pools: vec![],
            idle_hooks: vec![],
            next_slot: 0i64,
            active: 0i32,
            parked_frames: 0i32,
            main_waits: 0i32,
            externals: 0i32,
            retired: 0i32,
            virtual_mode: false,
            vnow: 0i64,
            rng: 1i64,
            rng_seeded: false,
            clock_hooks: vec![]
        };
        __s.init();
        __s
    }
    fn init(&mut self) {
        crate::core_list::add_platform::<crate::runtime::PoolRec>(&mut self.pools, crate::runtime::PoolRec { idle: vec![], tasks: crate::core_deque::mut_deque_of::<crate::runtime::TaskRun>(), ready: crate::core_deque::mut_deque_of::<i32>(), sink: i32::wrapping_neg(1i32), owed: 0i32, dedicated: false, retired: false });
    }
}

impl crate::runtime::__Stateful_SchedTable for Scheduler {
    fn new_pool(&mut self, sink: i32, dedicated: bool) -> i32 {
        crate::core_list::add_platform::<crate::runtime::PoolRec>(&mut self.pools, crate::runtime::PoolRec { idle: vec![], tasks: crate::core_deque::mut_deque_of::<crate::runtime::TaskRun>(), ready: crate::core_deque::mut_deque_of::<i32>(), sink: sink, owed: 0i32, dedicated: dedicated, retired: false });
        return i32::wrapping_sub(crate::core_list::size_platform::<crate::runtime::PoolRec>(&self.pools), 1i32);
    }
    fn new_actor(&mut self, pool: i32, bound: i32, body: crate::runtime::Body) -> i32 {
        crate::core_list::add_platform::<crate::runtime::ActorRec>(&mut self.actors, crate::runtime::ActorRec { body: crate::runtime::slot_of_platform::<crate::runtime::Body>(body), pool: pool, bound: bound, queue: crate::core_deque::mut_deque_of::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(), slots: crate::core_deque::mut_deque_of::<i64>(), user_len: 0i32, gate: None, running: false, dead: false, exit_reason: String::from(""), blocked: vec![], watchers: vec![], owed: 0i32, ready: false, proxy: false });
        return i32::wrapping_sub(crate::core_list::size_platform::<crate::runtime::ActorRec>(&self.actors), 1i32);
    }
    fn enqueue(&mut self, addr: i32, msg: crate::runtime::Dyn, waiter: crate::runtime::Parker) -> crate::unions::Union4<crate::runtime::Sent, crate::runtime::Dead, crate::runtime::Full, crate::runtime::Remote> {
        if ((addr < 0i32) || (addr >= crate::core_list::size_platform::<crate::runtime::ActorRec>(&self.actors))) {
            crate::runtime::drop_dyn_platform(msg);
            return crate::unions::Union4::U2(crate::runtime::Dead {});
        };
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:518:17");
        if self.actors[__h1].dead {
            crate::runtime::drop_dyn_platform(msg);
            return crate::unions::Union4::U2(crate::runtime::Dead {});
        };
        if self.actors[__h1].proxy {
            return crate::unions::Union4::U4(crate::runtime::Remote { msg: msg });
        };
        if (self.actors[__h1].user_len >= self.actors[__h1].bound) {
            crate::core_list::add_platform::<crate::runtime::Parker>(&mut self.actors[__h1].blocked, waiter);
            return crate::unions::Union4::U3(crate::runtime::Full { msg: msg });
        };
        let mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported> = crate::unions::Union3::U1(crate::runtime::Delivered { msg: msg, from: ((i32::wrapping_neg(1i32)) as i64) });
        crate::core_deque::add_last_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut self.actors[__h1].queue, e);
        crate::core_deque::add_last_platform::<i64>(&mut self.actors[__h1].slots, ((i32::wrapping_neg(1i32)) as i64));
        self.actors[__h1].user_len = i32::wrapping_add(self.actors[__h1].user_len, 1i32);
        if crate::runtime::mark_ready(&mut self.actors[__h1], &mut self.pools, addr) {
            let mut a_3 = &self.actors[__h1];
            let mut pool: i32 = a_3.pool;
            crate::runtime::wake_pool(&mut self.pools, pool);
        };
        return crate::unions::Union4::U1(crate::runtime::Sent {});
    }
    fn set_proxy(&mut self, addr: i32) {
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:792:17");
        self.actors[__h1].proxy = true;
    }
    fn enqueue_remote(&mut self, addr: i32, msg: crate::runtime::Dyn, from: i64) -> bool {
        if ((addr < 0i32) || (addr >= crate::core_list::size_platform::<crate::runtime::ActorRec>(&self.actors))) {
            crate::runtime::drop_dyn_platform(msg);
            return false;
        };
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:546:17");
        if self.actors[__h1].dead {
            crate::runtime::drop_dyn_platform(msg);
            return false;
        };
        let mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported> = crate::unions::Union3::U1(crate::runtime::Delivered { msg: msg, from: from });
        crate::core_deque::add_last_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut self.actors[__h1].queue, e);
        crate::core_deque::add_last_platform::<i64>(&mut self.actors[__h1].slots, ((i32::wrapping_neg(1i32)) as i64));
        self.actors[__h1].user_len = i32::wrapping_add(self.actors[__h1].user_len, 1i32);
        if crate::runtime::mark_ready(&mut self.actors[__h1], &mut self.pools, addr) {
            let mut a_3 = &self.actors[__h1];
            let mut pool: i32 = a_3.pool;
            crate::runtime::wake_pool(&mut self.pools, pool);
        };
        return true;
    }
    fn kill(&mut self, addr: i32, reason: String) {
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:563:17");
        if self.actors[__h1].dead {
            return;
        };
        self.actors[__h1].dead = true;
        self.actors[__h1].exit_reason = (reason).clone();
        loop {
            let mut __subject_3: Option<crate::runtime::Parker> = crate::core_list::remove_first_platform::<crate::runtime::Parker>(&mut self.actors[__h1].blocked);
            if !(__subject_3.is_some()) {
                break;
            };
            let mut b = __subject_3.as_ref().unwrap();
            crate::runtime::unpark_platform(b);
        }
        let mut watchers: Vec<crate::runtime::Token> = vec![];
        loop {
            let mut __subject_4: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut self.actors[__h1].watchers);
            if !(__subject_4.is_some()) {
                break;
            };
            let mut t = __subject_4.unwrap();
            crate::core_list::add_platform::<crate::runtime::Token>(&mut watchers, t);
        }
        loop {
            let mut __subject_5: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut watchers);
            if !(__subject_5.is_some()) {
                break;
            };
            let mut t = __subject_5.unwrap();
            crate::runtime::deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, crate::runtime::erase_platform::<crate::core_actor::Exit>(crate::core_actor::Exit { reason: (reason).clone() }));
        }
        crate::core_list::drain::<crate::runtime::Token>(watchers, &mut |mut t| {
            crate::runtime::drop_token(t)
        });
    }
    fn mint_actor(&mut self, addr: i32, gated: bool) -> crate::runtime::Token {
        self.next_slot = i64::wrapping_add(self.next_slot, 1i64);
        let mut slot: i64 = self.next_slot;
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:585:17");
        if gated {
            self.actors[__h1].gate = Some(slot);
        };
        self.actors[__h1].owed = i32::wrapping_add(self.actors[__h1].owed, 1i32);
        return crate::runtime::Token { target: crate::unions::Union3::U1(crate::runtime::ToActor { addr: addr }), slot: slot, tracked: true };
    }
    fn mint_task(&mut self, pool: i32, body: crate::runtime::Body) -> crate::runtime::Token {
        self.next_slot = i64::wrapping_add(self.next_slot, 1i64);
        let __h1: usize = crate::core_list::at__loc(&self.pools, pool).expect("salvo: value is absent at runtime:595:17");
        self.pools[__h1].owed = i32::wrapping_add(self.pools[__h1].owed, 1i32);
        return crate::runtime::Token { target: crate::unions::Union3::U3(crate::runtime::ToTask { pool: pool, body: body }), slot: self.next_slot, tracked: true };
    }
    fn mint_waiter(&mut self, pool: i32) -> crate::runtime::WaiterMint {
        let __h1: usize = crate::core_list::at__loc(&self.pools, pool).expect("salvo: value is absent at runtime:601:17");
        self.pools[__h1].owed = i32::wrapping_add(self.pools[__h1].owed, 1i32);
        self.next_slot = i64::wrapping_add(self.next_slot, 1i64);
        let mut wid: i32 = crate::runtime::reuse_waiter(&mut self.waiters, &mut self.free_waiters, pool, self.next_slot);
        let mut t: crate::runtime::Token = crate::runtime::Token { target: crate::unions::Union3::U2(crate::runtime::ToWaiter { wid: wid }), slot: self.next_slot, tracked: true };
        return crate::runtime::WaiterMint { token: t, wid: wid };
    }
    fn deliver(&mut self, t: crate::runtime::Token, value: crate::runtime::Dyn) {
        crate::runtime::deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, value);
    }
    fn watch_actor(&mut self, addr: i32, t: crate::runtime::Token) {
        let mut watch: crate::runtime::Token = crate::runtime::untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:615:17");
        if self.actors[__h1].dead {
            let mut reason: String = (self.actors[__h1].exit_reason).clone();
            crate::runtime::deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, watch, crate::runtime::erase_platform::<crate::core_actor::Exit>(crate::core_actor::Exit { reason: reason.clone() }));
            return;
        };
        crate::core_list::add_platform::<crate::runtime::Token>(&mut self.actors[__h1].watchers, watch);
    }
    fn idle_hook(&mut self, pool: i32, t: crate::runtime::Token) {
        let mut hook: crate::runtime::Token = crate::runtime::untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        crate::core_list::add_platform::<crate::runtime::IdleHook>(&mut self.idle_hooks, crate::runtime::IdleHook { pool: pool, token: hook });
        crate::runtime::wake_all_pools(&mut self.pools);
    }
    fn next_work(&mut self, pool: i32, idle: crate::runtime::Parker) -> Option<crate::unions::Union3<crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Retire>> {
        if {
            let mut __proj_3: &crate::runtime::PoolRec = {
                let mut __nn_1: Option<&crate::runtime::PoolRec> = crate::core_list::get_platform::<crate::runtime::PoolRec>(&self.pools, pool);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime:635:12");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            };
            __proj_3.retired
        } {
            self.retired = i32::wrapping_add(self.retired, 1i32);
            return Some(crate::unions::Union3::U3(crate::runtime::Retire {}));
        };
        let mut w: Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> = crate::runtime::take_work(&mut self.actors, &mut self.pools, pool, i32::wrapping_neg(1i32));
        if matches!(w, Some(crate::unions::Union2::U1(_))) {
            let mut ra = match w { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = i32::wrapping_add(self.active, 1i32);
            return Some(crate::unions::Union3::U1(ra));
        };
        if matches!(w, Some(crate::unions::Union2::U2(_))) {
            let mut rt = match w { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = i32::wrapping_add(self.active, 1i32);
            return Some(crate::unions::Union3::U2(rt));
        };
        let __h1: usize = crate::core_list::at__loc(&self.pools, pool).expect("salvo: value is absent at runtime:648:17");
        crate::core_list::add_platform::<crate::runtime::Parker>(&mut self.pools[__h1].idle, idle);
        if ((self.active == self.parked_frames) && crate::runtime::quiet(&mut self.actors, &mut self.waiters, &mut self.pools, self.externals)) {
            if (!((crate::core_list::size_platform::<crate::runtime::IdleHook>(&self.idle_hooks) == 0i32)) && (self.active == 0i32)) {
                crate::runtime::fire_idle(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.idle_hooks);
            };
            crate::runtime::wake_waiters(&mut self.waiters);
        };
        return None;
    }
    fn retired_workers(&mut self) -> i32 {
        return self.retired;
    }
    fn waiter_records(&mut self) -> i32 {
        return crate::core_list::size_platform::<crate::runtime::WaiterRec>(&self.waiters);
    }
    fn wait_step(&mut self, wid: i32, pool: i32, own: i32, frame: i32, me: crate::runtime::Parker) -> crate::unions::Union6<crate::runtime::Got, crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Sleep, crate::runtime::Again, crate::runtime::Stuck> {
        let __h1: usize = crate::core_list::at__loc(&self.waiters, wid).expect("salvo: value is absent at runtime:669:17");
        if (self.waiters[__h1].waiting == 0i32) {
            self.waiters[__h1].waiting = frame;
            self.waiters[__h1].waiting_actor = own;
            if (frame == 1i32) {
                self.parked_frames = i32::wrapping_add(self.parked_frames, 1i32);
            } else {
                self.main_waits = i32::wrapping_add(self.main_waits, 1i32);
            };
        };
        let mut got: Option<crate::runtime::Dyn> = crate::runtime::slot_take_platform::<crate::runtime::Dyn>(&mut self.waiters[__h1].value);
        if got.is_some() {
            let mut v = got.unwrap();
            self.waiters[__h1].filled = false;
            self.waiters[__h1].parker = None;
            if (self.waiters[__h1].waiting == 1i32) {
                self.parked_frames = i32::wrapping_sub(self.parked_frames, 1i32);
            } else {
                self.main_waits = i32::wrapping_sub(self.main_waits, 1i32);
            };
            self.waiters[__h1].waiting = 0i32;
            self.waiters[__h1].waiting_actor = i32::wrapping_neg(1i32);
            crate::core_list::add_platform::<i32>(&mut self.free_waiters, wid);
            crate::runtime::wake_pool(&mut self.pools, pool);
            return crate::unions::Union6::U1(crate::runtime::Got { value: v });
        };
        let mut work: Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> = crate::runtime::take_for(&mut self.actors, &mut self.pools, pool, own, self.virtual_mode);
        if matches!(work, Some(crate::unions::Union2::U1(_))) {
            let mut ra = match work { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = i32::wrapping_add(self.active, 1i32);
            return crate::unions::Union6::U2(ra);
        };
        if matches!(work, Some(crate::unions::Union2::U2(_))) {
            let mut rt = match work { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = i32::wrapping_add(self.active, 1i32);
            return crate::unions::Union6::U3(rt);
        };
        let mut q: bool = crate::runtime::quiet(&mut self.actors, &mut self.waiters, &mut self.pools, self.externals);
        if ((self.virtual_mode && (crate::core_list::size_platform::<crate::runtime::Token>(&self.clock_hooks) > 0i32)) && q) {
            crate::runtime::fire_clock(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.clock_hooks);
            return crate::unions::Union6::U5(crate::runtime::Again {});
        };
        if ((!((crate::core_list::size_platform::<crate::runtime::IdleHook>(&self.idle_hooks) == 0i32)) && (self.active == 0i32)) && q) {
            crate::runtime::fire_idle(&mut self.actors, &mut self.waiters, &mut self.pools, &mut self.idle_hooks);
            return crate::unions::Union6::U5(crate::runtime::Again {});
        };
        if (self.virtual_mode || (((self.active == self.parked_frames) && (self.main_waits > 0i32)) && q)) {
            return crate::unions::Union6::U6(crate::runtime::Stuck { report: crate::runtime::deadlock_report(&mut self.actors, &mut self.waiters, own) });
        };
        let __h2: usize = crate::core_list::at__loc(&self.waiters, wid).expect("salvo: value is absent at runtime:728:22");
        self.waiters[__h2].parker = Some((me).clone());
        let __h3: usize = crate::core_list::at__loc(&self.pools, pool).expect("salvo: value is absent at runtime:730:17");
        crate::core_list::add_platform::<crate::runtime::Parker>(&mut self.pools[__h3].idle, me);
        return crate::unions::Union6::U4(crate::runtime::Sleep {});
    }
    fn finish(&mut self, addr: i32, body: crate::runtime::Body, fault: Option<String>) {
        self.active = i32::wrapping_sub(self.active, 1i32);
        let __h1: usize = crate::core_list::at__loc(&self.actors, addr).expect("salvo: value is absent at runtime:737:17");
        self.actors[__h1].running = false;
        if fault.is_none() {
            crate::runtime::slot_put_platform::<crate::runtime::Body>(&mut self.actors[__h1].body, body);
            let mut _again: bool = crate::runtime::mark_ready(&mut self.actors[__h1], &mut self.pools, addr);
            return;
        };
        crate::runtime::drop_body_platform(body);
        let mut fault_3 = fault.as_ref().unwrap();
        let mut reason: String = (fault_3).clone();
        self.actors[__h1].dead = true;
        self.actors[__h1].exit_reason = (reason).clone();
        self.actors[__h1].gate = None;
        self.actors[__h1].user_len = 0i32;
        loop {
            if !((crate::core_deque::size_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&self.actors[__h1].queue) > 0i32)) {
                break;
            };
            { let __arg2 = {
                let mut __nn_4: Option<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>> = crate::core_deque::remove_first_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut self.actors[__h1].queue);
                if __nn_4.is_none() {
                    panic!("salvo: value is absent at runtime:753:24");
                } else {
                    let mut __some_5 = (match __nn_4 { Some(crate::unions::Union3::U1(__v)) => crate::unions::Union3::U1(__v), Some(crate::unions::Union3::U2(__v)) => crate::unions::Union3::U2(__v), Some(crate::unions::Union3::U3(__v)) => crate::unions::Union3::U3(__v), None => unreachable!("salvo: unreachable union arm"), #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
                    __some_5
                }
            }; crate::runtime::drop_entry(__arg2) };
        }
        loop {
            if !((crate::core_deque::size_platform::<i64>(&self.actors[__h1].slots) > 0i32)) {
                break;
            };
            let mut _s: Option<i64> = crate::core_deque::remove_first_platform::<i64>(&mut self.actors[__h1].slots);
        }
        loop {
            let mut __subject_6: Option<crate::runtime::Parker> = crate::core_list::remove_first_platform::<crate::runtime::Parker>(&mut self.actors[__h1].blocked);
            if !(__subject_6.is_some()) {
                break;
            };
            let mut b = __subject_6.as_ref().unwrap();
            crate::runtime::unpark_platform(b);
        }
        let mut pool: i32 = self.actors[__h1].pool;
        let mut watchers: Vec<crate::runtime::Token> = vec![];
        loop {
            let mut __subject_7: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut self.actors[__h1].watchers);
            if !(__subject_7.is_some()) {
                break;
            };
            let mut t = __subject_7.unwrap();
            crate::core_list::add_platform::<crate::runtime::Token>(&mut watchers, t);
        }
        if (crate::core_list::size_platform::<crate::runtime::Token>(&watchers) == 0i32) {
            crate::runtime::report_fault(&mut self.actors, &mut self.pools, pool, (reason).clone());
        };
        loop {
            let mut __subject_8: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut watchers);
            if !(__subject_8.is_some()) {
                break;
            };
            let mut t = __subject_8.unwrap();
            crate::runtime::deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, t, crate::runtime::erase_platform::<crate::core_actor::Exit>(crate::core_actor::Exit { reason: (reason).clone() }));
        }
        crate::core_list::drain::<crate::runtime::Token>(watchers, &mut |mut t| {
            crate::runtime::drop_token(t)
        });
        crate::runtime::retire_if_done(&mut self.actors, &mut self.pools, pool);
        crate::runtime::wake_all_pools(&mut self.pools);
    }
    fn task_done(&mut self, pool: i32, fault: Option<String>) {
        self.active = i32::wrapping_sub(self.active, 1i32);
        if fault.is_some() {
            let mut reason = fault.as_ref().unwrap();
            crate::runtime::report_fault(&mut self.actors, &mut self.pools, pool, (reason).clone());
        };
        crate::runtime::retire_if_done(&mut self.actors, &mut self.pools, pool);
    }
    fn pool_of_actor(&mut self, addr: i32) -> i32 {
        return {
            let mut __proj_3: &crate::runtime::ActorRec = {
                let mut __nn_1: Option<&crate::runtime::ActorRec> = crate::core_list::get_platform::<crate::runtime::ActorRec>(&self.actors, addr);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime:788:21");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            };
            __proj_3.pool
        };
    }
    fn pool_of_waiter(&mut self, wid: i32) -> i32 {
        return {
            let mut __proj_3: &crate::runtime::WaiterRec = {
                let mut __nn_1: Option<&crate::runtime::WaiterRec> = crate::core_list::get_platform::<crate::runtime::WaiterRec>(&self.waiters, wid);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime:797:21");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            };
            __proj_3.pool
        };
    }
    fn external(&mut self, delta: i32) {
        self.externals = i32::wrapping_add(self.externals, delta);
        if (self.externals < 0i32) {
            self.externals = 0i32;
        };
        crate::runtime::wake_all_pools(&mut self.pools);
    }
    fn room(&mut self, addr: i32) -> i32 {
        let mut a: &crate::runtime::ActorRec = {
            let mut __nn_1: Option<&crate::runtime::ActorRec> = crate::core_list::get_platform::<crate::runtime::ActorRec>(&self.actors, addr);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime:801:17");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        };
        return i32::wrapping_sub(a.bound, a.user_len);
    }
    fn is_dead(&mut self, addr: i32) -> bool {
        return {
            let mut __proj_3: &crate::runtime::ActorRec = {
                let mut __nn_1: Option<&crate::runtime::ActorRec> = crate::core_list::get_platform::<crate::runtime::ActorRec>(&self.actors, addr);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime:806:21");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            };
            __proj_3.dead
        };
    }
    fn queued(&mut self, addr: i32) -> i32 {
        return {
            let mut __proj_3: &crate::runtime::ActorRec = {
                let mut __nn_1: Option<&crate::runtime::ActorRec> = crate::core_list::get_platform::<crate::runtime::ActorRec>(&self.actors, addr);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime:810:21");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            };
            __proj_3.user_len
        };
    }
    fn go_virtual(&mut self, seed: i64) {
        self.virtual_mode = true;
        self.vnow = 0i64;
        self.rng = crate::runtime::seed_of(seed);
        crate::runtime::reset_all(&mut self.actors, &mut self.pools, &mut self.idle_hooks, &mut self.clock_hooks);
        self.active = 0i32;
        self.parked_frames = 0i32;
        self.main_waits = 0i32;
        self.externals = 0i32;
    }
    fn is_virtual(&mut self) -> bool {
        return self.virtual_mode;
    }
    fn virtual_now(&mut self) -> i64 {
        return self.vnow;
    }
    fn advance_to(&mut self, at: i64) {
        if (at > self.vnow) {
            self.vnow = at;
        };
    }
    fn random_bits(&mut self) -> i64 {
        let mut hi: i64 = crate::runtime::lehmer(self.rng);
        let mut lo: i64 = crate::runtime::lehmer(hi);
        self.rng = lo;
        return i64::wrapping_add(i64::wrapping_mul(hi, 2147483647i64), lo);
    }
    fn random_unit(&mut self) -> f64 {
        if (!(self.virtual_mode) && !(self.rng_seeded)) {
            self.rng = crate::runtime::seed_of(crate::runtime::__module_use0_0().secure_bits());
            self.rng_seeded = true;
        };
        let mut hi: i64 = crate::runtime::lehmer(self.rng);
        let mut lo: i64 = crate::runtime::lehmer(hi);
        self.rng = lo;
        let mut m: i64 = 2147483646i64;
        return (((i64::wrapping_add(i64::wrapping_mul(i64::wrapping_sub(hi, 1i64), m), i64::wrapping_sub(lo, 1i64))) as f64) / (((m) as f64) * ((m) as f64)));
    }
    fn clock_hook(&mut self, t: crate::runtime::Token) {
        let mut hook: crate::runtime::Token = crate::runtime::untrack(&mut self.actors, &mut self.waiters, &mut self.pools, t);
        crate::core_list::add_platform::<crate::runtime::Token>(&mut self.clock_hooks, hook);
    }
    fn virtual_work(&mut self, own: i32) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> {
        let mut w: Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> = crate::runtime::take_for(&mut self.actors, &mut self.pools, 0i32, own, true);
        if matches!(w, Some(crate::unions::Union2::U1(_))) {
            let mut ra = match w { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
            self.active = i32::wrapping_add(self.active, 1i32);
            return Some(crate::unions::Union2::U1(ra));
        };
        if matches!(w, Some(crate::unions::Union2::U2(_))) {
            let mut rt = match w { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
            self.active = i32::wrapping_add(self.active, 1i32);
            return Some(crate::unions::Union2::U2(rt));
        };
        return None;
    }
}

pub fn lehmer(mut x: i64) -> i64 {
    return i64::wrapping_rem(i64::wrapping_mul(x, 48271i64), 2147483647i64);
}

pub fn seed_of(mut seed: i64) -> i64 {
    let mut s: i64 = i64::wrapping_rem(seed, 2147483646i64);
    if (s < 0i64) {
        s = i64::wrapping_sub(0i64, s);
    };
    return i64::wrapping_add(s, 1i64);
}

pub fn reset_all(actors: &mut Vec<crate::runtime::ActorRec>, pools: &mut Vec<crate::runtime::PoolRec>, idle_hooks: &mut Vec<crate::runtime::IdleHook>, clock_hooks: &mut Vec<crate::runtime::Token>) {
    let mut k: i32 = 0i32;
    loop {
        if !((k < crate::core_list::size_platform::<crate::runtime::ActorRec>(&*actors))) {
            break;
        };
        let __h1: usize = crate::core_list::at__loc(&*actors, k).expect("salvo: value is absent at runtime:909:17");
        k = i32::wrapping_add(k, 1i32);
        actors[__h1].dead = true;
        actors[__h1].running = false;
        actors[__h1].ready = false;
        actors[__h1].gate = None;
        actors[__h1].user_len = 0i32;
        actors[__h1].owed = 0i32;
        loop {
            if !((crate::core_deque::size_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&actors[__h1].queue) > 0i32)) {
                break;
            };
            { let __arg2 = {
                let mut __nn_3: Option<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>> = crate::core_deque::remove_first_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut actors[__h1].queue);
                if __nn_3.is_none() {
                    panic!("salvo: value is absent at runtime:918:24");
                } else {
                    let mut __some_4 = (match __nn_3 { Some(crate::unions::Union3::U1(__v)) => crate::unions::Union3::U1(__v), Some(crate::unions::Union3::U2(__v)) => crate::unions::Union3::U2(__v), Some(crate::unions::Union3::U3(__v)) => crate::unions::Union3::U3(__v), None => unreachable!("salvo: unreachable union arm"), #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
                    __some_4
                }
            }; crate::runtime::drop_entry(__arg2) };
        }
        loop {
            if !((crate::core_deque::remove_first_platform::<i64>(&mut actors[__h1].slots)).is_some()) {
                break;
            };
        }
        loop {
            let mut __subject_5: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut actors[__h1].watchers);
            if !(__subject_5.is_some()) {
                break;
            };
            let mut t = __subject_5.unwrap();
            crate::runtime::drop_token(t);
        }
        loop {
            if !((crate::core_list::remove_first_platform::<crate::runtime::Parker>(&mut actors[__h1].blocked)).is_some()) {
                break;
            };
        }
        let mut __subject_6: Option<crate::runtime::Body> = crate::runtime::slot_take_platform::<crate::runtime::Body>(&mut actors[__h1].body);
        if __subject_6.is_some() {
            let mut b = __subject_6.unwrap();
            crate::runtime::drop_body_platform(b);
        };
    }
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::core_list::size_platform::<crate::runtime::PoolRec>(&*pools))) {
            break;
        };
        let __h3: usize = crate::core_list::at__loc(&*pools, i).expect("salvo: value is absent at runtime:933:17");
        loop {
            let mut __subject_9: Option<crate::runtime::TaskRun> = crate::core_deque::remove_first_platform::<crate::runtime::TaskRun>(&mut pools[__h3].tasks);
            if !(__subject_9.is_some()) {
                break;
            };
            let mut t = __subject_9.unwrap();
            crate::runtime::drop_task_run(t);
        }
        loop {
            if !((crate::core_deque::remove_first_platform::<i32>(&mut pools[__h3].ready)).is_some()) {
                break;
            };
        }
        pools[__h3].owed = 0i32;
        if (i > 0i32) {
            pools[__h3].retired = true;
        };
        i = i32::wrapping_add(i, 1i32);
    }
    loop {
        let mut __subject_10: Option<crate::runtime::IdleHook> = crate::core_list::remove_first_platform::<crate::runtime::IdleHook>(&mut *idle_hooks);
        if !(__subject_10.is_some()) {
            break;
        };
        let mut h = __subject_10.unwrap();
        crate::runtime::drop_idle_hook(h);
    }
    loop {
        let mut __subject_11: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut *clock_hooks);
        if !(__subject_11.is_some()) {
            break;
        };
        let mut t = __subject_11.unwrap();
        crate::runtime::drop_token(t);
    }
}

pub fn fire_clock(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, pools: &mut Vec<crate::runtime::PoolRec>, hooks: &mut Vec<crate::runtime::Token>) {
    let mut __subject_1: Option<crate::runtime::Token> = crate::core_list::remove_first_platform::<crate::runtime::Token>(&mut *hooks);
    if __subject_1.is_some() {
        let mut t = __subject_1.unwrap();
        crate::runtime::deliver_to(&mut *actors, &mut *waiters, &mut *pools, t, crate::runtime::erase_platform::<i32>(0i32));
    };
}

pub fn take_for(actors: &mut Vec<crate::runtime::ActorRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut pool: i32, mut exclude: i32, mut any: bool) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> {
    if !(any) {
        return crate::runtime::take_work(&mut *actors, &mut *pools, pool, exclude);
    };
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::core_list::size_platform::<crate::runtime::PoolRec>(&*pools))) {
            break;
        };
        let mut w: Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> = crate::runtime::take_work(&mut *actors, &mut *pools, i, exclude);
        if matches!(w, Some(crate::unions::Union2::U1(_))) {
            let mut ra = match w { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
            return Some(crate::unions::Union2::U1(ra));
        };
        if matches!(w, Some(crate::unions::Union2::U2(_))) {
            let mut rt = match w { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
            return Some(crate::unions::Union2::U2(rt));
        };
        i = i32::wrapping_add(i, 1i32);
    }
    return None;
}

pub fn untrack(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut t: crate::runtime::Token) -> crate::runtime::Token {
    let mut __destructured_1: crate::runtime::Token = t;
    let mut target: crate::unions::Union3<crate::runtime::ToActor, crate::runtime::ToWaiter, crate::runtime::ToTask> = __destructured_1.target;
    let mut slot: i64 = __destructured_1.slot;
    let mut tracked: bool = __destructured_1.tracked;
    if tracked {
        crate::runtime::release(&mut *actors, &mut *waiters, &mut *pools, &target);
    };
    return crate::runtime::Token { target: target, slot: slot, tracked: false };
}

pub fn release(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, pools: &mut Vec<crate::runtime::PoolRec>, target: &crate::unions::Union3<crate::runtime::ToActor, crate::runtime::ToWaiter, crate::runtime::ToTask>) {
    if matches!(target, crate::unions::Union3::U1(_)) {
        let mut to = match &target { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        let __h1: usize = crate::core_list::at__loc(&*actors, to.addr).expect("salvo: value is absent at runtime:997:17");
        if (actors[__h1].owed > 0i32) {
            actors[__h1].owed = i32::wrapping_sub(actors[__h1].owed, 1i32);
        };
    } else if matches!(target, crate::unions::Union3::U2(_)) {
        let mut tw = match &target { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        let mut pool: i32 = {
            let mut __proj_5: &crate::runtime::WaiterRec = {
                let mut __nn_3: Option<&crate::runtime::WaiterRec> = crate::core_list::get_platform::<crate::runtime::WaiterRec>(&*waiters, tw.wid);
                if __nn_3.is_none() {
                    panic!("salvo: value is absent at runtime:1002:25");
                } else {
                    let mut __some_4 = __nn_3.unwrap();
                    __some_4
                }
            };
            __proj_5.pool
        };
        let __h2: usize = crate::core_list::at__loc(&*pools, pool).expect("salvo: value is absent at runtime:1003:17");
        if (pools[__h2].owed > 0i32) {
            pools[__h2].owed = i32::wrapping_sub(pools[__h2].owed, 1i32);
        };
    } else {
        let mut target_8 = match &target { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        let __h3: usize = crate::core_list::at__loc(&*pools, target_8.pool).expect("salvo: value is absent at runtime:1008:17");
        if (pools[__h3].owed > 0i32) {
            pools[__h3].owed = i32::wrapping_sub(pools[__h3].owed, 1i32);
        };
    };
}

pub fn reuse_waiter(waiters: &mut Vec<crate::runtime::WaiterRec>, free: &mut Vec<i32>, mut pool: i32, mut slot: i64) -> i32 {
    let mut __subject_1: Option<i32> = crate::core_list::remove_first_platform::<i32>(&mut *free);
    if __subject_1.is_some() {
        let mut wid = __subject_1.unwrap();
        let __h1: usize = crate::core_list::at__loc(&*waiters, wid).expect("salvo: value is absent at runtime:1021:17");
        waiters[__h1].pool = pool;
        waiters[__h1].filled = false;
        waiters[__h1].parker = None;
        waiters[__h1].waiting = 0i32;
        waiters[__h1].waiting_actor = i32::wrapping_neg(1i32);
        waiters[__h1].slot = slot;
        return wid;
    };
    crate::core_list::add_platform::<crate::runtime::WaiterRec>(&mut *waiters, crate::runtime::WaiterRec { pool: pool, value: crate::runtime::slot_empty_platform::<crate::runtime::Dyn>(), filled: false, parker: None, waiting: 0i32, waiting_actor: i32::wrapping_neg(1i32), slot: slot });
    return i32::wrapping_sub(crate::core_list::size_platform::<crate::runtime::WaiterRec>(&*waiters), 1i32);
}

pub fn deliver_to(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut t: crate::runtime::Token, mut value: crate::runtime::Dyn) {
    let mut __destructured_1: crate::runtime::Token = t;
    let mut target: crate::unions::Union3<crate::runtime::ToActor, crate::runtime::ToWaiter, crate::runtime::ToTask> = __destructured_1.target;
    let mut slot: i64 = __destructured_1.slot;
    let mut tracked: bool = __destructured_1.tracked;
    if tracked {
        crate::runtime::release(&mut *actors, &mut *waiters, &mut *pools, &target);
    };
    if matches!(target, crate::unions::Union3::U1(_)) {
        let mut to = match &target { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        let __h1: usize = crate::core_list::at__loc(&*actors, to.addr).expect("salvo: value is absent at runtime:1044:17");
        if actors[__h1].dead {
            crate::runtime::drop_dyn_platform(value);
            return;
        };
        crate::core_deque::add_last_platform::<i64>(&mut actors[__h1].slots, slot);
        let mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported> = crate::unions::Union3::U2(crate::runtime::Answered { slot: slot, value: value });
        crate::core_deque::add_last_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut actors[__h1].queue, e);
        if crate::runtime::mark_ready(&mut actors[__h1], &mut *pools, to.addr) {
            let mut a_4 = &actors[__h1];
            let mut pool: i32 = a_4.pool;
            crate::runtime::wake_pool(&mut *pools, pool);
        };
    } else if matches!(target, crate::unions::Union3::U2(_)) {
        let mut tw = match &target { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        let __h2: usize = crate::core_list::at__loc(&*waiters, tw.wid).expect("salvo: value is absent at runtime:1057:17");
        if ((waiters[__h2].slot != slot) || waiters[__h2].filled) {
            crate::runtime::drop_dyn_platform(value);
            return;
        };
        crate::runtime::slot_put_platform::<crate::runtime::Dyn>(&mut waiters[__h2].value, value);
        waiters[__h2].filled = true;
        if waiters[__h2].parker.is_some() {
            let mut p = waiters[__h2].parker.as_ref().unwrap();
            crate::runtime::unpark_platform(&(p).clone());
        };
    } else {
        let mut target_7 = match target { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        let mut __destructured_8: crate::runtime::ToTask = target_7;
        let mut pool: i32 = __destructured_8.pool;
        let mut body: crate::runtime::Body = __destructured_8.body;
        let __h3: usize = crate::core_list::at__loc(&*pools, pool).expect("salvo: value is absent at runtime:1069:17");
        crate::core_deque::add_last_platform::<crate::runtime::TaskRun>(&mut pools[__h3].tasks, crate::runtime::TaskRun { body: body, value: value });
        crate::runtime::wake_pool(&mut *pools, pool);
    };
}

pub fn report_fault(actors: &mut Vec<crate::runtime::ActorRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut pool: i32, mut reason: String) {
    let mut sink: i32 = {
        let mut __proj_3: &crate::runtime::PoolRec = {
            let mut __nn_1: Option<&crate::runtime::PoolRec> = crate::core_list::get_platform::<crate::runtime::PoolRec>(&*pools, pool);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime:1080:21");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        };
        __proj_3.sink
    };
    if (sink >= 0i32) {
        let __h1: usize = crate::core_list::at__loc(&*actors, sink).expect("salvo: value is absent at runtime:1082:17");
        if !(actors[__h1].dead) {
            let mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported> = crate::unions::Union3::U3(crate::runtime::Reported { reason: reason.clone() });
            crate::core_deque::add_last_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut actors[__h1].queue, e);
            crate::core_deque::add_last_platform::<i64>(&mut actors[__h1].slots, ((i32::wrapping_neg(1i32)) as i64));
            if crate::runtime::mark_ready(&mut actors[__h1], &mut *pools, sink) {
                let mut s_6 = &actors[__h1];
                let mut sink_pool: i32 = s_6.pool;
                crate::runtime::wake_pool(&mut *pools, sink_pool);
            };
            return;
        };
    };
    crate::runtime::__module_use0_0().report(&format!("salvo: an uncaught fault on pool {}: {}", pool, reason));
}

pub fn quiet(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut externals: i32) -> bool {
    if (externals > 0i32) {
        return false;
    };
    for mut p in (&mut *pools).iter_mut() {
        if (crate::core_deque::size_platform::<crate::runtime::TaskRun>(&p.tasks) > 0i32) {
            return false;
        };
    }
    for mut a in (&mut *actors).iter_mut() {
        if ((!(a.running) && !(a.dead)) && !((crate::runtime::deliverable(&a.slots, &a.gate)).is_none())) {
            return false;
        };
    }
    for mut w in (&mut *waiters).iter_mut() {
        if ((w.waiting > 0i32) && w.filled) {
            return false;
        };
    }
    return true;
}

pub fn fire_idle(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, pools: &mut Vec<crate::runtime::PoolRec>, hooks: &mut Vec<crate::runtime::IdleHook>) {
    loop {
        let mut __subject_1: Option<crate::runtime::IdleHook> = crate::core_list::remove_first_platform::<crate::runtime::IdleHook>(&mut *hooks);
        if !(__subject_1.is_some()) {
            break;
        };
        let mut h = __subject_1.unwrap();
        let mut __destructured_2: crate::runtime::IdleHook = h;
        let mut pool: i32 = __destructured_2.pool;
        let mut token: crate::runtime::Token = __destructured_2.token;
        let mut gates: i32 = 0i32;
        let mut tokens: i32 = {
            let mut __proj_5: &crate::runtime::PoolRec = {
                let mut __nn_3: Option<&crate::runtime::PoolRec> = crate::core_list::get_platform::<crate::runtime::PoolRec>(&*pools, pool);
                if __nn_3.is_none() {
                    panic!("salvo: value is absent at runtime:1132:27");
                } else {
                    let mut __some_4 = __nn_3.unwrap();
                    __some_4
                }
            };
            __proj_5.owed
        };
        for mut a in (&mut *actors).iter_mut() {
            if (a.pool == pool) {
                tokens = i32::wrapping_add(tokens, a.owed);
                if (!(a.gate.is_none()) && !(a.dead)) {
                    gates = i32::wrapping_add(gates, 1i32);
                };
            };
        }
        crate::runtime::deliver_to(&mut *actors, &mut *waiters, &mut *pools, token, crate::runtime::erase_platform::<crate::core_actor::Idle>(crate::core_actor::Idle { parked_gates: gates, parked_tokens: tokens }));
    }
}

pub fn deadlock_report(actors: &mut Vec<crate::runtime::ActorRec>, waiters: &mut Vec<crate::runtime::WaiterRec>, mut own: i32) -> String {
    let mut occupied: Vec<String> = vec![];
    for mut w in (&mut *waiters).iter_mut() {
        if ((w.waiting > 0i32) && (w.waiting_actor >= 0i32)) {
            crate::core_list::add_platform::<String>(&mut occupied, format!("actor {}", w.waiting_actor));
        };
    }
    let mut gated: Vec<String> = vec![];
    let mut i: i32 = 0i32;
    for mut a in (&mut *actors).iter_mut() {
        if (!(a.gate.is_none()) && !(a.dead)) {
            crate::core_list::add_platform::<String>(&mut gated, format!("actor {}", i));
        };
        i = i32::wrapping_add(i, 1i32);
    }
    let mut who: String = if (own >= 0i32) {
        format!("actor {}", own)
    } else {
        String::from("main")
    };
    let mut clauses: Vec<String> = vec![];
    if (crate::core_list::size_platform::<String>(&occupied) > 0i32) {
        crate::core_list::add_platform::<String>(&mut clauses, format!("parked in a wait: {}", crate::core_string::join_platform(&occupied, &String::from(", "))));
    };
    if (crate::core_list::size_platform::<String>(&gated) > 0i32) {
        crate::core_list::add_platform::<String>(&mut clauses, format!("parked gates: {}", crate::core_string::join_platform(&gated, &String::from(", "))));
    };
    let mut detail: String = if (crate::core_list::size_platform::<String>(&clauses) == 0i32) {
        String::from("")
    } else {
        format!(" ({})", crate::core_string::join_platform(&clauses, &String::from("; ")))
    };
    return format!("salvo: deadlock: nothing can run while {} waits{}", who, detail);
}

pub fn take_work(actors: &mut Vec<crate::runtime::ActorRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut pool: i32, mut exclude: i32) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> {
    let __h1: usize = crate::core_list::at__loc(&*pools, pool).expect("salvo: value is absent at runtime:1181:13");
    let mut task: Option<crate::runtime::TaskRun> = crate::core_deque::remove_first_platform::<crate::runtime::TaskRun>(&mut pools[__h1].tasks);
    if task.is_some() {
        let mut t = task.unwrap();
        let mut __destructured_3: crate::runtime::TaskRun = t;
        let mut body: crate::runtime::Body = __destructured_3.body;
        let mut value: crate::runtime::Dyn = __destructured_3.value;
        return Some(crate::unions::Union2::U2(crate::runtime::RunTask { pool: pool, body: body, value: value }));
    };
    loop {
        let mut __subject_4: Option<i32> = crate::core_deque::remove_first_platform::<i32>(&mut pools[__h1].ready);
        if !(__subject_4.is_some()) {
            break;
        };
        let mut i = __subject_4.unwrap();
        let __h2: usize = crate::core_list::at__loc(&*actors, i).expect("salvo: value is absent at runtime:1188:17");
        actors[__h2].ready = false;
        if (((i != exclude) && !(actors[__h2].running)) && !(actors[__h2].dead)) {
            let mut at: Option<i32> = crate::runtime::deliverable(&actors[__h2].slots, &actors[__h2].gate);
            if at.is_some() {
                let mut k = at.unwrap();
                let mut _slot: Option<i64> = crate::core_deque::remove_at_platform::<i64>(&mut actors[__h2].slots, k);
                let mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported> = {
                    let mut __nn_7: Option<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>> = crate::core_deque::remove_at_platform::<crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>>(&mut actors[__h2].queue, k);
                    if __nn_7.is_none() {
                        panic!("salvo: value is absent at runtime:1194:25");
                    } else {
                        let mut __some_8 = (match __nn_7 { Some(crate::unions::Union3::U1(__v)) => crate::unions::Union3::U1(__v), Some(crate::unions::Union3::U2(__v)) => crate::unions::Union3::U2(__v), Some(crate::unions::Union3::U3(__v)) => crate::unions::Union3::U3(__v), None => unreachable!("salvo: unreachable union arm"), #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
                        __some_8
                    }
                };
                actors[__h2].running = true;
                let mut body: crate::runtime::Body = {
                    let mut __nn_9: Option<crate::runtime::Body> = crate::runtime::slot_take_platform::<crate::runtime::Body>(&mut actors[__h2].body);
                    if __nn_9.is_none() {
                        panic!("salvo: value is absent at runtime:1196:28");
                    } else {
                        let mut __some_10 = __nn_9.unwrap();
                        __some_10
                    }
                };
                return crate::runtime::work_of(&mut actors[__h2], i, e, body);
            };
        };
    }
    return None;
}

pub fn mark_ready(a: &mut crate::runtime::ActorRec, pools: &mut Vec<crate::runtime::PoolRec>, mut addr: i32) -> bool {
    if ((a.ready || a.running) || a.dead) {
        return false;
    };
    if (crate::runtime::deliverable(&a.slots, &a.gate)).is_none() {
        return false;
    };
    a.ready = true;
    let __h1: usize = crate::core_list::at__loc(&*pools, a.pool).expect("salvo: value is absent at runtime:1215:13");
    crate::core_deque::add_last_platform::<i32>(&mut pools[__h1].ready, addr);
    return true;
}

pub fn work_of(a: &mut crate::runtime::ActorRec, mut addr: i32, mut e: crate::unions::Union3<crate::runtime::Delivered, crate::runtime::Answered, crate::runtime::Reported>, mut body: crate::runtime::Body) -> Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> {
    if matches!(e, crate::unions::Union3::U1(_)) {
        let mut d = match e { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        a.user_len = i32::wrapping_sub(a.user_len, 1i32);
        let mut woken: Option<crate::runtime::Parker> = crate::core_list::remove_first_platform::<crate::runtime::Parker>(&mut a.blocked);
        if woken.is_some() {
            let mut b = woken.as_ref().unwrap();
            crate::runtime::unpark_platform(b);
        };
        let mut __destructured_1: crate::runtime::Delivered = d;
        let mut msg: crate::runtime::Dyn = __destructured_1.msg;
        let mut from: i64 = __destructured_1.from;
        if (from >= 0i64) {
            crate::runtime::granted_platform(addr, a.pool, from);
        };
        return Some(crate::unions::Union2::U1(crate::runtime::RunActor { addr: addr, pool: a.pool, kind: 0i32, slot: 0i64, value: msg, body: body }));
    };
    if matches!(e, crate::unions::Union3::U3(_)) {
        let mut r = match &e { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        return Some(crate::unions::Union2::U1(crate::runtime::RunActor { addr: addr, pool: a.pool, kind: 2i32, slot: 0i64, value: crate::runtime::erase_platform::<String>((r.reason).clone()), body: body }));
    };
    let mut e_2 = match e { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
    let mut __destructured_3: crate::runtime::Answered = e_2;
    let mut slot: i64 = __destructured_3.slot;
    let mut value: crate::runtime::Dyn = __destructured_3.value;
    let mut opens: bool = false;
    if a.gate.is_some() {
        let mut g = a.gate.unwrap();
        opens = (g == slot);
    };
    if opens {
        a.gate = None;
    };
    return Some(crate::unions::Union2::U1(crate::runtime::RunActor { addr: addr, pool: a.pool, kind: 1i32, slot: slot, value: value, body: body }));
}

pub fn deliverable(slots: &std::collections::VecDeque<i64>, gate: &Option<i64>) -> Option<i32> {
    if (crate::core_deque::size_platform::<i64>(slots) == 0i32) {
        return None;
    };
    if gate.is_none() {
        return Some(0i32);
    };
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::core_deque::size_platform::<i64>(slots))) {
            break;
        };
        let mut gate_3 = gate.unwrap();
        if ({
            let mut __nn_1: Option<i64> = crate::core_deque::get_platform::<i64>(slots, i).copied();
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime:1263:12");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        } == gate_3) {
            return Some(i);
        };
        i = i32::wrapping_add(i, 1i32);
    }
    return None;
}

pub fn wake_pool(pools: &mut Vec<crate::runtime::PoolRec>, mut pool: i32) {
    let __h1: usize = crate::core_list::at__loc(&*pools, pool).expect("salvo: value is absent at runtime:1278:13");
    let mut __subject_3: Option<crate::runtime::Parker> = crate::core_list::remove_first_platform::<crate::runtime::Parker>(&mut pools[__h1].idle);
    if __subject_3.is_some() {
        let mut w = __subject_3.as_ref().unwrap();
        crate::runtime::unpark_platform(w);
    };
}

pub fn wake_every(pools: &mut Vec<crate::runtime::PoolRec>, mut pool: i32) {
    let __h1: usize = crate::core_list::at__loc(&*pools, pool).expect("salvo: value is absent at runtime:1286:13");
    loop {
        let mut __subject_3: Option<crate::runtime::Parker> = crate::core_list::remove_first_platform::<crate::runtime::Parker>(&mut pools[__h1].idle);
        if !(__subject_3.is_some()) {
            break;
        };
        let mut w = __subject_3.as_ref().unwrap();
        crate::runtime::unpark_platform(w);
    }
}

pub fn wake_waiters(waiters: &mut Vec<crate::runtime::WaiterRec>) {
    for mut w in (&mut *waiters).iter_mut() {
        if ((w.waiting > 0i32) && w.parker.is_some()) {
            let mut p = w.parker.as_ref().unwrap();
            crate::runtime::unpark_platform(&(p).clone());
        };
    }
}

pub fn retire_if_done(actors: &mut Vec<crate::runtime::ActorRec>, pools: &mut Vec<crate::runtime::PoolRec>, mut pool: i32) {
    let __h1: usize = crate::core_list::at__loc(&*pools, pool).expect("salvo: value is absent at runtime:1308:13");
    if (((!(pools[__h1].dedicated) || pools[__h1].retired) || (crate::core_deque::size_platform::<crate::runtime::TaskRun>(&pools[__h1].tasks) > 0i32)) || (pools[__h1].owed > 0i32)) {
        return;
    };
    for mut a in (&mut *actors).iter_mut() {
        if ((a.pool == pool) && !(a.dead)) {
            return;
        };
    }
    pools[__h1].retired = true;
    crate::runtime::wake_every(&mut *pools, pool);
}

pub fn wake_all_pools(pools: &mut Vec<crate::runtime::PoolRec>) {
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::core_list::size_platform::<crate::runtime::PoolRec>(&*pools))) {
            break;
        };
        crate::runtime::wake_every(&mut *pools, i);
        i = i32::wrapping_add(i, 1i32);
    }
}

pub fn new_pool_of(mut n: i32, mut sink: i32) -> i32 {
    return crate::runtime::start_pool(n, sink, false);
}

pub fn new_dedicated_pool() -> i32 {
    return crate::runtime::start_pool(1i32, i32::wrapping_neg(1i32), true);
}

pub fn waiter_record_count() -> i32 {
    return crate::runtime::__module_use1_0().waiter_records();
}

pub fn retired_worker_count() -> i32 {
    return crate::runtime::__module_use1_0().retired_workers();
}

pub fn start_pool(mut n: i32, mut sink: i32, mut dedicated: bool) -> i32 {
    let mut id: i32 = crate::runtime::__module_use1_0().new_pool(sink, dedicated);
    if crate::runtime::__module_use1_0().is_virtual() {
        return id;
    };
    let mut i: i32 = 0i32;
    loop {
        if !((i < n)) {
            break;
        };
        crate::runtime::start_thread_platform(std::boxed::Box::new({ let mut id = id.clone(); move || {
            crate::runtime::serve_pool(id);
        } }));
        i = i32::wrapping_add(i, 1i32);
    }
    return id;
}

pub fn spawn_body(mut pool: i32, mut bound: i32, mut body: crate::runtime::Body) -> i32 {
    return crate::runtime::__module_use1_0().new_actor(pool, bound, body);
}

pub fn send_dyn(mut addr: i32, mut msg: crate::runtime::Dyn) {
    let mut back: Option<crate::runtime::Dyn> = crate::runtime::send_or_back(addr, msg);
    if back.is_some() {
        let mut d = back.unwrap();
        crate::runtime::drop_dyn_platform(d);
    };
}

pub fn send_or_back(mut addr: i32, mut msg: crate::runtime::Dyn) -> Option<crate::runtime::Dyn> {
    let mut r: crate::unions::Union4<crate::runtime::Sent, crate::runtime::Dead, crate::runtime::Full, crate::runtime::Remote> = crate::runtime::__module_use1_0().enqueue(addr, msg, crate::runtime::this_parker_platform());
    loop {
        if !(matches!(r, crate::unions::Union4::U3(_))) {
            break;
        };
        let mut full = match r { crate::unions::Union4::U3(__v) => __v, _ => unreachable!() };
        if crate::runtime::__module_use1_0().is_virtual() {
            crate::runtime::make_room(addr);
            let mut __destructured_1: crate::runtime::Full = full;
            let mut again: crate::runtime::Dyn = __destructured_1.msg;
            r = crate::runtime::__module_use1_0().enqueue(addr, again, crate::runtime::this_parker_platform());
            continue;
        };
        if (((crate::runtime::here_pool_platform() == crate::runtime::main_pool()) && (crate::runtime::here_actor_platform() == crate::runtime::no_frame())) && (crate::runtime::__module_use1_0().pool_of_actor(addr) == crate::runtime::main_pool())) {
            crate::runtime::__module_use0_0().report(&format!("salvo: deadlock: the main pool's actor {} has a full mailbox and the only thread that could drain it is the one sending: the main pool has one worker, `main` itself, and it serves work only inside a `waitfor` — send fewer messages before waiting, raise the handler's `mailbox` capacity, or place the actor on a pool of its own", addr));
            crate::runtime::exit_process_platform(1i32);
        };
        crate::runtime::park_platform(&crate::runtime::this_parker_platform());
        let mut __destructured_2: crate::runtime::Full = full;
        let mut back: crate::runtime::Dyn = __destructured_2.msg;
        r = crate::runtime::__module_use1_0().enqueue(addr, back, crate::runtime::this_parker_platform());
    }
    if matches!(r, crate::unions::Union4::U3(_)) {
        let mut full = match r { crate::unions::Union4::U3(__v) => __v, _ => unreachable!() };
        crate::runtime::drop_full(full);
        return None;
    };
    if matches!(r, crate::unions::Union4::U4(_)) {
        let mut remote = match r { crate::unions::Union4::U4(__v) => __v, _ => unreachable!() };
        let mut __destructured_3: crate::runtime::Remote = remote;
        let mut back: crate::runtime::Dyn = __destructured_3.msg;
        return Some(back);
    };
    return None;
}

pub fn make_room(mut addr: i32) {
    let mut w: Option<crate::unions::Union2<crate::runtime::RunActor, crate::runtime::RunTask>> = crate::runtime::__module_use1_0().virtual_work(crate::runtime::here_actor_platform());
    if matches!(w, Some(crate::unions::Union2::U1(_))) {
        let mut ra = match w { Some(crate::unions::Union2::U1(__v)) => __v, _ => unreachable!() };
        crate::runtime::run_actor(ra);
        return;
    };
    if matches!(w, Some(crate::unions::Union2::U2(_))) {
        let mut rt = match w { Some(crate::unions::Union2::U2(__v)) => __v, _ => unreachable!() };
        crate::runtime::run_task(rt);
        return;
    };
    crate::runtime::__module_use0_0().report(&format!("salvo: deadlock: actor {} has a full mailbox and nothing can run to drain it: an actor test runs every pool on one thread", addr));
    crate::runtime::exit_process_platform(1i32);
}

pub fn mark_proxy(mut addr: i32) {
    crate::runtime::__module_use1_0().set_proxy(addr);
}

pub fn mint(mut addr: i32, mut gated: bool) -> crate::runtime::Token {
    return crate::runtime::__module_use1_0().mint_actor(addr, gated);
}

pub fn mint_task_on(mut pool: i32, mut body: crate::runtime::Body) -> crate::runtime::Token {
    return crate::runtime::__module_use1_0().mint_task(pool, body);
}

pub fn waiter() -> crate::runtime::WaiterMint {
    return crate::runtime::__module_use1_0().mint_waiter(crate::runtime::here_pool_platform());
}

pub fn answer(mut t: crate::runtime::Token, mut value: crate::runtime::Dyn) {
    crate::runtime::__module_use1_0().deliver(t, value);
}

pub fn watch(mut addr: i32, mut t: crate::runtime::Token) {
    crate::runtime::__module_use1_0().watch_actor(addr, t);
}

pub fn on_idle(mut pool: i32, mut t: crate::runtime::Token) {
    crate::runtime::__module_use1_0().idle_hook(pool, t);
}

pub fn external_begin() {
    if crate::runtime::__module_use1_0().is_virtual() {
        crate::runtime::__module_use0_0().report(&String::from("salvo: an actor test opened a host thread (a platform handler that reads or listens on a thread of its own): the virtual runtime runs everything on one thread, so its work would not be deterministic — use an in-memory fake (`MemTransport`, `MemFs`)"));
        crate::runtime::exit_process_platform(1i32);
    };
    crate::runtime::__module_use1_0().external(1i32);
}

pub fn enter_virtual(mut seed: i64) {
    crate::runtime::__module_use1_0().go_virtual(seed);
}

pub fn random_double() -> f64 {
    return crate::runtime::__module_use1_0().random_unit();
}

pub fn virtual_runtime() -> bool {
    return crate::runtime::__module_use1_0().is_virtual();
}

pub fn set_virtual_now(mut at: i64) {
    crate::runtime::__module_use1_0().advance_to(at);
}

pub fn on_clock(mut t: crate::runtime::Token) {
    crate::runtime::__module_use1_0().clock_hook(t);
}

pub fn external_end() {
    crate::runtime::__module_use1_0().external(i32::wrapping_neg(1i32));
}

pub fn await_answer(mut wid: i32) -> crate::runtime::Dyn {
    let mut pool: i32 = crate::runtime::here_pool_platform();
    let mut own: i32 = crate::runtime::here_actor_platform();
    let mut frame: i32 = if (own == crate::runtime::no_frame()) {
        2i32
    } else {
        1i32
    };
    loop {
        if !(true) {
            break;
        };
        let mut step: crate::unions::Union6<crate::runtime::Got, crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Sleep, crate::runtime::Again, crate::runtime::Stuck> = crate::runtime::__module_use1_0().wait_step(wid, pool, own, frame, crate::runtime::this_parker_platform());
        if matches!(step, crate::unions::Union6::U1(_)) {
            let mut g = match step { crate::unions::Union6::U1(__v) => __v, _ => unreachable!() };
            let mut __destructured_1: crate::runtime::Got = g;
            let mut value: crate::runtime::Dyn = __destructured_1.value;
            return value;
        };
        if matches!(step, crate::unions::Union6::U2(_)) {
            let mut ra = match step { crate::unions::Union6::U2(__v) => __v, _ => unreachable!() };
            crate::runtime::run_actor(ra);
        } else if matches!(step, crate::unions::Union6::U3(_)) {
            let mut rt = match step { crate::unions::Union6::U3(__v) => __v, _ => unreachable!() };
            crate::runtime::run_task(rt);
        } else if matches!(step, crate::unions::Union6::U6(_)) {
            let mut s = match &step { crate::unions::Union6::U6(__v) => __v, _ => unreachable!() };
            crate::runtime::__module_use0_0().report(&(s.report).clone());
            crate::runtime::exit_process_platform(1i32);
        } else if matches!(step, crate::unions::Union6::U4(_)) {
            let mut step_2 = match &step { crate::unions::Union6::U4(__v) => __v, _ => unreachable!() };
            crate::runtime::park_platform(&crate::runtime::this_parker_platform());
        };
    }
    return crate::runtime::unerase_platform::<crate::runtime::Dyn>(crate::runtime::erase_platform::<i32>(0i32));
}

pub fn run_actor(mut ra: crate::runtime::RunActor) {
    crate::runtime::flush_frames_platform();
    let mut __destructured_1: crate::runtime::RunActor = ra;
    let mut addr: i32 = __destructured_1.addr;
    let mut pool: i32 = __destructured_1.pool;
    let mut kind: i32 = __destructured_1.kind;
    let mut slot: i64 = __destructured_1.slot;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    let mut body: crate::runtime::Body = __destructured_1.body;
    let mut saved_pool: i32 = crate::runtime::here_pool_platform();
    let mut saved_actor: i32 = crate::runtime::here_actor_platform();
    crate::runtime::set_here_platform(pool, addr);
    let mut ran: crate::runtime::Ran = crate::runtime::activate_platform(body, kind, slot, value);
    crate::runtime::set_here_platform(saved_pool, saved_actor);
    let mut __destructured_2: crate::runtime::Ran = ran;
    let mut back: crate::runtime::Body = __destructured_2.body;
    let mut fault: Option<String> = __destructured_2.fault;
    crate::runtime::__module_use1_0().finish(addr, back, fault);
}

pub fn run_task(mut rt: crate::runtime::RunTask) {
    let mut __destructured_1: crate::runtime::RunTask = rt;
    let mut pool: i32 = __destructured_1.pool;
    let mut body: crate::runtime::Body = __destructured_1.body;
    let mut value: crate::runtime::Dyn = __destructured_1.value;
    let mut saved_pool: i32 = crate::runtime::here_pool_platform();
    let mut saved_actor: i32 = crate::runtime::here_actor_platform();
    crate::runtime::set_here_platform(pool, crate::runtime::task_frame());
    let mut ran: crate::runtime::Ran = crate::runtime::activate_platform(body, 1i32, 0i64, value);
    crate::runtime::set_here_platform(saved_pool, saved_actor);
    let mut __destructured_2: crate::runtime::Ran = ran;
    let mut done: crate::runtime::Body = __destructured_2.body;
    let mut fault: Option<String> = __destructured_2.fault;
    crate::runtime::drop_body_platform(done);
    crate::runtime::__module_use1_0().task_done(pool, fault);
}

pub fn serve_pool(mut pool: i32) {
    crate::runtime::set_here_platform(pool, crate::runtime::no_frame());
    loop {
        if !(true) {
            break;
        };
        let mut w: Option<crate::unions::Union3<crate::runtime::RunActor, crate::runtime::RunTask, crate::runtime::Retire>> = crate::runtime::__module_use1_0().next_work(pool, crate::runtime::this_parker_platform());
        if matches!(w, Some(crate::unions::Union3::U1(_))) {
            let mut ra = match w { Some(crate::unions::Union3::U1(__v)) => __v, _ => unreachable!() };
            crate::runtime::run_actor(ra);
        } else if matches!(w, Some(crate::unions::Union3::U2(_))) {
            let mut rt = match w { Some(crate::unions::Union3::U2(__v)) => __v, _ => unreachable!() };
            crate::runtime::run_task(rt);
        } else if matches!(w, Some(crate::unions::Union3::U3(_))) {
            let mut w_1 = match &w { Some(crate::unions::Union3::U3(__v)) => __v, _ => unreachable!() };
            return;
        } else {
            crate::runtime::park_platform(&crate::runtime::this_parker_platform());
        };
    }
}

pub fn token_to_actor(mut addr: i32, mut slot: i64) -> crate::runtime::Token {
    return crate::runtime::Token { target: crate::unions::Union3::U1(crate::runtime::ToActor { addr: addr }), slot: slot, tracked: false };
}

pub fn token_to_waiter(mut wid: i32, mut slot: i64) -> crate::runtime::Token {
    return crate::runtime::Token { target: crate::unions::Union3::U2(crate::runtime::ToWaiter { wid: wid }), slot: slot, tracked: false };
}

pub struct Exported {
    pub kind: i32,
    pub id: i32,
    pub slot: i64,
    pub body: Option<crate::runtime::Body>,
}

impl std::fmt::Debug for Exported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exported")
            .field("kind", &self.kind)
            .field("id", &self.id)
            .field("slot", &self.slot)
            .field("body", &"<opaque>")
            .finish()
    }
}

pub fn export_token(mut t: crate::runtime::Token) -> crate::runtime::Exported {
    let mut __destructured_1: crate::runtime::Token = t;
    let mut target: crate::unions::Union3<crate::runtime::ToActor, crate::runtime::ToWaiter, crate::runtime::ToTask> = __destructured_1.target;
    let mut slot: i64 = __destructured_1.slot;
    let mut tracked: bool = __destructured_1.tracked;
    if matches!(target, crate::unions::Union3::U3(_)) {
        let mut tt = match target { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        let mut __destructured_2: crate::runtime::ToTask = tt;
        let mut pool: i32 = __destructured_2.pool;
        let mut body: crate::runtime::Body = __destructured_2.body;
        return crate::runtime::Exported { kind: 2i32, id: pool, slot: slot, body: Some(body) };
    };
    let mut kind: i32 = 0i32;
    let mut id: i32 = 0i32;
    if matches!(target, crate::unions::Union3::U1(_)) {
        let mut to = match &target { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        id = to.addr;
    } else if matches!(target, crate::unions::Union3::U2(_)) {
        let mut tw = match &target { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        kind = 1i32;
        id = tw.wid;
    };
    return crate::runtime::Exported { kind: kind, id: id, slot: slot, body: None };
}

pub fn drop_exported(mut e: crate::runtime::Exported) {
    let mut __destructured_1: crate::runtime::Exported = e;
    let mut kind: i32 = __destructured_1.kind;
    let mut id: i32 = __destructured_1.id;
    let mut slot: i64 = __destructured_1.slot;
    let mut body: Option<crate::runtime::Body> = __destructured_1.body;
    if body.is_some() {
        let mut b = body.unwrap();
        crate::runtime::drop_body_platform(b);
    };
}

pub fn deliver_remote(mut addr: i32, mut msg: crate::runtime::Dyn, mut from: i64) -> bool {
    return crate::runtime::__module_use1_0().enqueue_remote(addr, msg, from);
}

pub fn kill_actor(mut addr: i32, mut reason: String) {
    crate::runtime::__module_use1_0().kill(addr, reason);
}

pub fn mailbox_room(mut addr: i32) -> i32 {
    return crate::runtime::__module_use1_0().room(addr);
}

pub fn mailbox_queued(mut addr: i32) -> i32 {
    return crate::runtime::__module_use1_0().queued(addr);
}

pub fn current_pool() -> i32 {
    return crate::runtime::here_pool_platform();
}

pub fn mailbox_dead(mut addr: i32) -> bool {
    return crate::runtime::__module_use1_0().is_dead(addr);
}

pub fn identity_bits() -> i64 {
    return crate::runtime::fresh_bits();
}

pub fn actor_pool(mut addr: i32) -> i32 {
    return crate::runtime::__module_use1_0().pool_of_actor(addr);
}

pub fn waiter_pool(mut wid: i32) -> i32 {
    return crate::runtime::__module_use1_0().pool_of_waiter(wid);
}

pub fn spawn_inert() -> i32 {
    return crate::runtime::spawn_body(0i32, 0i32, crate::runtime::body_of_platform(std::boxed::Box::new(move |mut kind: i32, mut slot: i64, mut value: crate::runtime::Dyn| {
        crate::runtime::drop_dyn_platform(value)
    })));
}
