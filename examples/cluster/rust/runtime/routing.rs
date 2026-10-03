use crate::collections::*;
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
use crate::unions::*;

/// [mod-use] The module's `use` #0, bound on first use.
fn __module_use_0() -> &'static crate::runtime_routing::RtRouteTable {
    static CELL: std::sync::OnceLock<crate::runtime_routing::RtRouteTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
            let rt_route_table = crate::runtime_routing::RtRouteTable::locked({ let mut __h = RtRoutes::new(); __h.init(); __h });
        rt_route_table
    })
}

pub fn decode_message_platform(addr: i32, proto: &String, payload: &Vec<u8>) -> Option<Dyn> {
    crate::platform_runtime_routing::decode_message(addr, proto, payload)
}

pub fn decode_waiter_answer_platform(wid: i32, payload: &Vec<u8>) -> Option<Dyn> {
    crate::platform_runtime_routing::decode_waiter_answer(wid, payload)
}

pub fn decode_task_answer_platform(key: i64, payload: &Vec<u8>) -> Option<Dyn> {
    crate::platform_runtime_routing::decode_task_answer(key, payload)
}

pub fn raw_answer_platform(payload: Vec<u8>) -> Dyn {
    crate::platform_runtime_routing::raw_answer(payload)
}

pub fn control_message_platform(sink: i32, from: i64, payload: &Vec<u8>) -> Option<Dyn> {
    crate::platform_runtime_routing::control_message(sink, from, payload)
}

