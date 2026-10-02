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


pub struct Delivered {
    pub msg: Dyn,
}

impl std::fmt::Debug for Delivered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delivered")
            .field("msg", &"<fn>")
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

pub fn drop_delivered(d: Delivered) {
    let __destructured2 = d;
    let mut msg = __destructured2.msg;
    drop_dyn_platform(msg);
}

pub fn drop_answered(a: Answered) {
    let __destructured3 = a;
    let mut slot = __destructured3.slot;
    let mut value = __destructured3.value;
    drop_dyn_platform(value);
}

pub fn drop_entry(e: Union2<Delivered, Answered>) {
    if matches!(e, Union2::U1(_)) {
        let mut d = match e { Union2::U1(__v) => __v, _ => unreachable!() };
        drop_delivered(d);
    } else {
        drop_answered((match e { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
}


pub struct ActorRec {
    pub body: Slot<Body>,
    pub pool: i32,
    pub bound: i32,
    pub queue: std::collections::VecDeque<Union2<Delivered, Answered>>,
    pub slots: std::collections::VecDeque<i64>,
    pub user_len: i32,
    pub gate: Option<i64>,
    pub running: bool,
    pub dead: bool,
    pub blocked: Vec<Parker>,
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
            .field("blocked", &"<fn>")
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
    let mut blocked = __destructured4.blocked;
    drop_slot_platform(body);
    queue.into_iter().for_each(|e| drop_entry(e));
}


pub struct WaiterRec {
    pub value: Slot<Dyn>,
    pub parker: Option<Parker>,
}

impl std::fmt::Debug for WaiterRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaiterRec")
            .field("value", &"<fn>")
            .field("parker", &"<fn>")
            .finish()
    }
}

pub fn drop_waiter_rec(w: WaiterRec) {
    let __destructured5 = w;
    let mut value = __destructured5.value;
    let mut parker = __destructured5.parker;
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
}

impl std::fmt::Debug for PoolRec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoolRec")
            .field("idle", &"<fn>")
            .field("tasks", &"<fn>")
            .finish()
    }
}

pub fn drop_pool_rec(p: PoolRec) {
    let __destructured7 = p;
    let mut idle = __destructured7.idle;
    let mut tasks = __destructured7.tasks;
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
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("target", &"<fn>")
            .field("slot", &self.slot)
            .finish()
    }
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
    answer(token, erase_platform(0));
}


pub struct RunActor {
    pub addr: i32,
    pub kind: i32,
    pub slot: i64,
    pub value: Dyn,
    pub body: Body,
}

impl std::fmt::Debug for RunActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunActor")
            .field("addr", &self.addr)
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

pub fn drop_run_actor(a: RunActor) {
    let __destructured10 = a;
    let mut addr = __destructured10.addr;
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

pub fn drop_full(f: Full) {
    let __destructured12 = f;
    let mut msg = __destructured12.msg;
    drop_dyn_platform(msg);
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

pub fn drop_got(g: Got) {
    let __destructured13 = g;
    let mut value = __destructured13.value;
    drop_dyn_platform(value);
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

pub trait __Stateless_SchedTable: Send + Sync {
    fn new_pool(&self) -> i32;
    fn new_actor(&self, pool: i32, bound: i32, body: Body) -> i32;
    fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full>;
    fn mint_actor(&self, addr: i32, gated: bool) -> Token;
    fn mint_task(&self, pool: i32, body: Body) -> Token;
    fn mint_waiter(&self) -> WaiterMint;
    fn deliver(&self, t: Token, value: Dyn);
    fn next_work(&self, pool: i32, exclude: i32, idle: Parker) -> Option<Union2<RunActor, RunTask>>;
    fn wait_step(&self, wid: i32, pool: i32, exclude: i32, me: Parker) -> Union4<Got, RunActor, RunTask, Sleep>;
    fn finish(&self, addr: i32, body: Body, fault: Option<String>);
    fn pool_of_actor(&self, addr: i32) -> i32;
}

pub trait __Stateful_SchedTable: Send {
    fn new_pool(&mut self) -> i32;
    fn new_actor(&mut self, pool: i32, bound: i32, body: Body) -> i32;
    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full>;
    fn mint_actor(&mut self, addr: i32, gated: bool) -> Token;
    fn mint_task(&mut self, pool: i32, body: Body) -> Token;
    fn mint_waiter(&mut self) -> WaiterMint;
    fn deliver(&mut self, t: Token, value: Dyn);
    fn next_work(&mut self, pool: i32, exclude: i32, idle: Parker) -> Option<Union2<RunActor, RunTask>>;
    fn wait_step(&mut self, wid: i32, pool: i32, exclude: i32, me: Parker) -> Union4<Got, RunActor, RunTask, Sleep>;
    fn finish(&mut self, addr: i32, body: Body, fault: Option<String>);
    fn pool_of_actor(&mut self, addr: i32) -> i32;
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
    pub fn new_pool(&self) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_pool(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_pool(),
        }
    }
    pub fn new_actor(&self, pool: i32, bound: i32, body: Body) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.new_actor(pool, bound, body),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().new_actor(pool, bound, body),
        }
    }
    pub fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.enqueue(addr, msg, waiter),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().enqueue(addr, msg, waiter),
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
    pub fn mint_waiter(&self) -> WaiterMint {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.mint_waiter(),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().mint_waiter(),
        }
    }
    pub fn deliver(&self, t: Token, value: Dyn) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.deliver(t, value),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().deliver(t, value),
        }
    }
    pub fn next_work(&self, pool: i32, exclude: i32, idle: Parker) -> Option<Union2<RunActor, RunTask>> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.next_work(pool, exclude, idle),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().next_work(pool, exclude, idle),
        }
    }
    pub fn wait_step(&self, wid: i32, pool: i32, exclude: i32, me: Parker) -> Union4<Got, RunActor, RunTask, Sleep> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.wait_step(wid, pool, exclude, me),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().wait_step(wid, pool, exclude, me),
        }
    }
    pub fn finish(&self, addr: i32, body: Body, fault: Option<String>) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.finish(addr, body, fault),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().finish(addr, body, fault),
        }
    }
    pub fn pool_of_actor(&self, addr: i32) -> i32 {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.pool_of_actor(addr),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().pool_of_actor(addr),
        }
    }
}

