use crate::runtime::Body;
use crate::core_bytes::Bytes;
use crate::runtime::Dyn;
use crate::runtime::Exported;
use crate::core_map::Map;
use crate::runtime::Parker;
use crate::core_set::Set;
use crate::runtime::actor_pool;
use crate::runtime::answer;
use crate::core_map::contains_key_platform;
use crate::core_set::contains_platform;
use crate::runtime::current_pool;
use crate::runtime::deliver_remote;
use crate::runtime::drop_body_platform;
use crate::runtime::identity_bits;
use crate::core_map::keys_platform;
use crate::runtime::kill_actor;
use crate::runtime::mailbox_dead;
use crate::runtime::mailbox_queued;
use crate::runtime::mailbox_room;
use crate::runtime::mark_proxy;
use crate::runtime::mint_task_on;
use crate::core_compare::mix_hash;
use crate::core_map::mut_map_of_platform;
use crate::core_set::mut_set_of_platform;
use crate::runtime::new_pool_of;
use crate::runtime::park_nanos_platform;
use crate::runtime::park_platform;
use crate::core_map::put_platform;
use crate::core_list::remove_at_platform;
use crate::core_map::remove_platform;
use crate::core_list::size_platform;
use crate::runtime::spawn_inert;
use crate::runtime::this_parker_platform;
use crate::runtime::token_to_actor;
use crate::runtime::token_to_waiter;
use crate::runtime::unpark_platform;
use crate::runtime::waiter_pool;


pub fn __module_use0() -> &'static std::sync::Arc<std::sync::Mutex<crate::runtime_routing::Routes>> {
    static CELL: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<crate::runtime_routing::Routes>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Arc::new(std::sync::Mutex::new(crate::runtime_routing::Routes::new())))
}

pub fn __module_use0_0() -> &'static crate::runtime_routing::RouteTable {
    static CELL: std::sync::OnceLock<crate::runtime_routing::RouteTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| crate::runtime_routing::RouteTable::share_locked(crate::runtime_routing::__module_use0().clone()))
}

pub fn decode_message_platform(mut addr: i32, proto: &String, payload: &crate::core_bytes::Bytes) -> Option<crate::runtime::Dyn> {
    crate::platform_runtime_routing::decode_message(addr, proto, payload)
}

pub fn decode_waiter_answer_platform(mut wid: i32, payload: &crate::core_bytes::Bytes) -> Option<crate::runtime::Dyn> {
    crate::platform_runtime_routing::decode_waiter_answer(wid, payload)
}

pub fn decode_task_answer_platform(mut key: i64, payload: &crate::core_bytes::Bytes) -> Option<crate::runtime::Dyn> {
    crate::platform_runtime_routing::decode_task_answer(key, payload)
}

pub fn raw_answer_platform(mut payload: crate::core_bytes::Bytes) -> crate::runtime::Dyn {
    crate::platform_runtime_routing::raw_answer(payload)
}

pub fn control_message_platform(mut sink: i32, mut from: i64, payload: &crate::core_bytes::Bytes) -> Option<crate::runtime::Dyn> {
    crate::platform_runtime_routing::control_message(sink, from, payload)
}