pub fn wire_out_platform(from: i64, to: Vec<u8>, frame: Vec<u8>) {
    crate::platform_runtime_routing::wire_out(from, to, frame)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtMsgFrame {
    pub to: i64,
    pub actor: i64,
    pub bits: i64,
    pub from: i64,
    pub proto: String,
    pub payload: Vec<u8>,
}

impl crate::wire::__Wire for RtMsgFrame {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.actor, out);
        crate::wire::__Wire::__enc(&self.bits, out);
        crate::wire::__Wire::__enc(&self.from, out);
        crate::wire::__Wire::__enc(&self.proto, out);
        crate::wire::__Wire::__enc(&self.payload, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            actor: crate::wire::__Wire::__dec(r)?,
            bits: crate::wire::__Wire::__dec(r)?,
            from: crate::wire::__Wire::__dec(r)?,
            proto: crate::wire::__Wire::__dec(r)?,
            payload: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtAnswerFrame {
    pub to: i64,
    pub kind: i32,
    pub id: i64,
    pub slot: i64,
    pub bits: i64,
    pub payload: Vec<u8>,
}

impl crate::wire::__Wire for RtAnswerFrame {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.kind, out);
        crate::wire::__Wire::__enc(&self.id, out);
        crate::wire::__Wire::__enc(&self.slot, out);
        crate::wire::__Wire::__enc(&self.bits, out);
        crate::wire::__Wire::__enc(&self.payload, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            kind: crate::wire::__Wire::__dec(r)?,
            id: crate::wire::__Wire::__dec(r)?,
            slot: crate::wire::__Wire::__dec(r)?,
            bits: crate::wire::__Wire::__dec(r)?,
            payload: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtGrantFrame {
    pub to: i64,
    pub host: i64,
    pub actor: i64,
    pub bits: i64,
    pub n: i32,
}

impl crate::wire::__Wire for RtGrantFrame {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.host, out);
        crate::wire::__Wire::__enc(&self.actor, out);
        crate::wire::__Wire::__enc(&self.bits, out);
        crate::wire::__Wire::__enc(&self.n, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            host: crate::wire::__Wire::__dec(r)?,
            actor: crate::wire::__Wire::__dec(r)?,
            bits: crate::wire::__Wire::__dec(r)?,
            n: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtOpenFrame {
    pub to: i64,
    pub actor: i64,
    pub bits: i64,
    pub from: i64,
}

impl crate::wire::__Wire for RtOpenFrame {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.actor, out);
        crate::wire::__Wire::__enc(&self.bits, out);
        crate::wire::__Wire::__enc(&self.from, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            actor: crate::wire::__Wire::__dec(r)?,
            bits: crate::wire::__Wire::__dec(r)?,
            from: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtControlFrame {
    pub to: i64,
    pub from: i64,
    pub channel: String,
    pub payload: Vec<u8>,
}

impl crate::wire::__Wire for RtControlFrame {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.from, out);
        crate::wire::__Wire::__enc(&self.channel, out);
        crate::wire::__Wire::__enc(&self.payload, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            from: crate::wire::__Wire::__dec(r)?,
            channel: crate::wire::__Wire::__dec(r)?,
            payload: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RtRemoteRef {
    pub node: i64,
    pub actor: i64,
    pub bits: i64,
}

impl crate::wire::__Wire for RtRemoteRef {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.node, out);
        crate::wire::__Wire::__enc(&self.actor, out);
        crate::wire::__Wire::__enc(&self.bits, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            node: crate::wire::__Wire::__dec(r)?,
            actor: crate::wire::__Wire::__dec(r)?,
            bits: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtReplyParts {
    pub node: i64,
    pub kind: i32,
    pub id: i64,
    pub slot: i64,
    pub bits: i64,
}

impl crate::wire::__Wire for RtReplyParts {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.node, out);
        crate::wire::__Wire::__enc(&self.kind, out);
        crate::wire::__Wire::__enc(&self.id, out);
        crate::wire::__Wire::__enc(&self.slot, out);
        crate::wire::__Wire::__enc(&self.bits, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            node: crate::wire::__Wire::__dec(r)?,
            kind: crate::wire::__Wire::__dec(r)?,
            id: crate::wire::__Wire::__dec(r)?,
            slot: crate::wire::__Wire::__dec(r)?,
            bits: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RtControlKey {
    pub node: i64,
    pub channel: String,
}

impl crate::wire::__Wire for RtControlKey {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.node, out);
        crate::wire::__Wire::__enc(&self.channel, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            node: crate::wire::__Wire::__dec(r)?,
            channel: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtParked {
    pub from: i64,
    pub to: i64,
    pub frame: Vec<u8>,
}

impl crate::wire::__Wire for RtParked {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.from, out);
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.frame, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            from: crate::wire::__Wire::__dec(r)?,
            to: crate::wire::__Wire::__dec(r)?,
            frame: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtStaged {
    pub from: i64,
    pub to: Vec<u8>,
    pub frame: Vec<u8>,
}

impl crate::wire::__Wire for RtStaged {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.from, out);
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.frame, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            from: crate::wire::__Wire::__dec(r)?,
            to: crate::wire::__Wire::__dec(r)?,
            frame: crate::wire::__Wire::__dec(r)?,
        })
    }
}


pub struct RtExportedTask {
    pub pool: i32,
    pub body: RtBody,
}

impl std::fmt::Debug for RtExportedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RtExportedTask")
            .field("pool", &self.pool)
            .field("body", &"<fn>")
            .finish()
    }
}

pub fn drop_exported_task(t: RtExportedTask) {
    let __destructured1 = t;
    let mut pool = __destructured1.pool;
    let mut body = __destructured1.body;
    drop_body_platform(body);
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtFound {
    pub idx: i32,
}

impl crate::wire::__Wire for RtFound {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.idx, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            idx: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtMakeProxy {
}

impl crate::wire::__Wire for RtMakeProxy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RtMakeDead {
}

impl crate::wire::__Wire for RtMakeDead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub trait __Stateless_RtRouteTable: Send + Sync {
    fn node_of_pool(&self, pool: i32) -> i64;
    fn adopt_pool(&self, pool: i32, node: i64);
    fn add_node(&self) -> i64;
    fn hosts(&self, node: i64) -> bool;
    fn identity_of(&self, addr: i32, pool: i32) -> RtRemoteRef;
    fn find_import(&self, r: &RtRemoteRef, here: i64) -> Union3<RtFound, RtMakeProxy, RtMakeDead>;
    fn register_proxy(&self, r: RtRemoteRef, idx: i32, here: i64) -> i32;
    fn register_dead(&self, idx: i32) -> i32;
    fn is_proxy(&self, addr: i32) -> bool;
    fn proxy_ref(&self, addr: i32) -> Option<RtRemoteRef>;
    fn take_credit(&self, addr: i32, me: Parker) -> i32;
    fn stage(&self, from: i64, to: i64, frame: Vec<u8>);
    fn grant(&self, addr: i32, pool: i32, from: i64, n: i32);
    fn take_outbox(&self) -> Vec<RtStaged>;
    fn add_route(&self, node: i64, at: Vec<u8>);
    fn set_outbound(&self, node: i64);
    fn has_outbound(&self, node: i64) -> bool;
    fn accepts(&self, to: i64, actor: i64, claimed: i64) -> bool;
    fn received(&self, idx: i32);
    fn credited(&self, r: RtRemoteRef, to: i64, n: i32) -> bool;
    fn held(&self, idx: i32) -> i32;
    fn put_task(&self, key: i64, t: RtExportedTask);
    fn take_task(&self, key: i64) -> Option<RtExportedTask>;
    fn watch_channel(&self, node: i64, channel: String, sink: i32);
    fn channel_sink(&self, node: i64, channel: &String) -> i32;
    fn set_protocols(&self, table: Vec<(String, String)>);
    fn protocols(&self) -> Vec<(String, String)>;
    fn set_peer(&self, node: i64, table: Vec<(String, String)>);
    fn peer_hash(&self, node: i64, protocol: &String) -> Option<String>;
    fn forget_node(&self, node: i64) -> Vec<i32>;
    fn credits_of(&self, addr: i32) -> Option<i32>;
}

pub trait __Stateful_RtRouteTable: Send {
    fn node_of_pool(&mut self, pool: i32) -> i64;
    fn adopt_pool(&mut self, pool: i32, node: i64);
    fn add_node(&mut self) -> i64;
    fn hosts(&mut self, node: i64) -> bool;
    fn identity_of(&mut self, addr: i32, pool: i32) -> RtRemoteRef;
    fn find_import(&mut self, r: &RtRemoteRef, here: i64) -> Union3<RtFound, RtMakeProxy, RtMakeDead>;
    fn register_proxy(&mut self, r: RtRemoteRef, idx: i32, here: i64) -> i32;
    fn register_dead(&mut self, idx: i32) -> i32;
    fn is_proxy(&mut self, addr: i32) -> bool;
    fn proxy_ref(&mut self, addr: i32) -> Option<RtRemoteRef>;
    fn take_credit(&mut self, addr: i32, me: Parker) -> i32;
    fn stage(&mut self, from: i64, to: i64, frame: Vec<u8>);
    fn grant(&mut self, addr: i32, pool: i32, from: i64, n: i32);
    fn take_outbox(&mut self) -> Vec<RtStaged>;
    fn add_route(&mut self, node: i64, at: Vec<u8>);
    fn set_outbound(&mut self, node: i64);
    fn has_outbound(&mut self, node: i64) -> bool;
    fn accepts(&mut self, to: i64, actor: i64, claimed: i64) -> bool;
    fn received(&mut self, idx: i32);
    fn credited(&mut self, r: RtRemoteRef, to: i64, n: i32) -> bool;
    fn held(&mut self, idx: i32) -> i32;
    fn put_task(&mut self, key: i64, t: RtExportedTask);
    fn take_task(&mut self, key: i64) -> Option<RtExportedTask>;
    fn watch_channel(&mut self, node: i64, channel: String, sink: i32);
    fn channel_sink(&mut self, node: i64, channel: &String) -> i32;
    fn set_protocols(&mut self, table: Vec<(String, String)>);
    fn protocols(&mut self) -> Vec<(String, String)>;
    fn set_peer(&mut self, node: i64, table: Vec<(String, String)>);
    fn peer_hash(&mut self, node: i64, protocol: &String) -> Option<String>;
    fn forget_node(&mut self, node: i64) -> Vec<i32>;
    fn credits_of(&mut self, addr: i32) -> Option<i32>;
}

pub struct RtRouteTable {
    inner: __Inner_RtRouteTable,
}

pub enum __Inner_RtRouteTable {
    Shared(std::sync::Arc<dyn __Stateless_RtRouteTable>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_RtRouteTable>>),
}

impl Clone for RtRouteTable {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_RtRouteTable::Shared(h) => __Inner_RtRouteTable::Shared(h.clone()),
            __Inner_RtRouteTable::Locked(h) => __Inner_RtRouteTable::Locked(h.clone()),
        } }
    }
}

impl RtRouteTable {
    pub fn shared<__H: __Stateless_RtRouteTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RtRouteTable::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_RtRouteTable>) -> Self {
        Self { inner: __Inner_RtRouteTable::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_RtRouteTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RtRouteTable::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_RtRouteTable>>) -> Self {
        Self { inner: __Inner_RtRouteTable::Locked(inner) }
    }
    pub fn node_of_pool(&self, pool: i32) -> i64 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.node_of_pool(pool),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().node_of_pool(pool),
        }
    }
    pub fn adopt_pool(&self, pool: i32, node: i64) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.adopt_pool(pool, node),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().adopt_pool(pool, node),
        }
    }
    pub fn add_node(&self) -> i64 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.add_node(),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().add_node(),
        }
    }
    pub fn hosts(&self, node: i64) -> bool {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.hosts(node),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().hosts(node),
        }
    }
    pub fn identity_of(&self, addr: i32, pool: i32) -> RtRemoteRef {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.identity_of(addr, pool),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().identity_of(addr, pool),
        }
    }
    pub fn find_import(&self, r: &RtRemoteRef, here: i64) -> Union3<RtFound, RtMakeProxy, RtMakeDead> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.find_import(r, here),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().find_import(r, here),
        }
    }
    pub fn register_proxy(&self, r: RtRemoteRef, idx: i32, here: i64) -> i32 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.register_proxy(r, idx, here),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().register_proxy(r, idx, here),
        }
    }
    pub fn register_dead(&self, idx: i32) -> i32 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.register_dead(idx),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().register_dead(idx),
        }
    }
    pub fn is_proxy(&self, addr: i32) -> bool {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.is_proxy(addr),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().is_proxy(addr),
        }
    }
    pub fn proxy_ref(&self, addr: i32) -> Option<RtRemoteRef> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.proxy_ref(addr),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().proxy_ref(addr),
        }
    }
    pub fn take_credit(&self, addr: i32, me: Parker) -> i32 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.take_credit(addr, me),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().take_credit(addr, me),
        }
    }
    pub fn stage(&self, from: i64, to: i64, frame: Vec<u8>) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.stage(from, to, frame),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().stage(from, to, frame),
        }
    }
    pub fn grant(&self, addr: i32, pool: i32, from: i64, n: i32) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.grant(addr, pool, from, n),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().grant(addr, pool, from, n),
        }
    }
    pub fn take_outbox(&self) -> Vec<RtStaged> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.take_outbox(),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().take_outbox(),
        }
    }
    pub fn add_route(&self, node: i64, at: Vec<u8>) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.add_route(node, at),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().add_route(node, at),
        }
    }
    pub fn set_outbound(&self, node: i64) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.set_outbound(node),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().set_outbound(node),
        }
    }
    pub fn has_outbound(&self, node: i64) -> bool {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.has_outbound(node),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().has_outbound(node),
        }
    }
    pub fn accepts(&self, to: i64, actor: i64, claimed: i64) -> bool {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.accepts(to, actor, claimed),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().accepts(to, actor, claimed),
        }
    }
    pub fn received(&self, idx: i32) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.received(idx),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().received(idx),
        }
    }
    pub fn credited(&self, r: RtRemoteRef, to: i64, n: i32) -> bool {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.credited(r, to, n),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().credited(r, to, n),
        }
    }
    pub fn held(&self, idx: i32) -> i32 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.held(idx),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().held(idx),
        }
    }
    pub fn put_task(&self, key: i64, t: RtExportedTask) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.put_task(key, t),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().put_task(key, t),
        }
    }
    pub fn take_task(&self, key: i64) -> Option<RtExportedTask> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.take_task(key),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().take_task(key),
        }
    }
    pub fn watch_channel(&self, node: i64, channel: String, sink: i32) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.watch_channel(node, channel, sink),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().watch_channel(node, channel, sink),
        }
    }
    pub fn channel_sink(&self, node: i64, channel: &String) -> i32 {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.channel_sink(node, channel),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().channel_sink(node, channel),
        }
    }
    pub fn set_protocols(&self, table: Vec<(String, String)>) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.set_protocols(table),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().set_protocols(table),
        }
    }
    pub fn protocols(&self) -> Vec<(String, String)> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.protocols(),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().protocols(),
        }
    }
    pub fn set_peer(&self, node: i64, table: Vec<(String, String)>) {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.set_peer(node, table),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().set_peer(node, table),
        }
    }
    pub fn peer_hash(&self, node: i64, protocol: &String) -> Option<String> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.peer_hash(node, protocol),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().peer_hash(node, protocol),
        }
    }
    pub fn forget_node(&self, node: i64) -> Vec<i32> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.forget_node(node),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().forget_node(node),
        }
    }
    pub fn credits_of(&self, addr: i32) -> Option<i32> {
        match &self.inner {
            __Inner_RtRouteTable::Shared(h) => h.credits_of(addr),
            __Inner_RtRouteTable::Locked(h) => h.lock().unwrap().credits_of(addr),
        }
    }
}