pub struct Scheduler {
    actors: Vec<ActorRec>,
    waiters: Vec<WaiterRec>,
    pools: Vec<PoolRec>,
    next_slot: i64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            actors: vec![],
            waiters: vec![],
            pools: vec![],
            next_slot: 0i64,
        }
    }
}

impl crate::runtime::__Stateful_SchedTable for Scheduler {

    fn new_pool(&mut self) -> i32 {
        self.pools.push(PoolRec { idle: vec![], tasks: std::collections::VecDeque::<TaskRun>::new() });
        return (self.pools.len() as i32) - 1;
    }

    fn new_actor(&mut self, pool: i32, bound: i32, body: Body) -> i32 {
        self.actors.push(ActorRec { body: slot_of_platform(body), pool: pool, bound: bound, queue: std::collections::VecDeque::<Union2<Delivered, Answered>>::new(), slots: std::collections::VecDeque::<i64>::new(), user_len: 0, gate: None, running: false, dead: false, blocked: vec![] });
        return (self.actors.len() as i32) - 1;
    }

    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full> {
        if addr < 0 || addr >= (self.actors.len() as i32) {
            drop_dyn_platform(msg);
            return Union3::<Sent, Dead, Full>::U2(Dead {  });
        }
        let __h0 = (addr) as usize;
        self.actors.get(__h0).expect("salvo: value is absent at runtime:346:17");
        if self.actors[__h0].dead {
            drop_dyn_platform(msg);
            return Union3::<Sent, Dead, Full>::U2(Dead {  });
        }
        if self.actors[__h0].user_len >= self.actors[__h0].bound {
            self.actors[__h0].blocked.push(waiter);
            return Union3::<Sent, Dead, Full>::U3(Full { msg: msg });
        }
        let mut e: Union2<Delivered, Answered> = Union2::<Delivered, Answered>::U1(Delivered { msg: msg });
        self.actors[__h0].queue.push_back(e);
        self.actors[__h0].slots.push_back(((-1) as i64));
        self.actors[__h0].user_len = self.actors[__h0].user_len + 1;
        let mut pool = self.actors[__h0].pool;
        wake_pool(&mut self.pools, pool);
        return Union3::<Sent, Dead, Full>::U1(Sent {  });
    }

    fn mint_actor(&mut self, addr: i32, gated: bool) -> Token {
        self.next_slot = self.next_slot + ((1) as i64);
        let mut slot = self.next_slot.clone();
        if gated {
            let __h1 = (addr) as usize;
            self.actors.get(__h1).expect("salvo: value is absent at runtime:368:21");
            self.actors[__h1].gate = Some(slot.clone());
        }
        return Token { target: Union3::<ToActor, ToWaiter, ToTask>::U1(ToActor { addr: addr }), slot: slot };
    }

