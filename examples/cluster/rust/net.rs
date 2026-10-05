use crate::collections::*;
use crate::core_actor::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_seq::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::time::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

pub fn to_str__5(e: &NodeEndpoint) -> String {
    return format!("{}:{}", e.host.clone(), e.port);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unreachable {
    pub to: NodeEndpoint,
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
    pub to: NodeEndpoint,
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

pub type NetError = Union2<Unreachable, WireFailed>;

/// Factories for the host: one per arm of the union [platform-factory].
impl NetError {
    pub fn unreachable(value: Unreachable) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn wire_failed(value: WireFailed) -> Self {
        crate::unions::Union2::U2(value)
    }
}

pub fn to_str__6(e: &Union2<Unreachable, WireFailed>) -> String {
    match e {
        Union2::U1(_) => {
            return format!("unreachable: {}", to_str__5(&e.u1().clone().to));
        }
        Union2::U2(_) => {
            return format!("wire failed to {}: {}", to_str__5(&e.u2().clone().to), e.u2().clone().reason.clone());
        }
    }
}

pub trait __Stateless_Inbound: Send + Sync {
    fn receive_frame(&self, from: NodeEndpoint, frame: Bytes);
}

pub trait __Stateful_Inbound: Send {
    fn receive_frame(&mut self, from: NodeEndpoint, frame: Bytes);
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
    fn receive_frame(&self, from: NodeEndpoint, frame: Bytes) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Inbound::ReceiveFrame(from, frame), crate::net::__PROTO_Inbound);
    }
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
    pub fn receive_frame(&self, from: NodeEndpoint, frame: Bytes) {
        match &self.inner {
            __Inner_Inbound::Shared(h) => h.receive_frame(from, frame),
            __Inner_Inbound::Locked(h) => h.lock().unwrap().receive_frame(from, frame),
        }
    }
}

pub enum __Msg_Inbound {
    ReceiveFrame(NodeEndpoint, Bytes),
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

pub trait __Stateless_Transport: Send + Sync {
    fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn unlisten(&self, at: &NodeEndpoint);
    fn deliver(&self, to: &NodeEndpoint, frame: Bytes) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn local_endpoint(&self) -> NodeEndpoint;
}

pub trait __Stateful_Transport: Send {
    fn listen(&mut self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn unlisten(&mut self, at: &NodeEndpoint);
    fn deliver(&mut self, to: &NodeEndpoint, frame: Bytes) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn local_endpoint(&mut self) -> NodeEndpoint;
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
    pub fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.listen(at, sink),
            __Inner_Transport::Locked(h) => h.lock().unwrap().listen(at, sink),
        }
    }
    pub fn unlisten(&self, at: &NodeEndpoint) {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.unlisten(at),
            __Inner_Transport::Locked(h) => h.lock().unwrap().unlisten(at),
        }
    }
    pub fn deliver(&self, to: &NodeEndpoint, frame: Bytes) -> Union2<(), Union2<Unreachable, WireFailed>> {
        match &self.inner {
            __Inner_Transport::Shared(h) => h.deliver(to, frame),
            __Inner_Transport::Locked(h) => h.lock().unwrap().deliver(to, frame),
        }
    }
    pub fn local_endpoint(&self) -> NodeEndpoint {
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
    fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn unlisten(&self, at: &NodeEndpoint);
    fn deliver(&self, to: &NodeEndpoint, frame: Bytes) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn local_endpoint(&self) -> NodeEndpoint;
}

