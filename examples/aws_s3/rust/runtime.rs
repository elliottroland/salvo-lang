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
            let sched_table = crate::runtime::SchedTable::locked(Scheduler::new());
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

pub fn body_of_platform(f: Box<dyn FnMut(Dyn) + Send + 'static>) -> Body {
    crate::platform_runtime::body_of(f)
}

pub fn activate_platform(b: Body, msg: Dyn) -> Ran {
    crate::platform_runtime::activate(b, msg)
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

pub fn drop_body_platform(b: Body) {
    crate::platform_runtime::drop_body(b)
}

/// [platform-type] The host's `Slot`.
pub use crate::platform_runtime::Slot;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<Slot<i32>>(); };

pub fn slot_of_platform<T: Send + 'static>(v: T) -> Slot<T> {
    crate::platform_runtime::slot_of(v)
}

pub fn slot_take_platform<T: Send + 'static>(s: &mut Slot<T>) -> Option<T> {
    crate::platform_runtime::slot_take(s)
}

pub fn slot_put_platform<T: Send + 'static>(s: &mut Slot<T>, v: T) {
    crate::platform_runtime::slot_put(s, v)
}

pub fn drop_slot_platform(s: Slot<Body>) {
    crate::platform_runtime::drop_slot(s)
}


pub struct ActorRec {
    pub body: Slot<Body>,
    pub pool: i32,
    pub bound: i32,
    pub queue: std::collections::VecDeque<Dyn>,
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
            .field("running", &self.running)
            .field("dead", &self.dead)
            .field("blocked", &"<fn>")
            .finish()
    }
}

pub fn drop_actor_rec(a: ActorRec) {
    let __destructured2 = a;
    let mut body = __destructured2.body;
    let mut pool = __destructured2.pool;
    let mut bound = __destructured2.bound;
    let mut queue = __destructured2.queue;
    let mut running = __destructured2.running;
    let mut dead = __destructured2.dead;
    let mut blocked = __destructured2.blocked;
    drop_slot_platform(body);
    queue.into_iter().for_each(|d| drop_dyn_platform(d));
}


pub struct Job {
    pub addr: i32,
    pub msg: Dyn,
    pub body: Body,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("addr", &self.addr)
            .field("msg", &"<fn>")
            .field("body", &"<fn>")
            .finish()
    }
}

pub fn drop_job(j: Job) {
    let __destructured3 = j;
    let mut addr = __destructured3.addr;
    let mut msg = __destructured3.msg;
    let mut body = __destructured3.body;
    drop_dyn_platform(msg);
    drop_body_platform(body);
}

pub trait __Stateless_SchedTable: Send + Sync {
    fn new_pool(&self) -> i32;
    fn new_actor(&self, pool: i32, bound: i32, body: Body) -> i32;
    fn enqueue(&self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full>;
    fn next_job(&self, pool: i32, idle: Parker) -> Option<Job>;
    fn finish(&self, addr: i32, body: Body, fault: Option<String>);
}

pub trait __Stateful_SchedTable: Send {
    fn new_pool(&mut self) -> i32;
    fn new_actor(&mut self, pool: i32, bound: i32, body: Body) -> i32;
    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full>;
    fn next_job(&mut self, pool: i32, idle: Parker) -> Option<Job>;
    fn finish(&mut self, addr: i32, body: Body, fault: Option<String>);
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
    pub fn next_job(&self, pool: i32, idle: Parker) -> Option<Job> {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.next_job(pool, idle),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().next_job(pool, idle),
        }
    }
    pub fn finish(&self, addr: i32, body: Body, fault: Option<String>) {
        match &self.inner {
            __Inner_SchedTable::Shared(h) => h.finish(addr, body, fault),
            __Inner_SchedTable::Locked(h) => h.lock().unwrap().finish(addr, body, fault),
        }
    }
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
    let __destructured4 = f;
    let mut msg = __destructured4.msg;
    drop_dyn_platform(msg);
}

pub struct Scheduler {
    actors: Vec<ActorRec>,
    idle: Vec<Vec<Parker>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            actors: vec![],
            idle: vec![],
        }
    }
}

impl crate::runtime::__Stateful_SchedTable for Scheduler {

    fn new_pool(&mut self) -> i32 {
        self.idle.push(vec![]);
        return (self.idle.len() as i32) - 1;
    }

    fn new_actor(&mut self, pool: i32, bound: i32, body: Body) -> i32 {
        self.actors.push(ActorRec { body: slot_of_platform(body), pool: pool, bound: bound, queue: std::collections::VecDeque::<Dyn>::new(), running: false, dead: false, blocked: vec![] });
        return (self.actors.len() as i32) - 1;
    }