pub fn wire_out_platform(mut from: i64, mut to: crate::core_bytes::Bytes, mut frame: crate::core_bytes::Bytes) {
    crate::platform_runtime_routing::wire_out(from, to, frame)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MsgFrame {
    pub to: i64,
    pub actor: i64,
    pub bits: i64,
    pub from: i64,
    pub proto: String,
    pub payload: crate::core_bytes::Bytes,
}

impl crate::wire::__Wire for MsgFrame {
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
pub struct AnswerFrame {
    pub to: i64,
    pub kind: i32,
    pub id: i64,
    pub slot: i64,
    pub bits: i64,
    pub payload: crate::core_bytes::Bytes,
}

impl crate::wire::__Wire for AnswerFrame {
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
pub struct GrantFrame {
    pub to: i64,
    pub host: i64,
    pub actor: i64,
    pub bits: i64,
    pub n: i32,
}

impl crate::wire::__Wire for GrantFrame {
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
pub struct OpenFrame {
    pub to: i64,
    pub actor: i64,
    pub bits: i64,
    pub from: i64,
}

impl crate::wire::__Wire for OpenFrame {
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
pub struct ControlFrame {
    pub to: i64,
    pub from: i64,
    pub channel: String,
    pub payload: crate::core_bytes::Bytes,
}

impl crate::wire::__Wire for ControlFrame {
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

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteRef {
    pub node: i64,
    pub actor: i64,
    pub bits: i64,
}

impl crate::wire::__Wire for RemoteRef {
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
pub struct ReplyParts {
    pub node: i64,
    pub kind: i32,
    pub id: i64,
    pub slot: i64,
    pub bits: i64,
}

impl crate::wire::__Wire for ReplyParts {
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

#[derive(Clone, Debug, PartialEq)]
pub struct ControlKey {
    pub node: i64,
    pub channel: String,
}

impl crate::wire::__Wire for ControlKey {
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
pub struct Parked {
    pub from: i64,
    pub to: i64,
    pub frame: crate::core_bytes::Bytes,
}

impl crate::wire::__Wire for Parked {
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
pub struct Staged {
    pub from: i64,
    pub to: crate::core_bytes::Bytes,
    pub frame: crate::core_bytes::Bytes,
}

impl crate::wire::__Wire for Staged {
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

pub struct ExportedTask {
    pub pool: i32,
    pub body: crate::runtime::Body,
}

impl std::fmt::Debug for ExportedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportedTask")
            .field("pool", &self.pool)
            .field("body", &"<opaque>")
            .finish()
    }
}

pub fn drop_exported_task(mut t: crate::runtime_routing::ExportedTask) {
    let mut __destructured_1: crate::runtime_routing::ExportedTask = t;
    let mut pool: i32 = __destructured_1.pool;
    let mut body: crate::runtime::Body = __destructured_1.body;
    crate::runtime::drop_body_platform(body);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub idx: i32,
}

impl crate::wire::__Wire for Found {
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
pub struct MakeProxy {
}

impl crate::wire::__Wire for MakeProxy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MakeDead {
}

impl crate::wire::__Wire for MakeDead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub trait __Stateless_RouteTable: Send + Sync {
    fn node_of_pool(&self, pool: i32) -> i64;
    fn adopt_pool(&self, pool: i32, node: i64);
    fn add_node(&self) -> i64;
    fn hosts(&self, node: i64) -> bool;
    fn identity_of(&self, addr: i32, pool: i32) -> crate::runtime_routing::RemoteRef;
    fn find_import(&self, r: &crate::runtime_routing::RemoteRef, here: i64) -> crate::unions::Union3<crate::runtime_routing::Found, crate::runtime_routing::MakeProxy, crate::runtime_routing::MakeDead>;
    fn register_proxy(&self, r: crate::runtime_routing::RemoteRef, idx: i32, here: i64) -> i32;
    fn register_dead(&self, idx: i32) -> i32;
    fn is_proxy(&self, addr: i32) -> bool;
    fn proxy_ref(&self, addr: i32) -> Option<crate::runtime_routing::RemoteRef>;
    fn take_credit(&self, addr: i32, me: crate::runtime::Parker) -> i32;
    fn stage(&self, from: i64, to: i64, frame: crate::core_bytes::Bytes);
    fn grant(&self, addr: i32, pool: i32, from: i64, n: i32);
    fn take_outbox(&self) -> Vec<crate::runtime_routing::Staged>;
    fn add_route(&self, node: i64, at: crate::core_bytes::Bytes);
    fn set_outbound(&self, node: i64);
    fn has_outbound(&self, node: i64) -> bool;
    fn accepts(&self, to: i64, actor: i64, claimed: i64) -> bool;
    fn received(&self, idx: i32);
    fn credited(&self, r: crate::runtime_routing::RemoteRef, to: i64, n: i32) -> bool;
    fn held(&self, idx: i32) -> i32;
    fn put_task(&self, key: i64, t: crate::runtime_routing::ExportedTask);
    fn take_task(&self, key: i64) -> Option<crate::runtime_routing::ExportedTask>;
    fn watch_channel(&self, node: i64, channel: String, sink: i32);
    fn channel_sink(&self, node: i64, channel: &String) -> i32;
    fn set_protocols(&self, table: Vec<(String, String)>);
    fn protocols(&self) -> Vec<(String, String)>;
    fn set_peer(&self, node: i64, table: Vec<(String, String)>);
    fn peer_hash(&self, node: i64, protocol: &String) -> Option<String>;
    fn forget_node(&self, node: i64) -> Vec<i32>;
    fn credits_of(&self, addr: i32) -> Option<i32>;
    fn set_view(&self, group: i32, members: Vec<i32>);
    fn view_of(&self, group: i32) -> Vec<i32>;
    fn version_of(&self, group: i32) -> i64;
    fn bump(&self, group: i32);
    fn add_view_waiter(&self, group: i32, seen: i64, me: crate::runtime::Parker) -> i64;
    fn drop_view_waiter(&self, id: i64);
}

pub trait __Stateful_RouteTable: Send {
    fn node_of_pool(&mut self, pool: i32) -> i64;
    fn adopt_pool(&mut self, pool: i32, node: i64);
    fn add_node(&mut self) -> i64;
    fn hosts(&mut self, node: i64) -> bool;
    fn identity_of(&mut self, addr: i32, pool: i32) -> crate::runtime_routing::RemoteRef;
    fn find_import(&mut self, r: &crate::runtime_routing::RemoteRef, here: i64) -> crate::unions::Union3<crate::runtime_routing::Found, crate::runtime_routing::MakeProxy, crate::runtime_routing::MakeDead>;
    fn register_proxy(&mut self, r: crate::runtime_routing::RemoteRef, idx: i32, here: i64) -> i32;
    fn register_dead(&mut self, idx: i32) -> i32;
    fn is_proxy(&mut self, addr: i32) -> bool;
    fn proxy_ref(&mut self, addr: i32) -> Option<crate::runtime_routing::RemoteRef>;
    fn take_credit(&mut self, addr: i32, me: crate::runtime::Parker) -> i32;
    fn stage(&mut self, from: i64, to: i64, frame: crate::core_bytes::Bytes);
    fn grant(&mut self, addr: i32, pool: i32, from: i64, n: i32);
    fn take_outbox(&mut self) -> Vec<crate::runtime_routing::Staged>;
    fn add_route(&mut self, node: i64, at: crate::core_bytes::Bytes);
    fn set_outbound(&mut self, node: i64);
    fn has_outbound(&mut self, node: i64) -> bool;
    fn accepts(&mut self, to: i64, actor: i64, claimed: i64) -> bool;
    fn received(&mut self, idx: i32);
    fn credited(&mut self, r: crate::runtime_routing::RemoteRef, to: i64, n: i32) -> bool;
    fn held(&mut self, idx: i32) -> i32;
    fn put_task(&mut self, key: i64, t: crate::runtime_routing::ExportedTask);
    fn take_task(&mut self, key: i64) -> Option<crate::runtime_routing::ExportedTask>;
    fn watch_channel(&mut self, node: i64, channel: String, sink: i32);
    fn channel_sink(&mut self, node: i64, channel: &String) -> i32;
    fn set_protocols(&mut self, table: Vec<(String, String)>);
    fn protocols(&mut self) -> Vec<(String, String)>;
    fn set_peer(&mut self, node: i64, table: Vec<(String, String)>);
    fn peer_hash(&mut self, node: i64, protocol: &String) -> Option<String>;
    fn forget_node(&mut self, node: i64) -> Vec<i32>;
    fn credits_of(&mut self, addr: i32) -> Option<i32>;
    fn set_view(&mut self, group: i32, members: Vec<i32>);
    fn view_of(&mut self, group: i32) -> Vec<i32>;
    fn version_of(&mut self, group: i32) -> i64;
    fn bump(&mut self, group: i32);
    fn add_view_waiter(&mut self, group: i32, seen: i64, me: crate::runtime::Parker) -> i64;
    fn drop_view_waiter(&mut self, id: i64);
}

pub struct RouteTable {
    inner: __Inner_RouteTable,
}

pub enum __Inner_RouteTable {
    Shared(std::sync::Arc<dyn __Stateless_RouteTable>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_RouteTable>>),
}

impl Clone for RouteTable {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_RouteTable::Shared(h) => __Inner_RouteTable::Shared(h.clone()),
            __Inner_RouteTable::Locked(h) => __Inner_RouteTable::Locked(h.clone()),
        } }
    }
}

impl RouteTable {
    pub fn shared<__H: __Stateless_RouteTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RouteTable::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_RouteTable>) -> Self {
        Self { inner: __Inner_RouteTable::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_RouteTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RouteTable::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_RouteTable>>) -> Self {
        Self { inner: __Inner_RouteTable::Locked(inner) }
    }
    pub fn node_of_pool(&self, pool: i32) -> i64 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.node_of_pool(pool),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().node_of_pool(pool),
        }
    }
    pub fn adopt_pool(&self, pool: i32, node: i64) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.adopt_pool(pool, node),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().adopt_pool(pool, node),
        }
    }
    pub fn add_node(&self) -> i64 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.add_node(),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().add_node(),
        }
    }
    pub fn hosts(&self, node: i64) -> bool {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.hosts(node),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().hosts(node),
        }
    }
    pub fn identity_of(&self, addr: i32, pool: i32) -> crate::runtime_routing::RemoteRef {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.identity_of(addr, pool),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().identity_of(addr, pool),
        }
    }
    pub fn find_import(&self, r: &crate::runtime_routing::RemoteRef, here: i64) -> crate::unions::Union3<crate::runtime_routing::Found, crate::runtime_routing::MakeProxy, crate::runtime_routing::MakeDead> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.find_import(r, here),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().find_import(r, here),
        }
    }
    pub fn register_proxy(&self, r: crate::runtime_routing::RemoteRef, idx: i32, here: i64) -> i32 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.register_proxy(r, idx, here),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().register_proxy(r, idx, here),
        }
    }
    pub fn register_dead(&self, idx: i32) -> i32 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.register_dead(idx),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().register_dead(idx),
        }
    }
    pub fn is_proxy(&self, addr: i32) -> bool {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.is_proxy(addr),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().is_proxy(addr),
        }
    }
    pub fn proxy_ref(&self, addr: i32) -> Option<crate::runtime_routing::RemoteRef> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.proxy_ref(addr),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().proxy_ref(addr),
        }
    }
    pub fn take_credit(&self, addr: i32, me: crate::runtime::Parker) -> i32 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.take_credit(addr, me),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().take_credit(addr, me),
        }
    }
    pub fn stage(&self, from: i64, to: i64, frame: crate::core_bytes::Bytes) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.stage(from, to, frame),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().stage(from, to, frame),
        }
    }
    pub fn grant(&self, addr: i32, pool: i32, from: i64, n: i32) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.grant(addr, pool, from, n),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().grant(addr, pool, from, n),
        }
    }
    pub fn take_outbox(&self) -> Vec<crate::runtime_routing::Staged> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.take_outbox(),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().take_outbox(),
        }
    }
    pub fn add_route(&self, node: i64, at: crate::core_bytes::Bytes) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.add_route(node, at),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().add_route(node, at),
        }
    }
    pub fn set_outbound(&self, node: i64) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.set_outbound(node),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().set_outbound(node),
        }
    }
    pub fn has_outbound(&self, node: i64) -> bool {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.has_outbound(node),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().has_outbound(node),
        }
    }
    pub fn accepts(&self, to: i64, actor: i64, claimed: i64) -> bool {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.accepts(to, actor, claimed),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().accepts(to, actor, claimed),
        }
    }
    pub fn received(&self, idx: i32) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.received(idx),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().received(idx),
        }
    }
    pub fn credited(&self, r: crate::runtime_routing::RemoteRef, to: i64, n: i32) -> bool {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.credited(r, to, n),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().credited(r, to, n),
        }
    }
    pub fn held(&self, idx: i32) -> i32 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.held(idx),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().held(idx),
        }
    }
    pub fn put_task(&self, key: i64, t: crate::runtime_routing::ExportedTask) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.put_task(key, t),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().put_task(key, t),
        }
    }
    pub fn take_task(&self, key: i64) -> Option<crate::runtime_routing::ExportedTask> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.take_task(key),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().take_task(key),
        }
    }
    pub fn watch_channel(&self, node: i64, channel: String, sink: i32) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.watch_channel(node, channel, sink),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().watch_channel(node, channel, sink),
        }
    }
    pub fn channel_sink(&self, node: i64, channel: &String) -> i32 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.channel_sink(node, channel),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().channel_sink(node, channel),
        }
    }
    pub fn set_protocols(&self, table: Vec<(String, String)>) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.set_protocols(table),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().set_protocols(table),
        }
    }
    pub fn protocols(&self) -> Vec<(String, String)> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.protocols(),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().protocols(),
        }
    }
    pub fn set_peer(&self, node: i64, table: Vec<(String, String)>) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.set_peer(node, table),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().set_peer(node, table),
        }
    }
    pub fn peer_hash(&self, node: i64, protocol: &String) -> Option<String> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.peer_hash(node, protocol),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().peer_hash(node, protocol),
        }
    }
    pub fn forget_node(&self, node: i64) -> Vec<i32> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.forget_node(node),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().forget_node(node),
        }
    }
    pub fn credits_of(&self, addr: i32) -> Option<i32> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.credits_of(addr),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().credits_of(addr),
        }
    }
    pub fn set_view(&self, group: i32, members: Vec<i32>) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.set_view(group, members),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().set_view(group, members),
        }
    }
    pub fn view_of(&self, group: i32) -> Vec<i32> {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.view_of(group),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().view_of(group),
        }
    }
    pub fn version_of(&self, group: i32) -> i64 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.version_of(group),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().version_of(group),
        }
    }
    pub fn bump(&self, group: i32) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.bump(group),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().bump(group),
        }
    }
    pub fn add_view_waiter(&self, group: i32, seen: i64, me: crate::runtime::Parker) -> i64 {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.add_view_waiter(group, seen, me),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().add_view_waiter(group, seen, me),
        }
    }
    pub fn drop_view_waiter(&self, id: i64) {
        match &self.inner {
            __Inner_RouteTable::Shared(h) => h.drop_view_waiter(id),
            __Inner_RouteTable::Locked(h) => h.lock().unwrap().drop_view_waiter(id),
        }
    }
}

