use crate::std_core_bytes::Bytes;
use crate::std_time::Duration;
use crate::std_core_map::Map;
use crate::std_core_map::Mut_Map_platform;
use crate::std_core_set::Mut_Set_platform;
use crate::std_runtime_routing::RemoteRef;
use crate::std_core_set::Set;
use crate::std_core_map::contains_key_platform;
use crate::std_core_set::contains_platform;
use crate::std_runtime_routing::control_frame;
use crate::std_core_actor::eq;
use crate::std_runtime_routing::here_node;
use crate::std_runtime_routing::identity;
use crate::std_core_map::keys_platform;
use crate::std_runtime_routing::local_protocols;
use crate::std_core_compare::mix_hash;
use crate::std_runtime_routing::outbound_bound;
use crate::std_core_actor::pool;
use crate::std_core_map::put_platform;
use crate::std_core_list::remove_at_platform;
use crate::std_core_list::size_platform;


#[derive(Clone, Debug, PartialEq)]
pub struct NodeEndpoint {
    pub host: String,
    pub port: i32,
}

impl crate::wire::__Wire for NodeEndpoint {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.host, out);
        crate::wire::__Wire::__enc(&self.port, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            host: crate::wire::__Wire::__dec(r)?,
            port: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn to_str__NodeEndpoint(e: &crate::std_net::NodeEndpoint) -> String {
    return format!("{}:{}", e.host, e.port);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unreachable {
    pub to: crate::std_net::NodeEndpoint,
}

impl crate::wire::__Wire for Unreachable {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WireFailed {
    pub to: crate::std_net::NodeEndpoint,
    pub reason: String,
}

impl crate::wire::__Wire for WireFailed {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.reason, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            reason: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub type NetError = crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>;

/// Factories for the host: one per arm of the union [platform-factory].
impl NetError {
    pub fn unreachable(value: crate::std_net::Unreachable) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn wire_failed(value: crate::std_net::WireFailed) -> Self {
        crate::unions::Union2::U2(value)
    }
}

pub fn to_str__NetError(mut e: crate::unions::Union2<&crate::std_net::Unreachable, &crate::std_net::WireFailed>) -> String {
    return if matches!(e, crate::unions::Union2::U1(_)) {
        let mut e_1 = match e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return format!("unreachable: {}", crate::std_net::to_str__NodeEndpoint(&e_1.to));
    } else if matches!(e, crate::unions::Union2::U2(_)) {
        let mut e_2 = match e { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return format!("wire failed to {}: {}", crate::std_net::to_str__NodeEndpoint(&e_2.to), e_2.reason);
    } else {
        panic!("salvo: unhandled union arm at std.net:102:5");
    };
}

pub trait __Stateless_Inbound: Send + Sync {
    fn receive_frame(&self, from: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes);
}

pub trait __Stateful_Inbound: Send {
    fn receive_frame(&mut self, from: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes);
}

pub struct Inbound {
    inner: __Inner_Inbound,
}

pub enum __Inner_Inbound {
    Shared(std::sync::Arc<dyn __Stateless_Inbound>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Inbound>>),
}

impl Clone for Inbound {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Inbound::Shared(h) => __Inner_Inbound::Shared(h.clone()),
            __Inner_Inbound::Locked(h) => __Inner_Inbound::Locked(h.clone()),
        } }
    }
}

impl Inbound {
    pub fn shared<__H: __Stateless_Inbound + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Inbound::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Inbound>) -> Self {
        Self { inner: __Inner_Inbound::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Inbound + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Inbound::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Inbound>>) -> Self {
        Self { inner: __Inner_Inbound::Locked(inner) }
    }
    pub fn receive_frame(&self, from: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) {
        match &self.inner {
            __Inner_Inbound::Shared(h) => h.receive_frame(from, frame),
            __Inner_Inbound::Locked(h) => h.lock().unwrap().receive_frame(from, frame),
        }
    }
}

pub trait __Stateless_Transport: Send + Sync {
    fn listen(&self, at: &crate::std_net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>>;
    fn unlisten(&self, at: &crate::std_net::NodeEndpoint);
    fn deliver(&self, to: &crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>>;
    fn local_endpoint(&self) -> crate::std_net::NodeEndpoint;
}

pub trait __Stateful_Transport: Send {
    fn listen(&mut self, at: &crate::std_net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>>;
    fn unlisten(&mut self, at: &crate::std_net::NodeEndpoint);
    fn deliver(&mut self, to: &crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>>;
    fn local_endpoint(&mut self) -> crate::std_net::NodeEndpoint;
}

pub struct Transport {
    inner: __Inner_Transport,
}

pub enum __Inner_Transport {
    Shared(std::sync::Arc<dyn __Stateless_Transport>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Transport>>),
}

impl Clone for Transport {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Transport::Shared(h) => __Inner_Transport::Shared(h.clone()),
            __Inner_Transport::Locked(h) => __Inner_Transport::Locked(h.clone()),
        } }
    }
}

impl Transport {
    pub fn shared<__H: __Stateless_Transport + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Transport::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Transport>) -> Self {
        Self { inner: __Inner_Transport::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Transport + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Transport::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Transport>>) -> Self {
        Self { inner: __Inner_Transport::Locked(inner) }
    }
    pub fn listen(&self, at: &crate::std_net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.listen(at, sink),
            __Inner_Transport::Locked(h) => h.lock().unwrap().listen(at, sink),
        }
    }
    pub fn unlisten(&self, at: &crate::std_net::NodeEndpoint) {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.unlisten(at),
            __Inner_Transport::Locked(h) => h.lock().unwrap().unlisten(at),
        }
    }
    pub fn deliver(&self, to: &crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.deliver(to, frame),
            __Inner_Transport::Locked(h) => h.lock().unwrap().deliver(to, frame),
        }
    }
    pub fn local_endpoint(&self) -> crate::std_net::NodeEndpoint {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.local_endpoint(),
            __Inner_Transport::Locked(h) => h.lock().unwrap().local_endpoint(),
        }
    }
}

/// The adapter a `use` of a platform handler of `Transport` constructs [platform-abi].
pub struct __Platform_Transport<T>(pub T);