pub struct RtRoutes {
    node_id: i64,
    hosted: SalvoSet<i64>,
    pool_node: SalvoMap<i32, i64>,
    bits: SalvoMap<i32, i64>,
    remote: SalvoMap<i32, RtRemoteRef>,
    proxies: SalvoMap<RtRemoteRef, i32>,
    credits: SalvoMap<i32, i32>,
    held_n: SalvoMap<i32, i32>,
    routes: SalvoMap<i64, Vec<u8>>,
    outbound: SalvoSet<i64>,
    parked: Vec<RtParked>,
    outbox: Vec<RtStaged>,
    task_keys: Vec<i64>,
    tasks: Vec<RtExportedTask>,
    controls: SalvoMap<RtControlKey, i32>,
    local: Vec<(String, String)>,
    peers: SalvoMap<i64, Vec<(String, String)>>,
    dead_entry: i32,
    credit_waiters: Vec<Parker>,
}

impl RtRoutes {
    pub fn new() -> Self {
        Self {
            node_id: 0i64,
            hosted: SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]),
            pool_node: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            bits: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            remote: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            proxies: SalvoMap::from_entries::<__Hash_hash__RtRemoteRef_RtRemoteRef, __Eq_eq__RtRemoteRef_RtRemoteRef, _>(vec![]),
            credits: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            held_n: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            routes: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            outbound: SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]),
            parked: vec![],
            outbox: vec![],
            task_keys: vec![],
            tasks: vec![],
            controls: SalvoMap::from_entries::<__Hash_hash__RtControlKey_RtControlKey, __Eq_eq__RtControlKey_RtControlKey, _>(vec![]),
            local: vec![],
            peers: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            dead_entry: -1,
            credit_waiters: vec![],
        }
    }
}