    fn mint_task(&mut self, pool: i32, body: Body) -> Token {
        self.next_slot = self.next_slot + ((1) as i64);
        return Token { target: Union3::<ToActor, ToWaiter, ToTask>::U3(ToTask { pool: pool, body: body }), slot: self.next_slot.clone() };
    }

    fn mint_waiter(&mut self) -> WaiterMint {
        self.waiters.push(WaiterRec { value: slot_empty_platform(), parker: None });
        let mut wid = (self.waiters.len() as i32) - 1;
        self.next_slot = self.next_slot + ((1) as i64);
        let mut t = Token { target: Union3::<ToActor, ToWaiter, ToTask>::U2(ToWaiter { wid: wid.clone() }), slot: self.next_slot.clone() };
        return WaiterMint { token: t, wid: wid };
    }

    fn deliver(&mut self, t: Token, value: Dyn) {
        let __destructured14 = t;
        let mut target = __destructured14.target;
        let mut slot = __destructured14.slot;
        deliver_to(&mut self.actors, &mut self.waiters, &mut self.pools, target, slot, value);
    }

    fn next_work(&mut self, pool: i32, exclude: i32, idle: Parker) -> Option<Union2<RunActor, RunTask>> {
        let mut w = take_work(&mut self.actors, &mut self.pools, pool.clone(), exclude);
        if w.is_none() {
            let __h2 = (pool) as usize;
            self.pools.get(__h2).expect("salvo: value is absent at runtime:395:21");
            self.pools[__h2].idle.push(idle);
            return None;
        }
        return w;
    }

    fn wait_step(&mut self, wid: i32, pool: i32, exclude: i32, me: Parker) -> Union4<Got, RunActor, RunTask, Sleep> {
        let __h3 = (wid) as usize;
        self.waiters.get(__h3).expect("salvo: value is absent at runtime:404:17");
        let mut got = slot_take_platform(&mut self.waiters[__h3].value);
        if got.is_some() {
            let mut v = got.unwrap();
            self.waiters[__h3].parker = None;
            return Union4::<Got, RunActor, RunTask, Sleep>::U1(Got { value: v });
        }
        let mut work = take_work(&mut self.actors, &mut self.pools, pool.clone(), exclude);
        if matches!(work, Some(Union2::U1(_))) {
            let mut ra = match work { Some(Union2::U1(__v)) => __v, _ => unreachable!() };
            return Union4::<Got, RunActor, RunTask, Sleep>::U2(ra);
        }
        if matches!(work, Some(Union2::U2(_))) {
            let mut rt = match work { Some(Union2::U2(__v)) => __v, _ => unreachable!() };
            return Union4::<Got, RunActor, RunTask, Sleep>::U3(rt);
        }
        self.waiters[__h3].parker = Some(me.clone());
        let __h4 = (pool) as usize;
        self.pools.get(__h4).expect("salvo: value is absent at runtime:418:17");
        self.pools[__h4].idle.push(me);
        return Union4::<Got, RunActor, RunTask, Sleep>::U4(Sleep {  });
    }

    fn finish(&mut self, addr: i32, body: Body, fault: Option<String>) {
        let __h5 = (addr) as usize;
        self.actors.get(__h5).expect("salvo: value is absent at runtime:424:17");
        self.actors[__h5].running = false;
        if fault.is_none() {
            slot_put_platform(&mut self.actors[__h5].body, body);
            if ((self.actors[__h5].queue.len() as i32) > 0) {
                let mut pool = self.actors[__h5].pool;
                wake_pool(&mut self.pools, pool);
            }
            return;
        }
        drop_body_platform(body);
        self.actors[__h5].dead = true;
        self.actors[__h5].gate = None;
        self.actors[__h5].user_len = 0;
        while ((self.actors[__h5].queue.len() as i32) > 0) {
            drop_entry(self.actors[__h5].queue.pop_front().expect("salvo: value is absent at runtime:439:24"));
        }
        while ((self.actors[__h5].slots.len() as i32) > 0) {
            let mut _s = self.actors[__h5].slots.pop_front();
        }
        loop {
            let mut __is1 = self.actors[__h5].blocked.salvo_remove_first();
            if !(__is1.is_some()) {
                break;
            }
            let mut b = __is1.as_ref().unwrap().clone();
            unpark_platform(&b);
        }
    }

    fn pool_of_actor(&mut self, addr: i32) -> i32 {
        return self.actors.get((addr) as i64 as usize).expect("salvo: value is absent at runtime:450:21").pool;
    }
}

impl Scheduler {

    fn init(&mut self) {
        self.pools.push(PoolRec { idle: vec![], tasks: std::collections::VecDeque::<TaskRun>::new() });
    }
}