pub fn version_in(versions: &crate::core_map::Map<i32, i64>, mut group: i32) -> i64 {
    let mut v: Option<i64> = crate::core_map::get_platform::<i32, i64>(versions, &group, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
    if v.is_some() {
        let mut n = v.unwrap();
        return n;
    };
    return 0i64;
}

pub fn bump_in(versions: &mut crate::core_map::Map<i32, i64>, waiters: &mut Vec<crate::runtime_routing::ViewWaiter>, mut group: i32) {
    { let __arg1 = group; let __arg2 = i64::wrapping_add(crate::runtime_routing::version_in(&*versions, group), 1i64); crate::core_map::put_platform::<i32, i64>(&mut *versions, __arg1, __arg2, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))) };
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::core_list::size_platform::<crate::runtime_routing::ViewWaiter>(&*waiters))) {
            break;
        };
        if ({
            let mut __proj_3: &crate::runtime_routing::ViewWaiter = {
                let mut __nn_1: Option<&crate::runtime_routing::ViewWaiter> = crate::core_list::get_platform::<crate::runtime_routing::ViewWaiter>(&*waiters, i);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.routing:171:12");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            };
            __proj_3.group
        } == group) {
            let mut w: crate::runtime_routing::ViewWaiter = {
                let mut __nn_4: Option<crate::runtime_routing::ViewWaiter> = crate::core_list::remove_at_platform::<crate::runtime_routing::ViewWaiter>(&mut *waiters, i);
                if __nn_4.is_none() {
                    panic!("salvo: value is absent at runtime.routing:172:21");
                } else {
                    let mut __some_5 = __nn_4.unwrap();
                    __some_5
                }
            };
            crate::runtime::unpark_platform(&(w.parker).clone());
        } else {
            i = i32::wrapping_add(i, 1i32);
        };
    }
}

#[derive(Clone)]
pub struct ViewWaiter {
    pub group: i32,
    pub id: i64,
    pub parker: crate::runtime::Parker,
}

impl std::fmt::Debug for ViewWaiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewWaiter")
            .field("group", &self.group)
            .field("id", &self.id)
            .field("parker", &"<opaque>")
            .finish()
    }
}

pub struct Routes {
    node_id: i64,
    hosted: crate::core_set::Set<i64>,
    pool_node: crate::core_map::Map<i32, i64>,
    bits: crate::core_map::Map<i32, i64>,
    remote: crate::core_map::Map<i32, crate::runtime_routing::RemoteRef>,
    proxies: crate::core_map::Map<crate::runtime_routing::RemoteRef, i32>,
    credits: crate::core_map::Map<i32, i32>,
    held_n: crate::core_map::Map<i32, i32>,
    routes: crate::core_map::Map<i64, crate::core_bytes::Bytes>,
    outbound: crate::core_set::Set<i64>,
    parked: Vec<crate::runtime_routing::Parked>,
    outbox: Vec<crate::runtime_routing::Staged>,
    task_keys: Vec<i64>,
    tasks: Vec<crate::runtime_routing::ExportedTask>,
    controls: crate::core_map::Map<crate::runtime_routing::ControlKey, i32>,
    local: Vec<(String, String)>,
    peers: crate::core_map::Map<i64, Vec<(String, String)>>,
    dead_entry: i32,
    credit_waiters: Vec<crate::runtime::Parker>,
    views: crate::core_map::Map<i32, Vec<i32>>,
    versions: crate::core_map::Map<i32, i64>,
    view_waiters: Vec<crate::runtime_routing::ViewWaiter>,
    next_waiter: i64,
}