impl<T: TransportPlatformSync> __Stateless_Transport for __Platform_Transport<T> {
    fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        self.0.listen(at, sink)
    }
    fn unlisten(&self, at: &NodeEndpoint) {
        self.0.unlisten(at)
    }
    fn deliver(&self, to: &NodeEndpoint, frame: Bytes) -> Union2<(), Union2<Unreachable, WireFailed>> {
        self.0.deliver(to, frame)
    }
    fn local_endpoint(&self) -> NodeEndpoint {
        self.0.local_endpoint()
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct Listen;

impl Listen {
    pub fn ok(value: ()) -> Union2<(), Union2<Unreachable, WireFailed>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: Union2<Unreachable, WireFailed>) -> Union2<(), Union2<Unreachable, WireFailed>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct Deliver;

impl Deliver {
    pub fn ok(value: ()) -> Union2<(), Union2<Unreachable, WireFailed>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: Union2<Unreachable, WireFailed>) -> Union2<(), Union2<Unreachable, WireFailed>> {
        crate::unions::Union2::U2(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

pub trait __Stateless_Outbound: Send + Sync {
    fn send_frame(&self, to: NodeEndpoint, frame: Bytes);
}

pub trait __Stateful_Outbound: Send {
    fn send_frame(&mut self, to: NodeEndpoint, frame: Bytes);
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
    fn send_frame(&self, to: NodeEndpoint, frame: Bytes) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Outbound::SendFrame(to, frame), crate::net::__PROTO_Outbound);
    }
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
    pub fn send_frame(&self, to: NodeEndpoint, frame: Bytes) {
        match &self.inner {
            __Inner_Outbound::Shared(h) => h.send_frame(to, frame),
            __Inner_Outbound::Locked(h) => h.lock().unwrap().send_frame(to, frame),
        }
    }
}

pub enum __Msg_Outbound {
    SendFrame(NodeEndpoint, Bytes),
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

pub struct Sending {
    __dep_Transport: crate::net::Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sending>,
}

impl Sending {
    pub fn new(__dep_Transport: crate::net::Transport) -> Self {
        Self {
            __dep_Transport,
            __mailbox_capacity: 256,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::net::__Stateless_Outbound for Sending {

    fn send_frame(&self, to: NodeEndpoint, frame: Bytes) {
        let mut _sent = self.__dep_Transport.deliver(&to, frame);
    }
}

pub enum __Cont_Sending {
    SendFrame(NodeEndpoint),
}

pub struct __Actor_Sending {
    handler: Sending,
}

impl __Actor_Sending {
    pub fn new(handler: Sending) -> Self {
        Self { handler }
    }
}

impl __Actor_Sending {
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
            __Cont_Sending::SendFrame(to) => self.__dispatch(crate::net::__Msg_Outbound::SendFrame(to, *value.downcast::<Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sending::SendFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sending: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sending);
fn __decode_msg_Sending(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_Outbound {
            return crate::wire::salvo_decode::<crate::net::__Msg_Outbound>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
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
            __mailbox_capacity: 256,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::net::__Stateless_Inbound for Receiving {

    fn receive_frame(&self, from: NodeEndpoint, frame: Bytes) {
        let mut _delivered = crate::scheduler::salvo_deliver_frame(&frame);
    }
}

pub enum __Cont_Receiving {
    ReceiveFrame(NodeEndpoint),
}

pub struct __Actor_Receiving {
    handler: Receiving,
}

impl __Actor_Receiving {
    pub fn new(handler: Receiving) -> Self {
        Self { handler }
    }
}

impl __Actor_Receiving {
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
            __Cont_Receiving::ReceiveFrame(from) => self.__dispatch(crate::net::__Msg_Inbound::ReceiveFrame(from, *value.downcast::<Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Receiving::ReceiveFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Receiving: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Receiving);
fn __decode_msg_Receiving(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_Inbound {
            return crate::wire::salvo_decode::<crate::net::__Msg_Inbound>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn connect(transport: &crate::net::Transport, me: NodeEndpoint) -> bool {
    return connect__2(transport, me, crate::scheduler::salvo_pool(((1) as usize)));
}

pub fn connect__2(transport: &crate::net::Transport, me: NodeEndpoint, on: usize) -> bool {
    if crate::scheduler::salvo_connected() {
        return false;
    }
    let mut sending = ({ let __h = Sending::new(transport.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, std::boxed::Box::new(__Actor_Sending::new(__h)), __DECODE_Sending); __a });
    let mut receiving = ({ let __h = Receiving::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, std::boxed::Box::new(__Actor_Receiving::new(__h)), __DECODE_Receiving); __a });
    { let __out = sending; crate::scheduler::salvo_set_wire(std::sync::Arc::new(move |__ep: &[u8], __frame: Vec<u8>| { if let Some(__to) = crate::wire::salvo_decode::<NodeEndpoint>(__ep) { crate::scheduler::salvo_send_wire(__out, crate::net::__Msg_Outbound::SendFrame(__to, __frame), crate::net::__PROTO_Outbound); } })) };
    let mut _listening = transport.listen(&(me.clone()), receiving);
    crate::scheduler::salvo_add_route((NodeId { id: crate::scheduler::salvo_here_node() as i64 }).id as u64, crate::wire::salvo_encode(&me));
    return true;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Node {
    pub id: NodeId,
    pub at: NodeEndpoint,
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

pub trait __Stateless_NodeGroupWatcher: Send + Sync {
    fn joined(&self, n: Node);
    fn left(&self, n: Node, why: String);
}

pub trait __Stateful_NodeGroupWatcher: Send {
    fn joined(&mut self, n: Node);
    fn left(&mut self, n: Node, why: String);
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
    fn joined(&self, n: Node) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroupWatcher::Joined(n), crate::net::__PROTO_NodeGroupWatcher);
    }
    fn left(&self, n: Node, why: String) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroupWatcher::Left(n, why), crate::net::__PROTO_NodeGroupWatcher);
    }
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
    pub fn joined(&self, n: Node) {
        match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => h.joined(n),
            __Inner_NodeGroupWatcher::Locked(h) => h.lock().unwrap().joined(n),
        }
    }
    pub fn left(&self, n: Node, why: String) {
        match &self.inner {
            __Inner_NodeGroupWatcher::Shared(h) => h.left(n, why),
            __Inner_NodeGroupWatcher::Locked(h) => h.lock().unwrap().left(n, why),
        }
    }
}

pub enum __Msg_NodeGroupWatcher {
    Joined(Node),
    Left(Node, String),
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

#[derive(Clone, Debug, PartialEq)]
pub struct Hello {
    pub group: String,
    pub at: NodeEndpoint,
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
    pub at: NodeEndpoint,
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
    pub peers: Vec<NodeEndpoint>,
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

pub fn hello_frame(transport: &crate::net::Transport, group: &String) -> Bytes {
    let mut hello: Union4<Hello, Ack, Leaving, Intro> = Union4::<Hello, Ack, Leaving, Intro>::U1(Hello { group: group.clone(), at: transport.local_endpoint(), protocols: crate::scheduler::salvo_local_protocols() });
    return crate::scheduler::salvo_control_frame(&"".to_string(), &crate::wire::salvo_encode(&hello));
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeerHello {
    pub node: NodeId,
    pub at: NodeEndpoint,
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
    pub node: NodeId,
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
    pub peers: Vec<NodeEndpoint>,
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

pub fn handshake(transport: &crate::net::Transport, group: &String, from: NodeId, data: Bytes) -> Option<Union3<PeerHello, PeerGone, PeerIntro>> {
    let mut msg = crate::wire::salvo_decode::<Union4<Hello, Ack, Leaving, Intro>>(&data);
    match msg {
        Some(Union4::U1(_)) => {
            if !(&msg.as_ref().unwrap().u1().group[..] == &group[..]) {
                return None;
            }
            crate::scheduler::salvo_add_route((from.clone()).id as u64, crate::wire::salvo_encode(&msg.as_ref().unwrap().u1().clone().at.clone()));
            crate::scheduler::salvo_set_peer_protocols((from.clone()).id as u64, (msg.as_ref().unwrap().u1().clone().protocols.clone()).clone());
            let mut ack: Union4<Hello, Ack, Leaving, Intro> = Union4::<Hello, Ack, Leaving, Intro>::U2(Ack { group: group.clone(), at: transport.local_endpoint(), protocols: crate::scheduler::salvo_local_protocols() });
            crate::scheduler::salvo_send_control((from.clone()).id as u64, &"".to_string(), &crate::wire::salvo_encode(&ack));
            return Some(Union3::<PeerHello, PeerGone, PeerIntro>::U1(PeerHello { node: from, at: msg.as_ref().unwrap().u1().clone().at.clone() }));
        }
        Some(Union4::U2(_)) => {
            if !(&msg.as_ref().unwrap().u2().group[..] == &group[..]) {
                return None;
            }
            crate::scheduler::salvo_add_route((from.clone()).id as u64, crate::wire::salvo_encode(&msg.as_ref().unwrap().u2().clone().at.clone()));
            crate::scheduler::salvo_set_peer_protocols((from.clone()).id as u64, (msg.as_ref().unwrap().u2().clone().protocols.clone()).clone());
            return Some(Union3::<PeerHello, PeerGone, PeerIntro>::U1(PeerHello { node: from, at: msg.as_ref().unwrap().u2().clone().at.clone() }));
        }
        Some(Union4::U3(_)) => {
            crate::scheduler::salvo_node_left((from.clone()).id as u64);
            return Some(Union3::<PeerHello, PeerGone, PeerIntro>::U2(PeerGone { node: from }));
        }
        Some(Union4::U4(_)) => {
            return Some(Union3::<PeerHello, PeerGone, PeerIntro>::U3(PeerIntro { peers: msg.as_ref().unwrap().u4().clone().peers.clone() }));
        }
        None => {
            return None;
        }
    }
}

pub fn introduce(node: &NodeId, peers: &Vec<NodeEndpoint>) {
    let mut intro: Union4<Hello, Ack, Leaving, Intro> = Union4::<Hello, Ack, Leaving, Intro>::U4(Intro { peers: peers.clone() });
    crate::scheduler::salvo_send_control((node.clone()).id as u64, &"".to_string(), &crate::wire::salvo_encode(&intro));
}

pub fn leave_group(known: &SalvoMap<NodeId, Node>) {
    for mut id in crate::platform_core_list::each(&(known.keys().cloned().collect::<Vec<_>>())).map(|__x| __x.clone()) {
        let mut leaving: Union4<Hello, Ack, Leaving, Intro> = Union4::<Hello, Ack, Leaving, Intro>::U3(Leaving {  });
        crate::scheduler::salvo_send_control((id.clone()).id as u64, &"".to_string(), &crate::wire::salvo_encode(&leaving));
    }
}

pub struct StaticNodeGroup {
    name: String,
    all: Vec<NodeEndpoint>,
    known: SalvoMap<NodeId, Node>,
    watchers: Vec<usize>,
    __dep_Transport: crate::net::Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_StaticNodeGroup>,
}

impl StaticNodeGroup {
    pub fn new(name: String, all: Vec<NodeEndpoint>, __dep_Transport: crate::net::Transport) -> Self {
        Self {
            name,
            all,
            known: SalvoMap::from_entries::<__Hash_hash__NodeId_NodeId, __Eq_eq__NodeId_NodeId, _>(vec![]),
            watchers: vec![],
            __dep_Transport,
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::net::__Stateful_NodeGroup for StaticNodeGroup {

    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<Node>>(out, known_nodes(&self.known));
    }

    fn subscribe(&mut self, w: usize) {
        for mut id in crate::platform_core_list::each(&(self.known.keys().cloned().collect::<Vec<_>>())).map(|__x| __x.clone()) {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Joined(n.unwrap().clone()), crate::net::__PROTO_NodeGroupWatcher);
            }
        }
        crate::core_list::add_platform(&mut self.watchers, w);
    }

    fn leave(&mut self) {
        leave_group(&self.known);
    }
}

impl StaticNodeGroup {

    fn init(&mut self) {
        let mut me = self.__dep_Transport.local_endpoint();
        let mut _connected = connect(&self.__dep_Transport, me.clone());
        crate::scheduler::salvo_watch_control(("".to_string()).clone(), (self.__addr.expect("a handler naming its own address runs as an actor")).clone(), |__n, __d| std::boxed::Box::new(__Priv_StaticNodeGroup::Control(NodeId { id: __n as i64 }, __d)));
        for e in crate::platform_core_list::each(&self.all) {
            if !eq__2(e, &me) {
                let mut _sent = { let __a1 = hello_frame(&self.__dep_Transport, &self.name); self.__dep_Transport.deliver(&(e.clone()), __a1) };
            }
        }
    }

    fn control(&mut self, from: NodeId, data: Bytes) {
        let mut event = handshake(&self.__dep_Transport, &self.name, from, data);
        match event {
            Some(Union3::U1(_)) => {
                if self.known.contains_key(&event.as_ref().unwrap().u1().node) {
                    return;
                }
                let mut n = Node { id: event.as_ref().unwrap().u1().clone().node.clone(), at: event.as_ref().unwrap().u1().clone().at.clone() };
                self.known.insert(event.as_ref().unwrap().u1().clone().node.clone(), n.clone());
                for w in crate::platform_core_list::each(&self.watchers) {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeGroupWatcher::Joined(n.clone()), crate::net::__PROTO_NodeGroupWatcher);
                }
            }
            Some(Union3::U2(_)) => {
                let mut n = self.known.remove(&event.as_ref().unwrap().u2().node);
                if n.is_none() {
                    return;
                }
                for w in crate::platform_core_list::each(&self.watchers) {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeGroupWatcher::Left(n.as_ref().unwrap().clone(), "left".to_string()), crate::net::__PROTO_NodeGroupWatcher);
                }
            }
            Some(Union3::U3(_)) => {
            }
            None => {
            }
        }
    }
}

pub enum __Cont_StaticNodeGroup {
    Members,
    Subscribe,
    Control(NodeId),
}

pub enum __Priv_StaticNodeGroup {
    Init,
    Control(NodeId, Bytes),
}

pub struct __Actor_StaticNodeGroup {
    handler: StaticNodeGroup,
}

impl __Actor_StaticNodeGroup {
    pub fn new(handler: StaticNodeGroup) -> Self {
        Self { handler }
    }
}

impl __Actor_StaticNodeGroup {
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
            __Cont_StaticNodeGroup::Control(from) => self.__dispatch_priv(__Priv_StaticNodeGroup::Control(from, *value.downcast::<Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_StaticNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_StaticNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_StaticNodeGroup);
fn __decode_msg_StaticNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_NodeGroup {
            return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroup>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn known_nodes(known: &SalvoMap<NodeId, Node>) -> Vec<Node> {
    let mut all_known: Vec<Node> = vec![];
    for mut id in crate::platform_core_list::each(&(known.keys().cloned().collect::<Vec<_>>())).map(|__x| __x.clone()) {
        let mut n = known.get(&id);
        if !(n.is_none()) {
            crate::core_list::add_platform(&mut all_known, n.unwrap().clone());
        }
    }
    return all_known;
}

pub struct GossipNodeGroup {
    name: String,
    seeds: Vec<NodeEndpoint>,
    known: SalvoMap<NodeId, Node>,
    dialed: SalvoSet<String>,
    watchers: Vec<usize>,
    __dep_Transport: crate::net::Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_GossipNodeGroup>,
}

impl GossipNodeGroup {
    pub fn new(name: String, seeds: Vec<NodeEndpoint>, __dep_Transport: crate::net::Transport) -> Self {
        Self {
            name,
            seeds,
            known: SalvoMap::from_entries::<__Hash_hash__NodeId_NodeId, __Eq_eq__NodeId_NodeId, _>(vec![]),
            dialed: SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]),
            watchers: vec![],
            __dep_Transport,
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::net::__Stateful_NodeGroup for GossipNodeGroup {

    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<Node>>(out, known_nodes(&self.known));
    }

    fn subscribe(&mut self, w: usize) {
        for mut id in crate::platform_core_list::each(&(self.known.keys().cloned().collect::<Vec<_>>())).map(|__x| __x.clone()) {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                crate::scheduler::salvo_send_wire(w, crate::net::__Msg_NodeGroupWatcher::Joined(n.unwrap().clone()), crate::net::__PROTO_NodeGroupWatcher);
            }
        }
        crate::core_list::add_platform(&mut self.watchers, w);
    }

    fn leave(&mut self) {
        leave_group(&self.known);
    }
}

impl GossipNodeGroup {

    fn init(&mut self) {
        let mut me = self.__dep_Transport.local_endpoint();
        let mut _connected = connect(&self.__dep_Transport, me.clone());
        crate::scheduler::salvo_watch_control(("".to_string()).clone(), (self.__addr.expect("a handler naming its own address runs as an actor")).clone(), |__n, __d| std::boxed::Box::new(__Priv_GossipNodeGroup::Control(NodeId { id: __n as i64 }, __d)));
        for e in crate::platform_core_list::each(&self.seeds) {
            dial(&self.__dep_Transport, &mut self.dialed, &self.name, e.clone());
        }
    }

    fn control(&mut self, from: NodeId, data: Bytes) {
        let mut event = handshake(&self.__dep_Transport, &self.name, from, data);
        match event {
            Some(Union3::U1(_)) => {
                if self.known.contains_key(&event.as_ref().unwrap().u1().node) {
                    return;
                }
                let mut others: Vec<NodeEndpoint> = vec![];
                for mut id in crate::platform_core_list::each(&(self.known.keys().cloned().collect::<Vec<_>>())).map(|__x| __x.clone()) {
                    let mut n = self.known.get(&id);
                    if !(n.is_none()) {
                        crate::core_list::add_platform(&mut others, n.unwrap().clone().at.clone());
                    }
                    introduce(&(id.clone()), &(vec![event.as_ref().unwrap().u1().clone().at.clone()]));
                }
                introduce(&(event.as_ref().unwrap().u1().clone().node.clone()), &others);
                self.dialed.insert(to_str__5(&event.as_ref().unwrap().u1().clone().at));
                let mut n = Node { id: event.as_ref().unwrap().u1().clone().node.clone(), at: event.as_ref().unwrap().u1().clone().at.clone() };
                self.known.insert(event.as_ref().unwrap().u1().clone().node.clone(), n.clone());
                for w in crate::platform_core_list::each(&self.watchers) {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeGroupWatcher::Joined(n.clone()), crate::net::__PROTO_NodeGroupWatcher);
                }
            }
            Some(Union3::U2(_)) => {
                let mut n = self.known.remove(&event.as_ref().unwrap().u2().node);
                if n.is_none() {
                    return;
                }
                for w in crate::platform_core_list::each(&self.watchers) {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeGroupWatcher::Left(n.as_ref().unwrap().clone(), "left".to_string()), crate::net::__PROTO_NodeGroupWatcher);
                }
            }
            Some(Union3::U3(_)) => {
                for mut e in crate::platform_core_list::each(&(event.as_ref().unwrap().u3().clone().peers.clone())).map(|__x| __x.clone()) {
                    dial(&self.__dep_Transport, &mut self.dialed, &self.name, e.clone());
                }
            }
            None => {
            }
        }
    }
}

pub enum __Cont_GossipNodeGroup {
    Members,
    Subscribe,
    Control(NodeId),
}

pub enum __Priv_GossipNodeGroup {
    Init,
    Control(NodeId, Bytes),
}

pub struct __Actor_GossipNodeGroup {
    handler: GossipNodeGroup,
}

impl __Actor_GossipNodeGroup {
    pub fn new(handler: GossipNodeGroup) -> Self {
        Self { handler }
    }
}

impl __Actor_GossipNodeGroup {
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
            __Cont_GossipNodeGroup::Control(from) => self.__dispatch_priv(__Priv_GossipNodeGroup::Control(from, *value.downcast::<Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_GossipNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_GossipNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_GossipNodeGroup);
fn __decode_msg_GossipNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_NodeGroup {
            return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroup>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn dial(transport: &crate::net::Transport, dialed: &mut SalvoSet<String>, group: &String, e: NodeEndpoint) {
    if eq__2(&e, &(transport.local_endpoint())) || dialed.contains(&to_str__5(&e)) {
        return;
    }
    dialed.insert(to_str__5(&e));
    let mut sent = { let __a1 = hello_frame(transport, group); transport.deliver(&(e.clone()), __a1) };
    if matches!(sent, Union2::U2(_)) {
        let mut _forgot = dialed.remove(&to_str__5(&e));
    }
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

pub trait __Stateless_ActorGroupWatcher: Send + Sync {
    fn joined(&self, member: usize);
    fn left(&self, member: usize);
}

pub trait __Stateful_ActorGroupWatcher: Send {
    fn joined(&mut self, member: usize);
    fn left(&mut self, member: usize);
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

pub fn share_members(name: &String, hash: &String, node: &NodeId, members: &Vec<usize>) {
    crate::scheduler::salvo_send_control((node.clone()).id as u64, &name.clone(), &crate::wire::salvo_encode(&(hash.clone(), members.clone())));
}

pub fn actor_group(nodes: usize, protocol: &mut dyn FnMut() -> Protocol) -> usize {
    let mut proto = protocol();
    let mut name = proto.name.clone();
    return actor_group__2(name, nodes, &mut *protocol);
}

pub fn actor_group__2(name: String, nodes: usize, protocol: &mut dyn FnMut() -> Protocol) -> usize {
    let (mut group, mut watcher) = ({ let __h = ActorGrouping::new(name, protocol()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, std::boxed::Box::new(__Actor_ActorGrouping::new(__h)), __DECODE_ActorGrouping); crate::scheduler::salvo_send(__a, std::boxed::Box::new(__Priv_ActorGrouping::Init)); (__a, __a) });
    crate::scheduler::salvo_send_wire(nodes, crate::net::__Msg_NodeGroup::Subscribe(watcher), crate::net::__PROTO_NodeGroup);
    return group;
}

pub fn join__2(group: &usize, member: usize) {
    crate::scheduler::salvo_send_wire(group.clone(), crate::net::__Msg_ActorGroup::Join(member), crate::net::__PROTO_ActorGroup);
}

pub struct ActorGrouping {
    name: String,
    proto: Protocol,
    all: Vec<usize>,
    peers: Vec<NodeId>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_ActorGrouping>,
}

impl ActorGrouping {
    pub fn new(name: String, proto: Protocol) -> Self {
        Self {
            name,
            proto,
            all: vec![],
            peers: vec![],
            watchers: vec![],
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::net::__Stateful_ActorGroup for ActorGrouping {

    fn join(&mut self, member: usize) {
        if !admit(&mut self.all, member.clone()) {
            return;
        }
        mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        for w in crate::platform_core_list::each(&self.watchers) {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorGroupWatcher::Joined(member.clone()), crate::net::__PROTO_ActorGroupWatcher);
        }
        for p in crate::platform_core_list::each(&self.peers) {
            share_members(&self.name, &self.proto.hash, &(p.clone()), &(vec![member.clone()]));
        }
    }

    fn leave(&mut self, member: usize) {
        if !withdraw(&mut self.all, member.clone()) {
            return;
        }
        mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        for w in crate::platform_core_list::each(&self.watchers) {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorGroupWatcher::Left(member.clone()), crate::net::__PROTO_ActorGroupWatcher);
        }
    }

    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<usize>>(out, self.all.clone());
    }

    fn refresh(&mut self) {
        crate::runtime_routing::view_refresh((self.__addr.expect("a handler naming its own address runs as an actor")).clone() as i32);
    }

    fn subscribe(&mut self, w: usize) {
        crate::core_list::add_platform(&mut self.watchers, w);
    }
}

impl crate::net::__Stateful_NodeGroupWatcher for ActorGrouping {

    fn joined(&mut self, n: Node) {
        share_members(&self.name, &self.proto.hash, &n.id, &self.all);
    }

    fn left(&mut self, n: Node, why: String) {
        let mut gone: Vec<usize> = vec![];
        for m in crate::platform_core_list::each(&self.all) {
            if eq__3(&(NodeId { id: crate::scheduler::salvo_addr_identity((m.clone()).clone()).node as i64 }), &n.id) {
                crate::core_list::add_platform(&mut gone, m.clone());
            }
        }
        for m in crate::platform_core_list::each(&gone) {
            if withdraw(&mut self.all, m.clone()) {
                for w in crate::platform_core_list::each(&self.watchers) {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorGroupWatcher::Left(m.clone()), crate::net::__PROTO_ActorGroupWatcher);
                }
            }
        }
        if crate::core_list::size_platform(&gone) > 0 {
            mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        }
    }
}

impl ActorGrouping {

    fn init(&mut self) {
        crate::scheduler::salvo_watch_control((self.name.clone()).clone(), (self.__addr.expect("a handler naming its own address runs as an actor")).clone(), |__n, __d| std::boxed::Box::new(__Priv_ActorGrouping::Control(NodeId { id: __n as i64 }, __d)));
    }

    fn control(&mut self, from: NodeId, data: Bytes) {
        let mut got = crate::wire::salvo_decode::<(String, Vec<usize>)>(&data);
        if got.is_none() {
            return;
        }
        if !(&got.as_ref().unwrap().0[..] == &self.proto.hash[..]) {
            return;
        }
        if !contains_node(&self.peers, &(from.clone())) {
            crate::core_list::add_platform(&mut self.peers, from.clone());
            share_members(&self.name, &self.proto.hash, &(from.clone()), &self.all);
        }
        let mut changed = false;
        for mut m in crate::platform_core_list::each(&(got.as_ref().unwrap().clone().1.clone())).map(|__x| __x.clone()) {
            if admit(&mut self.all, m.clone()) {
                changed = true;
                for w in crate::platform_core_list::each(&self.watchers) {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorGroupWatcher::Joined(m.clone()), crate::net::__PROTO_ActorGroupWatcher);
                }
            }
        }
        if changed {
            mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        }
    }
}

pub enum __Cont_ActorGrouping {
    Join,
    Leave,
    Joined,
    Left(Node),
    Members,
    Subscribe,
    Control(NodeId),
}

pub enum __Priv_ActorGrouping {
    Init,
    Control(NodeId, Bytes),
}

pub struct __Actor_ActorGrouping {
    handler: ActorGrouping,
}

impl __Actor_ActorGrouping {
    pub fn new(handler: ActorGrouping) -> Self {
        Self { handler }
    }
}

impl __Actor_ActorGrouping {
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
            __Cont_ActorGrouping::Joined => self.__dispatch_NodeGroupWatcher(crate::net::__Msg_NodeGroupWatcher::Joined(*value.downcast::<Node>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Left(n) => self.__dispatch_NodeGroupWatcher(crate::net::__Msg_NodeGroupWatcher::Left(n, *value.downcast::<String>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Control(from) => self.__dispatch_priv(__Priv_ActorGrouping::Control(from, *value.downcast::<Bytes>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ActorGrouping::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Leave{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Joined{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Node>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Left{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Control{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Bytes>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_ActorGrouping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_ActorGrouping);
fn __decode_msg_ActorGrouping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_ActorGroup {
            return crate::wire::salvo_decode::<crate::net::__Msg_ActorGroup>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
        if proto == crate::net::__PROTO_NodeGroupWatcher {
            return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroupWatcher>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn mirror(group: &usize, members: &Vec<usize>) {
    crate::scheduler::salvo_view_set((group.clone()).clone(), &members.clone());
}

pub fn admit(list: &mut Vec<usize>, a: usize) -> bool {
    for x in crate::platform_core_list::each(&*list) {
        if ((crate::scheduler::salvo_addr_identity((x).clone()) == crate::scheduler::salvo_addr_identity((a).clone()))) {
            return false;
        }
    }
    crate::core_list::add_platform(list, a);
    return true;
}

pub fn contains_node(list: &Vec<NodeId>, n: &NodeId) -> bool {
    for x in crate::platform_core_list::each(list) {
        if eq__3(x, n) {
            return true;
        }
    }
    return false;
}

pub fn withdraw(list: &mut Vec<usize>, a: usize) -> bool {
    let mut mut_index: Option<i32> = None;
    let mut i = 0;
    for x in crate::platform_core_list::each(&*list) {
        if ((crate::scheduler::salvo_addr_identity((x).clone()) == crate::scheduler::salvo_addr_identity((a).clone()))) {
            mut_index = Some(i);
        }
        i = i + 1;
    }
    if mut_index.is_none() {
        return false;
    }
    let mut _removed = crate::core_list::remove_at_platform(list, mut_index.unwrap());
    return true;
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
    pub members: Vec<RouteMember>,
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

pub fn default_route_config() -> RouteConfig {
    return RouteConfig { first_wait: Duration { nanos: 1000000i64 }, max_wait: Duration { nanos: 5000000000i64 } };
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteConfig {
    pub first_wait: Duration,
    pub max_wait: Duration,
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
    fn changed(&self, view: &RouteView);
    fn select(&self, view: &RouteView, key: &Option<i64>) -> Option<usize>;
}

pub trait __Stateful_RouteSelector: Send {
    fn changed(&mut self, view: &RouteView);
    fn select(&mut self, view: &RouteView, key: &Option<i64>) -> Option<usize>;
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
    pub fn changed(&self, view: &RouteView) {
        match &self.inner {
            __Inner_RouteSelector::Shared(h) => h.changed(view),
            __Inner_RouteSelector::Locked(h) => h.lock().unwrap().changed(view),
        }
    }
    pub fn select(&self, view: &RouteView, key: &Option<i64>) -> Option<usize> {
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

pub fn route_pick(route_selector: &crate::net::RouteSelector, group: &usize, config: &RouteConfig, seen: i64) -> RoutePick {
    return route_keyed(route_selector, group, config, seen, None);
}

pub fn route_pick__2(route_selector: &crate::net::RouteSelector, group: &usize, config: &RouteConfig, seen: i64, key: i64) -> RoutePick {
    return route_keyed(route_selector, group, config, seen, Some(key));
}

pub fn route_keyed(route_selector: &crate::net::RouteSelector, group: &usize, config: &RouteConfig, seen: i64, key: Option<i64>) -> RoutePick {
    let mut wait = config.first_wait.nanos;
    let mut cap = config.max_wait.nanos;
    let mut last = seen.clone();
    loop {
        let mut version = crate::runtime_routing::view_version((group.clone()).clone() as i32);
        let mut view = route_view(&(group.clone()));
        if version != last {
            route_selector.changed(&(view.clone()));
            last = version.clone();
        }
        let mut picked = route_selector.select(&view, &(key.clone()));
        if !(picked.is_none()) {
            return RoutePick { to: picked.as_ref().unwrap().clone(), version: last.clone() };
        }
        crate::runtime_routing::view_wait((group.clone()).clone() as i32, version.clone(), wait.clone());
        wait = wait * ((2) as i64);
        if wait > cap {
            wait = cap.clone();
        }
    }
    return route_keyed(route_selector, group, config, seen, key);
}

pub fn route_view(group: &usize) -> RouteView {
    let mut members: Vec<RouteMember> = vec![];
    for mut m in crate::platform_core_list::each(&(crate::scheduler::salvo_view_members((group.clone()).clone()))).map(|__x| __x.clone()) {
        crate::core_list::add_platform(&mut members, RouteMember { addr: m.clone(), local: eq__3(&(NodeId { id: crate::scheduler::salvo_addr_identity((m).clone()).node as i64 }), &(NodeId { id: crate::scheduler::salvo_here_node() as i64 })) });
    }
    return RouteView { members: members.clone() };
}

#[derive(Clone)]
pub struct LeastLoaded {
    prefer_local: bool,
}

impl LeastLoaded {
    pub fn new(prefer_local: bool) -> Self {
        Self {
            prefer_local,
        }
    }
}

impl crate::net::__Stateless_RouteSelector for LeastLoaded {

    fn changed(&self, view: &RouteView) {
    }

    fn select(&self, view: &RouteView, key: &Option<i64>) -> Option<usize> {
        let mut best: Option<RouteMember> = None;
        let mut best_pending = 0;
        for a in crate::platform_core_list::each(&view.members) {
            let mut load = crate::scheduler::salvo_pending((a.addr.clone()).clone());
            if best.is_none() {
                best = Some(a.clone());
                best_pending = load;
            } else {
                let mut b: RouteMember = best.as_ref().unwrap().clone();
                let mut take = if self.prefer_local && a.local && !b.local {
                    true
                } else if self.prefer_local && !a.local && b.local {
                    false
                } else {
                    load < best_pending
                };
                if take {
                    best = Some(a.clone());
                    best_pending = load;
                }
            }
        }
        let mut chosen: RouteMember = if best.is_some() { best.as_ref().unwrap().clone() } else { return None };
        return Some(chosen.addr.clone());
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

    fn changed(&self, view: &RouteView) {
    }

    fn select(&self, view: &RouteView, key: &Option<i64>) -> Option<usize> {
        let mut n = crate::core_list::size_platform(&view.members);
        if n == 0 {
            return None;
        }
        let mut k = if key.is_some() { key.unwrap() } else { 0i64 };
        let mut magnitude = if k < 0i64 {
            0i64 - k
        } else {
            k
        };
        let mut slot = ((magnitude % ((n) as i64)) as i32);
        let mut picked = { let __pick1 = crate::core_list::get_platform(&view.members, slot); if __pick1.is_some() { __pick1.unwrap() } else { return None } };
        return Some(picked.addr.clone());
    }
}

pub trait __Stateless_Leader: Send + Sync {
    fn leader(&self) -> Option<NodeId>;
}

pub trait __Stateful_Leader: Send {
    fn leader(&mut self) -> Option<NodeId>;
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
    pub fn leader(&self) -> Option<NodeId> {
        match &self.inner {
            __Inner_Leader::Shared(h) => h.leader(),
            __Inner_Leader::Locked(h) => h.lock().unwrap().leader(),
        }
    }
}

#[derive(Clone)]
pub struct StaticLeader {
    node: NodeId,
}

impl StaticLeader {
    pub fn new(node: NodeId) -> Self {
        Self {
            node,
        }
    }
}

impl crate::net::__Stateless_Leader for StaticLeader {

    fn leader(&self) -> Option<NodeId> {
        return Some(self.node.clone());
    }
}

pub struct Elected {
    chosen: Option<usize>,
    __dep_Leader: crate::net::Leader,
}

impl Elected {
    pub fn new(__dep_Leader: crate::net::Leader) -> Self {
        Self {
            chosen: None,
            __dep_Leader,
        }
    }
}

impl crate::net::__Stateful_RouteSelector for Elected {

    fn changed(&mut self, view: &RouteView) {
        self.chosen = None;
        let mut l = { let __pick2 = self.__dep_Leader.leader(); if __pick2.is_some() { __pick2.as_ref().unwrap().clone() } else { return } };
        for a in crate::platform_core_list::each(&view.members) {
            if eq__3(&(NodeId { id: crate::scheduler::salvo_addr_identity((a.addr.clone()).clone()).node as i64 }), &l) {
                self.chosen = Some(a.addr.clone());
                return;
            }
        }
    }

    fn select(&mut self, view: &RouteView, key: &Option<i64>) -> Option<usize> {
        return self.chosen.clone();
    }
}

pub type __Platform_HostTcpTransport = crate::net::__Platform_Transport<crate::platform_net::HostTcpTransport>;

impl __Platform_HostTcpTransport {
    pub fn new(bind: NodeEndpoint) -> Self {
        crate::net::__Platform_Transport(crate::platform_net::HostTcpTransport::new(bind))
    }
}

pub trait __Stateless_MemNet: Send + Sync {
    fn attach(&self, at: NodeEndpoint, sink: usize);
    fn detach(&self, at: NodeEndpoint);
    fn route(&self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply);
    fn partition(&self, a: NodeEndpoint, b: NodeEndpoint);
    fn heal(&self, a: NodeEndpoint, b: NodeEndpoint);
    fn kill(&self, node: NodeEndpoint);
    fn delivered(&self, out: crate::scheduler::SalvoReply);
}

pub trait __Stateful_MemNet: Send {
    fn attach(&mut self, at: NodeEndpoint, sink: usize);
    fn detach(&mut self, at: NodeEndpoint);
    fn route(&mut self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply);
    fn partition(&mut self, a: NodeEndpoint, b: NodeEndpoint);
    fn heal(&mut self, a: NodeEndpoint, b: NodeEndpoint);
    fn kill(&mut self, node: NodeEndpoint);
    fn delivered(&mut self, out: crate::scheduler::SalvoReply);
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
    fn attach(&self, at: NodeEndpoint, sink: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Attach(at, sink), crate::net::__PROTO_MemNet);
    }
    fn detach(&self, at: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Detach(at), crate::net::__PROTO_MemNet);
    }
    fn route(&self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Route(from, to, out), crate::net::__PROTO_MemNet);
    }
    fn partition(&self, a: NodeEndpoint, b: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Partition(a, b), crate::net::__PROTO_MemNet);
    }
    fn heal(&self, a: NodeEndpoint, b: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Heal(a, b), crate::net::__PROTO_MemNet);
    }
    fn kill(&self, node: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Kill(node), crate::net::__PROTO_MemNet);
    }
    fn delivered(&self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Delivered(out), crate::net::__PROTO_MemNet);
    }
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
    pub fn attach(&self, at: NodeEndpoint, sink: usize) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.attach(at, sink),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().attach(at, sink),
        }
    }
    pub fn detach(&self, at: NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.detach(at),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().detach(at),
        }
    }
    pub fn route(&self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.route(from, to, out),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().route(from, to, out),
        }
    }
    pub fn partition(&self, a: NodeEndpoint, b: NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.partition(a, b),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().partition(a, b),
        }
    }
    pub fn heal(&self, a: NodeEndpoint, b: NodeEndpoint) {
        match &self.inner {
            __Inner_MemNet::Shared(h) => h.heal(a, b),
            __Inner_MemNet::Locked(h) => h.lock().unwrap().heal(a, b),
        }
    }
    pub fn kill(&self, node: NodeEndpoint) {
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

pub enum __Msg_MemNet {
    Attach(NodeEndpoint, usize),
    Detach(NodeEndpoint),
    Route(NodeEndpoint, NodeEndpoint, crate::scheduler::SalvoReply),
    Partition(NodeEndpoint, NodeEndpoint),
    Heal(NodeEndpoint, NodeEndpoint),
    Kill(NodeEndpoint),
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

pub struct MemNetwork {
    listeners: SalvoMap<NodeEndpoint, usize>,
    cuts: SalvoSet<String>,
    dead: SalvoSet<NodeEndpoint>,
    count: i32,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_MemNetwork>,
}

impl MemNetwork {
    pub fn new() -> Self {
        Self {
            listeners: SalvoMap::from_entries::<__Hash_hash__NodeEndpoint_NodeEndpoint, __Eq_eq__NodeEndpoint_NodeEndpoint, _>(vec![]),
            cuts: SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]),
            dead: SalvoSet::from_elements::<__Hash_hash__NodeEndpoint_NodeEndpoint, __Eq_eq__NodeEndpoint_NodeEndpoint, _>(vec![]),
            count: 0,
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl crate::net::__Stateful_MemNet for MemNetwork {

    fn attach(&mut self, at: NodeEndpoint, sink: usize) {
        self.dead.remove(&at);
        self.listeners.insert(at, sink);
    }

    fn detach(&mut self, at: NodeEndpoint) {
        self.listeners.remove(&at);
    }

    fn route(&mut self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply) {
        if self.dead.contains(&to) || self.cuts.contains(&cut_key(&from, &to)) {
            crate::scheduler::salvo_reply_wire::<Option<usize>>(out, None);
            return;
        }
        let mut sink = self.listeners.get(&to);
        if sink.is_none() {
            crate::scheduler::salvo_reply_wire::<Option<usize>>(out, None);
            return;
        }
        self.count = self.count + 1;
        crate::scheduler::salvo_reply_wire::<Option<usize>>(out, Some(sink.unwrap().clone()));
    }

    fn partition(&mut self, a: NodeEndpoint, b: NodeEndpoint) {
        self.cuts.insert(cut_key(&a, &b));
        self.cuts.insert(cut_key(&b, &a));
    }

    fn heal(&mut self, a: NodeEndpoint, b: NodeEndpoint) {
        self.cuts.remove(&cut_key(&a, &b));
        self.cuts.remove(&cut_key(&b, &a));
    }

    fn kill(&mut self, node: NodeEndpoint) {
        self.listeners.remove(&node);
        self.dead.insert(node);
    }

    fn delivered(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<i32>(out, self.count);
    }
}

pub enum __Cont_MemNetwork {
    Attach(NodeEndpoint),
    Detach,
    Route(NodeEndpoint, NodeEndpoint),
    Partition(NodeEndpoint),
    Heal(NodeEndpoint),
    Kill,
    Delivered,
}

pub struct __Actor_MemNetwork {
    handler: MemNetwork,
}

impl __Actor_MemNetwork {
    pub fn new(handler: MemNetwork) -> Self {
        Self { handler }
    }
}

impl __Actor_MemNetwork {
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
            __Cont_MemNetwork::Detach => self.__dispatch(crate::net::__Msg_MemNet::Detach(*value.downcast::<NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Route(from, to) => self.__dispatch(crate::net::__Msg_MemNet::Route(from, to, *value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_MemNetwork::Partition(a) => self.__dispatch(crate::net::__Msg_MemNet::Partition(a, *value.downcast::<NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Heal(a) => self.__dispatch(crate::net::__Msg_MemNet::Heal(a, *value.downcast::<NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Kill => self.__dispatch(crate::net::__Msg_MemNet::Kill(*value.downcast::<NodeEndpoint>().expect("the awaited answer"))),
            __Cont_MemNetwork::Delivered => self.__dispatch(crate::net::__Msg_MemNet::Delivered(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_MemNetwork::Attach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Detach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Route{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Partition{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Heal{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Kill{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Delivered{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_MemNetwork: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_MemNetwork);
fn __decode_msg_MemNetwork(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_MemNet {
            return crate::wire::salvo_decode::<crate::net::__Msg_MemNet>(payload)
                .map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

#[derive(Clone)]
pub struct MemTransport {
    me: NodeEndpoint,
    net: usize,
}

impl MemTransport {
    pub fn new(me: NodeEndpoint, net: usize) -> Self {
        Self {
            me,
            net,
        }
    }
}

impl crate::net::__Stateless_Transport for MemTransport {

    fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        crate::scheduler::salvo_send_wire(self.net.clone(), crate::net::__Msg_MemNet::Attach(at.clone(), sink), crate::net::__PROTO_MemNet);
        return Union2::<(), Union2<Unreachable, WireFailed>>::U1(ok(()));
    }

    fn unlisten(&self, at: &NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.net.clone(), crate::net::__Msg_MemNet::Detach(at.clone()), crate::net::__PROTO_MemNet);
    }

    fn deliver(&self, to: &NodeEndpoint, frame: Bytes) -> Union2<(), Union2<Unreachable, WireFailed>> {
        let mut sink = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Option<usize>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.net.clone(), crate::net::__Msg_MemNet::Route(self.me.clone(), to.clone(), out), crate::net::__PROTO_MemNet);
            *crate::scheduler::salvo_wait(__wid).downcast::<Option<usize>>().expect("the awaited answer")
        };
        if sink.is_none() {
            return Union2::<(), Union2<Unreachable, WireFailed>>::U2(Union2::<Unreachable, WireFailed>::U1(err(Unreachable { to: to.clone() })));
        }
        crate::scheduler::salvo_send_wire(sink.as_ref().unwrap().clone(), crate::net::__Msg_Inbound::ReceiveFrame(self.me.clone(), frame), crate::net::__PROTO_Inbound);
        return Union2::<(), Union2<Unreachable, WireFailed>>::U1(ok(()));
    }

    fn local_endpoint(&self) -> NodeEndpoint {
        return self.me.clone();
    }
}

pub fn cut_key(a: &NodeEndpoint, b: &NodeEndpoint) -> String {
    return format!("{}>{}", to_str__5(a), to_str__5(b));
}

pub fn cmp(a: &NodeEndpoint, b: &NodeEndpoint) -> i32 {
    let mut c__c1 = (Ord::cmp(&a.host[..], &b.host[..]) as i32);
    if c__c1 != 0 {
        return c__c1;
    }
    let mut c__c2 = (Ord::cmp(&(a.port), &(b.port)) as i32);
    if c__c2 != 0 {
        return c__c2;
    }
    return 0;
}

pub fn hash(value: &NodeEndpoint) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.host[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.port), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__2(a: &NodeEndpoint, b: &NodeEndpoint) -> bool {
    if !(&a.host[..] == &b.host[..]) {
        return false;
    }
    if !((a.port) == (b.port)) {
        return false;
    }
    return true;
}

pub fn hash__2(value: &NodeId) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(value.id), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__3(a: &NodeId, b: &NodeId) -> bool {
    if !((a.id) == (b.id)) {
        return false;
    }
    return true;
}

pub fn hash__3(value: &Node) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add(hash__2(&value.id)));
    h = ((h).wrapping_mul(31).wrapping_add(hash(&value.at)));
    return h;
}

pub fn eq__4(a: &Node, b: &Node) -> bool {
    if !eq__3(&a.id, &b.id) {
        return false;
    }
    if !eq__2(&a.at, &b.at) {
        return false;
    }
    return true;
}

pub struct __Hash_hash__NodeId_NodeId;
impl SalvoHash<NodeId> for __Hash_hash__NodeId_NodeId {
    fn hash(__v: &NodeId) -> i64 { hash__2(__v) }
}

pub struct __Eq_eq__NodeId_NodeId;
impl SalvoEq<NodeId> for __Eq_eq__NodeId_NodeId {
    fn eq(__a: &NodeId, __b: &NodeId) -> bool { eq__3(__a, __b) }
}

pub struct __Hash_hash__NodeEndpoint_NodeEndpoint;
impl SalvoHash<NodeEndpoint> for __Hash_hash__NodeEndpoint_NodeEndpoint {
    fn hash(__v: &NodeEndpoint) -> i64 { hash(__v) }
}

pub struct __Eq_eq__NodeEndpoint_NodeEndpoint;
impl SalvoEq<NodeEndpoint> for __Eq_eq__NodeEndpoint_NodeEndpoint {
    fn eq(__a: &NodeEndpoint, __b: &NodeEndpoint) -> bool { eq__2(__a, __b) }
}