impl crate::runtime_routing::__Stateful_RtRouteTable for RtRoutes {

    fn node_of_pool(&mut self, pool: i32) -> i64 {
        return node_in(&self.pool_node, self.node_id, pool);
    }

    fn adopt_pool(&mut self, pool: i32, node: i64) {
        self.pool_node.insert(pool, node);
    }

    fn add_node(&mut self) -> i64 {
        let mut n = fresh_node();
        self.hosted.insert(n.clone());
        return n;
    }

    fn hosts(&mut self, node: i64) -> bool {
        return self.hosted.contains(&node);
    }

    fn identity_of(&mut self, addr: i32, pool: i32) -> RtRemoteRef {
        return identity_in(&self.remote, &mut self.bits, &self.pool_node, self.node_id, addr, pool);
    }

    fn find_import(&mut self, r: &RtRemoteRef, here: i64) -> Union3<RtFound, RtMakeProxy, RtMakeDead> {
        if r.node == here {
            let mut idx = ((r.actor) as i32);
            let mut b = self.bits.get(&idx);
            if !self.remote.contains_key(&idx) && (b.is_some()) {
                let mut known = *b.unwrap();
                if known == r.bits {
                    return Union3::<RtFound, RtMakeProxy, RtMakeDead>::U1(RtFound { idx: idx });
                }
            }
            if self.dead_entry >= 0 {
                return Union3::<RtFound, RtMakeProxy, RtMakeDead>::U1(RtFound { idx: self.dead_entry.clone() });
            }
            return Union3::<RtFound, RtMakeProxy, RtMakeDead>::U3(RtMakeDead {  });
        }
        let mut p = self.proxies.get(&r);
        if p.is_some() {
            let mut idx = *p.unwrap();
            return Union3::<RtFound, RtMakeProxy, RtMakeDead>::U1(RtFound { idx: idx.clone() });
        }
        return Union3::<RtFound, RtMakeProxy, RtMakeDead>::U2(RtMakeProxy {  });
    }

    fn register_proxy(&mut self, r: RtRemoteRef, idx: i32, here: i64) -> i32 {
        let mut existing = self.proxies.get(&r);
        if existing.is_some() {
            let mut e = *existing.unwrap();
            return e.clone();
        }
        self.remote.insert(idx.clone(), r.clone());
        self.proxies.insert(r.clone(), idx.clone());
        self.credits.insert(idx.clone(), 0);
        let mut frame = crate::wire::salvo_encode(&Union5::<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>::U4(RtOpenFrame { to: r.node, actor: r.actor, bits: r.bits, from: here.clone() }));
        stage_in(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked, here, r.node, frame);
        return idx;
    }