/// What a `threadsafe platform handler` of `Transport` implements [platform-abi].
pub trait TransportPlatformSync: Send + Sync {
    fn listen(&self, at: &crate::std_net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>>;
    fn unlisten(&self, at: &crate::std_net::NodeEndpoint);
    fn deliver(&self, to: &crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>>;
    fn local_endpoint(&self) -> crate::std_net::NodeEndpoint;
}

impl<T: TransportPlatformSync> __Stateless_Transport for __Platform_Transport<T> {
    fn listen(&self, at: &crate::std_net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        self.0.listen(at, sink)
    }
    fn unlisten(&self, at: &crate::std_net::NodeEndpoint) {
        self.0.unlisten(at)
    }
    fn deliver(&self, to: &crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        self.0.deliver(to, frame)
    }
    fn local_endpoint(&self) -> crate::std_net::NodeEndpoint {
        self.0.local_endpoint()
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct Listen;

impl Listen {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct Deliver;

impl Deliver {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        crate::unions::Union2::U2(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeId {
    pub value: i64,
}

impl crate::wire::__Wire for NodeId {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn this_node() -> crate::std_net::NodeId {
    return crate::std_net::NodeId { value: crate::std_runtime_routing::here_node() };
}

pub fn new_node() -> crate::std_net::NodeId {
    return crate::std_net::NodeId { value: crate::std_runtime_routing::new_node() };
}

pub fn pool_at(node: &crate::std_net::NodeId, mut size: i32) -> usize {
    return ((crate::std_runtime_routing::pool_at(node.value, size)) as usize);
}

pub fn add_route(node: &crate::std_net::NodeId, at: &crate::std_net::NodeEndpoint) {
    crate::std_runtime_routing::route(node.value, { let __enc: crate::std_net::NodeEndpoint = (at).clone(); crate::wire::salvo_encode(&__enc) });
}

pub fn route_frames(mut out: usize) {
    crate::std_net::bind_outbound_platform(crate::std_runtime_routing::here_node(), (((out).clone()) as i32), |__a0: i32, __a1: &crate::std_core_bytes::Bytes, __a2: crate::std_core_bytes::Bytes| crate::std_net::forward_frame(__a0, __a1, __a2));
    crate::std_runtime_routing::outbound_bound();
}

pub fn forward_frame(mut out: i32, to: &crate::std_core_bytes::Bytes, mut frame: crate::std_core_bytes::Bytes) {
    let mut __subject_1: Option<crate::std_net::NodeEndpoint> = crate::wire::salvo_decode::<crate::std_net::NodeEndpoint>(&to);
    if __subject_1.is_some() {
        let mut ep = __subject_1.unwrap();
        let mut sending: usize = ((out) as usize);
        crate::scheduler::salvo_send_wire(sending, crate::std_net::__Msg_Outbound::SendFrame(ep, frame), crate::std_net::__PROTO_Outbound);
    };
}

pub fn bind_outbound_platform(mut node: i64, mut out: i32, hook: fn(i32, &crate::std_core_bytes::Bytes, crate::std_core_bytes::Bytes)) {
    crate::platform_std_net::bind_outbound(node, out, hook)
}

pub fn deliver_frame(data: &crate::std_core_bytes::Bytes) -> bool {
    return crate::std_runtime_routing::deliver(data);
}

pub fn credits(mut a: usize) -> Option<i32> {
    let mut c: Option<i32> = crate::std_runtime_routing::credits((((a).clone()) as i32));
    if c.is_none() {
        return None;
    };
    let mut c_1 = c.unwrap();
    if (c_1 < 0i32) {
        return Some(0i32);
    };
    let mut c_2 = c.unwrap();
    return Some(c_2);
}

pub trait __Stateless_Outbound: Send + Sync {
    fn send_frame(&self, to: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes);
}

pub trait __Stateful_Outbound: Send {
    fn send_frame(&mut self, to: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes);
}

pub struct Outbound {
    inner: __Inner_Outbound,
}

pub enum __Inner_Outbound {
    Shared(std::sync::Arc<dyn __Stateless_Outbound>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Outbound>>),
}

impl Clone for Outbound {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Outbound::Shared(h) => __Inner_Outbound::Shared(h.clone()),
            __Inner_Outbound::Locked(h) => __Inner_Outbound::Locked(h.clone()),
        } }
    }
}

impl Outbound {
    pub fn shared<__H: __Stateless_Outbound + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Outbound::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Outbound>) -> Self {
        Self { inner: __Inner_Outbound::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Outbound + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Outbound::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Outbound>>) -> Self {
        Self { inner: __Inner_Outbound::Locked(inner) }
    }
    pub fn send_frame(&self, to: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) {
        match &self.inner {
            __Inner_Outbound::Shared(h) => h.send_frame(to, frame),
            __Inner_Outbound::Locked(h) => h.lock().unwrap().send_frame(to, frame),
        }
    }
}

pub struct Sending {
    __dep0: crate::std_net::Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sending>,
}

impl Sending {
    pub fn new(__dep0: crate::std_net::Transport) -> Self {
        Self {
            __dep0,
            __mailbox_capacity: 256i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::std_net::__Stateless_Outbound for Sending {
    fn send_frame(&self, to: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) {
        let mut _sent: crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> = self.__dep0.deliver(&to, frame);
    }
}

pub struct __Actor_Sending {
    handler: Sending,
}

impl __Actor_Sending {
    pub fn new(handler: Sending) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Sending {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::std_net::__Msg_Outbound>().expect("message of this protocol");
        crate::std_net::__dispatch_Sending_Outbound(&mut self.handler, msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Sending::SendFrame(to) => crate::std_net::__dispatch_Sending_Outbound(&mut self.handler, crate::std_net::__Msg_Outbound::SendFrame(to, *value.downcast::<crate::std_core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sending::SendFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sending: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sending);
fn __decode_msg_Sending(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_net::__PROTO_Outbound {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_Outbound>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub struct Receiving {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Receiving>,
}

impl Receiving {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 256i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::std_net::__Stateless_Inbound for Receiving {
    fn receive_frame(&self, from: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) {
        let mut _delivered: bool = crate::std_net::deliver_frame(&frame);
    }
}

pub struct __Actor_Receiving {
    handler: Receiving,
}

impl __Actor_Receiving {
    pub fn new(handler: Receiving) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Receiving {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::std_net::__Msg_Inbound>().expect("message of this protocol");
        crate::std_net::__dispatch_Receiving_Inbound(&mut self.handler, msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Receiving::ReceiveFrame(from) => crate::std_net::__dispatch_Receiving_Inbound(&mut self.handler, crate::std_net::__Msg_Inbound::ReceiveFrame(from, *value.downcast::<crate::std_core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Receiving::ReceiveFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Receiving: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Receiving);
fn __decode_msg_Receiving(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_net::__PROTO_Inbound {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_Inbound>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn connected() -> bool {
    return crate::std_runtime_routing::connected();
}

pub fn connect__NodeEndpoint(transport: &crate::std_net::Transport, mut me: crate::std_net::NodeEndpoint) -> bool {
    return crate::std_net::connect__NodeEndpoint_Pool(transport, me, crate::std_core_actor::pool(1i32));
}

pub fn connect__NodeEndpoint_Pool(transport: &crate::std_net::Transport, mut me: crate::std_net::NodeEndpoint, mut on: usize) -> bool {
    if crate::std_net::connected() {
        return false;
    };
    let mut sending: usize = ({ let __h = crate::std_net::Sending::new(transport.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, std::boxed::Box::new(crate::std_net::__Actor_Sending::new(__h)), crate::std_net::__DECODE_Sending); __a });
    let mut receiving: usize = ({ let __h = crate::std_net::Receiving::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, std::boxed::Box::new(crate::std_net::__Actor_Receiving::new(__h)), crate::std_net::__DECODE_Receiving); __a });
    crate::std_net::route_frames(sending);
    let mut _listening: crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> = transport.listen(&(me).clone(), receiving);
    crate::std_net::add_route(&crate::std_net::this_node(), &me);
    return true;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: crate::std_net::NodeId,
    pub at: crate::std_net::NodeEndpoint,
}

impl crate::wire::__Wire for Node {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.id, out);
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            id: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub trait __Stateless_NodeGroup: Send + Sync {
    fn members(&self, out: crate::scheduler::SalvoReply);
    fn subscribe(&self, w: usize);
    fn leave(&self);
}

pub trait __Stateful_NodeGroup: Send {
    fn members(&mut self, out: crate::scheduler::SalvoReply);
    fn subscribe(&mut self, w: usize);
    fn leave(&mut self);
}

pub struct NodeGroup {
    inner: __Inner_NodeGroup,
}

pub enum __Inner_NodeGroup {
    Shared(std::sync::Arc<dyn __Stateless_NodeGroup>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_NodeGroup>>),
}

impl Clone for NodeGroup {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_NodeGroup::Shared(h) => __Inner_NodeGroup::Shared(h.clone()),
            __Inner_NodeGroup::Locked(h) => __Inner_NodeGroup::Locked(h.clone()),
        } }
    }
}

impl NodeGroup {
    pub fn shared<__H: __Stateless_NodeGroup + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_NodeGroup::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_NodeGroup>) -> Self {
        Self { inner: __Inner_NodeGroup::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_NodeGroup + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_NodeGroup::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_NodeGroup>>) -> Self {
        Self { inner: __Inner_NodeGroup::Locked(inner) }
    }
    pub fn members(&self, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_NodeGroup::Shared(h) => h.members(out),
            __Inner_NodeGroup::Locked(h) => h.lock().unwrap().members(out),
        }
    }
    pub fn subscribe(&self, w: usize) {
        match &self.inner {
            __Inner_NodeGroup::Shared(h) => h.subscribe(w),
            __Inner_NodeGroup::Locked(h) => h.lock().unwrap().subscribe(w),
        }
    }
    pub fn leave(&self) {
        match &self.inner {
            __Inner_NodeGroup::Shared(h) => h.leave(),
            __Inner_NodeGroup::Locked(h) => h.lock().unwrap().leave(),
        }
    }
}

pub trait __Stateless_NodeGroupWatcher: Send + Sync {
    fn joined(&self, n: crate::std_net::Node);
    fn left(&self, n: crate::std_net::Node, why: String);
}

pub trait __Stateful_NodeGroupWatcher: Send {
    fn joined(&mut self, n: crate::std_net::Node);
    fn left(&mut self, n: crate::std_net::Node, why: String);
}

pub struct NodeGroupWatcher {
    inner: __Inner_NodeGroupWatcher,
}

pub enum __Inner_NodeGroupWatcher {
    Shared(std::sync::Arc<dyn __Stateless_NodeGroupWatcher>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_NodeGroupWatcher>>),
}

impl Clone for NodeGroupWatcher {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => __Inner_NodeGroupWatcher::Shared(h.clone()),
            __Inner_NodeGroupWatcher::Locked(h) => __Inner_NodeGroupWatcher::Locked(h.clone()),
        } }
    }
}

impl NodeGroupWatcher {
    pub fn shared<__H: __Stateless_NodeGroupWatcher + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_NodeGroupWatcher::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_NodeGroupWatcher>) -> Self {
        Self { inner: __Inner_NodeGroupWatcher::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_NodeGroupWatcher + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_NodeGroupWatcher::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_NodeGroupWatcher>>) -> Self {
        Self { inner: __Inner_NodeGroupWatcher::Locked(inner) }
    }
    pub fn joined(&self, n: crate::std_net::Node) {
        match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => h.joined(n),
            __Inner_NodeGroupWatcher::Locked(h) => h.lock().unwrap().joined(n),
        }
    }
    pub fn left(&self, n: crate::std_net::Node, why: String) {
        match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => h.left(n, why),
            __Inner_NodeGroupWatcher::Locked(h) => h.lock().unwrap().left(n, why),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hello {
    pub group: String,
    pub at: crate::std_net::NodeEndpoint,
    pub protocols: Vec<(String, String)>,
}

impl crate::wire::__Wire for Hello {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.group, out);
        crate::wire::__Wire::__enc(&self.at, out);
        crate::wire::__Wire::__enc(&self.protocols, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            group: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
            protocols: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ack {
    pub group: String,
    pub at: crate::std_net::NodeEndpoint,
    pub protocols: Vec<(String, String)>,
}

impl crate::wire::__Wire for Ack {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.group, out);
        crate::wire::__Wire::__enc(&self.at, out);
        crate::wire::__Wire::__enc(&self.protocols, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            group: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
            protocols: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Leaving {
}

impl crate::wire::__Wire for Leaving {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Intro {
    pub peers: Vec<crate::std_net::NodeEndpoint>,
}

impl crate::wire::__Wire for Intro {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.peers, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            peers: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn send_control(to: &crate::std_net::NodeId, channel: &String, mut payload: crate::std_core_bytes::Bytes) {
    crate::std_runtime_routing::send_control(to.value, (channel).clone(), payload);
}

pub fn node_left(node: &crate::std_net::NodeId) {
    crate::std_runtime_routing::node_left(node.value);
}

pub fn set_peer_protocols(node: &crate::std_net::NodeId, table: &Vec<(String, String)>) {
    crate::std_runtime_routing::set_peer_protocols(node.value, (table).clone());
}

pub fn peer_protocol(node: &crate::std_net::NodeId, protocol: &String) -> Option<String> {
    return crate::std_runtime_routing::peer_protocol(node.value, protocol);
}

pub fn hello_frame(transport: &crate::std_net::Transport, group: &String) -> crate::std_core_bytes::Bytes {
    let mut hello: crate::unions::Union4<crate::std_net::Hello, crate::std_net::Ack, crate::std_net::Leaving, crate::std_net::Intro> = crate::unions::Union4::U1(crate::std_net::Hello { group: (group).clone(), at: transport.local_endpoint(), protocols: crate::std_runtime_routing::local_protocols() });
    return crate::std_runtime_routing::control_frame(String::from(""), crate::wire::salvo_encode(&hello));
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeerHello {
    pub node: crate::std_net::NodeId,
    pub at: crate::std_net::NodeEndpoint,
}

impl crate::wire::__Wire for PeerHello {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.node, out);
        crate::wire::__Wire::__enc(&self.at, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            node: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeerGone {
    pub node: crate::std_net::NodeId,
}

impl crate::wire::__Wire for PeerGone {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.node, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            node: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeerIntro {
    pub peers: Vec<crate::std_net::NodeEndpoint>,
}

impl crate::wire::__Wire for PeerIntro {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.peers, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            peers: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn handshake(transport: &crate::std_net::Transport, group: &String, mut from: crate::std_net::NodeId, mut data: crate::std_core_bytes::Bytes) -> Option<crate::unions::Union3<crate::std_net::PeerHello, crate::std_net::PeerGone, crate::std_net::PeerIntro>> {
    let mut msg: Option<crate::unions::Union4<crate::std_net::Hello, crate::std_net::Ack, crate::std_net::Leaving, crate::std_net::Intro>> = crate::wire::salvo_decode::<crate::unions::Union4<crate::std_net::Hello, crate::std_net::Ack, crate::std_net::Leaving, crate::std_net::Intro>>(&data);
    return if matches!(msg, Some(crate::unions::Union4::U1(_))) {
        let mut msg_1 = match &msg { Some(crate::unions::Union4::U1(__v)) => __v, _ => unreachable!() };
        if !((&msg_1.group[..] == &group[..])) {
            return None;
        };
        crate::std_net::add_route(&(from).clone(), &(msg_1.at).clone());
        crate::std_net::set_peer_protocols(&(from).clone(), &(msg_1.protocols).clone());
        let mut ack: crate::unions::Union4<crate::std_net::Hello, crate::std_net::Ack, crate::std_net::Leaving, crate::std_net::Intro> = crate::unions::Union4::U2(crate::std_net::Ack { group: (group).clone(), at: transport.local_endpoint(), protocols: crate::std_runtime_routing::local_protocols() });
        crate::std_net::send_control(&(from).clone(), &String::from(""), crate::wire::salvo_encode(&ack));
        return Some(crate::unions::Union3::U1(crate::std_net::PeerHello { node: from, at: (msg_1.at).clone() }));
    } else if matches!(msg, Some(crate::unions::Union4::U2(_))) {
        let mut msg_2 = match &msg { Some(crate::unions::Union4::U2(__v)) => __v, _ => unreachable!() };
        if !((&msg_2.group[..] == &group[..])) {
            return None;
        };
        crate::std_net::add_route(&(from).clone(), &(msg_2.at).clone());
        crate::std_net::set_peer_protocols(&(from).clone(), &(msg_2.protocols).clone());
        return Some(crate::unions::Union3::U1(crate::std_net::PeerHello { node: from, at: (msg_2.at).clone() }));
    } else if matches!(msg, Some(crate::unions::Union4::U3(_))) {
        let mut msg_3 = match &msg { Some(crate::unions::Union4::U3(__v)) => __v, _ => unreachable!() };
        crate::std_net::node_left(&(from).clone());
        return Some(crate::unions::Union3::U2(crate::std_net::PeerGone { node: from }));
    } else if matches!(msg, Some(crate::unions::Union4::U4(_))) {
        let mut msg_4 = match &msg { Some(crate::unions::Union4::U4(__v)) => __v, _ => unreachable!() };
        return Some(crate::unions::Union3::U3(crate::std_net::PeerIntro { peers: (msg_4.peers).clone() }));
    } else if msg.is_none() {
        return None;
    } else {
        panic!("salvo: unhandled union arm at std.net:458:5");
    };
}

pub fn introduce(node: &crate::std_net::NodeId, peers: &Vec<crate::std_net::NodeEndpoint>) {
    let mut intro: crate::unions::Union4<crate::std_net::Hello, crate::std_net::Ack, crate::std_net::Leaving, crate::std_net::Intro> = crate::unions::Union4::U4(crate::std_net::Intro { peers: (peers).clone() });
    crate::std_net::send_control(&(node).clone(), &String::from(""), crate::wire::salvo_encode(&intro));
}

pub fn leave_group(known: &crate::std_core_map::Map<crate::std_net::NodeId, crate::std_net::Node>) {
    for mut id in crate::std_core_map::keys_platform::<crate::std_net::NodeId, crate::std_net::Node>(known).into_iter() {
        let mut leaving: crate::unions::Union4<crate::std_net::Hello, crate::std_net::Ack, crate::std_net::Leaving, crate::std_net::Intro> = crate::unions::Union4::U3(crate::std_net::Leaving {});
        crate::std_net::send_control(&(id).clone(), &String::from(""), crate::wire::salvo_encode(&leaving));
    }
}

pub struct StaticNodeGroup {
    name: String,
    all: Vec<crate::std_net::NodeEndpoint>,
    __dep0: crate::std_net::Transport,
    known: crate::std_core_map::Map<crate::std_net::NodeId, crate::std_net::Node>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_StaticNodeGroup>,
}

impl StaticNodeGroup {
    pub fn new(name: String, all: Vec<crate::std_net::NodeEndpoint>, __dep0: crate::std_net::Transport) -> Self {
        Self {
            name: name.clone(),
            all: all.clone(),
            __dep0,
            known: crate::std_core_map::Mut_Map_platform::<crate::std_net::NodeId, crate::std_net::Node>(vec![], &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1)),
            watchers: vec![],
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
    fn init(&mut self) {
        let mut me: crate::std_net::NodeEndpoint = self.__dep0.local_endpoint();
        let mut _connected: bool = crate::std_net::connect__NodeEndpoint(&self.__dep0, (me).clone());
        crate::scheduler::salvo_watch_control((String::from("")).clone(), (self.__addr.expect("an actor's own addr")).clone(), |__n, __d| std::boxed::Box::new(crate::std_net::__Priv_StaticNodeGroup::Control(crate::std_net::NodeId { value: __n as i64 }, __d)));
        for mut e in self.all.iter() {
            if !(crate::std_net::eq__NodeEndpoint_NodeEndpoint(e, &me)) {
                let mut _sent: crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> = self.__dep0.deliver(&(e).clone(), crate::std_net::hello_frame(&self.__dep0, &self.name));
            };
        }
    }
    fn control(&mut self, from: crate::std_net::NodeId, data: crate::std_core_bytes::Bytes) {
        let mut event: Option<crate::unions::Union3<crate::std_net::PeerHello, crate::std_net::PeerGone, crate::std_net::PeerIntro>> = crate::std_net::handshake(&self.__dep0, &self.name, from, data);
        if matches!(event, Some(crate::unions::Union3::U1(_))) {
            let mut event_1 = match &event { Some(crate::unions::Union3::U1(__v)) => __v, _ => unreachable!() };
            if crate::std_core_map::contains_key_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known, &event_1.node, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1)) {
                return;
            };
            let mut n: crate::std_net::Node = crate::std_net::Node { id: (event_1.node).clone(), at: (event_1.at).clone() };
            crate::std_core_map::put_platform::<crate::std_net::NodeId, crate::std_net::Node>(&mut self.known, (event_1.node).clone(), (n).clone(), &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
            for mut w in self.watchers.iter().copied() {
                crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_NodeGroupWatcher::Joined((n).clone()), crate::std_net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U2(_))) {
            let mut event_2 = match &event { Some(crate::unions::Union3::U2(__v)) => __v, _ => unreachable!() };
            let mut n: Option<crate::std_net::Node> = crate::std_core_map::remove_platform::<crate::std_net::NodeId, crate::std_net::Node>(&mut self.known, &event_2.node, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
            if n.is_none() {
                return;
            };
            for mut w in self.watchers.iter().copied() {
                let mut n_3 = n.as_ref().unwrap();
                crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_NodeGroupWatcher::Left((n_3).clone(), String::from("left")), crate::std_net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U3(_))) {
            let mut event_4 = match &event { Some(crate::unions::Union3::U3(__v)) => __v, _ => unreachable!() };
        } else if event.is_none() {
        } else {
            panic!("salvo: unhandled union arm at std.net:559:9");
        };
    }
}

impl crate::std_net::__Stateful_NodeGroup for StaticNodeGroup {
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<crate::std_net::Node>>(out, crate::std_net::known_nodes(&self.known));
    }
    fn subscribe(&mut self, w: usize) {
        for mut id in crate::std_core_map::keys_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known).into_iter() {
            let mut n: Option<&crate::std_net::Node> = crate::std_core_map::get_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known, &id, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
            if !(n.is_none()) {
                let mut n_1 = n.unwrap();
                crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_NodeGroupWatcher::Joined((n_1).clone()), crate::std_net::__PROTO_NodeGroupWatcher);
            };
        }
        crate::std_core_list::add_platform::<usize>(&mut self.watchers, w);
    }
    fn leave(&mut self) {
        crate::std_net::leave_group(&self.known);
    }
}

pub struct __Actor_StaticNodeGroup {
    handler: StaticNodeGroup,
}

impl __Actor_StaticNodeGroup {
    pub fn new(handler: StaticNodeGroup) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_StaticNodeGroup {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::std_net::__Msg_NodeGroup>() {
            Ok(__m) => return crate::std_net::__dispatch_StaticNodeGroup_NodeGroup(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_StaticNodeGroup>() {
            Ok(__m) => return crate::std_net::__dispatch_priv_StaticNodeGroup(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let _ = msg;
        unreachable!("a message of one of this actor's protocols")
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_StaticNodeGroup::Members => crate::std_net::__dispatch_StaticNodeGroup_NodeGroup(&mut self.handler, crate::std_net::__Msg_NodeGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Subscribe => crate::std_net::__dispatch_StaticNodeGroup_NodeGroup(&mut self.handler, crate::std_net::__Msg_NodeGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Control(from) => crate::std_net::__dispatch_priv_StaticNodeGroup(&mut self.handler, __Priv_StaticNodeGroup::Control(from, *value.downcast::<crate::std_core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_StaticNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_StaticNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_StaticNodeGroup);
fn __decode_msg_StaticNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_net::__PROTO_NodeGroup {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_NodeGroup>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn known_nodes(known: &crate::std_core_map::Map<crate::std_net::NodeId, crate::std_net::Node>) -> Vec<crate::std_net::Node> {
    let mut all_known: Vec<crate::std_net::Node> = vec![];
    for mut id in crate::std_core_map::keys_platform::<crate::std_net::NodeId, crate::std_net::Node>(known).into_iter() {
        let mut n: Option<&crate::std_net::Node> = crate::std_core_map::get_platform::<crate::std_net::NodeId, crate::std_net::Node>(known, &id, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
        if !(n.is_none()) {
            let mut n_1 = n.unwrap();
            crate::std_core_list::add_platform::<crate::std_net::Node>(&mut all_known, (n_1).clone());
        };
    }
    return all_known;
}

pub struct GossipNodeGroup {
    name: String,
    seeds: Vec<crate::std_net::NodeEndpoint>,
    __dep0: crate::std_net::Transport,
    known: crate::std_core_map::Map<crate::std_net::NodeId, crate::std_net::Node>,
    dialed: crate::std_core_set::Set<String>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_GossipNodeGroup>,
}

impl GossipNodeGroup {
    pub fn new(name: String, seeds: Vec<crate::std_net::NodeEndpoint>, __dep0: crate::std_net::Transport) -> Self {
        Self {
            name: name.clone(),
            seeds: seeds.clone(),
            __dep0,
            known: crate::std_core_map::Mut_Map_platform::<crate::std_net::NodeId, crate::std_net::Node>(vec![], &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1)),
            dialed: crate::std_core_set::Mut_Set_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])),
            watchers: vec![],
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
    fn init(&mut self) {
        let mut me: crate::std_net::NodeEndpoint = self.__dep0.local_endpoint();
        let mut _connected: bool = crate::std_net::connect__NodeEndpoint(&self.__dep0, (me).clone());
        crate::scheduler::salvo_watch_control((String::from("")).clone(), (self.__addr.expect("an actor's own addr")).clone(), |__n, __d| std::boxed::Box::new(crate::std_net::__Priv_GossipNodeGroup::Control(crate::std_net::NodeId { value: __n as i64 }, __d)));
        for mut e in self.seeds.iter() {
            crate::std_net::dial(&self.__dep0, &mut self.dialed, &self.name, (e).clone());
        }
    }
    fn control(&mut self, from: crate::std_net::NodeId, data: crate::std_core_bytes::Bytes) {
        let mut event: Option<crate::unions::Union3<crate::std_net::PeerHello, crate::std_net::PeerGone, crate::std_net::PeerIntro>> = crate::std_net::handshake(&self.__dep0, &self.name, from, data);
        if matches!(event, Some(crate::unions::Union3::U1(_))) {
            let mut event_1 = match &event { Some(crate::unions::Union3::U1(__v)) => __v, _ => unreachable!() };
            if crate::std_core_map::contains_key_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known, &event_1.node, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1)) {
                return;
            };
            let mut others: Vec<crate::std_net::NodeEndpoint> = vec![];
            for mut id in crate::std_core_map::keys_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known).into_iter() {
                let mut n: Option<&crate::std_net::Node> = crate::std_core_map::get_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known, &id, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
                if !(n.is_none()) {
                    let mut n_2 = n.unwrap();
                    crate::std_core_list::add_platform::<crate::std_net::NodeEndpoint>(&mut others, (n_2.at).clone());
                };
                crate::std_net::introduce(&(id).clone(), &vec![(event_1.at).clone()]);
            }
            crate::std_net::introduce(&(event_1.node).clone(), &others);
            crate::std_core_set::add_platform::<String>(&mut self.dialed, crate::std_net::to_str__NodeEndpoint(&event_1.at), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
            let mut n: crate::std_net::Node = crate::std_net::Node { id: (event_1.node).clone(), at: (event_1.at).clone() };
            crate::std_core_map::put_platform::<crate::std_net::NodeId, crate::std_net::Node>(&mut self.known, (event_1.node).clone(), (n).clone(), &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
            for mut w in self.watchers.iter().copied() {
                crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_NodeGroupWatcher::Joined((n).clone()), crate::std_net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U2(_))) {
            let mut event_3 = match &event { Some(crate::unions::Union3::U2(__v)) => __v, _ => unreachable!() };
            let mut n: Option<crate::std_net::Node> = crate::std_core_map::remove_platform::<crate::std_net::NodeId, crate::std_net::Node>(&mut self.known, &event_3.node, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
            if n.is_none() {
                return;
            };
            for mut w in self.watchers.iter().copied() {
                let mut n_4 = n.as_ref().unwrap();
                crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_NodeGroupWatcher::Left((n_4).clone(), String::from("left")), crate::std_net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U3(_))) {
            let mut event_5 = match &event { Some(crate::unions::Union3::U3(__v)) => __v, _ => unreachable!() };
            for mut e in event_5.peers.iter() {
                crate::std_net::dial(&self.__dep0, &mut self.dialed, &self.name, (e).clone());
            }
        } else if event.is_none() {
        } else {
            panic!("salvo: unhandled union arm at std.net:647:9");
        };
    }
}

impl crate::std_net::__Stateful_NodeGroup for GossipNodeGroup {
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<crate::std_net::Node>>(out, crate::std_net::known_nodes(&self.known));
    }
    fn subscribe(&mut self, w: usize) {
        for mut id in crate::std_core_map::keys_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known).into_iter() {
            let mut n: Option<&crate::std_net::Node> = crate::std_core_map::get_platform::<crate::std_net::NodeId, crate::std_net::Node>(&self.known, &id, &mut |__a0: &crate::std_net::NodeId| crate::std_net::hash__NodeId(__a0), &mut |__a0: &crate::std_net::NodeId, __a1: &crate::std_net::NodeId| crate::std_net::eq__NodeId_NodeId(__a0, __a1));
            if !(n.is_none()) {
                let mut n_1 = n.unwrap();
                crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_NodeGroupWatcher::Joined((n_1).clone()), crate::std_net::__PROTO_NodeGroupWatcher);
            };
        }
        crate::std_core_list::add_platform::<usize>(&mut self.watchers, w);
    }
    fn leave(&mut self) {
        crate::std_net::leave_group(&self.known);
    }
}

pub struct __Actor_GossipNodeGroup {
    handler: GossipNodeGroup,
}

impl __Actor_GossipNodeGroup {
    pub fn new(handler: GossipNodeGroup) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_GossipNodeGroup {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::std_net::__Msg_NodeGroup>() {
            Ok(__m) => return crate::std_net::__dispatch_GossipNodeGroup_NodeGroup(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_GossipNodeGroup>() {
            Ok(__m) => return crate::std_net::__dispatch_priv_GossipNodeGroup(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let _ = msg;
        unreachable!("a message of one of this actor's protocols")
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_GossipNodeGroup::Members => crate::std_net::__dispatch_GossipNodeGroup_NodeGroup(&mut self.handler, crate::std_net::__Msg_NodeGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Subscribe => crate::std_net::__dispatch_GossipNodeGroup_NodeGroup(&mut self.handler, crate::std_net::__Msg_NodeGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Control(from) => crate::std_net::__dispatch_priv_GossipNodeGroup(&mut self.handler, __Priv_GossipNodeGroup::Control(from, *value.downcast::<crate::std_core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_GossipNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_GossipNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_GossipNodeGroup);
fn __decode_msg_GossipNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_net::__PROTO_NodeGroup {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_NodeGroup>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn dial(transport: &crate::std_net::Transport, dialed: &mut crate::std_core_set::Set<String>, group: &String, mut e: crate::std_net::NodeEndpoint) {
    if (crate::std_net::eq__NodeEndpoint_NodeEndpoint(&e, &transport.local_endpoint()) || crate::std_core_set::contains_platform::<String>(&*dialed, &crate::std_net::to_str__NodeEndpoint(&e), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
        return;
    };
    crate::std_core_set::add_platform::<String>(&mut *dialed, crate::std_net::to_str__NodeEndpoint(&e), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    let mut sent: crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> = transport.deliver(&(e).clone(), crate::std_net::hello_frame(transport, group));
    if matches!(sent, crate::unions::Union2::U2(_)) {
        let mut sent_1 = match &sent { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut _forgot: bool = crate::std_core_set::remove_platform::<String>(&mut *dialed, &crate::std_net::to_str__NodeEndpoint(&e), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Protocol {
    pub name: String,
    pub hash: String,
}

impl crate::wire::__Wire for Protocol {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.name, out);
        crate::wire::__Wire::__enc(&self.hash, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            name: crate::wire::__Wire::__dec(r)?,
            hash: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub trait __Stateless_ActorGroup: Send + Sync {
    fn join(&self, member: usize);
    fn leave(&self, member: usize);
    fn members(&self, out: crate::scheduler::SalvoReply);
    fn subscribe(&self, w: usize);
    fn refresh(&self);
}

pub trait __Stateful_ActorGroup: Send {
    fn join(&mut self, member: usize);
    fn leave(&mut self, member: usize);
    fn members(&mut self, out: crate::scheduler::SalvoReply);
    fn subscribe(&mut self, w: usize);
    fn refresh(&mut self);
}

pub struct ActorGroup {
    inner: __Inner_ActorGroup,
}

pub enum __Inner_ActorGroup {
    Shared(std::sync::Arc<dyn __Stateless_ActorGroup>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_ActorGroup>>),
}

impl Clone for ActorGroup {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_ActorGroup::Shared(h) => __Inner_ActorGroup::Shared(h.clone()),
            __Inner_ActorGroup::Locked(h) => __Inner_ActorGroup::Locked(h.clone()),
        } }
    }
}

impl ActorGroup {
    pub fn shared<__H: __Stateless_ActorGroup + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_ActorGroup::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_ActorGroup>) -> Self {
        Self { inner: __Inner_ActorGroup::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_ActorGroup + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_ActorGroup::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_ActorGroup>>) -> Self {
        Self { inner: __Inner_ActorGroup::Locked(inner) }
    }
    pub fn join(&self, member: usize) {
        match &self.inner {
            __Inner_ActorGroup::Shared(h) => h.join(member),
            __Inner_ActorGroup::Locked(h) => h.lock().unwrap().join(member),
        }
    }
    pub fn leave(&self, member: usize) {
        match &self.inner {
            __Inner_ActorGroup::Shared(h) => h.leave(member),
            __Inner_ActorGroup::Locked(h) => h.lock().unwrap().leave(member),
        }
    }
    pub fn members(&self, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_ActorGroup::Shared(h) => h.members(out),
            __Inner_ActorGroup::Locked(h) => h.lock().unwrap().members(out),
        }
    }
    pub fn subscribe(&self, w: usize) {
        match &self.inner {
            __Inner_ActorGroup::Shared(h) => h.subscribe(w),
            __Inner_ActorGroup::Locked(h) => h.lock().unwrap().subscribe(w),
        }
    }
    pub fn refresh(&self) {
        match &self.inner {
            __Inner_ActorGroup::Shared(h) => h.refresh(),
            __Inner_ActorGroup::Locked(h) => h.lock().unwrap().refresh(),
        }
    }
}

pub trait __Stateless_ActorGroupWatcher: Send + Sync {
    fn joined(&self, member: usize);
    fn left(&self, member: usize);
}

pub trait __Stateful_ActorGroupWatcher: Send {
    fn joined(&mut self, member: usize);
    fn left(&mut self, member: usize);
}

pub struct ActorGroupWatcher {
    inner: __Inner_ActorGroupWatcher,
}

pub enum __Inner_ActorGroupWatcher {
    Shared(std::sync::Arc<dyn __Stateless_ActorGroupWatcher>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_ActorGroupWatcher>>),
}

impl Clone for ActorGroupWatcher {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_ActorGroupWatcher::Shared(h) => __Inner_ActorGroupWatcher::Shared(h.clone()),
            __Inner_ActorGroupWatcher::Locked(h) => __Inner_ActorGroupWatcher::Locked(h.clone()),
        } }
    }
}

impl ActorGroupWatcher {
    pub fn shared<__H: __Stateless_ActorGroupWatcher + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_ActorGroupWatcher::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_ActorGroupWatcher>) -> Self {
        Self { inner: __Inner_ActorGroupWatcher::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_ActorGroupWatcher + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_ActorGroupWatcher::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_ActorGroupWatcher>>) -> Self {
        Self { inner: __Inner_ActorGroupWatcher::Locked(inner) }
    }
    pub fn joined(&self, member: usize) {
        match &self.inner {
            __Inner_ActorGroupWatcher::Shared(h) => h.joined(member),
            __Inner_ActorGroupWatcher::Locked(h) => h.lock().unwrap().joined(member),
        }
    }
    pub fn left(&self, member: usize) {
        match &self.inner {
            __Inner_ActorGroupWatcher::Shared(h) => h.left(member),
            __Inner_ActorGroupWatcher::Locked(h) => h.lock().unwrap().left(member),
        }
    }
}

pub fn share_members(name: &String, hash: &String, node: &crate::std_net::NodeId, members: &Vec<usize>) {
    crate::std_net::send_control(&(node).clone(), &(name).clone(), { let __enc: (String, Vec<usize>) = ((hash).clone(), (members).clone()); crate::wire::salvo_encode(&__enc) });
}

pub fn pending(mut a: usize) -> i32 {
    return crate::std_runtime_routing::pending((((a).clone()) as i32));
}

pub fn actor_group__Addr(mut nodes: usize, protocol: &mut dyn FnMut() -> crate::std_net::Protocol) -> usize {
    let mut proto: crate::std_net::Protocol = protocol();
    let mut name: String = (proto.name).clone();
    return crate::std_net::actor_group__Str_Addr(name, nodes, &mut *protocol);
}

pub fn actor_group__Str_Addr(mut name: String, mut nodes: usize, protocol: &mut dyn FnMut() -> crate::std_net::Protocol) -> usize {
    let mut __destructured_1: (usize, usize) = ({ let __h = crate::std_net::ActorGrouping::new(name, protocol()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::std_core_actor::pool(1i32), __cap as usize, std::boxed::Box::new(crate::std_net::__Actor_ActorGrouping::new(__h)), crate::std_net::__DECODE_ActorGrouping); crate::scheduler::salvo_send(__a, std::boxed::Box::new(crate::std_net::__Priv_ActorGrouping::Init)); (__a, __a) });
    let mut group: usize = __destructured_1.0;
    let mut watcher: usize = __destructured_1.1;
    crate::scheduler::salvo_send_wire(nodes, crate::std_net::__Msg_NodeGroup::Subscribe(watcher), crate::std_net::__PROTO_NodeGroup);
    return group;
}

pub fn join(mut group: usize, mut member: usize) {
    crate::scheduler::salvo_send_wire(group, crate::std_net::__Msg_ActorGroup::Join(member), crate::std_net::__PROTO_ActorGroup);
}

pub struct ActorGrouping {
    name: String,
    proto: crate::std_net::Protocol,
    all: Vec<usize>,
    peers: Vec<crate::std_net::NodeId>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_ActorGrouping>,
}

impl ActorGrouping {
    pub fn new(name: String, proto: crate::std_net::Protocol) -> Self {
        Self {
            name: name.clone(),
            proto: proto.clone(),
            all: vec![],
            peers: vec![],
            watchers: vec![],
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
    fn init(&mut self) {
        crate::scheduler::salvo_watch_control(((self.name).clone()).clone(), (self.__addr.expect("an actor's own addr")).clone(), |__n, __d| std::boxed::Box::new(crate::std_net::__Priv_ActorGrouping::Control(crate::std_net::NodeId { value: __n as i64 }, __d)));
    }
    fn control(&mut self, from: crate::std_net::NodeId, data: crate::std_core_bytes::Bytes) {
        let mut got: Option<(String, Vec<usize>)> = crate::wire::salvo_decode::<(String, Vec<usize>)>(&data);
        if got.is_none() {
            return;
        };
        let mut got_1 = got.as_ref().unwrap();
        if !((&got_1.0[..] == &self.proto.hash[..])) {
            return;
        };
        if !(crate::std_net::contains_node(&self.peers, &(from).clone())) {
            crate::std_core_list::add_platform::<crate::std_net::NodeId>(&mut self.peers, (from).clone());
            crate::std_net::share_members(&self.name, &self.proto.hash, &(from).clone(), &self.all);
        };
        let mut changed: bool = false;
        let mut got_2 = got.as_ref().unwrap();
        for mut m in got_2.1.iter().copied() {
            if crate::std_net::admit(&mut self.all, m) {
                changed = true;
                for mut w in self.watchers.iter().copied() {
                    crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_ActorGroupWatcher::Joined(m), crate::std_net::__PROTO_ActorGroupWatcher);
                }
            };
        }
        if changed {
            crate::std_net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        };
    }
}

impl crate::std_net::__Stateful_ActorGroup for ActorGrouping {
    fn join(&mut self, member: usize) {
        if !(crate::std_net::admit(&mut self.all, member)) {
            return;
        };
        crate::std_net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        for mut w in self.watchers.iter().copied() {
            crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_ActorGroupWatcher::Joined(member), crate::std_net::__PROTO_ActorGroupWatcher);
        }
        for mut p in self.peers.iter() {
            crate::std_net::share_members(&self.name, &self.proto.hash, &(p).clone(), &vec![member]);
        }
    }
    fn leave(&mut self, member: usize) {
        if !(crate::std_net::withdraw(&mut self.all, member)) {
            return;
        };
        crate::std_net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        for mut w in self.watchers.iter().copied() {
            crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_ActorGroupWatcher::Left(member), crate::std_net::__PROTO_ActorGroupWatcher);
        }
    }
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<usize>>(out, (self.all).clone());
    }
    fn subscribe(&mut self, w: usize) {
        crate::std_core_list::add_platform::<usize>(&mut self.watchers, w);
    }
    fn refresh(&mut self) {
        crate::std_net::view_refresh(self.__addr.expect("an actor's own addr"));
    }
}

impl crate::std_net::__Stateful_NodeGroupWatcher for ActorGrouping {
    fn joined(&mut self, n: crate::std_net::Node) {
        crate::std_net::share_members(&self.name, &self.proto.hash, &n.id, &self.all);
    }
    fn left(&mut self, n: crate::std_net::Node, why: String) {
        let mut gone: Vec<usize> = vec![];
        for mut m in self.all.iter().copied() {
            if crate::std_net::eq__NodeId_NodeId(&crate::std_net::node_of(m), &n.id) {
                crate::std_core_list::add_platform::<usize>(&mut gone, m);
            };
        }
        for mut m in gone.iter().copied() {
            if crate::std_net::withdraw(&mut self.all, m) {
                for mut w in self.watchers.iter().copied() {
                    crate::scheduler::salvo_send_wire(w, crate::std_net::__Msg_ActorGroupWatcher::Left(m), crate::std_net::__PROTO_ActorGroupWatcher);
                }
            };
        }
        if (crate::std_core_list::size_platform::<usize>(&gone) > 0i32) {
            crate::std_net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        };
    }
}

pub struct __Actor_ActorGrouping {
    handler: ActorGrouping,
}

impl __Actor_ActorGrouping {
    pub fn new(handler: ActorGrouping) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_ActorGrouping {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::std_net::__Msg_ActorGroup>() {
            Ok(__m) => return crate::std_net::__dispatch_ActorGrouping_ActorGroup(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<crate::std_net::__Msg_NodeGroupWatcher>() {
            Ok(__m) => return crate::std_net::__dispatch_ActorGrouping_NodeGroupWatcher(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_ActorGrouping>() {
            Ok(__m) => return crate::std_net::__dispatch_priv_ActorGrouping(&mut self.handler, *__m),
            Err(__m) => __m,
        };
        let _ = msg;
        unreachable!("a message of one of this actor's protocols")
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_ActorGrouping::Join => crate::std_net::__dispatch_ActorGrouping_ActorGroup(&mut self.handler, crate::std_net::__Msg_ActorGroup::Join(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Leave => crate::std_net::__dispatch_ActorGrouping_ActorGroup(&mut self.handler, crate::std_net::__Msg_ActorGroup::Leave(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Members => crate::std_net::__dispatch_ActorGrouping_ActorGroup(&mut self.handler, crate::std_net::__Msg_ActorGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Subscribe => crate::std_net::__dispatch_ActorGrouping_ActorGroup(&mut self.handler, crate::std_net::__Msg_ActorGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Joined => crate::std_net::__dispatch_ActorGrouping_NodeGroupWatcher(&mut self.handler, crate::std_net::__Msg_NodeGroupWatcher::Joined(*value.downcast::<crate::std_net::Node>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Left(n) => crate::std_net::__dispatch_ActorGrouping_NodeGroupWatcher(&mut self.handler, crate::std_net::__Msg_NodeGroupWatcher::Left(n, *value.downcast::<String>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Control(from) => crate::std_net::__dispatch_priv_ActorGrouping(&mut self.handler, __Priv_ActorGrouping::Control(from, *value.downcast::<crate::std_core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ActorGrouping::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Leave{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Joined{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_net::Node>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Left{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_ActorGrouping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_ActorGrouping);
fn __decode_msg_ActorGrouping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_net::__PROTO_ActorGroup {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_ActorGroup>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    if proto == crate::std_net::__PROTO_NodeGroupWatcher {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_NodeGroupWatcher>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn mirror(mut group: usize, members: &Vec<usize>) {
    crate::std_net::view_set(group, (members).clone());
}

pub fn admit(list: &mut Vec<usize>, mut a: usize) -> bool {
    for mut x in list.iter().copied() {
        if crate::std_core_actor::eq(x, a) {
            return false;
        };
    }
    crate::std_core_list::add_platform::<usize>(&mut *list, a);
    return true;
}

pub fn contains_node(list: &Vec<crate::std_net::NodeId>, n: &crate::std_net::NodeId) -> bool {
    for mut x in list.iter() {
        if crate::std_net::eq__NodeId_NodeId(x, n) {
            return true;
        };
    }
    return false;
}

pub fn withdraw(list: &mut Vec<usize>, mut a: usize) -> bool {
    let mut mut_index: Option<i32> = None;
    let mut i: i32 = 0i32;
    for mut x in list.iter().copied() {
        if crate::std_core_actor::eq(x, a) {
            mut_index = Some(i);
        };
        i = i32::wrapping_add(i, 1i32);
    }
    if mut_index.is_none() {
        return false;
    };
    let mut mut_index_1 = mut_index.unwrap();
    let mut _removed: Option<usize> = crate::std_core_list::remove_at_platform::<usize>(&mut *list, mut_index_1);
    return true;
}

pub fn node_of(mut a: usize) -> crate::std_net::NodeId {
    return crate::std_net::NodeId { value: {
        let mut __proj_1: crate::std_runtime_routing::RemoteRef = crate::std_runtime_routing::identity((((a).clone()) as i32));
        __proj_1.node
    } };
}

pub fn view_set(mut group: usize, mut members: Vec<usize>) {
    let mut ixs: Vec<i32> = vec![];
    for mut m in members.iter().copied() {
        crate::std_core_list::add_platform::<i32>(&mut ixs, (((m).clone()) as i32));
    }
    crate::std_runtime_routing::view_set((((group).clone()) as i32), ixs);
}

pub fn view_members(mut group: usize) -> Vec<usize> {
    let mut out: Vec<usize> = vec![];
    for mut ix in crate::std_runtime_routing::view_members((((group).clone()) as i32)).iter().copied() {
        crate::std_core_list::add_platform::<usize>(&mut out, ((ix) as usize));
    }
    return out;
}

pub fn view_version(mut group: usize) -> i64 {
    return crate::std_runtime_routing::view_version((((group).clone()) as i32));
}

pub fn view_refresh(mut group: usize) {
    crate::std_runtime_routing::view_refresh((((group).clone()) as i32));
}

pub fn view_wait(mut group: usize, mut seen: i64, mut nanos: i64) {
    crate::std_runtime_routing::view_wait((((group).clone()) as i32), seen, nanos);
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteMember {
    pub addr: usize,
    pub local: bool,
}

impl crate::wire::__Wire for RouteMember {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.addr, out);
        crate::wire::__Wire::__enc(&self.local, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            addr: crate::wire::__Wire::__dec(r)?,
            local: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteView {
    pub members: Vec<crate::std_net::RouteMember>,
}

impl crate::wire::__Wire for RouteView {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.members, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            members: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn default_route_config() -> crate::std_net::RouteConfig {
    return crate::std_net::RouteConfig { first_wait: crate::std_time::Duration { nanos: 1000000i64 }, max_wait: crate::std_time::Duration { nanos: 5000000000i64 } };
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteConfig {
    pub first_wait: crate::std_time::Duration,
    pub max_wait: crate::std_time::Duration,
}

impl crate::wire::__Wire for RouteConfig {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.first_wait, out);
        crate::wire::__Wire::__enc(&self.max_wait, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            first_wait: crate::wire::__Wire::__dec(r)?,
            max_wait: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub trait __Stateless_RouteSelector: Send + Sync {
    fn changed(&self, view: &crate::std_net::RouteView);
    fn select(&self, view: &crate::std_net::RouteView, key: &Option<i64>) -> Option<usize>;
    fn config(&self) -> crate::std_net::RouteConfig;
}

pub trait __Stateful_RouteSelector: Send {
    fn changed(&mut self, view: &crate::std_net::RouteView);
    fn select(&mut self, view: &crate::std_net::RouteView, key: &Option<i64>) -> Option<usize>;
    fn config(&mut self) -> crate::std_net::RouteConfig;
}

pub struct RouteSelector {
    inner: __Inner_RouteSelector,
}

pub enum __Inner_RouteSelector {
    Shared(std::sync::Arc<dyn __Stateless_RouteSelector>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_RouteSelector>>),
}

impl Clone for RouteSelector {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_RouteSelector::Shared(h) => __Inner_RouteSelector::Shared(h.clone()),
            __Inner_RouteSelector::Locked(h) => __Inner_RouteSelector::Locked(h.clone()),
        } }
    }
}

impl RouteSelector {
    pub fn shared<__H: __Stateless_RouteSelector + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RouteSelector::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_RouteSelector>) -> Self {
        Self { inner: __Inner_RouteSelector::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_RouteSelector + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RouteSelector::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_RouteSelector>>) -> Self {
        Self { inner: __Inner_RouteSelector::Locked(inner) }
    }
    pub fn changed(&self, view: &crate::std_net::RouteView) {
        match &self.inner {
            __Inner_RouteSelector::Shared(h) => h.changed(view),
            __Inner_RouteSelector::Locked(h) => h.lock().unwrap().changed(view),
        }
    }
    pub fn select(&self, view: &crate::std_net::RouteView, key: &Option<i64>) -> Option<usize> {
        match &self.inner {
            __Inner_RouteSelector::Shared(h) => h.select(view, key),
            __Inner_RouteSelector::Locked(h) => h.lock().unwrap().select(view, key),
        }
    }
    pub fn config(&self) -> crate::std_net::RouteConfig {
        match &self.inner {
            __Inner_RouteSelector::Shared(h) => h.config(),
            __Inner_RouteSelector::Locked(h) => h.lock().unwrap().config(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutePick {
    pub to: usize,
    pub version: i64,
}

impl crate::wire::__Wire for RoutePick {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.to, out);
        crate::wire::__Wire::__enc(&self.version, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            to: crate::wire::__Wire::__dec(r)?,
            version: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn route_pick__Addr_Long(route_selector: &crate::std_net::RouteSelector, mut group: usize, mut seen: i64) -> crate::std_net::RoutePick {
    return crate::std_net::route_keyed(route_selector, group, seen, None);
}

pub fn route_pick__Addr_Long_Long(route_selector: &crate::std_net::RouteSelector, mut group: usize, mut seen: i64, mut key: i64) -> crate::std_net::RoutePick {
    return crate::std_net::route_keyed(route_selector, group, seen, Some(key));
}

pub fn route_keyed(route_selector: &crate::std_net::RouteSelector, mut group: usize, mut seen: i64, mut key: Option<i64>) -> crate::std_net::RoutePick {
    let mut waits: crate::std_net::RouteConfig = route_selector.config();
    let mut wait: i64 = waits.first_wait.nanos;
    let mut cap: i64 = waits.max_wait.nanos;
    let mut last: i64 = seen;
    loop {
        let mut version: i64 = crate::std_net::view_version(group);
        let mut view: crate::std_net::RouteView = crate::std_net::route_view(group);
        if (version != last) {
            route_selector.changed(&(view).clone());
            last = version;
        };
        let mut picked: Option<usize> = route_selector.select(&view, &(key).clone());
        if !(picked.is_none()) {
            let mut picked_1 = picked.unwrap();
            return crate::std_net::RoutePick { to: picked_1, version: last };
        };
        crate::std_net::view_wait(group, version, wait);
        wait = i64::wrapping_mul(wait, 2i64);
        if (wait > cap) {
            wait = cap;
        };
    }
    panic!("salvo: unreachable at std.net:1144:5");
    return crate::std_net::route_keyed(route_selector, group, seen, key);
}

pub fn route_view(mut group: usize) -> crate::std_net::RouteView {
    let mut members: Vec<crate::std_net::RouteMember> = vec![];
    for mut m in crate::std_net::view_members(group).iter().copied() {
        crate::std_core_list::add_platform::<crate::std_net::RouteMember>(&mut members, crate::std_net::RouteMember { addr: m, local: crate::std_net::eq__NodeId_NodeId(&crate::std_net::node_of(m), &crate::std_net::this_node()) });
    }
    return crate::std_net::RouteView { members: (members).clone() };
}

#[derive(Clone)]
pub struct LeastLoaded {
    prefer_local: bool,
    config: crate::std_net::RouteConfig,
}

impl LeastLoaded {
    pub fn new(prefer_local: bool, config: crate::std_net::RouteConfig) -> Self {
        Self {
            prefer_local,
            config
        }
    }
}

impl crate::std_net::__Stateless_RouteSelector for LeastLoaded {
    fn changed(&self, view: &crate::std_net::RouteView) {
    }
    fn select(&self, view: &crate::std_net::RouteView, key: &Option<i64>) -> Option<usize> {
        let mut best: Option<crate::std_net::RouteMember> = None;
        let mut best_pending: i32 = 0i32;
        for mut a in view.members.iter() {
            let mut load: i32 = crate::std_net::pending(a.addr);
            if best.is_none() {
                best = Some((a).clone());
                best_pending = load;
            } else {
                let mut best_1 = best.as_ref().unwrap();
                let mut b = best_1;
                let mut take: bool = if ((self.prefer_local && a.local) && !(b.local)) {
                    true
                } else if ((self.prefer_local && !(a.local)) && b.local) {
                    false
                } else {
                    (load < best_pending)
                };
                if take {
                    best = Some((a).clone());
                    best_pending = load;
                };
            };
        }
        let mut chosen: crate::std_net::RouteMember = {
            let mut __pick_2 = &best;
            if __pick_2.is_some() {
                let mut __picked_3 = __pick_2.as_ref().unwrap();
                __picked_3.clone()
            } else {
                {
                    return None;
                }
            }
        };
        return Some(chosen.addr);
    }
    fn config(&self) -> crate::std_net::RouteConfig {
        return (self.config).clone();
    }
}

#[derive(Clone)]
pub struct Sharded {
    config: crate::std_net::RouteConfig,
}

impl Sharded {
    pub fn new(config: crate::std_net::RouteConfig) -> Self {
        Self {
            config
        }
    }
}

impl crate::std_net::__Stateless_RouteSelector for Sharded {
    fn changed(&self, view: &crate::std_net::RouteView) {
    }
    fn select(&self, view: &crate::std_net::RouteView, key: &Option<i64>) -> Option<usize> {
        let mut n: i32 = crate::std_core_list::size_platform::<crate::std_net::RouteMember>(&view.members);
        if (n == 0i32) {
            return None;
        };
        let mut k: i64 = {
            let mut __pick_1 = key;
            if __pick_1.is_some() {
                let mut __picked_2 = __pick_1.unwrap();
                __picked_2
            } else {
                0i64
            }
        };
        let mut magnitude: i64 = if (k < 0i64) {
            i64::wrapping_sub(0i64, k)
        } else {
            k
        };
        let mut slot: i32 = ((i64::wrapping_rem(magnitude, ((n) as i64))) as i32);
        let mut picked: &crate::std_net::RouteMember = {
            let mut __pick_4: Option<&crate::std_net::RouteMember> = crate::std_core_list::get_platform::<crate::std_net::RouteMember>(&view.members, slot);
            if __pick_4.is_some() {
                let mut __picked_5 = __pick_4.unwrap();
                __picked_5
            } else {
                {
                    return None;
                }
            }
        };
        return Some(picked.addr);
    }
    fn config(&self) -> crate::std_net::RouteConfig {
        return (self.config).clone();
    }
}

pub trait __Stateless_Leader: Send + Sync {
    fn leader(&self) -> Option<crate::std_net::NodeId>;
}

pub trait __Stateful_Leader: Send {
    fn leader(&mut self) -> Option<crate::std_net::NodeId>;
}

pub struct Leader {
    inner: __Inner_Leader,
}

pub enum __Inner_Leader {
    Shared(std::sync::Arc<dyn __Stateless_Leader>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Leader>>),
}

impl Clone for Leader {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Leader::Shared(h) => __Inner_Leader::Shared(h.clone()),
            __Inner_Leader::Locked(h) => __Inner_Leader::Locked(h.clone()),
        } }
    }
}

impl Leader {
    pub fn shared<__H: __Stateless_Leader + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Leader::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Leader>) -> Self {
        Self { inner: __Inner_Leader::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Leader + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Leader::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Leader>>) -> Self {
        Self { inner: __Inner_Leader::Locked(inner) }
    }
    pub fn leader(&self) -> Option<crate::std_net::NodeId> {
        match &self.inner {
            __Inner_Leader::Shared(h) => h.leader(),
            __Inner_Leader::Locked(h) => h.lock().unwrap().leader(),
        }
    }
}

#[derive(Clone)]
pub struct StaticLeader {
    node: crate::std_net::NodeId,
}

impl StaticLeader {
    pub fn new(node: crate::std_net::NodeId) -> Self {
        Self {
            node
        }
    }
}

impl crate::std_net::__Stateless_Leader for StaticLeader {
    fn leader(&self) -> Option<crate::std_net::NodeId> {
        return Some((self.node).clone());
    }
}

pub struct Elected {
    config: crate::std_net::RouteConfig,
    __dep0: crate::std_net::Leader,
    chosen: Option<usize>,
}

impl Elected {
    pub fn new(config: crate::std_net::RouteConfig, __dep0: crate::std_net::Leader) -> Self {
        Self {
            config: config.clone(),
            __dep0,
            chosen: None
        }
    }
}

impl crate::std_net::__Stateful_RouteSelector for Elected {
    fn changed(&mut self, view: &crate::std_net::RouteView) {
        self.chosen = None;
        let mut l: crate::std_net::NodeId = {
            let mut __pick_1: Option<crate::std_net::NodeId> = self.__dep0.leader();
            if __pick_1.is_some() {
                let mut __picked_2 = __pick_1.unwrap();
                __picked_2
            } else {
                {
                    return;
                }
            }
        };
        for mut a in view.members.iter() {
            if crate::std_net::eq__NodeId_NodeId(&crate::std_net::node_of(a.addr), &l) {
                self.chosen = Some(a.addr);
                return;
            };
        }
    }
    fn select(&mut self, view: &crate::std_net::RouteView, key: &Option<i64>) -> Option<usize> {
        return (self.chosen).clone();
    }
    fn config(&mut self) -> crate::std_net::RouteConfig {
        return (self.config).clone();
    }
}

pub type __Platform_HostTcpTransport = crate::std_net::__Platform_Transport<crate::platform_std_net::HostTcpTransport>;

impl __Platform_HostTcpTransport {
    pub fn new(bind: crate::std_net::NodeEndpoint) -> Self {
        crate::std_net::__Platform_Transport(crate::platform_std_net::HostTcpTransport::new(bind))
    }
}

pub trait __Stateless_MemNet: Send + Sync {
    fn attach(&self, at: crate::std_net::NodeEndpoint, sink: usize);
    fn detach(&self, at: crate::std_net::NodeEndpoint);
    fn route(&self, from: crate::std_net::NodeEndpoint, to: crate::std_net::NodeEndpoint, out: crate::scheduler::SalvoReply);
    fn partition(&self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint);
    fn heal(&self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint);
    fn kill(&self, node: crate::std_net::NodeEndpoint);
    fn delivered(&self, out: crate::scheduler::SalvoReply);
}

pub trait __Stateful_MemNet: Send {
    fn attach(&mut self, at: crate::std_net::NodeEndpoint, sink: usize);
    fn detach(&mut self, at: crate::std_net::NodeEndpoint);
    fn route(&mut self, from: crate::std_net::NodeEndpoint, to: crate::std_net::NodeEndpoint, out: crate::scheduler::SalvoReply);
    fn partition(&mut self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint);
    fn heal(&mut self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint);
    fn kill(&mut self, node: crate::std_net::NodeEndpoint);
    fn delivered(&mut self, out: crate::scheduler::SalvoReply);
}

pub struct MemNet {
    inner: __Inner_MemNet,
}

pub enum __Inner_MemNet {
    Shared(std::sync::Arc<dyn __Stateless_MemNet>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_MemNet>>),
}

impl Clone for MemNet {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_MemNet::Shared(h) => __Inner_MemNet::Shared(h.clone()),
            __Inner_MemNet::Locked(h) => __Inner_MemNet::Locked(h.clone()),
        } }
    }
}

impl MemNet {
    pub fn shared<__H: __Stateless_MemNet + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_MemNet::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_MemNet>) -> Self {
        Self { inner: __Inner_MemNet::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_MemNet + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_MemNet::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_MemNet>>) -> Self {
        Self { inner: __Inner_MemNet::Locked(inner) }
    }
    pub fn attach(&self, at: crate::std_net::NodeEndpoint, sink: usize) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.attach(at, sink),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().attach(at, sink),
        }
    }
    pub fn detach(&self, at: crate::std_net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.detach(at),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().detach(at),
        }
    }
    pub fn route(&self, from: crate::std_net::NodeEndpoint, to: crate::std_net::NodeEndpoint, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.route(from, to, out),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().route(from, to, out),
        }
    }
    pub fn partition(&self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.partition(a, b),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().partition(a, b),
        }
    }
    pub fn heal(&self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.heal(a, b),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().heal(a, b),
        }
    }
    pub fn kill(&self, node: crate::std_net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.kill(node),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().kill(node),
        }
    }
    pub fn delivered(&self, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.delivered(out),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().delivered(out),
        }
    }
}

pub struct MemNetwork {
    listeners: crate::std_core_map::Map<crate::std_net::NodeEndpoint, usize>,
    cuts: crate::std_core_set::Set<String>,
    dead: crate::std_core_set::Set<crate::std_net::NodeEndpoint>,
    count: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_MemNetwork>,
}

impl MemNetwork {
    pub fn new() -> Self {
        Self {
            listeners: crate::std_core_map::Mut_Map_platform::<crate::std_net::NodeEndpoint, usize>(vec![], &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)),
            cuts: crate::std_core_set::Mut_Set_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])),
            dead: crate::std_core_set::Mut_Set_platform::<crate::std_net::NodeEndpoint>(vec![], &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)),
            count: 0i32,
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::std_net::__Stateful_MemNet for MemNetwork {
    fn attach(&mut self, at: crate::std_net::NodeEndpoint, sink: usize) {
        crate::std_core_set::remove_platform::<crate::std_net::NodeEndpoint>(&mut self.dead, &at, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
        crate::std_core_map::put_platform::<crate::std_net::NodeEndpoint, usize>(&mut self.listeners, at, sink, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
    }
    fn detach(&mut self, at: crate::std_net::NodeEndpoint) {
        crate::std_core_map::remove_platform::<crate::std_net::NodeEndpoint, usize>(&mut self.listeners, &at, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
    }
    fn route(&mut self, from: crate::std_net::NodeEndpoint, to: crate::std_net::NodeEndpoint, out: crate::scheduler::SalvoReply) {
        if (crate::std_core_set::contains_platform::<crate::std_net::NodeEndpoint>(&self.dead, &to, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)) || crate::std_core_set::contains_platform::<String>(&self.cuts, &crate::std_net::cut_key(&from, &to), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            crate::scheduler::salvo_reply_wire::<Option<usize>>(out, None);
            return;
        };
        let mut sink: Option<usize> = crate::std_core_map::get_platform::<crate::std_net::NodeEndpoint, usize>(&self.listeners, &to, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)).copied();
        if sink.is_none() {
            crate::scheduler::salvo_reply_wire::<Option<usize>>(out, None);
            return;
        };
        self.count = i32::wrapping_add(self.count, 1i32);
        let mut sink_1 = sink.unwrap();
        crate::scheduler::salvo_reply_wire::<Option<usize>>(out, Some(sink_1));
    }
    fn partition(&mut self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint) {
        crate::std_core_set::add_platform::<String>(&mut self.cuts, crate::std_net::cut_key(&a, &b), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::std_core_set::add_platform::<String>(&mut self.cuts, crate::std_net::cut_key(&b, &a), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    }
    fn heal(&mut self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint) {
        crate::std_core_set::remove_platform::<String>(&mut self.cuts, &crate::std_net::cut_key(&a, &b), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::std_core_set::remove_platform::<String>(&mut self.cuts, &crate::std_net::cut_key(&b, &a), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    }
    fn kill(&mut self, node: crate::std_net::NodeEndpoint) {
        crate::std_core_map::remove_platform::<crate::std_net::NodeEndpoint, usize>(&mut self.listeners, &node, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
        crate::std_core_set::add_platform::<crate::std_net::NodeEndpoint>(&mut self.dead, node, &mut |__a0: &crate::std_net::NodeEndpoint| crate::std_net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::std_net::NodeEndpoint, __a1: &crate::std_net::NodeEndpoint| crate::std_net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
    }
    fn delivered(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<i32>(out, self.count);
    }
}

pub struct __Actor_MemNetwork {
    handler: MemNetwork,
}

impl __Actor_MemNetwork {
    pub fn new(handler: MemNetwork) -> Self {
        Self { handler }
    }
}

impl crate::scheduler::SalvoActor for __Actor_MemNetwork {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::std_net::__Msg_MemNet>().expect("message of this protocol");
        crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_MemNetwork::Attach(at) => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Attach(at, *value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_MemNetwork::Detach => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Detach(*value.downcast::<crate::std_net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Route(from, to) => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Route(from, to, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_MemNetwork::Partition(a) => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Partition(a, *value.downcast::<crate::std_net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Heal(a) => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Heal(a, *value.downcast::<crate::std_net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Kill => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Kill(*value.downcast::<crate::std_net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Delivered => crate::std_net::__dispatch_MemNetwork_MemNet(&mut self.handler, crate::std_net::__Msg_MemNet::Delivered(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_MemNetwork::Attach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Detach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Route{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Partition{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Heal{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Kill{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::std_net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Delivered{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_MemNetwork: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_MemNetwork);
fn __decode_msg_MemNetwork(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::std_net::__PROTO_MemNet {
        return crate::wire::salvo_decode::<crate::std_net::__Msg_MemNet>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

#[derive(Clone)]
pub struct MemTransport {
    me: crate::std_net::NodeEndpoint,
    net: usize,
}

impl MemTransport {
    pub fn new(me: crate::std_net::NodeEndpoint, net: usize) -> Self {
        Self {
            me,
            net
        }
    }
}

impl crate::std_net::__Stateless_Transport for MemTransport {
    fn listen(&self, at: &crate::std_net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        crate::scheduler::salvo_send_wire(self.net, crate::std_net::__Msg_MemNet::Attach((at).clone(), sink), crate::std_net::__PROTO_MemNet);
        return crate::unions::Union2::U1(crate::std_core_result::ok::<()>(()));
    }
    fn unlisten(&self, at: &crate::std_net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.net, crate::std_net::__Msg_MemNet::Detach((at).clone()), crate::std_net::__PROTO_MemNet);
    }
    fn deliver(&self, to: &crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::std_net::Unreachable, crate::std_net::WireFailed>> {
        let mut sink: Option<usize> = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Option<usize>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.net, crate::std_net::__Msg_MemNet::Route((self.me).clone(), (to).clone(), out), crate::std_net::__PROTO_MemNet);
            *crate::scheduler::salvo_wait(__wid).downcast::<Option<usize>>().expect("the awaited answer")
        };
        if sink.is_none() {
            return crate::unions::Union2::U2(crate::unions::Union2::U1(crate::std_core_result::err::<crate::std_net::Unreachable>(crate::std_net::Unreachable { to: (to).clone() })));
        };
        let mut sink_1 = sink.unwrap();
        crate::scheduler::salvo_send_wire(sink_1, crate::std_net::__Msg_Inbound::ReceiveFrame((self.me).clone(), frame), crate::std_net::__PROTO_Inbound);
        return crate::unions::Union2::U1(crate::std_core_result::ok::<()>(()));
    }
    fn local_endpoint(&self) -> crate::std_net::NodeEndpoint {
        return (self.me).clone();
    }
}

pub fn cut_key(a: &crate::std_net::NodeEndpoint, b: &crate::std_net::NodeEndpoint) -> String {
    return format!("{}>{}", crate::std_net::to_str__NodeEndpoint(a), crate::std_net::to_str__NodeEndpoint(b));
}

pub fn cmp(a: &crate::std_net::NodeEndpoint, b: &crate::std_net::NodeEndpoint) -> i32 {
    let mut c__c1: i32 = (Ord::cmp(&a.host[..], &b.host[..]) as i32);
    if (c__c1 != 0i32) {
        return c__c1;
    };
    let mut c__c2: i32 = (Ord::cmp(&(a.port), &(b.port)) as i32);
    if (c__c2 != 0i32) {
        return c__c2;
    };
    return 0i32;
}

pub fn hash__NodeEndpoint(value: &crate::std_net::NodeEndpoint) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.host[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.port), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__NodeEndpoint_NodeEndpoint(a: &crate::std_net::NodeEndpoint, b: &crate::std_net::NodeEndpoint) -> bool {
    if !((&a.host[..] == &b.host[..])) {
        return false;
    };
    if !(((a.port) == (b.port))) {
        return false;
    };
    return true;
}

pub fn hash__NodeId(value: &crate::std_net::NodeId) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.value), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__NodeId_NodeId(a: &crate::std_net::NodeId, b: &crate::std_net::NodeId) -> bool {
    if !(((a.value) == (b.value))) {
        return false;
    };
    return true;
}

pub fn hash__Node(value: &crate::std_net::Node) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::std_core_compare::mix_hash(h, crate::std_net::hash__NodeId(&value.id));
    h = crate::std_core_compare::mix_hash(h, crate::std_net::hash__NodeEndpoint(&value.at));
    return h;
}

pub fn eq__Node_Node(a: &crate::std_net::Node, b: &crate::std_net::Node) -> bool {
    if !(crate::std_net::eq__NodeId_NodeId(&a.id, &b.id)) {
        return false;
    };
    if !(crate::std_net::eq__NodeEndpoint_NodeEndpoint(&a.at, &b.at)) {
        return false;
    };
    return true;
}

pub enum __Msg_Inbound {
    ReceiveFrame(crate::std_net::NodeEndpoint, crate::std_core_bytes::Bytes),
}

impl crate::wire::__Wire for __Msg_Inbound {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Inbound::ReceiveFrame(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Inbound::ReceiveFrame(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `Inbound`.
pub const __PROTO_Inbound: &str = "8e46ddb2a3e90b03";

pub enum __Msg_Outbound {
    SendFrame(crate::std_net::NodeEndpoint, crate::std_core_bytes::Bytes),
}

impl crate::wire::__Wire for __Msg_Outbound {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Outbound::SendFrame(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Outbound::SendFrame(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `Outbound`.
pub const __PROTO_Outbound: &str = "e754249e848e9986";

pub enum __Msg_NodeGroup {
    Members(crate::scheduler::SalvoReply),
    Subscribe(usize),
    Leave,
}

impl crate::wire::__Wire for __Msg_NodeGroup {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_NodeGroup::Members(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeGroup::Subscribe(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeGroup::Leave => out.push(2),
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_NodeGroup::Members(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_NodeGroup::Subscribe(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_NodeGroup::Leave),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `NodeGroup`.
pub const __PROTO_NodeGroup: &str = "471318a2c85d1f6d";

pub enum __Msg_NodeGroupWatcher {
    Joined(crate::std_net::Node),
    Left(crate::std_net::Node, String),
}

impl crate::wire::__Wire for __Msg_NodeGroupWatcher {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_NodeGroupWatcher::Joined(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeGroupWatcher::Left(__p0, __p1) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_NodeGroupWatcher::Joined(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_NodeGroupWatcher::Left(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `NodeGroupWatcher`.
pub const __PROTO_NodeGroupWatcher: &str = "db3e0aa5831c2e44";

pub enum __Msg_ActorGroup {
    Join(usize),
    Leave(usize),
    Members(crate::scheduler::SalvoReply),
    Subscribe(usize),
    Refresh,
}

impl crate::wire::__Wire for __Msg_ActorGroup {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_ActorGroup::Join(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorGroup::Leave(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorGroup::Members(__p0) => {
                out.push(2);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorGroup::Subscribe(__p0) => {
                out.push(3);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorGroup::Refresh => out.push(4),
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_ActorGroup::Join(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_ActorGroup::Leave(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_ActorGroup::Members(crate::wire::__Wire::__dec(r)?)),
            3 => Some(__Msg_ActorGroup::Subscribe(crate::wire::__Wire::__dec(r)?)),
            4 => Some(__Msg_ActorGroup::Refresh),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `ActorGroup`.
pub const __PROTO_ActorGroup: &str = "695f43128bdada4a";

pub enum __Msg_ActorGroupWatcher {
    Joined(usize),
    Left(usize),
}

impl crate::wire::__Wire for __Msg_ActorGroupWatcher {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_ActorGroupWatcher::Joined(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorGroupWatcher::Left(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_ActorGroupWatcher::Joined(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_ActorGroupWatcher::Left(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `ActorGroupWatcher`.
pub const __PROTO_ActorGroupWatcher: &str = "9fa424e5858bb3bd";

pub enum __Msg_MemNet {
    Attach(crate::std_net::NodeEndpoint, usize),
    Detach(crate::std_net::NodeEndpoint),
    Route(crate::std_net::NodeEndpoint, crate::std_net::NodeEndpoint, crate::scheduler::SalvoReply),
    Partition(crate::std_net::NodeEndpoint, crate::std_net::NodeEndpoint),
    Heal(crate::std_net::NodeEndpoint, crate::std_net::NodeEndpoint),
    Kill(crate::std_net::NodeEndpoint),
    Delivered(crate::scheduler::SalvoReply),
}

impl crate::wire::__Wire for __Msg_MemNet {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_MemNet::Attach(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
            __Msg_MemNet::Detach(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_MemNet::Route(__p0, __p1, __p2) => {
                out.push(2);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
            __Msg_MemNet::Partition(__p0, __p1) => {
                out.push(3);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
            __Msg_MemNet::Heal(__p0, __p1) => {
                out.push(4);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
            __Msg_MemNet::Kill(__p0) => {
                out.push(5);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_MemNet::Delivered(__p0) => {
                out.push(6);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_MemNet::Attach(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_MemNet::Detach(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_MemNet::Route(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            3 => Some(__Msg_MemNet::Partition(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            4 => Some(__Msg_MemNet::Heal(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            5 => Some(__Msg_MemNet::Kill(crate::wire::__Wire::__dec(r)?)),
            6 => Some(__Msg_MemNet::Delivered(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [actor-protocol-hash] The canonical hash of `MemNet`.
pub const __PROTO_MemNet: &str = "2815c14392023d5e";

pub enum __Cont_Sending {
    SendFrame(crate::std_net::NodeEndpoint),
}

pub enum __Cont_Receiving {
    ReceiveFrame(crate::std_net::NodeEndpoint),
}

pub enum __Cont_StaticNodeGroup {
    Members,
    Subscribe,
    Control(crate::std_net::NodeId),
}

pub enum __Priv_StaticNodeGroup {
    Init,
    Control(crate::std_net::NodeId, crate::std_core_bytes::Bytes),
}

pub enum __Cont_GossipNodeGroup {
    Members,
    Subscribe,
    Control(crate::std_net::NodeId),
}

pub enum __Priv_GossipNodeGroup {
    Init,
    Control(crate::std_net::NodeId, crate::std_core_bytes::Bytes),
}

pub enum __Cont_ActorGrouping {
    Join,
    Leave,
    Members,
    Subscribe,
    Joined,
    Left(crate::std_net::Node),
    Control(crate::std_net::NodeId),
}

pub enum __Priv_ActorGrouping {
    Init,
    Control(crate::std_net::NodeId, crate::std_core_bytes::Bytes),
}

pub enum __Cont_MemNetwork {
    Attach(crate::std_net::NodeEndpoint),
    Detach,
    Route(crate::std_net::NodeEndpoint, crate::std_net::NodeEndpoint),
    Partition(crate::std_net::NodeEndpoint),
    Heal(crate::std_net::NodeEndpoint),
    Kill,
    Delivered,
}

#[derive(Clone)]
pub struct __Stub_Inbound {
    addr: usize,
}

impl __Stub_Inbound {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_Inbound for __Stub_Inbound {
    fn receive_frame(&self, from: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_Inbound::ReceiveFrame(from, frame), crate::std_net::__PROTO_Inbound);
    }
}

#[derive(Clone)]
pub struct __Stub_Outbound {
    addr: usize,
}

impl __Stub_Outbound {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_Outbound for __Stub_Outbound {
    fn send_frame(&self, to: crate::std_net::NodeEndpoint, frame: crate::std_core_bytes::Bytes) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_Outbound::SendFrame(to, frame), crate::std_net::__PROTO_Outbound);
    }
}

#[derive(Clone)]
pub struct __Stub_NodeGroup {
    addr: usize,
}

impl __Stub_NodeGroup {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_NodeGroup for __Stub_NodeGroup {
    fn members(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_NodeGroup::Members(out), crate::std_net::__PROTO_NodeGroup);
    }
    fn subscribe(&self, w: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_NodeGroup::Subscribe(w), crate::std_net::__PROTO_NodeGroup);
    }
    fn leave(&self) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_NodeGroup::Leave, crate::std_net::__PROTO_NodeGroup);
    }
}

#[derive(Clone)]
pub struct __Stub_NodeGroupWatcher {
    addr: usize,
}

impl __Stub_NodeGroupWatcher {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_NodeGroupWatcher for __Stub_NodeGroupWatcher {
    fn joined(&self, n: crate::std_net::Node) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_NodeGroupWatcher::Joined(n), crate::std_net::__PROTO_NodeGroupWatcher);
    }
    fn left(&self, n: crate::std_net::Node, why: String) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_NodeGroupWatcher::Left(n, why), crate::std_net::__PROTO_NodeGroupWatcher);
    }
}

#[derive(Clone)]
pub struct __Stub_ActorGroup {
    addr: usize,
}

impl __Stub_ActorGroup {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_ActorGroup for __Stub_ActorGroup {
    fn join(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroup::Join(member), crate::std_net::__PROTO_ActorGroup);
    }
    fn leave(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroup::Leave(member), crate::std_net::__PROTO_ActorGroup);
    }
    fn members(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroup::Members(out), crate::std_net::__PROTO_ActorGroup);
    }
    fn subscribe(&self, w: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroup::Subscribe(w), crate::std_net::__PROTO_ActorGroup);
    }
    fn refresh(&self) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroup::Refresh, crate::std_net::__PROTO_ActorGroup);
    }
}

#[derive(Clone)]
pub struct __Stub_ActorGroupWatcher {
    addr: usize,
}

impl __Stub_ActorGroupWatcher {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_ActorGroupWatcher for __Stub_ActorGroupWatcher {
    fn joined(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroupWatcher::Joined(member), crate::std_net::__PROTO_ActorGroupWatcher);
    }
    fn left(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_ActorGroupWatcher::Left(member), crate::std_net::__PROTO_ActorGroupWatcher);
    }
}

#[derive(Clone)]
pub struct __Stub_MemNet {
    addr: usize,
}

impl __Stub_MemNet {
    pub fn new(addr: usize) -> Self {
        Self {
            addr
        }
    }
}

impl crate::std_net::__Stateless_MemNet for __Stub_MemNet {
    fn attach(&self, at: crate::std_net::NodeEndpoint, sink: usize) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Attach(at, sink), crate::std_net::__PROTO_MemNet);
    }
    fn detach(&self, at: crate::std_net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Detach(at), crate::std_net::__PROTO_MemNet);
    }
    fn route(&self, from: crate::std_net::NodeEndpoint, to: crate::std_net::NodeEndpoint, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Route(from, to, out), crate::std_net::__PROTO_MemNet);
    }
    fn partition(&self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Partition(a, b), crate::std_net::__PROTO_MemNet);
    }
    fn heal(&self, a: crate::std_net::NodeEndpoint, b: crate::std_net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Heal(a, b), crate::std_net::__PROTO_MemNet);
    }
    fn kill(&self, node: crate::std_net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Kill(node), crate::std_net::__PROTO_MemNet);
    }
    fn delivered(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, crate::std_net::__Msg_MemNet::Delivered(out), crate::std_net::__PROTO_MemNet);
    }
}

pub fn __dispatch_Sending_Outbound(__handler: &mut crate::std_net::Sending, mut __msg: crate::std_net::__Msg_Outbound) {
    if matches!(__msg, crate::std_net::__Msg_Outbound::SendFrame(..)) {
        let crate::std_net::__Msg_Outbound::SendFrame(to, frame) = __msg else { unreachable!() };
        crate::std_net::__Stateless_Outbound::send_frame(&mut *__handler, to, frame);
    };
}

pub fn __dispatch_Receiving_Inbound(__handler: &mut crate::std_net::Receiving, mut __msg: crate::std_net::__Msg_Inbound) {
    if matches!(__msg, crate::std_net::__Msg_Inbound::ReceiveFrame(..)) {
        let crate::std_net::__Msg_Inbound::ReceiveFrame(from, frame) = __msg else { unreachable!() };
        crate::std_net::__Stateless_Inbound::receive_frame(&mut *__handler, from, frame);
    };
}

pub fn __dispatch_StaticNodeGroup_NodeGroup(__handler: &mut crate::std_net::StaticNodeGroup, mut __msg: crate::std_net::__Msg_NodeGroup) {
    if matches!(__msg, crate::std_net::__Msg_NodeGroup::Members(..)) {
        let crate::std_net::__Msg_NodeGroup::Members(out) = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroup::members(&mut *__handler, out);
    } else if matches!(__msg, crate::std_net::__Msg_NodeGroup::Subscribe(..)) {
        let crate::std_net::__Msg_NodeGroup::Subscribe(w) = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroup::subscribe(&mut *__handler, w);
    } else {
        let crate::std_net::__Msg_NodeGroup::Leave = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroup::leave(&mut *__handler);
    };
}

pub fn __dispatch_priv_StaticNodeGroup(__handler: &mut crate::std_net::StaticNodeGroup, mut __msg: crate::std_net::__Priv_StaticNodeGroup) {
    if matches!(__msg, crate::std_net::__Priv_StaticNodeGroup::Init) {
        let crate::std_net::__Priv_StaticNodeGroup::Init = __msg else { unreachable!() };
        __handler.init();
    } else {
        let crate::std_net::__Priv_StaticNodeGroup::Control(from, data) = __msg else { unreachable!() };
        __handler.control(from, data);
    };
}

pub fn __dispatch_GossipNodeGroup_NodeGroup(__handler: &mut crate::std_net::GossipNodeGroup, mut __msg: crate::std_net::__Msg_NodeGroup) {
    if matches!(__msg, crate::std_net::__Msg_NodeGroup::Members(..)) {
        let crate::std_net::__Msg_NodeGroup::Members(out) = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroup::members(&mut *__handler, out);
    } else if matches!(__msg, crate::std_net::__Msg_NodeGroup::Subscribe(..)) {
        let crate::std_net::__Msg_NodeGroup::Subscribe(w) = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroup::subscribe(&mut *__handler, w);
    } else {
        let crate::std_net::__Msg_NodeGroup::Leave = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroup::leave(&mut *__handler);
    };
}

pub fn __dispatch_priv_GossipNodeGroup(__handler: &mut crate::std_net::GossipNodeGroup, mut __msg: crate::std_net::__Priv_GossipNodeGroup) {
    if matches!(__msg, crate::std_net::__Priv_GossipNodeGroup::Init) {
        let crate::std_net::__Priv_GossipNodeGroup::Init = __msg else { unreachable!() };
        __handler.init();
    } else {
        let crate::std_net::__Priv_GossipNodeGroup::Control(from, data) = __msg else { unreachable!() };
        __handler.control(from, data);
    };
}

pub fn __dispatch_ActorGrouping_ActorGroup(__handler: &mut crate::std_net::ActorGrouping, mut __msg: crate::std_net::__Msg_ActorGroup) {
    if matches!(__msg, crate::std_net::__Msg_ActorGroup::Join(..)) {
        let crate::std_net::__Msg_ActorGroup::Join(member) = __msg else { unreachable!() };
        crate::std_net::__Stateful_ActorGroup::join(&mut *__handler, member);
    } else if matches!(__msg, crate::std_net::__Msg_ActorGroup::Leave(..)) {
        let crate::std_net::__Msg_ActorGroup::Leave(member) = __msg else { unreachable!() };
        crate::std_net::__Stateful_ActorGroup::leave(&mut *__handler, member);
    } else if matches!(__msg, crate::std_net::__Msg_ActorGroup::Members(..)) {
        let crate::std_net::__Msg_ActorGroup::Members(out) = __msg else { unreachable!() };
        crate::std_net::__Stateful_ActorGroup::members(&mut *__handler, out);
    } else if matches!(__msg, crate::std_net::__Msg_ActorGroup::Subscribe(..)) {
        let crate::std_net::__Msg_ActorGroup::Subscribe(w) = __msg else { unreachable!() };
        crate::std_net::__Stateful_ActorGroup::subscribe(&mut *__handler, w);
    } else {
        let crate::std_net::__Msg_ActorGroup::Refresh = __msg else { unreachable!() };
        crate::std_net::__Stateful_ActorGroup::refresh(&mut *__handler);
    };
}

pub fn __dispatch_ActorGrouping_NodeGroupWatcher(__handler: &mut crate::std_net::ActorGrouping, mut __msg: crate::std_net::__Msg_NodeGroupWatcher) {
    if matches!(__msg, crate::std_net::__Msg_NodeGroupWatcher::Joined(..)) {
        let crate::std_net::__Msg_NodeGroupWatcher::Joined(n) = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroupWatcher::joined(&mut *__handler, n);
    } else {
        let crate::std_net::__Msg_NodeGroupWatcher::Left(n, why) = __msg else { unreachable!() };
        crate::std_net::__Stateful_NodeGroupWatcher::left(&mut *__handler, n, why);
    };
}

pub fn __dispatch_priv_ActorGrouping(__handler: &mut crate::std_net::ActorGrouping, mut __msg: crate::std_net::__Priv_ActorGrouping) {
    if matches!(__msg, crate::std_net::__Priv_ActorGrouping::Init) {
        let crate::std_net::__Priv_ActorGrouping::Init = __msg else { unreachable!() };
        __handler.init();
    } else {
        let crate::std_net::__Priv_ActorGrouping::Control(from, data) = __msg else { unreachable!() };
        __handler.control(from, data);
    };
}

pub fn __dispatch_MemNetwork_MemNet(__handler: &mut crate::std_net::MemNetwork, mut __msg: crate::std_net::__Msg_MemNet) {
    if matches!(__msg, crate::std_net::__Msg_MemNet::Attach(..)) {
        let crate::std_net::__Msg_MemNet::Attach(at, sink) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::attach(&mut *__handler, at, sink);
    } else if matches!(__msg, crate::std_net::__Msg_MemNet::Detach(..)) {
        let crate::std_net::__Msg_MemNet::Detach(at) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::detach(&mut *__handler, at);
    } else if matches!(__msg, crate::std_net::__Msg_MemNet::Route(..)) {
        let crate::std_net::__Msg_MemNet::Route(from, to, out) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::route(&mut *__handler, from, to, out);
    } else if matches!(__msg, crate::std_net::__Msg_MemNet::Partition(..)) {
        let crate::std_net::__Msg_MemNet::Partition(a, b) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::partition(&mut *__handler, a, b);
    } else if matches!(__msg, crate::std_net::__Msg_MemNet::Heal(..)) {
        let crate::std_net::__Msg_MemNet::Heal(a, b) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::heal(&mut *__handler, a, b);
    } else if matches!(__msg, crate::std_net::__Msg_MemNet::Kill(..)) {
        let crate::std_net::__Msg_MemNet::Kill(node) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::kill(&mut *__handler, node);
    } else {
        let crate::std_net::__Msg_MemNet::Delivered(out) = __msg else { unreachable!() };
        crate::std_net::__Stateful_MemNet::delivered(&mut *__handler, out);
    };
}