impl Routes {
    pub fn new() -> Self {
        let mut __s = Self {
            node_id: 0i64,
            hosted: crate::core_set::mut_set_of_platform::<i64>(vec![], &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1))),
            pool_node: crate::core_map::mut_map_of_platform::<i32, i64>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            bits: crate::core_map::mut_map_of_platform::<i32, i64>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            remote: crate::core_map::mut_map_of_platform::<i32, crate::runtime_routing::RemoteRef>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            proxies: crate::core_map::mut_map_of_platform::<crate::runtime_routing::RemoteRef, i32>(vec![], &mut |__a0: &crate::runtime_routing::RemoteRef| crate::runtime_routing::hash__RemoteRef(__a0), &mut |__a0: &crate::runtime_routing::RemoteRef, __a1: &crate::runtime_routing::RemoteRef| crate::runtime_routing::eq__RemoteRef_RemoteRef(__a0, __a1)),
            credits: crate::core_map::mut_map_of_platform::<i32, i32>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            held_n: crate::core_map::mut_map_of_platform::<i32, i32>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            routes: crate::core_map::mut_map_of_platform::<i64, crate::core_bytes::Bytes>(vec![], &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1))),
            outbound: crate::core_set::mut_set_of_platform::<i64>(vec![], &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1))),
            parked: vec![],
            outbox: vec![],
            task_keys: vec![],
            tasks: vec![],
            controls: crate::core_map::mut_map_of_platform::<crate::runtime_routing::ControlKey, i32>(vec![], &mut |__a0: &crate::runtime_routing::ControlKey| crate::runtime_routing::hash__ControlKey(__a0), &mut |__a0: &crate::runtime_routing::ControlKey, __a1: &crate::runtime_routing::ControlKey| crate::runtime_routing::eq__ControlKey_ControlKey(__a0, __a1)),
            local: vec![],
            peers: crate::core_map::mut_map_of_platform::<i64, Vec<(String, String)>>(vec![], &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1))),
            dead_entry: i32::wrapping_neg(1i32),
            credit_waiters: vec![],
            views: crate::core_map::mut_map_of_platform::<i32, Vec<i32>>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            versions: crate::core_map::mut_map_of_platform::<i32, i64>(vec![], &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))),
            view_waiters: vec![],
            next_waiter: 0i64
        };
        __s.init();
        __s
    }
    fn init(&mut self) {
        self.node_id = crate::runtime_routing::fresh_node();
        crate::core_set::add_platform::<i64>(&mut self.hosted, self.node_id, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        crate::core_map::put_platform::<i32, i64>(&mut self.pool_node, 0i32, self.node_id, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
    }
}

impl crate::runtime_routing::__Stateful_RouteTable for Routes {
    fn node_of_pool(&mut self, pool: i32) -> i64 {
        return crate::runtime_routing::node_in(&self.pool_node, self.node_id, pool);
    }
    fn adopt_pool(&mut self, pool: i32, node: i64) {
        crate::core_map::put_platform::<i32, i64>(&mut self.pool_node, pool, node, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
    }
    fn add_node(&mut self) -> i64 {
        let mut n: i64 = crate::runtime_routing::fresh_node();
        crate::core_set::add_platform::<i64>(&mut self.hosted, n, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return n;
    }
    fn hosts(&mut self, node: i64) -> bool {
        return crate::core_set::contains_platform::<i64>(&self.hosted, &node, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    }
    fn identity_of(&mut self, addr: i32, pool: i32) -> crate::runtime_routing::RemoteRef {
        return crate::runtime_routing::identity_in(&self.remote, &mut self.bits, &self.pool_node, self.node_id, addr, pool);
    }
    fn find_import(&mut self, r: &crate::runtime_routing::RemoteRef, here: i64) -> crate::unions::Union3<crate::runtime_routing::Found, crate::runtime_routing::MakeProxy, crate::runtime_routing::MakeDead> {
        if (r.node == here) {
            let mut idx: i32 = ((r.actor) as i32);
            let mut b: Option<i64> = crate::core_map::get_platform::<i32, i64>(&self.bits, &idx, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
            if if (!(crate::core_map::contains_key_platform::<i32, crate::runtime_routing::RemoteRef>(&self.remote, &idx, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)))) && b.is_some()) {
                let mut known = b.unwrap();
                (known == r.bits)
            } else {
                false
            } {
                let mut known = b.unwrap();
                return crate::unions::Union3::U1(crate::runtime_routing::Found { idx: idx });
            };
            if (self.dead_entry >= 0i32) {
                return crate::unions::Union3::U1(crate::runtime_routing::Found { idx: self.dead_entry });
            };
            return crate::unions::Union3::U3(crate::runtime_routing::MakeDead {});
        };
        let mut p: Option<i32> = crate::core_map::get_platform::<crate::runtime_routing::RemoteRef, i32>(&self.proxies, r, &mut |__a0: &crate::runtime_routing::RemoteRef| crate::runtime_routing::hash__RemoteRef(__a0), &mut |__a0: &crate::runtime_routing::RemoteRef, __a1: &crate::runtime_routing::RemoteRef| crate::runtime_routing::eq__RemoteRef_RemoteRef(__a0, __a1)).copied();
        if p.is_some() {
            let mut idx = p.unwrap();
            return crate::unions::Union3::U1(crate::runtime_routing::Found { idx: idx });
        };
        return crate::unions::Union3::U2(crate::runtime_routing::MakeProxy {});
    }
    fn register_proxy(&mut self, r: crate::runtime_routing::RemoteRef, idx: i32, here: i64) -> i32 {
        let mut existing: Option<i32> = crate::core_map::get_platform::<crate::runtime_routing::RemoteRef, i32>(&self.proxies, &r, &mut |__a0: &crate::runtime_routing::RemoteRef| crate::runtime_routing::hash__RemoteRef(__a0), &mut |__a0: &crate::runtime_routing::RemoteRef, __a1: &crate::runtime_routing::RemoteRef| crate::runtime_routing::eq__RemoteRef_RemoteRef(__a0, __a1)).copied();
        if existing.is_some() {
            let mut e = existing.unwrap();
            return e;
        };
        crate::core_map::put_platform::<i32, crate::runtime_routing::RemoteRef>(&mut self.remote, idx, (r).clone(), &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
        crate::core_map::put_platform::<crate::runtime_routing::RemoteRef, i32>(&mut self.proxies, (r).clone(), idx, &mut |__a0: &crate::runtime_routing::RemoteRef| crate::runtime_routing::hash__RemoteRef(__a0), &mut |__a0: &crate::runtime_routing::RemoteRef, __a1: &crate::runtime_routing::RemoteRef| crate::runtime_routing::eq__RemoteRef_RemoteRef(__a0, __a1));
        crate::core_map::put_platform::<i32, i32>(&mut self.credits, idx, 0i32, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
        let mut frame: crate::core_bytes::Bytes = { let __enc: crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame> = crate::unions::Union5::U4(crate::runtime_routing::OpenFrame { to: r.node, actor: r.actor, bits: r.bits, from: here }); crate::wire::salvo_encode(&__enc) };
        crate::runtime_routing::stage_in(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked, here, r.node, frame);
        return idx;
    }
    fn register_dead(&mut self, idx: i32) -> i32 {
        if (self.dead_entry >= 0i32) {
            return self.dead_entry;
        };
        self.dead_entry = idx;
        return self.dead_entry;
    }
    fn is_proxy(&mut self, addr: i32) -> bool {
        return crate::core_map::contains_key_platform::<i32, crate::runtime_routing::RemoteRef>(&self.remote, &addr, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
    }
    fn proxy_ref(&mut self, addr: i32) -> Option<crate::runtime_routing::RemoteRef> {
        let mut r: Option<&crate::runtime_routing::RemoteRef> = crate::core_map::get_platform::<i32, crate::runtime_routing::RemoteRef>(&self.remote, &addr, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
        if r.is_some() {
            let mut found = r.unwrap();
            return Some((found).clone());
        };
        return None;
    }
    fn take_credit(&mut self, addr: i32, me: crate::runtime::Parker) -> i32 {
        let mut c: Option<i32> = crate::core_map::get_platform::<i32, i32>(&self.credits, &addr, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
        if c.is_some() {
            let mut n = c.unwrap();
            if (n > 0i32) {
                crate::core_map::put_platform::<i32, i32>(&mut self.credits, addr, i32::wrapping_sub(n, 1i32), &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
                { let __arg1 = addr; let __arg2 = i32::wrapping_add(crate::runtime_routing::held_in(&self.held_n, addr), 1i32); crate::core_map::put_platform::<i32, i32>(&mut self.held_n, __arg1, __arg2, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))) };
                return 1i32;
            };
            crate::core_list::add_platform::<crate::runtime::Parker>(&mut self.credit_waiters, me);
            return 0i32;
        };
        return i32::wrapping_neg(1i32);
    }
    fn stage(&mut self, from: i64, to: i64, frame: crate::core_bytes::Bytes) {
        crate::runtime_routing::stage_in(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked, from, to, frame);
    }
    fn grant(&mut self, addr: i32, pool: i32, from: i64, n: i32) {
        { let __arg1 = addr; let __arg2 = i32::wrapping_add(crate::runtime_routing::held_in(&self.held_n, addr), n); crate::core_map::put_platform::<i32, i32>(&mut self.held_n, __arg1, __arg2, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))) };
        let mut me: crate::runtime_routing::RemoteRef = crate::runtime_routing::identity_in(&self.remote, &mut self.bits, &self.pool_node, self.node_id, addr, pool);
        let mut frame: crate::core_bytes::Bytes = { let __enc: crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame> = crate::unions::Union5::U3(crate::runtime_routing::GrantFrame { to: from, host: me.node, actor: me.actor, bits: me.bits, n: n }); crate::wire::salvo_encode(&__enc) };
        crate::runtime_routing::stage_in(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked, me.node, from, frame);
    }
    fn take_outbox(&mut self) -> Vec<crate::runtime_routing::Staged> {
        let mut out: Vec<crate::runtime_routing::Staged> = vec![];
        loop {
            if !((crate::core_list::size_platform::<crate::runtime_routing::Staged>(&self.outbox) > 0i32)) {
                break;
            };
            { let __arg1 = {
                let mut __nn_1: Option<crate::runtime_routing::Staged> = crate::core_list::remove_at_platform::<crate::runtime_routing::Staged>(&mut self.outbox, 0i32);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.routing:327:22");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            }; crate::core_list::add_platform::<crate::runtime_routing::Staged>(&mut out, __arg1) };
        }
        return out;
    }
    fn add_route(&mut self, node: i64, at: crate::core_bytes::Bytes) {
        crate::core_map::put_platform::<i64, crate::core_bytes::Bytes>(&mut self.routes, node, at, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        crate::runtime_routing::restage(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked);
    }
    fn set_outbound(&mut self, node: i64) {
        crate::core_set::add_platform::<i64>(&mut self.outbound, node, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        crate::runtime_routing::restage(&self.routes, &self.outbound, &mut self.outbox, &mut self.parked);
    }
    fn has_outbound(&mut self, node: i64) -> bool {
        return crate::core_set::contains_platform::<i64>(&self.outbound, &node, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    }
    fn accepts(&mut self, to: i64, actor: i64, claimed: i64) -> bool {
        if !(crate::core_set::contains_platform::<i64>(&self.hosted, &to, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)))) {
            return false;
        };
        let mut idx: i32 = ((actor) as i32);
        let mut b: Option<i64> = crate::core_map::get_platform::<i32, i64>(&self.bits, &idx, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
        if if (!(crate::core_map::contains_key_platform::<i32, crate::runtime_routing::RemoteRef>(&self.remote, &idx, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)))) && b.is_some()) {
            let mut known = b.unwrap();
            (known == claimed)
        } else {
            false
        } {
            let mut known = b.unwrap();
            return true;
        };
        return false;
    }
    fn received(&mut self, idx: i32) {
        let mut h: i32 = crate::runtime_routing::held_in(&self.held_n, idx);
        if (h > 0i32) {
            crate::core_map::put_platform::<i32, i32>(&mut self.held_n, idx, i32::wrapping_sub(h, 1i32), &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
        };
    }
    fn credited(&mut self, r: crate::runtime_routing::RemoteRef, to: i64, n: i32) -> bool {
        if !(crate::core_set::contains_platform::<i64>(&self.hosted, &to, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)))) {
            return false;
        };
        let mut p: Option<i32> = crate::core_map::get_platform::<crate::runtime_routing::RemoteRef, i32>(&self.proxies, &r, &mut |__a0: &crate::runtime_routing::RemoteRef| crate::runtime_routing::hash__RemoteRef(__a0), &mut |__a0: &crate::runtime_routing::RemoteRef, __a1: &crate::runtime_routing::RemoteRef| crate::runtime_routing::eq__RemoteRef_RemoteRef(__a0, __a1)).copied();
        if p.is_some() {
            let mut idx = p.unwrap();
            let mut i: i32 = idx;
            let mut c: Option<i32> = crate::core_map::get_platform::<i32, i32>(&self.credits, &i, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
            let mut now: i32 = 0i32;
            if c.is_some() {
                let mut have = c.unwrap();
                now = have;
            };
            crate::core_map::put_platform::<i32, i32>(&mut self.credits, i, i32::wrapping_add(now, n), &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
            let mut h: i32 = i32::wrapping_sub(crate::runtime_routing::held_in(&self.held_n, i), n);
            if (h < 0i32) {
                h = 0i32;
            };
            crate::core_map::put_platform::<i32, i32>(&mut self.held_n, i, h, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
            crate::runtime_routing::wake_senders(&mut self.credit_waiters);
            return true;
        };
        return false;
    }
    fn held(&mut self, idx: i32) -> i32 {
        return crate::runtime_routing::held_in(&self.held_n, idx);
    }
    fn put_task(&mut self, key: i64, t: crate::runtime_routing::ExportedTask) {
        crate::core_list::add_platform::<i64>(&mut self.task_keys, key);
        crate::core_list::add_platform::<crate::runtime_routing::ExportedTask>(&mut self.tasks, t);
    }
    fn take_task(&mut self, key: i64) -> Option<crate::runtime_routing::ExportedTask> {
        let mut i: i32 = 0i32;
        loop {
            if !((i < crate::core_list::size_platform::<i64>(&self.task_keys))) {
                break;
            };
            if ({
                let mut __nn_1: Option<i64> = crate::core_list::get_platform::<i64>(&self.task_keys, i).copied();
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.routing:401:16");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            } == key) {
                let mut _k: Option<i64> = crate::core_list::remove_at_platform::<i64>(&mut self.task_keys, i);
                return crate::core_list::remove_at_platform::<crate::runtime_routing::ExportedTask>(&mut self.tasks, i);
            };
            i = i32::wrapping_add(i, 1i32);
        }
        return None;
    }
    fn watch_channel(&mut self, node: i64, channel: String, sink: i32) {
        crate::core_map::put_platform::<crate::runtime_routing::ControlKey, i32>(&mut self.controls, crate::runtime_routing::ControlKey { node: node, channel: channel.clone() }, sink, &mut |__a0: &crate::runtime_routing::ControlKey| crate::runtime_routing::hash__ControlKey(__a0), &mut |__a0: &crate::runtime_routing::ControlKey, __a1: &crate::runtime_routing::ControlKey| crate::runtime_routing::eq__ControlKey_ControlKey(__a0, __a1));
    }
    fn channel_sink(&mut self, node: i64, channel: &String) -> i32 {
        let mut s: Option<i32> = crate::core_map::get_platform::<crate::runtime_routing::ControlKey, i32>(&self.controls, &crate::runtime_routing::ControlKey { node: node, channel: (channel).clone() }, &mut |__a0: &crate::runtime_routing::ControlKey| crate::runtime_routing::hash__ControlKey(__a0), &mut |__a0: &crate::runtime_routing::ControlKey, __a1: &crate::runtime_routing::ControlKey| crate::runtime_routing::eq__ControlKey_ControlKey(__a0, __a1)).copied();
        if s.is_some() {
            let mut sink = s.unwrap();
            return sink;
        };
        return i32::wrapping_neg(1i32);
    }
    fn set_protocols(&mut self, table: Vec<(String, String)>) {
        self.local = table.clone();
    }
    fn protocols(&mut self) -> Vec<(String, String)> {
        return (self.local).clone();
    }
    fn set_peer(&mut self, node: i64, table: Vec<(String, String)>) {
        crate::core_map::put_platform::<i64, Vec<(String, String)>>(&mut self.peers, node, table, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    }
    fn peer_hash(&mut self, node: i64, protocol: &String) -> Option<String> {
        let mut t: Option<&Vec<(String, String)>> = crate::core_map::get_platform::<i64, Vec<(String, String)>>(&self.peers, &node, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        if t.is_some() {
            let mut table = t.unwrap();
            for mut entry in table.iter() {
                let mut __destructured_1: &(String, String) = entry;
                let mut name = &__destructured_1.0;
                let mut hash = &__destructured_1.1;
                if (&name[..] == &protocol[..]) {
                    return Some((hash).clone());
                };
            }
        };
        return None;
    }
    fn forget_node(&mut self, node: i64) -> Vec<i32> {
        let mut _route: Option<crate::core_bytes::Bytes> = crate::core_map::remove_platform::<i64, crate::core_bytes::Bytes>(&mut self.routes, &node, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        let mut _peer: Option<Vec<(String, String)>> = crate::core_map::remove_platform::<i64, Vec<(String, String)>>(&mut self.peers, &node, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        let mut gone: Vec<i32> = vec![];
        for mut idx in crate::core_map::keys_platform::<i32, crate::runtime_routing::RemoteRef>(&self.remote).iter().copied() {
            let mut r: Option<&crate::runtime_routing::RemoteRef> = crate::core_map::get_platform::<i32, crate::runtime_routing::RemoteRef>(&self.remote, &idx, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
            if r.is_some() {
                let mut found = r.unwrap();
                if (found.node == node) {
                    crate::core_list::add_platform::<i32>(&mut gone, idx);
                };
            };
        }
        crate::runtime_routing::wake_senders(&mut self.credit_waiters);
        return gone;
    }
    fn credits_of(&mut self, addr: i32) -> Option<i32> {
        let mut c: Option<i32> = crate::core_map::get_platform::<i32, i32>(&self.credits, &addr, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
        if c.is_some() {
            let mut n = c.unwrap();
            return Some(n);
        };
        return None;
    }
    fn set_view(&mut self, group: i32, members: Vec<i32>) {
        crate::core_map::put_platform::<i32, Vec<i32>>(&mut self.views, group, members, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
        crate::runtime_routing::bump_in(&mut self.versions, &mut self.view_waiters, group);
    }
    fn view_of(&mut self, group: i32) -> Vec<i32> {
        let mut v: Option<&Vec<i32>> = crate::core_map::get_platform::<i32, Vec<i32>>(&self.views, &group, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
        if v.is_some() {
            let mut found = v.unwrap();
            return (found).clone();
        };
        return vec![];
    }
    fn version_of(&mut self, group: i32) -> i64 {
        return crate::runtime_routing::version_in(&self.versions, group);
    }
    fn bump(&mut self, group: i32) {
        crate::runtime_routing::bump_in(&mut self.versions, &mut self.view_waiters, group);
    }
    fn add_view_waiter(&mut self, group: i32, seen: i64, me: crate::runtime::Parker) -> i64 {
        if (crate::runtime_routing::version_in(&self.versions, group) != seen) {
            return i64::wrapping_neg(1i64);
        };
        self.next_waiter = i64::wrapping_add(self.next_waiter, 1i64);
        crate::core_list::add_platform::<crate::runtime_routing::ViewWaiter>(&mut self.view_waiters, crate::runtime_routing::ViewWaiter { group: group, id: self.next_waiter, parker: me });
        return self.next_waiter;
    }
    fn drop_view_waiter(&mut self, id: i64) {
        let mut i: i32 = 0i32;
        loop {
            if !((i < crate::core_list::size_platform::<crate::runtime_routing::ViewWaiter>(&self.view_waiters))) {
                break;
            };
            if ({
                let mut __proj_3: &crate::runtime_routing::ViewWaiter = {
                    let mut __nn_1: Option<&crate::runtime_routing::ViewWaiter> = crate::core_list::get_platform::<crate::runtime_routing::ViewWaiter>(&self.view_waiters, i);
                    if __nn_1.is_none() {
                        panic!("salvo: value is absent at runtime.routing:496:16");
                    } else {
                        let mut __some_2 = __nn_1.unwrap();
                        __some_2
                    }
                };
                __proj_3.id
            } == id) {
                let mut _w: Option<crate::runtime_routing::ViewWaiter> = crate::core_list::remove_at_platform::<crate::runtime_routing::ViewWaiter>(&mut self.view_waiters, i);
                return;
            };
            i = i32::wrapping_add(i, 1i32);
        }
    }
}

pub fn node_in(pool_node: &crate::core_map::Map<i32, i64>, mut node_id: i64, mut pool: i32) -> i64 {
    let mut n: Option<i64> = crate::core_map::get_platform::<i32, i64>(pool_node, &pool, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
    if n.is_some() {
        let mut v = n.unwrap();
        return v;
    };
    return node_id;
}

pub fn held_in(held_n: &crate::core_map::Map<i32, i32>, mut idx: i32) -> i32 {
    let mut h: Option<i32> = crate::core_map::get_platform::<i32, i32>(held_n, &idx, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
    if h.is_some() {
        let mut n = h.unwrap();
        return n;
    };
    return 0i32;
}

pub fn identity_in(remote: &crate::core_map::Map<i32, crate::runtime_routing::RemoteRef>, bits: &mut crate::core_map::Map<i32, i64>, pool_node: &crate::core_map::Map<i32, i64>, mut node_id: i64, mut addr: i32, mut pool: i32) -> crate::runtime_routing::RemoteRef {
    let mut r: Option<&crate::runtime_routing::RemoteRef> = crate::core_map::get_platform::<i32, crate::runtime_routing::RemoteRef>(remote, &addr, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
    if r.is_some() {
        let mut found = r.unwrap();
        return (found).clone();
    };
    let mut n: i64 = crate::runtime_routing::node_in(pool_node, node_id, pool);
    let mut b: Option<i64> = crate::core_map::get_platform::<i32, i64>(&*bits, &addr, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1))).copied();
    if b.is_some() {
        let mut known = b.unwrap();
        return crate::runtime_routing::RemoteRef { node: n, actor: ((addr) as i64), bits: known };
    };
    let mut minted: i64 = crate::runtime::identity_bits();
    crate::core_map::put_platform::<i32, i64>(&mut *bits, addr, minted, &mut |__a0: &i32| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i32, __a1: &i32| ((*__a0) == (*__a1)));
    return crate::runtime_routing::RemoteRef { node: n, actor: ((addr) as i64), bits: minted };
}

pub fn wake_senders(waiters: &mut Vec<crate::runtime::Parker>) {
    loop {
        if !((crate::core_list::size_platform::<crate::runtime::Parker>(&*waiters) > 0i32)) {
            break;
        };
        { let __arg1 = {
            let mut __nn_1: Option<crate::runtime::Parker> = crate::core_list::remove_at_platform::<crate::runtime::Parker>(&mut *waiters, 0i32);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime.routing:554:16");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }; crate::runtime::unpark_platform(&__arg1) };
    }
}

pub fn stage_in(routes: &crate::core_map::Map<i64, crate::core_bytes::Bytes>, outbound: &crate::core_set::Set<i64>, outbox: &mut Vec<crate::runtime_routing::Staged>, parked: &mut Vec<crate::runtime_routing::Parked>, mut from: i64, mut to: i64, mut frame: crate::core_bytes::Bytes) {
    let mut ep: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<i64, crate::core_bytes::Bytes>(routes, &to, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    if ep.is_some() {
        let mut at = ep.unwrap();
        if crate::core_set::contains_platform::<i64>(outbound, &from, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1))) {
            crate::core_list::add_platform::<crate::runtime_routing::Staged>(&mut *outbox, crate::runtime_routing::Staged { from: from, to: (at).clone(), frame: frame.clone() });
            return;
        };
    };
    crate::core_list::add_platform::<crate::runtime_routing::Parked>(&mut *parked, crate::runtime_routing::Parked { from: from, to: to, frame: frame.clone() });
}

pub fn restage(routes: &crate::core_map::Map<i64, crate::core_bytes::Bytes>, outbound: &crate::core_set::Set<i64>, outbox: &mut Vec<crate::runtime_routing::Staged>, parked: &mut Vec<crate::runtime_routing::Parked>) {
    let mut waiting: Vec<crate::runtime_routing::Parked> = vec![];
    loop {
        if !((crate::core_list::size_platform::<crate::runtime_routing::Parked>(&*parked) > 0i32)) {
            break;
        };
        { let __arg1 = {
            let mut __nn_1: Option<crate::runtime_routing::Parked> = crate::core_list::remove_at_platform::<crate::runtime_routing::Parked>(&mut *parked, 0i32);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime.routing:577:22");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }; crate::core_list::add_platform::<crate::runtime_routing::Parked>(&mut waiting, __arg1) };
    }
    for mut p in waiting.iter() {
        crate::runtime_routing::stage_in(routes, outbound, &mut *outbox, &mut *parked, p.from, p.to, (p.frame).clone());
    }
}

pub fn fresh_node() -> i64 {
    let mut b: i64 = crate::runtime::identity_bits();
    if (b < 0i64) {
        return i64::wrapping_neg(i64::wrapping_add(b, 1i64));
    };
    return b;
}

pub fn here_node() -> i64 {
    return crate::runtime_routing::__module_use0_0().node_of_pool(crate::runtime::current_pool());
}

pub fn adopt(mut pool: i32) {
    crate::runtime_routing::__module_use0_0().adopt_pool(pool, crate::runtime_routing::here_node());
}

pub fn new_node() -> i64 {
    return crate::runtime_routing::__module_use0_0().add_node();
}

pub fn pool_at(mut node: i64, mut n: i32) -> i32 {
    let mut p: i32 = crate::runtime::new_pool_of(n, i32::wrapping_neg(1i32));
    crate::runtime_routing::__module_use0_0().adopt_pool(p, node);
    return p;
}

pub fn identity(mut addr: i32) -> crate::runtime_routing::RemoteRef {
    return crate::runtime_routing::__module_use0_0().identity_of(addr, crate::runtime::actor_pool(addr));
}

pub fn same_actor(mut a: i32, mut b: i32) -> bool {
    if (a == b) {
        return true;
    };
    if (!(crate::runtime_routing::__module_use0_0().is_proxy(a)) && !(crate::runtime_routing::__module_use0_0().is_proxy(b))) {
        return false;
    };
    return crate::runtime_routing::eq__RemoteRef_RemoteRef(&crate::runtime_routing::identity(a), &crate::runtime_routing::identity(b));
}

pub fn import_addr(mut node: i64, mut actor: i64, mut bits: i64) -> i32 {
    let mut r: crate::runtime_routing::RemoteRef = crate::runtime_routing::RemoteRef { node: node, actor: actor, bits: bits };
    let mut here: i64 = crate::runtime_routing::here_node();
    let mut found: crate::unions::Union3<crate::runtime_routing::Found, crate::runtime_routing::MakeProxy, crate::runtime_routing::MakeDead> = crate::runtime_routing::__module_use0_0().find_import(&(r).clone(), here);
    if matches!(found, crate::unions::Union3::U1(_)) {
        let mut f = match &found { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        return f.idx;
    };
    let mut idx: i32 = crate::runtime::spawn_inert();
    if matches!(found, crate::unions::Union3::U3(_)) {
        let mut found_1 = match &found { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        crate::runtime::kill_actor(idx, String::from("unknown identity"));
        return crate::runtime_routing::__module_use0_0().register_dead(idx);
    };
    let mut got: i32 = crate::runtime_routing::__module_use0_0().register_proxy(r, idx, here);
    if (got == idx) {
        crate::runtime::mark_proxy(idx);
    };
    crate::runtime_routing::flush();
    return got;
}

pub fn remote(mut addr: i32) -> bool {
    return crate::runtime_routing::__module_use0_0().is_proxy(addr);
}

pub fn send_remote(mut addr: i32, mut proto: String, mut payload: crate::core_bytes::Bytes) {
    let mut r: Option<crate::runtime_routing::RemoteRef> = crate::runtime_routing::__module_use0_0().proxy_ref(addr);
    if r.is_none() {
        return;
    };
    loop {
        if !(true) {
            break;
        };
        let mut got: i32 = crate::runtime_routing::__module_use0_0().take_credit(addr, crate::runtime::this_parker_platform());
        if (got == 1i32) {
            let mut from: i64 = crate::runtime_routing::here_node();
            let mut r_1 = r.as_ref().unwrap();
            let mut frame: crate::core_bytes::Bytes = { let __enc: crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame> = crate::unions::Union5::U1(crate::runtime_routing::MsgFrame { to: r_1.node, actor: r_1.actor, bits: r_1.bits, from: from, proto: proto.clone(), payload: payload.clone() }); crate::wire::salvo_encode(&__enc) };
            crate::runtime_routing::__module_use0_0().stage(from, r_1.node, frame);
            crate::runtime_routing::flush();
            return;
        };
        if ((got < 0i32) || crate::runtime::mailbox_dead(addr)) {
            return;
        };
        crate::runtime::park_platform(&crate::runtime::this_parker_platform());
        if crate::runtime::mailbox_dead(addr) {
            return;
        };
    }
}

pub fn answer_remote(mut t: crate::runtime_routing::ReplyParts, mut payload: crate::core_bytes::Bytes) {
    let mut from: i64 = crate::runtime_routing::here_node();
    let mut frame: crate::core_bytes::Bytes = { let __enc: crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame> = crate::unions::Union5::U2(crate::runtime_routing::AnswerFrame { to: t.node, kind: t.kind, id: t.id, slot: t.slot, bits: t.bits, payload: payload.clone() }); crate::wire::salvo_encode(&__enc) };
    crate::runtime_routing::__module_use0_0().stage(from, t.node, frame);
    crate::runtime_routing::flush();
}

pub fn export_reply(mut e: crate::runtime::Exported) -> crate::runtime_routing::ReplyParts {
    let mut __destructured_1: crate::runtime::Exported = e;
    let mut kind: i32 = __destructured_1.kind;
    let mut id: i32 = __destructured_1.id;
    let mut slot: i64 = __destructured_1.slot;
    let mut body: Option<crate::runtime::Body> = __destructured_1.body;
    if (kind == 2i32) {
        if body.is_some() {
            let mut b = body.unwrap();
            crate::runtime_routing::__module_use0_0().put_task(slot, crate::runtime_routing::ExportedTask { pool: id, body: b });
        };
        return crate::runtime_routing::ReplyParts { node: crate::runtime_routing::__module_use0_0().node_of_pool(id), kind: 2i32, id: slot, slot: slot, bits: 0i64 };
    };
    if body.is_some() {
        let mut b = body.unwrap();
        crate::runtime::drop_body_platform(b);
    };
    if (kind == 1i32) {
        return crate::runtime_routing::ReplyParts { node: crate::runtime_routing::__module_use0_0().node_of_pool(crate::runtime::waiter_pool(id)), kind: 1i32, id: ((id) as i64), slot: slot, bits: 0i64 };
    };
    let mut me: crate::runtime_routing::RemoteRef = crate::runtime_routing::identity(id);
    return crate::runtime_routing::ReplyParts { node: me.node, kind: 0i32, id: me.actor, slot: slot, bits: me.bits };
}

pub fn credit_back(mut addr: i32, mut pool: i32, mut from: i64) {
    crate::runtime_routing::__module_use0_0().grant(addr, pool, from, 1i32);
}

pub fn flush() {
    let mut out: Vec<crate::runtime_routing::Staged> = crate::runtime_routing::__module_use0_0().take_outbox();
    for mut s in out.iter() {
        crate::runtime_routing::wire_out_platform(s.from, (s.to).clone(), (s.frame).clone());
    }
}

pub fn route(mut node: i64, mut at: crate::core_bytes::Bytes) {
    crate::runtime_routing::__module_use0_0().add_route(node, at);
    crate::runtime_routing::flush();
}

pub fn outbound_bound() {
    crate::runtime_routing::__module_use0_0().set_outbound(crate::runtime_routing::here_node());
    crate::runtime_routing::flush();
}

pub fn connected() -> bool {
    return crate::runtime_routing::__module_use0_0().has_outbound(crate::runtime_routing::here_node());
}

pub fn credits(mut addr: i32) -> Option<i32> {
    return crate::runtime_routing::__module_use0_0().credits_of(addr);
}

pub fn pending(mut addr: i32) -> i32 {
    if crate::runtime_routing::__module_use0_0().is_proxy(addr) {
        return crate::runtime_routing::__module_use0_0().held(addr);
    };
    return crate::runtime::mailbox_queued(addr);
}

pub fn watch_control(mut channel: String, mut sink: i32) {
    crate::runtime_routing::__module_use0_0().watch_channel(crate::runtime_routing::here_node(), channel, sink);
}

pub fn send_control(mut to: i64, mut channel: String, mut payload: crate::core_bytes::Bytes) {
    let mut from: i64 = crate::runtime_routing::here_node();
    let mut frame: crate::core_bytes::Bytes = { let __enc: crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame> = crate::unions::Union5::U5(crate::runtime_routing::ControlFrame { to: to, from: from, channel: channel.clone(), payload: payload.clone() }); crate::wire::salvo_encode(&__enc) };
    crate::runtime_routing::__module_use0_0().stage(from, to, frame);
    crate::runtime_routing::flush();
}

pub fn control_frame(mut channel: String, mut payload: crate::core_bytes::Bytes) -> crate::core_bytes::Bytes {
    return { let __enc: crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame> = crate::unions::Union5::U5(crate::runtime_routing::ControlFrame { to: 0i64, from: crate::runtime_routing::here_node(), channel: channel.clone(), payload: payload.clone() }); crate::wire::salvo_encode(&__enc) };
}

pub fn node_left(mut node: i64) {
    for mut idx in crate::runtime_routing::__module_use0_0().forget_node(node).iter().copied() {
        crate::runtime::kill_actor(idx, String::from("node left"));
    }
}

pub fn register_protocols(mut table: Vec<(String, String)>) {
    crate::runtime_routing::__module_use0_0().set_protocols(table);
}

pub fn local_protocols() -> Vec<(String, String)> {
    return crate::runtime_routing::__module_use0_0().protocols();
}

pub fn set_peer_protocols(mut node: i64, mut table: Vec<(String, String)>) {
    crate::runtime_routing::__module_use0_0().set_peer(node, table);
}

pub fn peer_protocol(mut node: i64, protocol: &String) -> Option<String> {
    return crate::runtime_routing::__module_use0_0().peer_hash(node, protocol);
}

pub fn deliver(data: &crate::core_bytes::Bytes) -> bool {
    let mut f: Option<crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame>> = crate::wire::salvo_decode::<crate::unions::Union5<crate::runtime_routing::MsgFrame, crate::runtime_routing::AnswerFrame, crate::runtime_routing::GrantFrame, crate::runtime_routing::OpenFrame, crate::runtime_routing::ControlFrame>>(&data);
    if matches!(f, Some(crate::unions::Union5::U1(_))) {
        let mut m = match &f { Some(crate::unions::Union5::U1(__v)) => __v, _ => unreachable!() };
        if !(crate::runtime_routing::__module_use0_0().accepts(m.to, m.actor, m.bits)) {
            return false;
        };
        let mut idx: i32 = ((m.actor) as i32);
        let mut msg: Option<crate::runtime::Dyn> = crate::runtime_routing::decode_message_platform(idx, &(m.proto).clone(), &(m.payload).clone());
        if msg.is_some() {
            let mut v = msg.unwrap();
            crate::runtime_routing::__module_use0_0().received(idx);
            let mut _queued: bool = crate::runtime::deliver_remote(idx, v, m.from);
            return true;
        };
        return false;
    };
    if matches!(f, Some(crate::unions::Union5::U2(_))) {
        let mut a = match &f { Some(crate::unions::Union5::U2(__v)) => __v, _ => unreachable!() };
        return crate::runtime_routing::deliver_answer(a);
    };
    if matches!(f, Some(crate::unions::Union5::U3(_))) {
        let mut g = match &f { Some(crate::unions::Union5::U3(__v)) => __v, _ => unreachable!() };
        return crate::runtime_routing::__module_use0_0().credited(crate::runtime_routing::RemoteRef { node: g.host, actor: g.actor, bits: g.bits }, g.to, g.n);
    };
    if matches!(f, Some(crate::unions::Union5::U4(_))) {
        let mut o = match &f { Some(crate::unions::Union5::U4(__v)) => __v, _ => unreachable!() };
        if !(crate::runtime_routing::__module_use0_0().accepts(o.to, o.actor, o.bits)) {
            return false;
        };
        let mut idx: i32 = ((o.actor) as i32);
        let mut room: i32 = i32::wrapping_sub(crate::runtime::mailbox_room(idx), crate::runtime_routing::__module_use0_0().held(idx));
        if (room < 1i32) {
            room = 1i32;
        };
        crate::runtime_routing::__module_use0_0().grant(idx, crate::runtime::actor_pool(idx), o.from, room);
        crate::runtime_routing::flush();
        return true;
    };
    if matches!(f, Some(crate::unions::Union5::U5(_))) {
        let mut c = match &f { Some(crate::unions::Union5::U5(__v)) => __v, _ => unreachable!() };
        let mut node: i64 = c.to;
        if (node == 0i64) {
            node = crate::runtime_routing::here_node();
        };
        if !(crate::runtime_routing::__module_use0_0().hosts(node)) {
            return false;
        };
        let mut sink: i32 = crate::runtime_routing::__module_use0_0().channel_sink(node, &(c.channel).clone());
        if (sink < 0i32) {
            return false;
        };
        let mut msg: Option<crate::runtime::Dyn> = crate::runtime_routing::control_message_platform(sink, c.from, &(c.payload).clone());
        if msg.is_some() {
            let mut v = msg.unwrap();
            let mut queued: bool = crate::runtime::deliver_remote(sink, v, ((i32::wrapping_neg(1i32)) as i64));
            return queued;
        };
        return false;
    };
    return false;
}

pub fn deliver_answer(a: &crate::runtime_routing::AnswerFrame) -> bool {
    if !(crate::runtime_routing::__module_use0_0().hosts(a.to)) {
        return false;
    };
    if (a.kind == 0i32) {
        if !(crate::runtime_routing::__module_use0_0().accepts(a.to, a.id, a.bits)) {
            return false;
        };
        crate::runtime::answer(crate::runtime::token_to_actor(((a.id) as i32), a.slot), crate::runtime_routing::raw_answer_platform((a.payload).clone()));
        return true;
    };
    if (a.kind == 1i32) {
        let mut wid: i32 = ((a.id) as i32);
        let mut v: Option<crate::runtime::Dyn> = crate::runtime_routing::decode_waiter_answer_platform(wid, &(a.payload).clone());
        if v.is_some() {
            let mut value = v.unwrap();
            crate::runtime::answer(crate::runtime::token_to_waiter(wid, a.slot), value);
            return true;
        };
        return false;
    };
    let mut t: Option<crate::runtime_routing::ExportedTask> = crate::runtime_routing::__module_use0_0().take_task(a.id);
    if t.is_some() {
        let mut task = t.unwrap();
        let mut v: Option<crate::runtime::Dyn> = crate::runtime_routing::decode_task_answer_platform(a.id, &(a.payload).clone());
        let mut __destructured_1: crate::runtime_routing::ExportedTask = task;
        let mut pool: i32 = __destructured_1.pool;
        let mut body: crate::runtime::Body = __destructured_1.body;
        if v.is_some() {
            let mut value = v.unwrap();
            crate::runtime::answer(crate::runtime::mint_task_on(pool, body), value);
            return true;
        };
        crate::runtime::drop_body_platform(body);
    };
    return false;
}

pub fn view_set(mut group: i32, mut members: Vec<i32>) {
    crate::runtime_routing::__module_use0_0().set_view(group, members);
}

pub fn view_members(mut group: i32) -> Vec<i32> {
    let mut left: Vec<i32> = vec![];
    for mut m in crate::runtime_routing::__module_use0_0().view_of(group).iter().copied() {
        crate::core_list::add_platform::<i32>(&mut left, m);
    }
    let mut out: Vec<i32> = vec![];
    loop {
        if !((crate::core_list::size_platform::<i32>(&left) > 0i32)) {
            break;
        };
        let mut best: i32 = 0i32;
        let mut i: i32 = 1i32;
        loop {
            if !((i < crate::core_list::size_platform::<i32>(&left))) {
                break;
            };
            if crate::runtime_routing::before(&crate::runtime_routing::identity({
                let mut __nn_1: Option<i32> = crate::core_list::get_platform::<i32>(&left, i).copied();
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.routing:937:37");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            }), &crate::runtime_routing::identity({
                let mut __nn_3: Option<i32> = crate::core_list::get_platform::<i32>(&left, best).copied();
                if __nn_3.is_none() {
                    panic!("salvo: value is absent at runtime.routing:937:68");
                } else {
                    let mut __some_4 = __nn_3.unwrap();
                    __some_4
                }
            })) {
                best = i;
            };
            i = i32::wrapping_add(i, 1i32);
        }
        { let __arg1 = {
            let mut __nn_5: Option<i32> = crate::core_list::remove_at_platform::<i32>(&mut left, best);
            if __nn_5.is_none() {
                panic!("salvo: value is absent at runtime.routing:942:18");
            } else {
                let mut __some_6 = __nn_5.unwrap();
                __some_6
            }
        }; crate::core_list::add_platform::<i32>(&mut out, __arg1) };
    }
    return (out).clone();
}

pub fn before(a: &crate::runtime_routing::RemoteRef, b: &crate::runtime_routing::RemoteRef) -> bool {
    if (a.node != b.node) {
        return (a.node < b.node);
    };
    return (a.actor <= b.actor);
}

pub fn view_version(mut group: i32) -> i64 {
    return crate::runtime_routing::__module_use0_0().version_of(group);
}

pub fn view_refresh(mut group: i32) {
    crate::runtime_routing::__module_use0_0().bump(group);
}

pub fn view_wait(mut group: i32, mut seen: i64, mut nanos: i64) {
    let mut me: crate::runtime::Parker = crate::runtime::this_parker_platform();
    let mut id: i64 = crate::runtime_routing::__module_use0_0().add_view_waiter(group, seen, (me).clone());
    if (id < 0i64) {
        return;
    };
    crate::runtime::park_nanos_platform(&me, nanos);
    crate::runtime_routing::__module_use0_0().drop_view_waiter(id);
}

pub fn hash__RemoteRef(value: &crate::runtime_routing::RemoteRef) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.node), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.actor), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.bits), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__RemoteRef_RemoteRef(a: &crate::runtime_routing::RemoteRef, b: &crate::runtime_routing::RemoteRef) -> bool {
    if !(((a.node) == (b.node))) {
        return false;
    };
    if !(((a.actor) == (b.actor))) {
        return false;
    };
    if !(((a.bits) == (b.bits))) {
        return false;
    };
    return true;
}

pub fn hash__ControlKey(value: &crate::runtime_routing::ControlKey) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.node), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.channel[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__ControlKey_ControlKey(a: &crate::runtime_routing::ControlKey, b: &crate::runtime_routing::ControlKey) -> bool {
    if !(((a.node) == (b.node))) {
        return false;
    };
    if !((&a.channel[..] == &b.channel[..])) {
        return false;
    };
    return true;
}