    fn register_dead(&mut self, idx: i32) -> i32 {
        if self.dead_entry >= 0 {
            return self.dead_entry.clone();
        }
        self.dead_entry = idx;
        return self.dead_entry.clone();
    }

    fn is_proxy(&mut self, addr: i32) -> bool {
        return self.remote.contains_key(&addr);
    }

    fn proxy_ref(&mut self, addr: i32) -> Option<RtRemoteRef> {
        let mut r = self.remote.get(&addr);
        if r.is_some() {
            let mut found = r.unwrap();
            return Some(found.clone());
        }
        return None;
    }

    fn take_credit(&mut self, addr: i32, me: Parker) -> i32 {
        let mut c = self.credits.get(&addr);
        if c.is_some() {
            let mut n = *c.unwrap();
            if n > 0 {
                self.credits.insert(addr.clone(), n - 1);
                self.held_n.insert(addr.clone(), held_in(&self.held_n, addr.clone()) + 1);
                return 1;
            }
            self.credit_waiters.push(me);
            return 0;
        }
        return -1;
    }

    fn stage(&mut self, from: i64, to: i64, frame: Vec<u8>) {
        stage_in(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked, from, to, frame);
    }

    fn grant(&mut self, addr: i32, pool: i32, from: i64, n: i32) {
        self.held_n.insert(addr.clone(), held_in(&self.held_n, addr.clone()) + n);
        let mut me = identity_in(&self.remote, &mut self.bits, &self.pool_node, self.node_id, addr.clone(), pool);
        let mut frame = crate::wire::salvo_encode(&Union5::<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>::U3(RtGrantFrame { to: from.clone(), host: me.node, actor: me.actor, bits: me.bits, n: n }));
        stage_in(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked, me.node, from, frame);
    }

    fn take_outbox(&mut self) -> Vec<RtStaged> {
        let mut out: Vec<RtStaged> = vec![];
        while ((self.outbox.len() as i32) > 0) {
            out.push(self.outbox.salvo_remove_at(0).expect("salvo: value is absent at runtime.routing:281:22"));
        }
        return out;
    }

    fn add_route(&mut self, node: i64, at: Vec<u8>) {
        self.routes.insert(node, at);
        restage(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked);
    }

    fn set_outbound(&mut self, node: i64) {
        self.outbound.insert(node);
        restage(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked);
    }

    fn has_outbound(&mut self, node: i64) -> bool {
        return self.outbound.contains(&node);
    }

    fn accepts(&mut self, to: i64, actor: i64, claimed: i64) -> bool {
        if !self.hosted.contains(&to) {
            return false;
        }
        let mut idx = ((actor) as i32);
        if self.remote.contains_key(&idx) {
            return false;
        }
        let mut b = self.bits.get(&idx);
        if b.is_some() {
            let mut known = *b.unwrap();
            return known == claimed;
        }
        return false;
    }

    fn received(&mut self, idx: i32) {
        let mut h = held_in(&self.held_n, idx.clone());
        if h > 0 {
            self.held_n.insert(idx, h - 1);
        }
    }

    fn credited(&mut self, r: RtRemoteRef, to: i64, n: i32) -> bool {
        if !self.hosted.contains(&to) {
            return false;
        }
        let mut p = self.proxies.get(&r);
        if p.is_some() {
            let mut idx = *p.unwrap();
            let mut i = idx.clone();
            let mut c = self.credits.get(&i);
            let mut now = 0;
            if c.is_some() {
                let mut have = *c.unwrap();
                now = have.clone();
            }
            self.credits.insert(i.clone(), now + n);
            let mut h = held_in(&self.held_n, i.clone()) - n;
            if h < 0 {
                h = 0;
            }
            self.held_n.insert(i, h);
            wake_senders(&mut self.credit_waiters);
            return true;
        }
        return false;
    }

    fn held(&mut self, idx: i32) -> i32 {
        return held_in(&self.held_n, idx);
    }

    fn put_task(&mut self, key: i64, t: RtExportedTask) {
        self.task_keys.push(key);
        self.tasks.push(t);
    }

    fn take_task(&mut self, key: i64) -> Option<RtExportedTask> {
        let mut i = 0;
        while i < (self.task_keys.len() as i32) {
            if *self.task_keys.get((i) as i64 as usize).expect("salvo: value is absent at runtime.routing:358:16") == key {
                let mut _k = self.task_keys.salvo_remove_at(i.clone());
                return self.tasks.salvo_remove_at(i);
            }
            i = i + 1;
        }
        return None;
    }

    fn watch_channel(&mut self, node: i64, channel: String, sink: i32) {
        self.controls.insert(RtControlKey { node: node, channel: channel }, sink);
    }

    fn channel_sink(&mut self, node: i64, channel: &String) -> i32 {
        let mut s = self.controls.get(&RtControlKey { node: node.clone(), channel: channel.clone() });
        if s.is_some() {
            let mut sink = *s.unwrap();
            return sink.clone();
        }
        return -1;
    }

    fn set_protocols(&mut self, table: Vec<(String, String)>) {
        self.local = table.clone();
    }

    fn protocols(&mut self) -> Vec<(String, String)> {
        return self.local.clone();
    }

    fn set_peer(&mut self, node: i64, table: Vec<(String, String)>) {
        self.peers.insert(node, table);
    }