    fn enqueue(&mut self, addr: i32, msg: Dyn, waiter: Parker) -> Union3<Sent, Dead, Full> {
        if addr < 0 || addr >= (self.actors.len() as i32) {
            drop_dyn_platform(msg);
            return Union3::<Sent, Dead, Full>::U2(Dead {  });
        }
        let __h0 = (addr) as usize;
        self.actors.get(__h0).expect("salvo: value is absent at runtime:210:17");
        if self.actors[__h0].dead {
            drop_dyn_platform(msg);
            return Union3::<Sent, Dead, Full>::U2(Dead {  });
        }
        if ((self.actors[__h0].queue.len() as i32) >= self.actors[__h0].bound) {
            self.actors[__h0].blocked.push(waiter);
            return Union3::<Sent, Dead, Full>::U3(Full { msg: msg });
        }
        self.actors[__h0].queue.push_back(msg);
        let mut pool = self.actors[__h0].pool;
        wake_all(&mut self.idle, pool);
        return Union3::<Sent, Dead, Full>::U1(Sent {  });
    }

    fn next_job(&mut self, pool: i32, idle_parker: Parker) -> Option<Job> {
        let mut i = 0;
        while i < (self.actors.len() as i32) {
            let __h1 = (i) as usize;
            self.actors.get(__h1).expect("salvo: value is absent at runtime:228:21");
            if self.actors[__h1].pool == pool && !self.actors[__h1].running && !self.actors[__h1].dead && (self.actors[__h1].queue.len() as i32) > 0 {
                let mut msg = self.actors[__h1].queue.pop_front().expect("salvo: value is absent at runtime:230:27");
                self.actors[__h1].running = true;
                let mut woken = self.actors[__h1].blocked.salvo_remove_first();
                if woken.is_some() {
                    unpark_platform(woken.as_ref().unwrap());
                }
                let mut body = slot_take_platform(&mut self.actors[__h1].body).expect("salvo: value is absent at runtime:236:28");
                return Some(Job { addr: i.clone(), msg: msg, body: body });
            }
            i = i + 1;
        }
        let __h2 = (pool) as usize;
        self.idle.get(__h2).expect("salvo: value is absent at runtime:241:18");
        self.idle[__h2].push(idle_parker);
        return None;
    }

    fn finish(&mut self, addr: i32, body: Body, fault: Option<String>) {
        let __h3 = (addr) as usize;
        self.actors.get(__h3).expect("salvo: value is absent at runtime:247:17");
        self.actors[__h3].running = false;
        if fault.is_none() {
            slot_put_platform(&mut self.actors[__h3].body, body);
            if ((self.actors[__h3].queue.len() as i32) > 0) {
                let mut pool = self.actors[__h3].pool;
                wake_all(&mut self.idle, pool);
            }
            return;
        }
        drop_body_platform(body);
        self.actors[__h3].dead = true;
        loop {
            let mut __is1 = self.actors[__h3].queue.pop_front();
            if !(__is1.is_some()) {
                break;
            }
            let mut d = __is1.unwrap();
            drop_dyn_platform(d);
        }
    }
}

pub fn wake_all(idle: &mut Vec<Vec<Parker>>, pool: i32) {
    let __h4 = (pool) as usize;
    idle.get(__h4).expect("salvo: value is absent at runtime:267:14");
    loop {
        let mut __is2 = idle[__h4].salvo_remove_first();
        if !(__is2.is_some()) {
            break;
        }
        let mut p = __is2.as_ref().unwrap().clone();
        unpark_platform(&p);
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
        let __destructured5 = full;
        let mut back = __destructured5.msg;
        r = __module_use_1().enqueue(addr.clone(), back, this_parker_platform());
    }
    if matches!(r, Union3::U3(_)) {
        let mut full = match r { Union3::U3(__v) => __v, _ => unreachable!() };
        drop_full(full);
    }
}

pub fn serve_pool(pool: i32) {
    loop {
        let mut job = __module_use_1().next_job(pool.clone(), this_parker_platform());
        if job.is_some() {
            let mut j = job.unwrap();
            let __destructured6 = j;
            let mut addr = __destructured6.addr;
            let mut msg = __destructured6.msg;
            let mut body = __destructured6.body;
            let mut ran = activate_platform(body, msg);
            let __destructured7 = ran;
            let mut back = __destructured7.body;
            let mut fault = __destructured7.fault;
            __module_use_1().finish(addr, back, fault);
        } else {
            park_platform(&(this_parker_platform()));
        }
    }
}