pub enum __Priv_Scheduler {
    Init,
}

pub fn deliver_to(actors: &mut Vec<ActorRec>, waiters: &mut Vec<WaiterRec>, pools: &mut Vec<PoolRec>, target: Union3<ToActor, ToWaiter, ToTask>, slot: i64, value: Dyn) {
    if matches!(target, Union3::U1(_)) {
        let mut to = target.u1().clone();
        let __h6 = (to.addr) as usize;
        actors.get(__h6).expect("salvo: value is absent at runtime:459:17");
        if actors[__h6].dead {
            drop_dyn_platform(value);
            return;
        }
        actors[__h6].slots.push_back(slot.clone());
        let mut e: Union2<Delivered, Answered> = Union2::<Delivered, Answered>::U2(Answered { slot: slot, value: value });
        actors[__h6].queue.push_back(e);
        let mut pool = actors[__h6].pool;
        wake_pool(pools, pool);
    } else if matches!(target, Union3::U2(_)) {
        let mut tw = target.u2().clone();
        let __h7 = (tw.wid) as usize;
        waiters.get(__h7).expect("salvo: value is absent at runtime:470:17");
        slot_put_platform(&mut waiters[__h7].value, value);
        if waiters[__h7].parker.is_some() {
            let mut p = waiters[__h7].parker.as_ref().unwrap().clone();
            unpark_platform(&(p.clone()));
        }
    } else {
        let __destructured15 = (match target { Union3::U3(__v) => __v, _ => unreachable!() });
        let mut pool = __destructured15.pool;
        let mut body = __destructured15.body;
        let __h8 = (pool) as usize;
        pools.get(__h8).expect("salvo: value is absent at runtime:477:17");
        pools[__h8].tasks.push_back(TaskRun { body: body, value: value });
        wake_pool(pools, pool.clone());
    }
}

pub fn take_work(actors: &mut Vec<ActorRec>, pools: &mut Vec<PoolRec>, pool: i32, exclude: i32) -> Option<Union2<RunActor, RunTask>> {
    let __h9 = (pool) as usize;
    pools.get(__h9).expect("salvo: value is absent at runtime:488:13");
    let mut task = pools[__h9].tasks.pop_front();
    if task.is_some() {
        let mut t = task.unwrap();
        let __destructured16 = t;
        let mut body = __destructured16.body;
        let mut value = __destructured16.value;
        return Some(Union2::<RunActor, RunTask>::U2(RunTask { pool: pool, body: body, value: value }));
    }
    let mut i = 0;
    while i < (actors.len() as i32) {
        let __h10 = (i) as usize;
        actors.get(__h10).expect("salvo: value is absent at runtime:496:17");
        if actors[__h10].pool == pool && i != exclude && !actors[__h10].running && !actors[__h10].dead {
            let mut at = deliverable(&actors[__h10].slots, &actors[__h10].gate);
            if at.is_some() {
                let mut k = at.unwrap();
                let mut _slot = actors[__h10].slots.remove((k.clone()) as i64 as usize);
                let mut e = actors[__h10].queue.remove((k) as i64 as usize).expect("salvo: value is absent at runtime:501:25");
                actors[__h10].running = true;
                let mut body = slot_take_platform(&mut actors[__h10].body).expect("salvo: value is absent at runtime:503:28");
                return work_of(&mut actors[__h10], i.clone(), e, body);
            }
        }
        i = i + 1;
    }
    return None;
}

pub fn work_of(a: &mut ActorRec, addr: i32, e: Union2<Delivered, Answered>, body: Body) -> Option<Union2<RunActor, RunTask>> {
    if matches!(e, Union2::U1(_)) {
        let mut d = match e { Union2::U1(__v) => __v, _ => unreachable!() };
        a.user_len = a.user_len - 1;
        let mut woken = a.blocked.salvo_remove_first();
        if woken.is_some() {
            let mut b = woken.as_ref().unwrap().clone();
            unpark_platform(&b);
        }
        let __destructured17 = d;
        let mut msg = __destructured17.msg;
        return Some(Union2::<RunActor, RunTask>::U1(RunActor { addr: addr, kind: 0, slot: 0i64, value: msg, body: body }));
    }
    let __destructured18 = (match e { Union2::U2(__v) => __v, _ => unreachable!() });
    let mut slot = __destructured18.slot;
    let mut value = __destructured18.value;
    let mut opens = false;
    if a.gate.is_some() {
        let mut g = a.gate.unwrap();
        opens = g == slot;
    }
    if opens {
        a.gate = None;
    }
    return Some(Union2::<RunActor, RunTask>::U1(RunActor { addr: addr, kind: 1, slot: slot, value: value, body: body }));
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
        if *slots.get((i) as i64 as usize).expect("salvo: value is absent at runtime:546:12") == gate.unwrap() {
            return Some(i.clone());
        }
        i = i + 1;
    }
    return None;
}