    fn peer_hash(&mut self, node: i64, protocol: &String) -> Option<String> {
        let mut t = self.peers.get(&node);
        if t.is_some() {
            let mut table = t.unwrap();
            for mut entry in table.clone() {
                let (mut name, mut hash) = entry.clone();
                if name == protocol.clone() {
                    return Some(hash.clone());
                }
            }
        }
        return None;
    }

    fn forget_node(&mut self, node: i64) -> Vec<i32> {
        let mut _route = self.routes.remove(&node);
        let mut _peer = self.peers.remove(&node);
        let mut gone: Vec<i32> = vec![];
        for mut idx in self.remote.keys().cloned().collect::<Vec<_>>() {
            let mut r = self.remote.get(&idx);
            if r.is_some() {
                let mut found = r.unwrap();
                if found.node == node {
                    gone.push(idx.clone());
                }
            }
        }
        wake_senders(&mut self.credit_waiters);
        return gone;
    }

    fn credits_of(&mut self, addr: i32) -> Option<i32> {
        let mut c = self.credits.get(&addr);
        if c.is_some() {
            let mut n = *c.unwrap();
            return Some(n.clone());
        }
        return None;
    }
}

impl RtRoutes {

    fn init(&mut self) {
        self.node_id = fresh_node();
        self.hosted.insert(self.node_id.clone());
        self.pool_node.insert(0, self.node_id.clone());
    }
}

pub enum __Priv_RtRoutes {
    Init,
}

pub fn node_in(pool_node: &SalvoMap<i32, i64>, node_id: i64, pool: i32) -> i64 {
    let mut n = pool_node.get(&pool);
    if n.is_some() {
        let mut v = *n.unwrap();
        return v.clone();
    }
    return node_id.clone();
}

pub fn held_in(held_n: &SalvoMap<i32, i32>, idx: i32) -> i32 {
    let mut h = held_n.get(&idx);
    if h.is_some() {
        let mut n = *h.unwrap();
        return n.clone();
    }
    return 0;
}

pub fn identity_in(remote: &SalvoMap<i32, RtRemoteRef>, bits: &mut SalvoMap<i32, i64>, pool_node: &SalvoMap<i32, i64>, node_id: i64, addr: i32, pool: i32) -> RtRemoteRef {
    let mut r = remote.get(&addr);
    if r.is_some() {
        let mut found = r.unwrap();
        return found.clone();
    }
    let mut n = node_in(pool_node, node_id, pool);
    let mut b = bits.get(&addr);
    if b.is_some() {
        let mut known = *b.unwrap();
        return RtRemoteRef { node: n, actor: ((addr) as i64), bits: known.clone() };
    }
    let mut minted = identity_bits();
    bits.insert(addr.clone(), minted.clone());
    return RtRemoteRef { node: n, actor: ((addr) as i64), bits: minted };
}

pub fn wake_senders(waiters: &mut Vec<Parker>) {
    while ((waiters.len() as i32) > 0) {
        unpark_platform(&(waiters.salvo_remove_at(0).expect("salvo: value is absent at runtime.routing:470:16")));
    }
}

pub fn stage_in(routes: &SalvoMap<i64, Vec<u8>>, outbound: &SalvoSet<i64>, outbox: &mut Vec<RtStaged>, parked: &mut Vec<RtParked>, from: i64, to: i64, frame: Vec<u8>) {
    let mut ep = routes.get(&to);
    if ep.is_some() {
        let mut at = ep.unwrap();
        if outbound.contains(&from) {
            outbox.push(RtStaged { from: from, to: at.clone(), frame: frame });
            return;
        }
    }
    parked.push(RtParked { from: from, to: to, frame: frame });
}

pub fn restage(routes: &SalvoMap<i64, Vec<u8>>, outbound: &SalvoSet<i64>, outbox: &mut Vec<RtStaged>, parked: &mut Vec<RtParked>) {
    let mut waiting: Vec<RtParked> = vec![];
    while ((parked.len() as i32) > 0) {
        waiting.push(parked.salvo_remove_at(0).expect("salvo: value is absent at runtime.routing:493:22"));
    }
    for mut p in waiting.clone() {
        stage_in(routes, outbound, outbox, parked, p.from, p.to, p.frame.clone());
    }
}

pub fn fresh_node() -> i64 {
    let mut b = identity_bits();
    if b < ((0) as i64) {
        return -(b + ((1) as i64));
    }
    return b;
}

pub fn here_node() -> i64 {
    return __module_use_0().node_of_pool(current_pool());
}

pub fn adopt(pool: i32) {
    __module_use_0().adopt_pool(pool, here_node());
}

pub fn new_node() -> i64 {
    return __module_use_0().add_node();
}

pub fn pool_at(node: i64, n: i32) -> i32 {
    let mut p = new_pool_of(n, -1);
    __module_use_0().adopt_pool(p.clone(), node);
    return p;
}

pub fn identity(addr: i32) -> RtRemoteRef {
    return __module_use_0().identity_of(addr.clone(), actor_pool(addr.clone()));
}

pub fn same_actor(a: i32, b: i32) -> bool {
    if a == b {
        return true;
    }
    if !__module_use_0().is_proxy(a.clone()) && !__module_use_0().is_proxy(b.clone()) {
        return false;
    }
    return eq__4(&(identity(a)), &(identity(b)));
}

