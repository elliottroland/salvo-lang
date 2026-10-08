use crate::core_bytes::Bytes;
use crate::time::Duration;
use crate::core_map::Map;
use crate::runtime_routing::RemoteRef;
use crate::core_set::Set;
use crate::core_map::contains_key_platform;
use crate::core_set::contains_platform;
use crate::runtime_routing::control_frame;
use crate::core_actor::eq;
use crate::runtime_routing::here_node;
use crate::runtime_routing::identity;
use crate::core_map::keys_platform;
use crate::runtime_routing::local_protocols;
use crate::core_compare::mix_hash;
use crate::core_map::mut_map_of_platform;
use crate::core_set::mut_set_of_platform;
use crate::runtime_routing::outbound_bound;
use crate::core_actor::pool;
use crate::core_map::put_platform;
use crate::core_list::remove_at_platform;
use crate::core_list::size_platform;


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

pub fn to_str__NodeEndpoint(e: &crate::net::NodeEndpoint) -> String {
    return format!("{}:{}", e.host, e.port);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unreachable {
    pub to: crate::net::NodeEndpoint,
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
    pub to: crate::net::NodeEndpoint,
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

pub type NetError = crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>;

/// Factories for the host: one per arm of the union [platform-factory].
impl NetError {
    pub fn unreachable(value: crate::net::Unreachable) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn wire_failed(value: crate::net::WireFailed) -> Self {
        crate::unions::Union2::U2(value)
    }
}

pub fn to_str__NetError(e: &crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>) -> String {
    return if matches!(e, crate::unions::Union2::U1(_)) {
        let mut e_1 = match &e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return format!("unreachable: {}", crate::net::to_str__NodeEndpoint(&e_1.to));
    } else {
        let mut e_2 = match &e { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return format!("wire failed to {}: {}", crate::net::to_str__NodeEndpoint(&e_2.to), e_2.reason);
    };
}

pub trait __Stateless_Inbound: Send + Sync {
    fn receive_frame(&self, from: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes);
}

pub trait __Stateful_Inbound: Send {
    fn receive_frame(&mut self, from: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes);
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
    pub fn receive_frame(&self, from: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) {
        match &self.inner {
            __Inner_Inbound::Shared(h) => h.receive_frame(from, frame),
            __Inner_Inbound::Locked(h) => h.lock().unwrap().receive_frame(from, frame),
        }
    }
}

pub struct __Stub_Inbound {
    addr: usize,
}

impl __Stub_Inbound {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Inbound for __Stub_Inbound {
    fn receive_frame(&self, from: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Inbound::ReceiveFrame(from, frame), crate::net::__PROTO_Inbound);
    }
}

pub trait __Stateless_Transport: Send + Sync {
    fn listen(&self, at: &crate::net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>>;
    fn unlisten(&self, at: &crate::net::NodeEndpoint);
    fn deliver(&self, to: &crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>>;
    fn local_endpoint(&self) -> crate::net::NodeEndpoint;
}

pub trait __Stateful_Transport: Send {
    fn listen(&mut self, at: &crate::net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>>;
    fn unlisten(&mut self, at: &crate::net::NodeEndpoint);
    fn deliver(&mut self, to: &crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>>;
    fn local_endpoint(&mut self) -> crate::net::NodeEndpoint;
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
    pub fn listen(&self, at: &crate::net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.listen(at, sink),
            __Inner_Transport::Locked(h) => h.lock().unwrap().listen(at, sink),
        }
    }
    pub fn unlisten(&self, at: &crate::net::NodeEndpoint) {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.unlisten(at),
            __Inner_Transport::Locked(h) => h.lock().unwrap().unlisten(at),
        }
    }
    pub fn deliver(&self, to: &crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.deliver(to, frame),
            __Inner_Transport::Locked(h) => h.lock().unwrap().deliver(to, frame),
        }
    }
    pub fn local_endpoint(&self) -> crate::net::NodeEndpoint {
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
    fn listen(&self, at: &crate::net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>>;
    fn unlisten(&self, at: &crate::net::NodeEndpoint);
    fn deliver(&self, to: &crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>>;
    fn local_endpoint(&self) -> crate::net::NodeEndpoint;
}

impl<T: TransportPlatformSync> __Stateless_Transport for __Platform_Transport<T> {
    fn listen(&self, at: &crate::net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        self.0.listen(at, sink)
    }
    fn unlisten(&self, at: &crate::net::NodeEndpoint) {
        self.0.unlisten(at)
    }
    fn deliver(&self, to: &crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        self.0.deliver(to, frame)
    }
    fn local_endpoint(&self) -> crate::net::NodeEndpoint {
        self.0.local_endpoint()
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct Listen;

impl Listen {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct Deliver;

impl Deliver {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        crate::unions::Union2::U2(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeId {
    pub id: i64,
}

impl crate::wire::__Wire for NodeId {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.id, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            id: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn this_node() -> crate::net::NodeId {
    return crate::net::NodeId { id: crate::runtime_routing::here_node() };
}

pub fn new_node() -> crate::net::NodeId {
    return crate::net::NodeId { id: crate::runtime_routing::new_node() };
}

pub fn pool_at(node: &crate::net::NodeId, mut size: i32) -> usize {
    return ((crate::runtime_routing::pool_at(node.id, size)) as usize);
}

pub fn add_route(node: &crate::net::NodeId, at: &crate::net::NodeEndpoint) {
    crate::runtime_routing::route(node.id, { let __enc: crate::net::NodeEndpoint = (at).clone(); crate::wire::salvo_encode(&__enc) });
}

pub fn route_frames(mut out: usize) {
    crate::net::bind_outbound_platform(crate::runtime_routing::here_node(), (((out).clone()) as i32), |__a0: i32, __a1: &crate::core_bytes::Bytes, __a2: crate::core_bytes::Bytes| crate::net::forward_frame(__a0, __a1, __a2));
    crate::runtime_routing::outbound_bound();
}

pub fn forward_frame(mut out: i32, to: &crate::core_bytes::Bytes, mut frame: crate::core_bytes::Bytes) {
    let mut __subject_1: Option<crate::net::NodeEndpoint> = crate::wire::salvo_decode::<crate::net::NodeEndpoint>(&to);
    if __subject_1.is_some() {
        let mut ep = __subject_1.as_ref().unwrap();
        let mut sending: usize = ((out) as usize);
        crate::scheduler::salvo_send_wire(sending, crate::net::__Msg_Outbound::SendFrame(ep.clone(), frame.clone()), crate::net::__PROTO_Outbound);
    };
}

pub fn bind_outbound_platform(mut node: i64, mut out: i32, hook: fn(i32, &crate::core_bytes::Bytes, crate::core_bytes::Bytes)) {
    crate::platform_net::bind_outbound(node, out, hook)
}

pub fn deliver_frame(data: &crate::core_bytes::Bytes) -> bool {
    return crate::runtime_routing::deliver(data);
}

pub fn credits(mut a: usize) -> Option<i32> {
    let mut c: Option<i32> = crate::runtime_routing::credits((((a).clone()) as i32));
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
    fn send_frame(&self, to: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes);
}

pub trait __Stateful_Outbound: Send {
    fn send_frame(&mut self, to: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes);
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
    pub fn send_frame(&self, to: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) {
        match &self.inner {
            __Inner_Outbound::Shared(h) => h.send_frame(to, frame),
            __Inner_Outbound::Locked(h) => h.lock().unwrap().send_frame(to, frame),
        }
    }
}

pub struct __Stub_Outbound {
    addr: usize,
}

impl __Stub_Outbound {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Outbound for __Stub_Outbound {
    fn send_frame(&self, to: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Outbound::SendFrame(to, frame), crate::net::__PROTO_Outbound);
    }
}

pub struct Sending {
    __dep0: crate::net::Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sending>,
}

impl Sending {
    pub fn new(__dep0: crate::net::Transport) -> Self {
        Self {
            __dep0,
            __mailbox_capacity: 256i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::net::__Stateless_Outbound for Sending {
    fn send_frame(&self, to: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) {
        let mut _sent: crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> = self.__dep0.deliver(&to, frame);
    }
}

pub struct __Actor_Sending {
    handler: Sending,
}

impl __Actor_Sending {
    pub fn new(handler: Sending) -> Self {
        Self { handler }
    }
    fn __dispatch(&mut self, msg: crate::net::__Msg_Outbound) {
        match msg {
            crate::net::__Msg_Outbound::SendFrame(to, frame) => crate::net::__Stateless_Outbound::send_frame(&mut self.handler, to, frame),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Sending {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::net::__Msg_Outbound>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Sending::SendFrame(to) => self.__dispatch(crate::net::__Msg_Outbound::SendFrame(to, *value.downcast::<crate::core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sending::SendFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sending: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sending);
fn __decode_msg_Sending(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::net::__PROTO_Outbound {
        return crate::wire::salvo_decode::<crate::net::__Msg_Outbound>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
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

impl crate::net::__Stateless_Inbound for Receiving {
    fn receive_frame(&self, from: crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) {
        let mut _delivered: bool = crate::net::deliver_frame(&frame);
    }
}

pub struct __Actor_Receiving {
    handler: Receiving,
}

impl __Actor_Receiving {
    pub fn new(handler: Receiving) -> Self {
        Self { handler }
    }
    fn __dispatch(&mut self, msg: crate::net::__Msg_Inbound) {
        match msg {
            crate::net::__Msg_Inbound::ReceiveFrame(from, frame) => crate::net::__Stateless_Inbound::receive_frame(&mut self.handler, from, frame),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_Receiving {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::net::__Msg_Inbound>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_Receiving::ReceiveFrame(from) => self.__dispatch(crate::net::__Msg_Inbound::ReceiveFrame(from, *value.downcast::<crate::core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Receiving::ReceiveFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Receiving: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Receiving);
fn __decode_msg_Receiving(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::net::__PROTO_Inbound {
        return crate::wire::salvo_decode::<crate::net::__Msg_Inbound>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn connected() -> bool {
    return crate::runtime_routing::connected();
}

pub fn connect__NodeEndpoint(transport: &crate::net::Transport, mut me: crate::net::NodeEndpoint) -> bool {
    return crate::net::connect__NodeEndpoint_Pool(transport, me, crate::core_actor::pool(1i32));
}

pub fn connect__NodeEndpoint_Pool(transport: &crate::net::Transport, mut me: crate::net::NodeEndpoint, mut on: usize) -> bool {
    if crate::net::connected() {
        return false;
    };
    let mut sending: usize = ({ let __h = crate::net::Sending::new(transport.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, std::boxed::Box::new(crate::net::__Actor_Sending::new(__h)), crate::net::__DECODE_Sending); __a });
    let mut receiving: usize = ({ let __h = crate::net::Receiving::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, std::boxed::Box::new(crate::net::__Actor_Receiving::new(__h)), crate::net::__DECODE_Receiving); __a });
    crate::net::route_frames(sending);
    let mut _listening: crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> = transport.listen(&(me).clone(), receiving);
    crate::net::add_route(&crate::net::this_node(), &me);
    return true;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: crate::net::NodeId,
    pub at: crate::net::NodeEndpoint,
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

pub struct __Stub_NodeGroup {
    addr: usize,
}

impl __Stub_NodeGroup {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_NodeGroup for __Stub_NodeGroup {
    fn members(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Members(out), crate::net::__PROTO_NodeGroup);
    }
    fn subscribe(&self, w: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Subscribe(w), crate::net::__PROTO_NodeGroup);
    }
    fn leave(&self) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Leave, crate::net::__PROTO_NodeGroup);
    }
}

pub trait __Stateless_NodeGroupWatcher: Send + Sync {
    fn joined(&self, n: crate::net::Node);
    fn left(&self, n: crate::net::Node, why: String);
}

pub trait __Stateful_NodeGroupWatcher: Send {
    fn joined(&mut self, n: crate::net::Node);
    fn left(&mut self, n: crate::net::Node, why: String);
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
    pub fn joined(&self, n: crate::net::Node) {
        match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => h.joined(n),
            __Inner_NodeGroupWatcher::Locked(h) => h.lock().unwrap().joined(n),
        }
    }
    pub fn left(&self, n: crate::net::Node, why: String) {
        match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => h.left(n, why),
            __Inner_NodeGroupWatcher::Locked(h) => h.lock().unwrap().left(n, why),
        }
    }
}

pub struct __Stub_NodeGroupWatcher {
    addr: usize,
}

impl __Stub_NodeGroupWatcher {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_NodeGroupWatcher for __Stub_NodeGroupWatcher {
    fn joined(&self, n: crate::net::Node) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroupWatcher::Joined(n), crate::net::__PROTO_NodeGroupWatcher);
    }
    fn left(&self, n: crate::net::Node, why: String) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroupWatcher::Left(n, why), crate::net::__PROTO_NodeGroupWatcher);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hello {
    pub group: String,
    pub at: crate::net::NodeEndpoint,
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
    pub at: crate::net::NodeEndpoint,
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
    pub peers: Vec<crate::net::NodeEndpoint>,
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

pub fn send_control(to: &crate::net::NodeId, channel: &String, mut payload: crate::core_bytes::Bytes) {
    crate::runtime_routing::send_control(to.id, (channel).clone(), payload);
}

pub fn node_left(node: &crate::net::NodeId) {
    crate::runtime_routing::node_left(node.id);
}

pub fn set_peer_protocols(node: &crate::net::NodeId, table: &Vec<(String, String)>) {
    crate::runtime_routing::set_peer_protocols(node.id, (table).clone());
}

pub fn peer_protocol(node: &crate::net::NodeId, protocol: &String) -> Option<String> {
    return crate::runtime_routing::peer_protocol(node.id, protocol);
}

pub fn hello_frame(transport: &crate::net::Transport, group: &String) -> crate::core_bytes::Bytes {
    let mut hello: crate::unions::Union4<crate::net::Hello, crate::net::Ack, crate::net::Leaving, crate::net::Intro> = crate::unions::Union4::U1(crate::net::Hello { group: (group).clone(), at: transport.local_endpoint(), protocols: crate::runtime_routing::local_protocols() });
    return crate::runtime_routing::control_frame(String::from(""), crate::wire::salvo_encode(&hello));
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeerHello {
    pub node: crate::net::NodeId,
    pub at: crate::net::NodeEndpoint,
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
    pub node: crate::net::NodeId,
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
    pub peers: Vec<crate::net::NodeEndpoint>,
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

pub fn handshake(transport: &crate::net::Transport, group: &String, mut from: crate::net::NodeId, mut data: crate::core_bytes::Bytes) -> Option<crate::unions::Union3<crate::net::PeerHello, crate::net::PeerGone, crate::net::PeerIntro>> {
    let mut msg: Option<crate::unions::Union4<crate::net::Hello, crate::net::Ack, crate::net::Leaving, crate::net::Intro>> = crate::wire::salvo_decode::<crate::unions::Union4<crate::net::Hello, crate::net::Ack, crate::net::Leaving, crate::net::Intro>>(&data);
    return if matches!(msg, Some(crate::unions::Union4::U1(_))) {
        let mut msg_1 = match &msg { Some(crate::unions::Union4::U1(__v)) => __v, _ => unreachable!() };
        if !((&msg_1.group[..] == &group[..])) {
            return None;
        };
        crate::net::add_route(&(from).clone(), &(msg_1.at).clone());
        crate::net::set_peer_protocols(&(from).clone(), &(msg_1.protocols).clone());
        let mut ack: crate::unions::Union4<crate::net::Hello, crate::net::Ack, crate::net::Leaving, crate::net::Intro> = crate::unions::Union4::U2(crate::net::Ack { group: (group).clone(), at: transport.local_endpoint(), protocols: crate::runtime_routing::local_protocols() });
        crate::net::send_control(&(from).clone(), &String::from(""), crate::wire::salvo_encode(&ack));
        return Some(crate::unions::Union3::U1(crate::net::PeerHello { node: from.clone(), at: (msg_1.at).clone() }));
    } else if matches!(msg, Some(crate::unions::Union4::U2(_))) {
        let mut msg_2 = match &msg { Some(crate::unions::Union4::U2(__v)) => __v, _ => unreachable!() };
        if !((&msg_2.group[..] == &group[..])) {
            return None;
        };
        crate::net::add_route(&(from).clone(), &(msg_2.at).clone());
        crate::net::set_peer_protocols(&(from).clone(), &(msg_2.protocols).clone());
        return Some(crate::unions::Union3::U1(crate::net::PeerHello { node: from.clone(), at: (msg_2.at).clone() }));
    } else if matches!(msg, Some(crate::unions::Union4::U3(_))) {
        let mut msg_3 = match &msg { Some(crate::unions::Union4::U3(__v)) => __v, _ => unreachable!() };
        crate::net::node_left(&(from).clone());
        return Some(crate::unions::Union3::U2(crate::net::PeerGone { node: from.clone() }));
    } else if matches!(msg, Some(crate::unions::Union4::U4(_))) {
        let mut msg_4 = match &msg { Some(crate::unions::Union4::U4(__v)) => __v, _ => unreachable!() };
        return Some(crate::unions::Union3::U3(crate::net::PeerIntro { peers: (msg_4.peers).clone() }));
    } else {
        return None;
    };
}

pub fn introduce(node: &crate::net::NodeId, peers: &Vec<crate::net::NodeEndpoint>) {
    let mut intro: crate::unions::Union4<crate::net::Hello, crate::net::Ack, crate::net::Leaving, crate::net::Intro> = crate::unions::Union4::U4(crate::net::Intro { peers: (peers).clone() });
    crate::net::send_control(&(node).clone(), &String::from(""), crate::wire::salvo_encode(&intro));
}

pub fn leave_group(known: &crate::core_map::Map<crate::net::NodeId, crate::net::Node>) {
    for mut id in crate::core_map::keys_platform::<crate::net::NodeId, crate::net::Node>(known).into_iter() {
        let mut leaving: crate::unions::Union4<crate::net::Hello, crate::net::Ack, crate::net::Leaving, crate::net::Intro> = crate::unions::Union4::U3(crate::net::Leaving {});
        crate::net::send_control(&(id).clone(), &String::from(""), crate::wire::salvo_encode(&leaving));
    }
}

pub struct StaticNodeGroup {
    name: String,
    all: Vec<crate::net::NodeEndpoint>,
    __dep0: crate::net::Transport,
    known: crate::core_map::Map<crate::net::NodeId, crate::net::Node>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_StaticNodeGroup>,
}

impl StaticNodeGroup {
    pub fn new(name: String, all: Vec<crate::net::NodeEndpoint>, __dep0: crate::net::Transport) -> Self {
        Self {
            name: name.clone(),
            all: all.clone(),
            __dep0,
            known: crate::core_map::mut_map_of_platform::<crate::net::NodeId, crate::net::Node>(vec![], &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1)),
            watchers: vec![],
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
    fn init(&mut self) {
        let mut me: crate::net::NodeEndpoint = self.__dep0.local_endpoint();
        let mut _connected: bool = crate::net::connect__NodeEndpoint(&self.__dep0, (me).clone());
        crate::scheduler::salvo_watch_control((String::from("")).clone(), (self.__addr.expect("an actor's own addr")).clone(), |__n, __d| std::boxed::Box::new(crate::net::__Priv_StaticNodeGroup::Control(crate::net::NodeId { id: __n as i64 }, __d)));
        for mut e in self.all.iter() {
            if !(crate::net::eq__NodeEndpoint_NodeEndpoint(e, &me)) {
                let mut _sent: crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> = self.__dep0.deliver(&(e).clone(), crate::net::hello_frame(&self.__dep0, &self.name));
            };
        }
    }
    fn control(&mut self, from: crate::net::NodeId, data: crate::core_bytes::Bytes) {
        let mut event: Option<crate::unions::Union3<crate::net::PeerHello, crate::net::PeerGone, crate::net::PeerIntro>> = crate::net::handshake(&self.__dep0, &self.name, from, data);
        if matches!(event, Some(crate::unions::Union3::U1(_))) {
            let mut event_1 = match &event { Some(crate::unions::Union3::U1(__v)) => __v, _ => unreachable!() };
            if crate::core_map::contains_key_platform::<crate::net::NodeId, crate::net::Node>(&self.known, &event_1.node, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1)) {
                return;
            };
            let mut n: crate::net::Node = crate::net::Node { id: (event_1.node).clone(), at: (event_1.at).clone() };
            crate::core_map::put_platform::<crate::net::NodeId, crate::net::Node>(&mut self.known, (event_1.node).clone(), (n).clone(), &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
            for mut w in self.watchers.iter().copied() {
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Joined((n).clone()), crate::net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U2(_))) {
            let mut event_2 = match &event { Some(crate::unions::Union3::U2(__v)) => __v, _ => unreachable!() };
            let mut n: Option<crate::net::Node> = crate::core_map::remove_platform::<crate::net::NodeId, crate::net::Node>(&mut self.known, &event_2.node, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
            if n.is_none() {
                return;
            };
            for mut w in self.watchers.iter().copied() {
                let mut n_3 = n.as_ref().unwrap();
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Left((n_3).clone(), String::from("left")), crate::net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U3(_))) {
            let mut event_4 = match &event { Some(crate::unions::Union3::U3(__v)) => __v, _ => unreachable!() };
        } else {
        };
    }
}

impl crate::net::__Stateful_NodeGroup for StaticNodeGroup {
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<crate::net::Node>>(out, crate::net::known_nodes(&self.known));
    }
    fn subscribe(&mut self, w: usize) {
        for mut id in crate::core_map::keys_platform::<crate::net::NodeId, crate::net::Node>(&self.known).into_iter() {
            let mut n: Option<&crate::net::Node> = crate::core_map::get_platform::<crate::net::NodeId, crate::net::Node>(&self.known, &id, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
            if !(n.is_none()) {
                let mut n_1 = n.unwrap();
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Joined((n_1).clone()), crate::net::__PROTO_NodeGroupWatcher);
            };
        }
        crate::core_list::add_platform::<usize>(&mut self.watchers, w);
    }
    fn leave(&mut self) {
        crate::net::leave_group(&self.known);
    }
}

pub struct __Actor_StaticNodeGroup {
    handler: StaticNodeGroup,
}

impl __Actor_StaticNodeGroup {
    pub fn new(handler: StaticNodeGroup) -> Self {
        Self { handler }
    }
    fn __dispatch_NodeGroup(&mut self, msg: crate::net::__Msg_NodeGroup) {
        match msg {
            crate::net::__Msg_NodeGroup::Members(out) => crate::net::__Stateful_NodeGroup::members(&mut self.handler, out),
            crate::net::__Msg_NodeGroup::Subscribe(w) => crate::net::__Stateful_NodeGroup::subscribe(&mut self.handler, w),
            crate::net::__Msg_NodeGroup::Leave => crate::net::__Stateful_NodeGroup::leave(&mut self.handler),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_StaticNodeGroup) {
        match msg {
            __Priv_StaticNodeGroup::Init => self.handler.init(),
            __Priv_StaticNodeGroup::Control(from, data) => self.handler.control(from, data),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_StaticNodeGroup {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::net::__Msg_NodeGroup>() {
            Ok(__m) => return self.__dispatch_NodeGroup(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_StaticNodeGroup>() {
            Ok(__m) => return self.__dispatch_priv(*__m),
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
            __Cont_StaticNodeGroup::Members => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Subscribe => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Control(from) => self.__dispatch_priv(__Priv_StaticNodeGroup::Control(from, *value.downcast::<crate::core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_StaticNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_StaticNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_StaticNodeGroup);
fn __decode_msg_StaticNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::net::__PROTO_NodeGroup {
        return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroup>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn known_nodes(known: &crate::core_map::Map<crate::net::NodeId, crate::net::Node>) -> Vec<crate::net::Node> {
    let mut all_known: Vec<crate::net::Node> = vec![];
    for mut id in crate::core_map::keys_platform::<crate::net::NodeId, crate::net::Node>(known).into_iter() {
        let mut n: Option<&crate::net::Node> = crate::core_map::get_platform::<crate::net::NodeId, crate::net::Node>(known, &id, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
        if !(n.is_none()) {
            let mut n_1 = n.unwrap();
            crate::core_list::add_platform::<crate::net::Node>(&mut all_known, (n_1).clone());
        };
    }
    return all_known;
}

pub struct GossipNodeGroup {
    name: String,
    seeds: Vec<crate::net::NodeEndpoint>,
    __dep0: crate::net::Transport,
    known: crate::core_map::Map<crate::net::NodeId, crate::net::Node>,
    dialed: crate::core_set::Set<String>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_GossipNodeGroup>,
}

impl GossipNodeGroup {
    pub fn new(name: String, seeds: Vec<crate::net::NodeEndpoint>, __dep0: crate::net::Transport) -> Self {
        Self {
            name: name.clone(),
            seeds: seeds.clone(),
            __dep0,
            known: crate::core_map::mut_map_of_platform::<crate::net::NodeId, crate::net::Node>(vec![], &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1)),
            dialed: crate::core_set::mut_set_of_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])),
            watchers: vec![],
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
    fn init(&mut self) {
        let mut me: crate::net::NodeEndpoint = self.__dep0.local_endpoint();
        let mut _connected: bool = crate::net::connect__NodeEndpoint(&self.__dep0, (me).clone());
        crate::scheduler::salvo_watch_control((String::from("")).clone(), (self.__addr.expect("an actor's own addr")).clone(), |__n, __d| std::boxed::Box::new(crate::net::__Priv_GossipNodeGroup::Control(crate::net::NodeId { id: __n as i64 }, __d)));
        for mut e in self.seeds.iter() {
            crate::net::dial(&self.__dep0, &mut self.dialed, &self.name, (e).clone());
        }
    }
    fn control(&mut self, from: crate::net::NodeId, data: crate::core_bytes::Bytes) {
        let mut event: Option<crate::unions::Union3<crate::net::PeerHello, crate::net::PeerGone, crate::net::PeerIntro>> = crate::net::handshake(&self.__dep0, &self.name, from, data);
        if matches!(event, Some(crate::unions::Union3::U1(_))) {
            let mut event_1 = match &event { Some(crate::unions::Union3::U1(__v)) => __v, _ => unreachable!() };
            if crate::core_map::contains_key_platform::<crate::net::NodeId, crate::net::Node>(&self.known, &event_1.node, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1)) {
                return;
            };
            let mut others: Vec<crate::net::NodeEndpoint> = vec![];
            for mut id in crate::core_map::keys_platform::<crate::net::NodeId, crate::net::Node>(&self.known).into_iter() {
                let mut n: Option<&crate::net::Node> = crate::core_map::get_platform::<crate::net::NodeId, crate::net::Node>(&self.known, &id, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
                if !(n.is_none()) {
                    let mut n_2 = n.unwrap();
                    crate::core_list::add_platform::<crate::net::NodeEndpoint>(&mut others, (n_2.at).clone());
                };
                crate::net::introduce(&(id).clone(), &vec![(event_1.at).clone()]);
            }
            crate::net::introduce(&(event_1.node).clone(), &others);
            crate::core_set::add_platform::<String>(&mut self.dialed, crate::net::to_str__NodeEndpoint(&event_1.at), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
            let mut n: crate::net::Node = crate::net::Node { id: (event_1.node).clone(), at: (event_1.at).clone() };
            crate::core_map::put_platform::<crate::net::NodeId, crate::net::Node>(&mut self.known, (event_1.node).clone(), (n).clone(), &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
            for mut w in self.watchers.iter().copied() {
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Joined((n).clone()), crate::net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U2(_))) {
            let mut event_3 = match &event { Some(crate::unions::Union3::U2(__v)) => __v, _ => unreachable!() };
            let mut n: Option<crate::net::Node> = crate::core_map::remove_platform::<crate::net::NodeId, crate::net::Node>(&mut self.known, &event_3.node, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
            if n.is_none() {
                return;
            };
            for mut w in self.watchers.iter().copied() {
                let mut n_4 = n.as_ref().unwrap();
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Left((n_4).clone(), String::from("left")), crate::net::__PROTO_NodeGroupWatcher);
            }
        } else if matches!(event, Some(crate::unions::Union3::U3(_))) {
            let mut event_5 = match &event { Some(crate::unions::Union3::U3(__v)) => __v, _ => unreachable!() };
            for mut e in event_5.peers.iter() {
                crate::net::dial(&self.__dep0, &mut self.dialed, &self.name, (e).clone());
            }
        } else {
        };
    }
}

impl crate::net::__Stateful_NodeGroup for GossipNodeGroup {
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<crate::net::Node>>(out, crate::net::known_nodes(&self.known));
    }
    fn subscribe(&mut self, w: usize) {
        for mut id in crate::core_map::keys_platform::<crate::net::NodeId, crate::net::Node>(&self.known).into_iter() {
            let mut n: Option<&crate::net::Node> = crate::core_map::get_platform::<crate::net::NodeId, crate::net::Node>(&self.known, &id, &mut |__a0: &crate::net::NodeId| crate::net::hash__NodeId(__a0), &mut |__a0: &crate::net::NodeId, __a1: &crate::net::NodeId| crate::net::eq__NodeId_NodeId(__a0, __a1));
            if !(n.is_none()) {
                let mut n_1 = n.unwrap();
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Joined((n_1).clone()), crate::net::__PROTO_NodeGroupWatcher);
            };
        }
        crate::core_list::add_platform::<usize>(&mut self.watchers, w);
    }
    fn leave(&mut self) {
        crate::net::leave_group(&self.known);
    }
}

pub struct __Actor_GossipNodeGroup {
    handler: GossipNodeGroup,
}

impl __Actor_GossipNodeGroup {
    pub fn new(handler: GossipNodeGroup) -> Self {
        Self { handler }
    }
    fn __dispatch_NodeGroup(&mut self, msg: crate::net::__Msg_NodeGroup) {
        match msg {
            crate::net::__Msg_NodeGroup::Members(out) => crate::net::__Stateful_NodeGroup::members(&mut self.handler, out),
            crate::net::__Msg_NodeGroup::Subscribe(w) => crate::net::__Stateful_NodeGroup::subscribe(&mut self.handler, w),
            crate::net::__Msg_NodeGroup::Leave => crate::net::__Stateful_NodeGroup::leave(&mut self.handler),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_GossipNodeGroup) {
        match msg {
            __Priv_GossipNodeGroup::Init => self.handler.init(),
            __Priv_GossipNodeGroup::Control(from, data) => self.handler.control(from, data),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_GossipNodeGroup {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::net::__Msg_NodeGroup>() {
            Ok(__m) => return self.__dispatch_NodeGroup(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_GossipNodeGroup>() {
            Ok(__m) => return self.__dispatch_priv(*__m),
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
            __Cont_GossipNodeGroup::Members => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Subscribe => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Control(from) => self.__dispatch_priv(__Priv_GossipNodeGroup::Control(from, *value.downcast::<crate::core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_GossipNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_GossipNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_GossipNodeGroup);
fn __decode_msg_GossipNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::net::__PROTO_NodeGroup {
        return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroup>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn dial(transport: &crate::net::Transport, dialed: &mut crate::core_set::Set<String>, group: &String, mut e: crate::net::NodeEndpoint) {
    if (crate::net::eq__NodeEndpoint_NodeEndpoint(&e, &transport.local_endpoint()) || crate::core_set::contains_platform::<String>(&*dialed, &crate::net::to_str__NodeEndpoint(&e), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
        return;
    };
    crate::core_set::add_platform::<String>(&mut *dialed, crate::net::to_str__NodeEndpoint(&e), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    let mut sent: crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> = transport.deliver(&(e).clone(), crate::net::hello_frame(transport, group));
    if matches!(sent, crate::unions::Union2::U2(_)) {
        let mut sent_1 = match &sent { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut _forgot: bool = crate::core_set::remove_platform::<String>(&mut *dialed, &crate::net::to_str__NodeEndpoint(&e), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
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

pub struct __Stub_ActorGroup {
    addr: usize,
}

impl __Stub_ActorGroup {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_ActorGroup for __Stub_ActorGroup {
    fn join(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Join(member), crate::net::__PROTO_ActorGroup);
    }
    fn leave(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Leave(member), crate::net::__PROTO_ActorGroup);
    }
    fn members(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Members(out), crate::net::__PROTO_ActorGroup);
    }
    fn subscribe(&self, w: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Subscribe(w), crate::net::__PROTO_ActorGroup);
    }
    fn refresh(&self) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Refresh, crate::net::__PROTO_ActorGroup);
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

pub struct __Stub_ActorGroupWatcher {
    addr: usize,
}

impl __Stub_ActorGroupWatcher {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_ActorGroupWatcher for __Stub_ActorGroupWatcher {
    fn joined(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroupWatcher::Joined(member), crate::net::__PROTO_ActorGroupWatcher);
    }
    fn left(&self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroupWatcher::Left(member), crate::net::__PROTO_ActorGroupWatcher);
    }
}

pub fn share_members(name: &String, hash: &String, node: &crate::net::NodeId, members: &Vec<usize>) {
    crate::net::send_control(&(node).clone(), &(name).clone(), { let __enc: (String, Vec<usize>) = ((hash).clone(), (members).clone()); crate::wire::salvo_encode(&__enc) });
}

pub fn pending(mut a: usize) -> i32 {
    return crate::runtime_routing::pending((((a).clone()) as i32));
}

pub fn actor_group__Addr(mut nodes: usize, protocol: &mut dyn FnMut() -> crate::net::Protocol) -> usize {
    let mut proto: crate::net::Protocol = protocol();
    let mut name: String = (proto.name).clone();
    return crate::net::actor_group__Str_Addr(name, nodes, &mut *protocol);
}

pub fn actor_group__Str_Addr(mut name: String, mut nodes: usize, protocol: &mut dyn FnMut() -> crate::net::Protocol) -> usize {
    let mut __destructured_1: (usize, usize) = ({ let __h = crate::net::ActorGrouping::new(name.clone(), protocol()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::core_actor::pool(1i32), __cap as usize, std::boxed::Box::new(crate::net::__Actor_ActorGrouping::new(__h)), crate::net::__DECODE_ActorGrouping); crate::scheduler::salvo_send(__a, std::boxed::Box::new(crate::net::__Priv_ActorGrouping::Init)); (__a, __a) });
    let mut group: usize = __destructured_1.0;
    let mut watcher: usize = __destructured_1.1;
    crate::scheduler::salvo_send_wire(nodes, crate::net::__Msg_NodeGroup::Subscribe(watcher), crate::net::__PROTO_NodeGroup);
    return group;
}

pub fn join(mut group: usize, mut member: usize) {
    crate::scheduler::salvo_send_wire(group, crate::net::__Msg_ActorGroup::Join(member), crate::net::__PROTO_ActorGroup);
}

pub struct ActorGrouping {
    name: String,
    proto: crate::net::Protocol,
    all: Vec<usize>,
    peers: Vec<crate::net::NodeId>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_ActorGrouping>,
}

impl ActorGrouping {
    pub fn new(name: String, proto: crate::net::Protocol) -> Self {
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
        crate::scheduler::salvo_watch_control(((self.name).clone()).clone(), (self.__addr.expect("an actor's own addr")).clone(), |__n, __d| std::boxed::Box::new(crate::net::__Priv_ActorGrouping::Control(crate::net::NodeId { id: __n as i64 }, __d)));
    }
    fn control(&mut self, from: crate::net::NodeId, data: crate::core_bytes::Bytes) {
        let mut got: Option<(String, Vec<usize>)> = crate::wire::salvo_decode::<(String, Vec<usize>)>(&data);
        if got.is_none() {
            return;
        };
        let mut got_1 = got.as_ref().unwrap();
        if !((&got_1.0[..] == &self.proto.hash[..])) {
            return;
        };
        if !(crate::net::contains_node(&self.peers, &(from).clone())) {
            crate::core_list::add_platform::<crate::net::NodeId>(&mut self.peers, (from).clone());
            crate::net::share_members(&self.name, &self.proto.hash, &(from).clone(), &self.all);
        };
        let mut changed: bool = false;
        let mut got_2 = got.as_ref().unwrap();
        for mut m in got_2.1.iter().copied() {
            if crate::net::admit(&mut self.all, m) {
                changed = true;
                for mut w in self.watchers.iter().copied() {
                    crate::scheduler::salvo_send_wire(w, crate::net::__Msg_ActorGroupWatcher::Joined(m), crate::net::__PROTO_ActorGroupWatcher);
                }
            };
        }
        if changed {
            crate::net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        };
    }
}

impl crate::net::__Stateful_ActorGroup for ActorGrouping {
    fn join(&mut self, member: usize) {
        if !(crate::net::admit(&mut self.all, member)) {
            return;
        };
        crate::net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        for mut w in self.watchers.iter().copied() {
            crate::scheduler::salvo_send_wire(w, crate::net::__Msg_ActorGroupWatcher::Joined(member), crate::net::__PROTO_ActorGroupWatcher);
        }
        for mut p in self.peers.iter() {
            crate::net::share_members(&self.name, &self.proto.hash, &(p).clone(), &vec![member]);
        }
    }
    fn leave(&mut self, member: usize) {
        if !(crate::net::withdraw(&mut self.all, member)) {
            return;
        };
        crate::net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
        for mut w in self.watchers.iter().copied() {
            crate::scheduler::salvo_send_wire(w, crate::net::__Msg_ActorGroupWatcher::Left(member), crate::net::__PROTO_ActorGroupWatcher);
        }
    }
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<usize>>(out, (self.all).clone());
    }
    fn subscribe(&mut self, w: usize) {
        crate::core_list::add_platform::<usize>(&mut self.watchers, w);
    }
    fn refresh(&mut self) {
        crate::net::view_refresh(self.__addr.expect("an actor's own addr"));
    }
}

impl crate::net::__Stateful_NodeGroupWatcher for ActorGrouping {
    fn joined(&mut self, n: crate::net::Node) {
        crate::net::share_members(&self.name, &self.proto.hash, &n.id, &self.all);
    }
    fn left(&mut self, n: crate::net::Node, why: String) {
        let mut gone: Vec<usize> = vec![];
        for mut m in self.all.iter().copied() {
            if crate::net::eq__NodeId_NodeId(&crate::net::node_of(m), &n.id) {
                crate::core_list::add_platform::<usize>(&mut gone, m);
            };
        }
        for mut m in gone.iter().copied() {
            if crate::net::withdraw(&mut self.all, m) {
                for mut w in self.watchers.iter().copied() {
                    crate::scheduler::salvo_send_wire(w, crate::net::__Msg_ActorGroupWatcher::Left(m), crate::net::__PROTO_ActorGroupWatcher);
                }
            };
        }
        if (crate::core_list::size_platform::<usize>(&gone) > 0i32) {
            crate::net::mirror(self.__addr.expect("an actor's own addr"), &self.all);
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
    fn __dispatch_ActorGroup(&mut self, msg: crate::net::__Msg_ActorGroup) {
        match msg {
            crate::net::__Msg_ActorGroup::Join(member) => crate::net::__Stateful_ActorGroup::join(&mut self.handler, member),
            crate::net::__Msg_ActorGroup::Leave(member) => crate::net::__Stateful_ActorGroup::leave(&mut self.handler, member),
            crate::net::__Msg_ActorGroup::Members(out) => crate::net::__Stateful_ActorGroup::members(&mut self.handler, out),
            crate::net::__Msg_ActorGroup::Subscribe(w) => crate::net::__Stateful_ActorGroup::subscribe(&mut self.handler, w),
            crate::net::__Msg_ActorGroup::Refresh => crate::net::__Stateful_ActorGroup::refresh(&mut self.handler),
        }
    }
    fn __dispatch_NodeGroupWatcher(&mut self, msg: crate::net::__Msg_NodeGroupWatcher) {
        match msg {
            crate::net::__Msg_NodeGroupWatcher::Joined(n) => crate::net::__Stateful_NodeGroupWatcher::joined(&mut self.handler, n),
            crate::net::__Msg_NodeGroupWatcher::Left(n, why) => crate::net::__Stateful_NodeGroupWatcher::left(&mut self.handler, n, why),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_ActorGrouping) {
        match msg {
            __Priv_ActorGrouping::Init => self.handler.init(),
            __Priv_ActorGrouping::Control(from, data) => self.handler.control(from, data),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_ActorGrouping {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::net::__Msg_ActorGroup>() {
            Ok(__m) => return self.__dispatch_ActorGroup(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<crate::net::__Msg_NodeGroupWatcher>() {
            Ok(__m) => return self.__dispatch_NodeGroupWatcher(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<__Priv_ActorGrouping>() {
            Ok(__m) => return self.__dispatch_priv(*__m),
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
            __Cont_ActorGrouping::Join => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Join(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Leave => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Leave(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Members => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Subscribe => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Joined => self.__dispatch_NodeGroupWatcher(crate::net::__Msg_NodeGroupWatcher::Joined(*value.downcast::<crate::net::Node>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Left(n) => self.__dispatch_NodeGroupWatcher(crate::net::__Msg_NodeGroupWatcher::Left(n, *value.downcast::<String>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Control(from) => self.__dispatch_priv(__Priv_ActorGrouping::Control(from, *value.downcast::<crate::core_bytes::Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ActorGrouping::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Leave{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Joined{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::net::Node>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Left{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::core_bytes::Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_ActorGrouping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_ActorGrouping);
fn __decode_msg_ActorGrouping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::net::__PROTO_ActorGroup {
        return crate::wire::salvo_decode::<crate::net::__Msg_ActorGroup>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    if proto == crate::net::__PROTO_NodeGroupWatcher {
        return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroupWatcher>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

pub fn mirror(mut group: usize, members: &Vec<usize>) {
    crate::net::view_set(group, (members).clone());
}

pub fn admit(list: &mut Vec<usize>, mut a: usize) -> bool {
    for mut x in list.iter().copied() {
        if crate::core_actor::eq(x, a) {
            return false;
        };
    }
    crate::core_list::add_platform::<usize>(&mut *list, a);
    return true;
}

pub fn contains_node(list: &Vec<crate::net::NodeId>, n: &crate::net::NodeId) -> bool {
    for mut x in list.iter() {
        if crate::net::eq__NodeId_NodeId(x, n) {
            return true;
        };
    }
    return false;
}

pub fn withdraw(list: &mut Vec<usize>, mut a: usize) -> bool {
    let mut mut_index: Option<i32> = None;
    let mut i: i32 = 0i32;
    for mut x in list.iter().copied() {
        if crate::core_actor::eq(x, a) {
            mut_index = Some(i);
        };
        i = i32::wrapping_add(i, 1i32);
    }
    if mut_index.is_none() {
        return false;
    };
    let mut mut_index_1 = mut_index.unwrap();
    let mut _removed: Option<usize> = crate::core_list::remove_at_platform::<usize>(&mut *list, mut_index_1);
    return true;
}

pub fn node_of(mut a: usize) -> crate::net::NodeId {
    return crate::net::NodeId { id: {
        let mut __proj_1: crate::runtime_routing::RemoteRef = crate::runtime_routing::identity((((a).clone()) as i32));
        __proj_1.node
    } };
}

pub fn view_set(mut group: usize, mut members: Vec<usize>) {
    let mut ixs: Vec<i32> = vec![];
    for mut m in members.iter().copied() {
        crate::core_list::add_platform::<i32>(&mut ixs, (((m).clone()) as i32));
    }
    crate::runtime_routing::view_set((((group).clone()) as i32), ixs);
}

pub fn view_members(mut group: usize) -> Vec<usize> {
    let mut out: Vec<usize> = vec![];
    for mut ix in crate::runtime_routing::view_members((((group).clone()) as i32)).iter().copied() {
        crate::core_list::add_platform::<usize>(&mut out, ((ix) as usize));
    }
    return out;
}

pub fn view_version(mut group: usize) -> i64 {
    return crate::runtime_routing::view_version((((group).clone()) as i32));
}

pub fn view_refresh(mut group: usize) {
    crate::runtime_routing::view_refresh((((group).clone()) as i32));
}

pub fn view_wait(mut group: usize, mut seen: i64, mut nanos: i64) {
    crate::runtime_routing::view_wait((((group).clone()) as i32), seen, nanos);
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
    pub members: Vec<crate::net::RouteMember>,
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

pub fn default_route_config() -> crate::net::RouteConfig {
    return crate::net::RouteConfig { first_wait: crate::time::Duration { nanos: 1000000i64 }, max_wait: crate::time::Duration { nanos: 5000000000i64 } };
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteConfig {
    pub first_wait: crate::time::Duration,
    pub max_wait: crate::time::Duration,
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
    fn changed(&self, view: &crate::net::RouteView);
    fn select(&self, view: &crate::net::RouteView, key: &Option<i64>) -> Option<usize>;
}

pub trait __Stateful_RouteSelector: Send {
    fn changed(&mut self, view: &crate::net::RouteView);
    fn select(&mut self, view: &crate::net::RouteView, key: &Option<i64>) -> Option<usize>;
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
    pub fn changed(&self, view: &crate::net::RouteView) {
        match &self.inner {
            __Inner_RouteSelector::Shared(h) => h.changed(view),
            __Inner_RouteSelector::Locked(h) => h.lock().unwrap().changed(view),
        }
    }
    pub fn select(&self, view: &crate::net::RouteView, key: &Option<i64>) -> Option<usize> {
        match &self.inner {
            __Inner_RouteSelector::Shared(h) => h.select(view, key),
            __Inner_RouteSelector::Locked(h) => h.lock().unwrap().select(view, key),
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

pub fn route_pick__Addr_RouteConfig_Long(route_selector: &crate::net::RouteSelector, mut group: usize, config: &crate::net::RouteConfig, mut seen: i64) -> crate::net::RoutePick {
    return crate::net::route_keyed(route_selector, group, config, seen, None);
}

pub fn route_pick__Addr_RouteConfig_Long_Long(route_selector: &crate::net::RouteSelector, mut group: usize, config: &crate::net::RouteConfig, mut seen: i64, mut key: i64) -> crate::net::RoutePick {
    return crate::net::route_keyed(route_selector, group, config, seen, Some(key));
}

pub fn route_keyed(route_selector: &crate::net::RouteSelector, mut group: usize, config: &crate::net::RouteConfig, mut seen: i64, mut key: Option<i64>) -> crate::net::RoutePick {
    let mut wait: i64 = config.first_wait.nanos;
    let mut cap: i64 = config.max_wait.nanos;
    let mut last: i64 = seen;
    loop {
        if !(true) {
            break;
        };
        let mut version: i64 = crate::net::view_version(group);
        let mut view: crate::net::RouteView = crate::net::route_view(group);
        if !(((version) == (last))) {
            route_selector.changed(&(view).clone());
            last = version;
        };
        let mut picked: Option<usize> = route_selector.select(&view, &(key).clone());
        if !(picked.is_none()) {
            let mut picked_1 = picked.unwrap();
            return crate::net::RoutePick { to: picked_1, version: last };
        };
        crate::net::view_wait(group, version, wait);
        wait = i64::wrapping_mul(wait, 2i64);
        if (wait > cap) {
            wait = cap;
        };
    }
    return crate::net::route_keyed(route_selector, group, config, seen, key);
}

pub fn route_view(mut group: usize) -> crate::net::RouteView {
    let mut members: Vec<crate::net::RouteMember> = vec![];
    for mut m in crate::net::view_members(group).iter().copied() {
        crate::core_list::add_platform::<crate::net::RouteMember>(&mut members, crate::net::RouteMember { addr: m, local: crate::net::eq__NodeId_NodeId(&crate::net::node_of(m), &crate::net::this_node()) });
    }
    return crate::net::RouteView { members: (members).clone() };
}

#[derive(Clone)]
pub struct LeastLoaded {
    prefer_local: bool,
}

impl LeastLoaded {
    pub fn new(prefer_local: bool) -> Self {
        Self {
            prefer_local
        }
    }
}

impl crate::net::__Stateless_RouteSelector for LeastLoaded {
    fn changed(&self, view: &crate::net::RouteView) {
    }
    fn select(&self, view: &crate::net::RouteView, key: &Option<i64>) -> Option<usize> {
        let mut best: Option<crate::net::RouteMember> = None;
        let mut best_pending: i32 = 0i32;
        for mut a in view.members.iter() {
            let mut load: i32 = crate::net::pending(a.addr);
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
        let mut chosen: crate::net::RouteMember = {
            let mut __elv_2 = &best;
            if __elv_2.is_none() {
                {
                    return None;
                }
            } else {
                let mut __some_3 = __elv_2.as_ref().unwrap();
                __some_3.clone()
            }
        };
        return Some(chosen.addr);
    }
}

#[derive(Clone)]
pub struct Sharded {
}

impl Sharded {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl crate::net::__Stateless_RouteSelector for Sharded {
    fn changed(&self, view: &crate::net::RouteView) {
    }
    fn select(&self, view: &crate::net::RouteView, key: &Option<i64>) -> Option<usize> {
        let mut n: i32 = crate::core_list::size_platform::<crate::net::RouteMember>(&view.members);
        if ((n) == (0i32)) {
            return None;
        };
        let mut k: i64 = {
            let mut __elv_1 = key;
            if __elv_1.is_none() {
                0i64
            } else {
                let mut __some_2 = __elv_1.unwrap();
                __some_2
            }
        };
        let mut magnitude: i64 = if (k < 0i64) {
            i64::wrapping_sub(0i64, k)
        } else {
            k
        };
        let mut slot: i32 = ((i64::wrapping_rem(magnitude, ((n) as i64))) as i32);
        let mut picked: &crate::net::RouteMember = {
            let mut __elv_3: Option<&crate::net::RouteMember> = crate::core_list::get_platform::<crate::net::RouteMember>(&view.members, slot);
            if __elv_3.is_none() {
                {
                    return None;
                }
            } else {
                let mut __some_4 = __elv_3.unwrap();
                __some_4
            }
        };
        return Some(picked.addr);
    }
}

pub trait __Stateless_Leader: Send + Sync {
    fn leader(&self) -> Option<crate::net::NodeId>;
}

pub trait __Stateful_Leader: Send {
    fn leader(&mut self) -> Option<crate::net::NodeId>;
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
    pub fn leader(&self) -> Option<crate::net::NodeId> {
        match &self.inner {
            __Inner_Leader::Shared(h) => h.leader(),
            __Inner_Leader::Locked(h) => h.lock().unwrap().leader(),
        }
    }
}

#[derive(Clone)]
pub struct StaticLeader {
    node: crate::net::NodeId,
}

impl StaticLeader {
    pub fn new(node: crate::net::NodeId) -> Self {
        Self {
            node
        }
    }
}

impl crate::net::__Stateless_Leader for StaticLeader {
    fn leader(&self) -> Option<crate::net::NodeId> {
        return Some((self.node).clone());
    }
}

pub struct Elected {
    __dep0: crate::net::Leader,
    chosen: Option<usize>,
}

impl Elected {
    pub fn new(__dep0: crate::net::Leader) -> Self {
        Self {
            __dep0,
            chosen: None
        }
    }
}

impl crate::net::__Stateful_RouteSelector for Elected {
    fn changed(&mut self, view: &crate::net::RouteView) {
        self.chosen = None;
        let mut l: crate::net::NodeId = {
            let mut __elv_1: Option<crate::net::NodeId> = self.__dep0.leader();
            if __elv_1.is_none() {
                {
                    return;
                }
            } else {
                let mut __some_2 = __elv_1.unwrap();
                __some_2
            }
        };
        for mut a in view.members.iter() {
            if crate::net::eq__NodeId_NodeId(&crate::net::node_of(a.addr), &l) {
                self.chosen = Some(a.addr);
                return;
            };
        }
    }
    fn select(&mut self, view: &crate::net::RouteView, key: &Option<i64>) -> Option<usize> {
        return (self.chosen).clone();
    }
}

pub type __Platform_HostTcpTransport = crate::net::__Platform_Transport<crate::platform_net::HostTcpTransport>;

impl __Platform_HostTcpTransport {
    pub fn new(bind: crate::net::NodeEndpoint) -> Self {
        crate::net::__Platform_Transport(crate::platform_net::HostTcpTransport::new(bind))
    }
}

pub trait __Stateless_MemNet: Send + Sync {
    fn attach(&self, at: crate::net::NodeEndpoint, sink: usize);
    fn detach(&self, at: crate::net::NodeEndpoint);
    fn route(&self, from: crate::net::NodeEndpoint, to: crate::net::NodeEndpoint, out: crate::scheduler::SalvoReply);
    fn partition(&self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint);
    fn heal(&self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint);
    fn kill(&self, node: crate::net::NodeEndpoint);
    fn delivered(&self, out: crate::scheduler::SalvoReply);
}

pub trait __Stateful_MemNet: Send {
    fn attach(&mut self, at: crate::net::NodeEndpoint, sink: usize);
    fn detach(&mut self, at: crate::net::NodeEndpoint);
    fn route(&mut self, from: crate::net::NodeEndpoint, to: crate::net::NodeEndpoint, out: crate::scheduler::SalvoReply);
    fn partition(&mut self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint);
    fn heal(&mut self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint);
    fn kill(&mut self, node: crate::net::NodeEndpoint);
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
    pub fn attach(&self, at: crate::net::NodeEndpoint, sink: usize) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.attach(at, sink),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().attach(at, sink),
        }
    }
    pub fn detach(&self, at: crate::net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.detach(at),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().detach(at),
        }
    }
    pub fn route(&self, from: crate::net::NodeEndpoint, to: crate::net::NodeEndpoint, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.route(from, to, out),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().route(from, to, out),
        }
    }
    pub fn partition(&self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.partition(a, b),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().partition(a, b),
        }
    }
    pub fn heal(&self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.heal(a, b),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().heal(a, b),
        }
    }
    pub fn kill(&self, node: crate::net::NodeEndpoint) {
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

pub struct __Stub_MemNet {
    addr: usize,
}

impl __Stub_MemNet {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_MemNet for __Stub_MemNet {
    fn attach(&self, at: crate::net::NodeEndpoint, sink: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Attach(at, sink), crate::net::__PROTO_MemNet);
    }
    fn detach(&self, at: crate::net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Detach(at), crate::net::__PROTO_MemNet);
    }
    fn route(&self, from: crate::net::NodeEndpoint, to: crate::net::NodeEndpoint, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Route(from, to, out), crate::net::__PROTO_MemNet);
    }
    fn partition(&self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Partition(a, b), crate::net::__PROTO_MemNet);
    }
    fn heal(&self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Heal(a, b), crate::net::__PROTO_MemNet);
    }
    fn kill(&self, node: crate::net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Kill(node), crate::net::__PROTO_MemNet);
    }
    fn delivered(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Delivered(out), crate::net::__PROTO_MemNet);
    }
}

pub struct MemNetwork {
    listeners: crate::core_map::Map<crate::net::NodeEndpoint, usize>,
    cuts: crate::core_set::Set<String>,
    dead: crate::core_set::Set<crate::net::NodeEndpoint>,
    count: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_MemNetwork>,
}

impl MemNetwork {
    pub fn new() -> Self {
        Self {
            listeners: crate::core_map::mut_map_of_platform::<crate::net::NodeEndpoint, usize>(vec![], &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)),
            cuts: crate::core_set::mut_set_of_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])),
            dead: crate::core_set::mut_set_of_platform::<crate::net::NodeEndpoint>(vec![], &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)),
            count: 0i32,
            __mailbox_capacity: 64i32,
            __addr: None,
            __parked: std::collections::HashMap::new()
        }
    }
}

impl crate::net::__Stateful_MemNet for MemNetwork {
    fn attach(&mut self, at: crate::net::NodeEndpoint, sink: usize) {
        crate::core_set::remove_platform::<crate::net::NodeEndpoint>(&mut self.dead, &at, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
        crate::core_map::put_platform::<crate::net::NodeEndpoint, usize>(&mut self.listeners, at, sink, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
    }
    fn detach(&mut self, at: crate::net::NodeEndpoint) {
        crate::core_map::remove_platform::<crate::net::NodeEndpoint, usize>(&mut self.listeners, &at, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
    }
    fn route(&mut self, from: crate::net::NodeEndpoint, to: crate::net::NodeEndpoint, out: crate::scheduler::SalvoReply) {
        if (crate::core_set::contains_platform::<crate::net::NodeEndpoint>(&self.dead, &to, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)) || crate::core_set::contains_platform::<String>(&self.cuts, &crate::net::cut_key(&from, &to), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))) {
            crate::scheduler::salvo_reply_wire::<Option<usize>>(out, None);
            return;
        };
        let mut sink: Option<usize> = crate::core_map::get_platform::<crate::net::NodeEndpoint, usize>(&self.listeners, &to, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1)).copied();
        if sink.is_none() {
            crate::scheduler::salvo_reply_wire::<Option<usize>>(out, None);
            return;
        };
        self.count = i32::wrapping_add(self.count, 1i32);
        let mut sink_1 = sink.unwrap();
        crate::scheduler::salvo_reply_wire::<Option<usize>>(out, Some(sink_1));
    }
    fn partition(&mut self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint) {
        crate::core_set::add_platform::<String>(&mut self.cuts, crate::net::cut_key(&a, &b), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::core_set::add_platform::<String>(&mut self.cuts, crate::net::cut_key(&b, &a), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    }
    fn heal(&mut self, a: crate::net::NodeEndpoint, b: crate::net::NodeEndpoint) {
        crate::core_set::remove_platform::<String>(&mut self.cuts, &crate::net::cut_key(&a, &b), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::core_set::remove_platform::<String>(&mut self.cuts, &crate::net::cut_key(&b, &a), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
    }
    fn kill(&mut self, node: crate::net::NodeEndpoint) {
        crate::core_map::remove_platform::<crate::net::NodeEndpoint, usize>(&mut self.listeners, &node, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
        crate::core_set::add_platform::<crate::net::NodeEndpoint>(&mut self.dead, node, &mut |__a0: &crate::net::NodeEndpoint| crate::net::hash__NodeEndpoint(__a0), &mut |__a0: &crate::net::NodeEndpoint, __a1: &crate::net::NodeEndpoint| crate::net::eq__NodeEndpoint_NodeEndpoint(__a0, __a1));
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
    fn __dispatch(&mut self, msg: crate::net::__Msg_MemNet) {
        match msg {
            crate::net::__Msg_MemNet::Attach(at, sink) => crate::net::__Stateful_MemNet::attach(&mut self.handler, at, sink),
            crate::net::__Msg_MemNet::Detach(at) => crate::net::__Stateful_MemNet::detach(&mut self.handler, at),
            crate::net::__Msg_MemNet::Route(from, to, out) => crate::net::__Stateful_MemNet::route(&mut self.handler, from, to, out),
            crate::net::__Msg_MemNet::Partition(a, b) => crate::net::__Stateful_MemNet::partition(&mut self.handler, a, b),
            crate::net::__Msg_MemNet::Heal(a, b) => crate::net::__Stateful_MemNet::heal(&mut self.handler, a, b),
            crate::net::__Msg_MemNet::Kill(node) => crate::net::__Stateful_MemNet::kill(&mut self.handler, node),
            crate::net::__Msg_MemNet::Delivered(out) => crate::net::__Stateful_MemNet::delivered(&mut self.handler, out),
        }
    }
}

impl crate::scheduler::SalvoActor for __Actor_MemNetwork {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = *msg.downcast::<crate::net::__Msg_MemNet>().expect("message of this protocol");
        self.__dispatch(msg);
    }

    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let Some(__cont) = self.handler.__parked.remove(&slot) else {
            return; // a reply whose continuation is gone: nothing to run
        };
        match __cont {
            __Cont_MemNetwork::Attach(at) => self.__dispatch(crate::net::__Msg_MemNet::Attach(at, *value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_MemNetwork::Detach => self.__dispatch(crate::net::__Msg_MemNet::Detach(*value.downcast::<crate::net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Route(from, to) => self.__dispatch(crate::net::__Msg_MemNet::Route(from, to, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_MemNetwork::Partition(a) => self.__dispatch(crate::net::__Msg_MemNet::Partition(a, *value.downcast::<crate::net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Heal(a) => self.__dispatch(crate::net::__Msg_MemNet::Heal(a, *value.downcast::<crate::net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Kill => self.__dispatch(crate::net::__Msg_MemNet::Kill(*value.downcast::<crate::net::NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Delivered => self.__dispatch(crate::net::__Msg_MemNet::Delivered(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_MemNetwork::Attach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Detach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Route{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Partition{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Heal{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Kill{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::net::NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Delivered{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_MemNetwork: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_MemNetwork);
fn __decode_msg_MemNetwork(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
    if proto == crate::net::__PROTO_MemNet {
        return crate::wire::salvo_decode::<crate::net::__Msg_MemNet>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
    }
    None
}

#[derive(Clone)]
pub struct MemTransport {
    me: crate::net::NodeEndpoint,
    net: usize,
}

impl MemTransport {
    pub fn new(me: crate::net::NodeEndpoint, net: usize) -> Self {
        Self {
            me,
            net
        }
    }
}

impl crate::net::__Stateless_Transport for MemTransport {
    fn listen(&self, at: &crate::net::NodeEndpoint, sink: usize) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        crate::scheduler::salvo_send_wire(self.net, crate::net::__Msg_MemNet::Attach((at).clone(), sink), crate::net::__PROTO_MemNet);
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn unlisten(&self, at: &crate::net::NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.net, crate::net::__Msg_MemNet::Detach((at).clone()), crate::net::__PROTO_MemNet);
    }
    fn deliver(&self, to: &crate::net::NodeEndpoint, frame: crate::core_bytes::Bytes) -> crate::unions::Union2<(), crate::unions::Union2<crate::net::Unreachable, crate::net::WireFailed>> {
        let mut sink: Option<usize> = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Option<usize>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.net, crate::net::__Msg_MemNet::Route((self.me).clone(), (to).clone(), out), crate::net::__PROTO_MemNet);
            *crate::scheduler::salvo_wait(__wid).downcast::<Option<usize>>().expect("the awaited answer")
        };
        if sink.is_none() {
            return crate::unions::Union2::U2(crate::unions::Union2::U1(crate::core_result::err::<crate::net::Unreachable>(crate::net::Unreachable { to: (to).clone() })));
        };
        let mut sink_1 = sink.unwrap();
        crate::scheduler::salvo_send_wire(sink_1, crate::net::__Msg_Inbound::ReceiveFrame((self.me).clone(), frame.clone()), crate::net::__PROTO_Inbound);
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn local_endpoint(&self) -> crate::net::NodeEndpoint {
        return (self.me).clone();
    }
}

pub fn cut_key(a: &crate::net::NodeEndpoint, b: &crate::net::NodeEndpoint) -> String {
    return format!("{}>{}", crate::net::to_str__NodeEndpoint(a), crate::net::to_str__NodeEndpoint(b));
}

pub fn cmp(a: &crate::net::NodeEndpoint, b: &crate::net::NodeEndpoint) -> i32 {
    let mut c__c1: i32 = (Ord::cmp(&a.host[..], &b.host[..]) as i32);
    if !(((c__c1) == (0i32))) {
        return c__c1;
    };
    let mut c__c2: i32 = (Ord::cmp(&(a.port), &(b.port)) as i32);
    if !(((c__c2) == (0i32))) {
        return c__c2;
    };
    return 0i32;
}

pub fn hash__NodeEndpoint(value: &crate::net::NodeEndpoint) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.host[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.port), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__NodeEndpoint_NodeEndpoint(a: &crate::net::NodeEndpoint, b: &crate::net::NodeEndpoint) -> bool {
    if !((&a.host[..] == &b.host[..])) {
        return false;
    };
    if !(((a.port) == (b.port))) {
        return false;
    };
    return true;
}

pub fn hash__NodeId(value: &crate::net::NodeId) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.id), &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq__NodeId_NodeId(a: &crate::net::NodeId, b: &crate::net::NodeId) -> bool {
    if !(((a.id) == (b.id))) {
        return false;
    };
    return true;
}

pub fn hash__Node(value: &crate::net::Node) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::core_compare::mix_hash(h, crate::net::hash__NodeId(&value.id));
    h = crate::core_compare::mix_hash(h, crate::net::hash__NodeEndpoint(&value.at));
    return h;
}

pub fn eq__Node_Node(a: &crate::net::Node, b: &crate::net::Node) -> bool {
    if !(crate::net::eq__NodeId_NodeId(&a.id, &b.id)) {
        return false;
    };
    if !(crate::net::eq__NodeEndpoint_NodeEndpoint(&a.at, &b.at)) {
        return false;
    };
    return true;
}

pub enum __Msg_Inbound {
    ReceiveFrame(crate::net::NodeEndpoint, crate::core_bytes::Bytes),
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

/// [protocol-hash] The canonical hash of `Inbound`.
pub const __PROTO_Inbound: &str = "8e46ddb2a3e90b03";

pub enum __Msg_Outbound {
    SendFrame(crate::net::NodeEndpoint, crate::core_bytes::Bytes),
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

/// [protocol-hash] The canonical hash of `Outbound`.
pub const __PROTO_Outbound: &str = "e754249e848e9986";

pub enum __Cont_Sending {
    SendFrame(crate::net::NodeEndpoint),
}

pub enum __Cont_Receiving {
    ReceiveFrame(crate::net::NodeEndpoint),
}

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

/// [protocol-hash] The canonical hash of `NodeGroup`.
pub const __PROTO_NodeGroup: &str = "471318a2c85d1f6d";

pub enum __Msg_NodeGroupWatcher {
    Joined(crate::net::Node),
    Left(crate::net::Node, String),
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

/// [protocol-hash] The canonical hash of `NodeGroupWatcher`.
pub const __PROTO_NodeGroupWatcher: &str = "db3e0aa5831c2e44";

pub enum __Cont_StaticNodeGroup {
    Members,
    Subscribe,
    Control(crate::net::NodeId),
}

pub enum __Priv_StaticNodeGroup {
    Init,
    Control(crate::net::NodeId, crate::core_bytes::Bytes),
}

pub enum __Cont_GossipNodeGroup {
    Members,
    Subscribe,
    Control(crate::net::NodeId),
}

pub enum __Priv_GossipNodeGroup {
    Init,
    Control(crate::net::NodeId, crate::core_bytes::Bytes),
}

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

/// [protocol-hash] The canonical hash of `ActorGroup`.
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

/// [protocol-hash] The canonical hash of `ActorGroupWatcher`.
pub const __PROTO_ActorGroupWatcher: &str = "9fa424e5858bb3bd";

pub enum __Cont_ActorGrouping {
    Join,
    Leave,
    Members,
    Subscribe,
    Joined,
    Left(crate::net::Node),
    Control(crate::net::NodeId),
}

pub enum __Priv_ActorGrouping {
    Init,
    Control(crate::net::NodeId, crate::core_bytes::Bytes),
}

pub enum __Msg_MemNet {
    Attach(crate::net::NodeEndpoint, usize),
    Detach(crate::net::NodeEndpoint),
    Route(crate::net::NodeEndpoint, crate::net::NodeEndpoint, crate::scheduler::SalvoReply),
    Partition(crate::net::NodeEndpoint, crate::net::NodeEndpoint),
    Heal(crate::net::NodeEndpoint, crate::net::NodeEndpoint),
    Kill(crate::net::NodeEndpoint),
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

/// [protocol-hash] The canonical hash of `MemNet`.
pub const __PROTO_MemNet: &str = "2815c14392023d5e";

pub enum __Cont_MemNetwork {
    Attach(crate::net::NodeEndpoint),
    Detach,
    Route(crate::net::NodeEndpoint, crate::net::NodeEndpoint),
    Partition(crate::net::NodeEndpoint),
    Heal(crate::net::NodeEndpoint),
    Kill,
    Delivered,
}