pub fn wake_pool(pools: &mut Vec<PoolRec>, pool: i32) {
    let __h11 = (pool) as usize;
    pools.get(__h11).expect("salvo: value is absent at runtime:556:13");
    loop {
        let mut __is2 = pools[__h11].idle.salvo_remove_first();
        if !(__is2.is_some()) {
            break;
        }
        let mut w = __is2.as_ref().unwrap().clone();
        unpark_platform(&w);
    }
}

pub fn new_pool_of(n: i32) -> i32 {
    let mut id = __module_use_1().new_pool();
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
    let mut r = __module_use_1().enqueue(addr.clone(), msg, this_parker_platform());
    while matches!(r, Union3::U3(_)) {
        let mut full = match r { Union3::U3(__v) => __v, _ => unreachable!() };
        park_platform(&(this_parker_platform()));
        let __destructured19 = full;
        let mut back = __destructured19.msg;
        r = __module_use_1().enqueue(addr.clone(), back, this_parker_platform());
    }
    if matches!(r, Union3::U3(_)) {
        let mut full = match r { Union3::U3(__v) => __v, _ => unreachable!() };
        drop_full(full);
    }
}

pub fn mint(addr: i32, gated: bool) -> Token {
    return __module_use_1().mint_actor(addr, gated);
}

pub fn mint_task_on(pool: i32, body: Body) -> Token {
    return __module_use_1().mint_task(pool, body);
}

pub fn waiter() -> WaiterMint {
    return __module_use_1().mint_waiter();
}

pub fn answer(t: Token, value: Dyn) {
    __module_use_1().deliver(t, value);
}

pub fn await_answer(wid: i32) -> Dyn {
    let mut pool = here_pool_platform();
    let mut own = here_actor_platform();
    loop {
        let mut step = __module_use_1().wait_step(wid.clone(), pool.clone(), own.clone(), this_parker_platform());
        if matches!(step, Union4::U1(_)) {
            let mut g = match step { Union4::U1(__v) => __v, _ => unreachable!() };
            let __destructured20 = g;
            let mut value = __destructured20.value;
            return value;
        }
        if matches!(step, Union4::U2(_)) {
            let mut ra = match step { Union4::U2(__v) => __v, _ => unreachable!() };
            run_actor(ra);
        } else if matches!(step, Union4::U3(_)) {
            let mut rt = match step { Union4::U3(__v) => __v, _ => unreachable!() };
            run_task(rt);
        } else if matches!(step, Union4::U4(_)) {
            park_platform(&(this_parker_platform()));
        }
    }
    return unerase_platform(erase_platform(0));
}

pub fn run_actor(ra: RunActor) {
    let __destructured21 = ra;
    let mut addr = __destructured21.addr;
    let mut kind = __destructured21.kind;
    let mut slot = __destructured21.slot;
    let mut value = __destructured21.value;
    let mut body = __destructured21.body;
    let mut saved_pool = here_pool_platform();
    let mut saved_actor = here_actor_platform();
    let mut pool = actor_pool(addr.clone());
    set_here_platform(pool, addr.clone());
    let mut ran = activate_platform(body, kind, slot, value);
    set_here_platform(saved_pool, saved_actor);
    let __destructured22 = ran;
    let mut back = __destructured22.body;
    let mut fault = __destructured22.fault;
    __module_use_1().finish(addr, back, fault);
}

pub fn run_task(rt: RunTask) {
    let __destructured23 = rt;
    let mut pool = __destructured23.pool;
    let mut body = __destructured23.body;
    let mut value = __destructured23.value;
    let mut saved_pool = here_pool_platform();
    let mut saved_actor = here_actor_platform();
    set_here_platform(pool, -1);
    let mut ran = activate_platform(body, 1, 0i64, value);
    set_here_platform(saved_pool, saved_actor);
    let __destructured24 = ran;
    let mut done = __destructured24.body;
    let mut fault = __destructured24.fault;
    drop_body_platform(done);
}

pub fn actor_pool(addr: i32) -> i32 {
    return __module_use_1().pool_of_actor(addr);
}

pub fn serve_pool(pool: i32) {
    set_here_platform(pool.clone(), -1);
    loop {
        let mut w = __module_use_1().next_work(pool.clone(), -1, this_parker_platform());
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