pub fn import_addr(node: i64, actor: i64, bits: i64) -> i32 {
    let mut r = RtRemoteRef { node: node, actor: actor, bits: bits };
    let mut here = here_node();
    let mut found = __module_use_0().find_import(&(r.clone()), here.clone());
    if matches!(found, Union3::U1(_)) {
        let mut f = found.u1().clone();
        return f.idx;
    }
    let mut idx = spawn_inert();
    if matches!(found, Union3::U3(_)) {
        kill_actor(idx.clone(), "unknown identity".to_string());
        return __module_use_0().register_dead(idx);
    }
    let mut got = __module_use_0().register_proxy(r, idx.clone(), here);
    if got == idx {
        mark_proxy(idx);
    }
    flush();
    return got;
}

pub fn remote(addr: i32) -> bool {
    return __module_use_0().is_proxy(addr);
}

pub fn send_remote(addr: i32, proto: String, payload: Vec<u8>) {
    let mut r = __module_use_0().proxy_ref(addr.clone());
    if r.is_none() {
        return;
    }
    loop {
        let mut got = __module_use_0().take_credit(addr.clone(), this_parker_platform());
        if got == 1 {
            let mut from = here_node();
            let mut frame = crate::wire::salvo_encode(&Union5::<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>::U1(RtMsgFrame { to: r.as_ref().unwrap().clone().node, actor: r.as_ref().unwrap().clone().actor, bits: r.as_ref().unwrap().clone().bits, from: from.clone(), proto: proto, payload: payload }));
            __module_use_0().stage(from, r.as_ref().unwrap().clone().node, frame);
            flush();
            return;
        }
        if got < 0 || mailbox_dead(addr.clone()) {
            return;
        }
        park_platform(&(this_parker_platform()));
        if mailbox_dead(addr.clone()) {
            return;
        }
    }
}

pub fn answer_remote(t: RtReplyParts, payload: Vec<u8>) {
    let mut from = here_node();
    let mut frame = crate::wire::salvo_encode(&Union5::<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>::U2(RtAnswerFrame { to: t.node, kind: t.kind, id: t.id, slot: t.slot, bits: t.bits, payload: payload }));
    __module_use_0().stage(from, t.node, frame);
    flush();
}

pub fn export_reply(e: RtExported) -> RtReplyParts {
    let __destructured2 = e;
    let mut kind = __destructured2.kind;
    let mut id = __destructured2.id;
    let mut slot = __destructured2.slot;
    let mut body = __destructured2.body;
    if kind == 2 {
        if body.is_some() {
            let mut b = body.unwrap();
            __module_use_0().put_task(slot.clone(), RtExportedTask { pool: id.clone(), body: b });
        }
        return RtReplyParts { node: __module_use_0().node_of_pool(id.clone()), kind: 2, id: slot.clone(), slot: slot, bits: 0i64 };
    }
    if body.is_some() {
        let mut b = body.unwrap();
        drop_body_platform(b);
    }
    if kind == 1 {
        return RtReplyParts { node: __module_use_0().node_of_pool(waiter_pool(id.clone())), kind: 1, id: ((id) as i64), slot: slot, bits: 0i64 };
    }
    let mut me = identity(id);
    return RtReplyParts { node: me.node, kind: 0, id: me.actor, slot: slot, bits: me.bits };
}

pub fn credit_back(addr: i32, pool: i32, from: i64) {
    __module_use_0().grant(addr, pool, from, 1);
}

pub fn flush() {
    let mut out = __module_use_0().take_outbox();
    for s in &out {
        wire_out_platform(s.from, s.to.clone(), s.frame.clone());
    }
}

pub fn route(node: i64, at: Vec<u8>) {
    __module_use_0().add_route(node, at);
    flush();
}

pub fn outbound_bound() {
    __module_use_0().set_outbound(here_node());
    flush();
}

pub fn connected() -> bool {
    return __module_use_0().has_outbound(here_node());
}

pub fn credits(addr: i32) -> Option<i32> {
    return __module_use_0().credits_of(addr);
}

pub fn pending(addr: i32) -> i32 {
    if __module_use_0().is_proxy(addr.clone()) {
        return __module_use_0().held(addr);
    }
    return mailbox_queued(addr);
}

pub fn watch_control(channel: String, sink: i32) {
    __module_use_0().watch_channel(here_node(), channel, sink);
}

pub fn send_control(to: i64, channel: String, payload: Vec<u8>) {
    let mut from = here_node();
    let mut frame = crate::wire::salvo_encode(&Union5::<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>::U5(RtControlFrame { to: to.clone(), from: from.clone(), channel: channel, payload: payload }));
    __module_use_0().stage(from, to, frame);
    flush();
}

pub fn control_frame(channel: String, payload: Vec<u8>) -> Vec<u8> {
    return crate::wire::salvo_encode(&Union5::<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>::U5(RtControlFrame { to: 0i64, from: here_node(), channel: channel, payload: payload }));
}

pub fn node_left(node: i64) {
    for mut idx in __module_use_0().forget_node(node) {
        kill_actor(idx.clone(), "node left".to_string());
    }
}

pub fn register_protocols(table: Vec<(String, String)>) {
    __module_use_0().set_protocols(table);
}

pub fn local_protocols() -> Vec<(String, String)> {
    return __module_use_0().protocols();
}

pub fn set_peer_protocols(node: i64, table: Vec<(String, String)>) {
    __module_use_0().set_peer(node, table);
}

pub fn peer_protocol(node: i64, protocol: &String) -> Option<String> {
    return __module_use_0().peer_hash(node, protocol);
}

pub fn deliver(data: &Vec<u8>) -> bool {
    let mut f = crate::wire::salvo_decode::<Union5<RtMsgFrame, RtAnswerFrame, RtGrantFrame, RtOpenFrame, RtControlFrame>>(&data.clone());
    if matches!(f, Some(Union5::U1(_))) {
        let mut m = f.as_ref().unwrap().u1().clone();
        if !__module_use_0().accepts(m.to, m.actor, m.bits) {
            return false;
        }
        let mut idx = ((m.actor) as i32);
        let mut msg = decode_message_platform(idx.clone(), &(m.proto.clone()), &(m.payload.clone()));
        if msg.is_some() {
            let mut v = msg.unwrap();
            __module_use_0().received(idx.clone());
            let mut _queued = deliver_remote(idx, v, m.from);
            return true;
        }
        return false;
    }
    if matches!(f, Some(Union5::U2(_))) {
        let mut a = f.as_ref().unwrap().u2().clone();
        return deliver_answer(&a);
    }
    if matches!(f, Some(Union5::U3(_))) {
        let mut g = f.as_ref().unwrap().u3().clone();
        return __module_use_0().credited(RtRemoteRef { node: g.host, actor: g.actor, bits: g.bits }, g.to, g.n);
    }
    if matches!(f, Some(Union5::U4(_))) {
        let mut o = f.as_ref().unwrap().u4().clone();
        if !__module_use_0().accepts(o.to, o.actor, o.bits) {
            return false;
        }
        let mut idx = ((o.actor) as i32);
        let mut room = mailbox_room(idx.clone()) - __module_use_0().held(idx.clone());
        if room < 1 {
            room = 1;
        }
        __module_use_0().grant(idx.clone(), actor_pool(idx.clone()), o.from, room);
        flush();
        return true;
    }
    if matches!(f, Some(Union5::U5(_))) {
        let mut c = f.as_ref().unwrap().u5().clone();
        let mut node = c.to;
        if node == ((0) as i64) {
            node = here_node();
        }
        if !__module_use_0().hosts(node.clone()) {
            return false;
        }
        let mut sink = __module_use_0().channel_sink(node, &(c.channel.clone()));
        if sink < 0 {
            return false;
        }
        let mut msg = control_message_platform(sink.clone(), c.from, &(c.payload.clone()));
        if msg.is_some() {
            let mut v = msg.unwrap();
            let mut queued = deliver_remote(sink, v, ((-1) as i64));
            return queued;
        }
        return false;
    }
    return false;
}

pub fn deliver_answer(a: &RtAnswerFrame) -> bool {
    if !__module_use_0().hosts(a.to) {
        return false;
    }
    if a.kind == 0 {
        if !__module_use_0().accepts(a.to, a.id, a.bits) {
            return false;
        }
        answer(token_to_actor(((a.id) as i32), a.slot), raw_answer_platform(a.payload.clone()));
        return true;
    }
    if a.kind == 1 {
        let mut wid = ((a.id) as i32);
        let mut v = decode_waiter_answer_platform(wid.clone(), &(a.payload.clone()));
        if v.is_some() {
            let mut value = v.unwrap();
            answer(token_to_waiter(wid, a.slot), value);
            return true;
        }
        return false;
    }
    let mut t = __module_use_0().take_task(a.id);
    if t.is_some() {
        let mut task = t.unwrap();
        let mut v = decode_task_answer_platform(a.id, &(a.payload.clone()));
        let __destructured3 = task;
        let mut pool = __destructured3.pool;
        let mut body = __destructured3.body;
        if v.is_some() {
            let mut value = v.unwrap();
            answer(mint_task_on(pool, body), value);
            return true;
        }
        drop_body_platform(body);
    }
    return false;
}

pub fn hash__4(value: &RtRemoteRef) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.node), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.actor), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.bits), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__4(a: &RtRemoteRef, b: &RtRemoteRef) -> bool {
    if !((a.node) == (b.node)) {
        return false;
    }
    if !((a.actor) == (b.actor)) {
        return false;
    }
    if !((a.bits) == (b.bits)) {
        return false;
    }
    return true;
}

pub fn hash__5(value: &RtControlKey) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.node), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.channel[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__5(a: &RtControlKey, b: &RtControlKey) -> bool {
    if !((a.node) == (b.node)) {
        return false;
    }
    if !(&a.channel[..] == &b.channel[..]) {
        return false;
    }
    return true;
}

pub struct __Hash_hash__RtRemoteRef_RtRemoteRef;
impl SalvoHash<RtRemoteRef> for __Hash_hash__RtRemoteRef_RtRemoteRef {
    fn hash(__v: &RtRemoteRef) -> i64 { hash__4(__v) }
}

pub struct __Eq_eq__RtRemoteRef_RtRemoteRef;
impl SalvoEq<RtRemoteRef> for __Eq_eq__RtRemoteRef_RtRemoteRef {
    fn eq(__a: &RtRemoteRef, __b: &RtRemoteRef) -> bool { eq__4(__a, __b) }
}

pub struct __Hash_hash__RtControlKey_RtControlKey;
impl SalvoHash<RtControlKey> for __Hash_hash__RtControlKey_RtControlKey {
    fn hash(__v: &RtControlKey) -> i64 { hash__5(__v) }
}

pub struct __Eq_eq__RtControlKey_RtControlKey;
impl SalvoEq<RtControlKey> for __Eq_eq__RtControlKey_RtControlKey {
    fn eq(__a: &RtControlKey, __b: &RtControlKey) -> bool { eq__5(__a, __b) }
}
