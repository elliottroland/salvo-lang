use crate::collections::*;
use crate::core_actor::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::seq::*;
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

pub fn to_str__2(e: &NodeEndpoint) -> String {
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

pub fn to_str__3(e: &Union2<Unreachable, WireFailed>) -> String {
    match e {
        Union2::U1(_) => {
            return format!("unreachable: {}", to_str__2(&e.u1().clone().to));
        }
        Union2::U2(_) => {
            return format!("wire failed to {}: {}", to_str__2(&e.u2().clone().to), e.u2().clone().reason.clone());
        }
    }
}

pub trait Inbound {
    fn receive_frame(&mut self, from: NodeEndpoint, frame: Vec<u8>);
}

pub struct __Stub_Inbound {
    addr: usize,
}

impl __Stub_Inbound {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Inbound for __Stub_Inbound {
    fn receive_frame(&mut self, from: NodeEndpoint, frame: Vec<u8>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Inbound::ReceiveFrame(from, frame), crate::net::__PROTO_Inbound);
    }
}

pub struct __Handle_Inbound {
    inner: std::sync::Arc<std::sync::Mutex<dyn Inbound + Send>>,
}

impl Clone for __Handle_Inbound {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Inbound {
    pub fn new<__H: Inbound + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Inbound + Send>>) -> Self {
        Self { inner }
    }
}

impl Inbound for __Handle_Inbound {
    fn receive_frame(&mut self, from: NodeEndpoint, frame: Vec<u8>) {
        self.inner.lock().unwrap().receive_frame(from, frame)
    }
}

pub enum __Msg_Inbound {
    ReceiveFrame(NodeEndpoint, Vec<u8>),
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

pub trait Transport {
    fn listen(&mut self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn unlisten(&mut self, at: &NodeEndpoint);
    fn deliver(&mut self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn local_endpoint(&mut self) -> NodeEndpoint;
}

pub struct __Handle_Transport {
    inner: std::sync::Arc<std::sync::Mutex<dyn Transport + Send>>,
}

impl Clone for __Handle_Transport {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Transport {
    pub fn new<__H: Transport + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Transport + Send>>) -> Self {
        Self { inner }
    }
}

impl Transport for __Handle_Transport {
    fn listen(&mut self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        self.inner.lock().unwrap().listen(at, sink)
    }
    fn unlisten(&mut self, at: &NodeEndpoint) {
        self.inner.lock().unwrap().unlisten(at)
    }
    fn deliver(&mut self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>> {
        self.inner.lock().unwrap().deliver(to, frame)
    }
    fn local_endpoint(&mut self) -> NodeEndpoint {
        self.inner.lock().unwrap().local_endpoint()
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

pub trait Outbound {
    fn send_frame(&mut self, to: NodeEndpoint, frame: Vec<u8>);
}

pub struct __Stub_Outbound {
    addr: usize,
}

impl __Stub_Outbound {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Outbound for __Stub_Outbound {
    fn send_frame(&mut self, to: NodeEndpoint, frame: Vec<u8>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Outbound::SendFrame(to, frame), crate::net::__PROTO_Outbound);
    }
}

pub struct __Handle_Outbound {
    inner: std::sync::Arc<std::sync::Mutex<dyn Outbound + Send>>,
}

impl Clone for __Handle_Outbound {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Outbound {
    pub fn new<__H: Outbound + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Outbound + Send>>) -> Self {
        Self { inner }
    }
}

impl Outbound for __Handle_Outbound {
    fn send_frame(&mut self, to: NodeEndpoint, frame: Vec<u8>) {
        self.inner.lock().unwrap().send_frame(to, frame)
    }
}

pub enum __Msg_Outbound {
    SendFrame(NodeEndpoint, Vec<u8>),
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
    __dep_Transport: crate::net::__Handle_Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sending>,
}

impl Sending {
    pub fn new(__dep_Transport: crate::net::__Handle_Transport) -> Self {
        Self {
            __dep_Transport,
            __mailbox_capacity: 256,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl Outbound for Sending {

    fn send_frame(&mut self, to: NodeEndpoint, frame: Vec<u8>) {
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
            crate::net::__Msg_Outbound::SendFrame(to, frame) => crate::net::Outbound::send_frame(&mut self.handler, to, frame),
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
            __Cont_Sending::SendFrame(to) => self.__dispatch(crate::net::__Msg_Outbound::SendFrame(to, *value.downcast::<Vec<u8>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sending::SendFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<u8>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Sending: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Sending);
fn __decode_msg_Sending(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_Outbound {
            return crate::wire::salvo_decode::<crate::net::__Msg_Outbound>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
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

impl Inbound for Receiving {

    fn receive_frame(&mut self, from: NodeEndpoint, frame: Vec<u8>) {
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
            crate::net::__Msg_Inbound::ReceiveFrame(from, frame) => crate::net::Inbound::receive_frame(&mut self.handler, from, frame),
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
            __Cont_Receiving::ReceiveFrame(from) => self.__dispatch(crate::net::__Msg_Inbound::ReceiveFrame(from, *value.downcast::<Vec<u8>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Receiving::ReceiveFrame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<u8>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_Receiving: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_Receiving);
fn __decode_msg_Receiving(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_Inbound {
            return crate::wire::salvo_decode::<crate::net::__Msg_Inbound>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn connect(transport: &mut crate::net::__Handle_Transport, me: NodeEndpoint) -> bool {
    return connect__2(transport, me, crate::scheduler::salvo_pool(((1) as usize)));
}

pub fn connect__2(transport: &mut crate::net::__Handle_Transport, me: NodeEndpoint, on: usize) -> bool {
    if crate::scheduler::salvo_connected() {
        return false;
    }
    return { let __a1 = ({ let __h = Sending::new(transport.clone()); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, Box::new(__Actor_Sending::new(__h)), __DECODE_Sending); __a }); connect__3(transport, me, __a1, ({ let __h = Receiving::new(); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(on, __cap as usize, Box::new(__Actor_Receiving::new(__h)), __DECODE_Receiving); __a })) };
}

pub fn connect__3(transport: &mut crate::net::__Handle_Transport, me: NodeEndpoint, sending: usize, receiving: usize) -> bool {
    if crate::scheduler::salvo_connected() {
        return false;
    }
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

pub trait NodeGroup {
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

impl NodeGroup for __Stub_NodeGroup {
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Members(out), crate::net::__PROTO_NodeGroup);
    }
    fn subscribe(&mut self, w: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Subscribe(w), crate::net::__PROTO_NodeGroup);
    }
    fn leave(&mut self) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Leave, crate::net::__PROTO_NodeGroup);
    }
}

pub struct __Handle_NodeGroup {
    inner: std::sync::Arc<std::sync::Mutex<dyn NodeGroup + Send>>,
}

impl Clone for __Handle_NodeGroup {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_NodeGroup {
    pub fn new<__H: NodeGroup + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn NodeGroup + Send>>) -> Self {
        Self { inner }
    }
}

impl NodeGroup for __Handle_NodeGroup {
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().members(out)
    }
    fn subscribe(&mut self, w: usize) {
        self.inner.lock().unwrap().subscribe(w)
    }
    fn leave(&mut self) {
        self.inner.lock().unwrap().leave()
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
pub const __PROTO_NodeGroup: &str = "ffd5bcc19200cc53";

pub trait NodeChanges {
    fn joined(&mut self, n: Node);
    fn left(&mut self, n: Node, why: String);
}

pub struct __Stub_NodeChanges {
    addr: usize,
}

impl __Stub_NodeChanges {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl NodeChanges for __Stub_NodeChanges {
    fn joined(&mut self, n: Node) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeChanges::Joined(n), crate::net::__PROTO_NodeChanges);
    }
    fn left(&mut self, n: Node, why: String) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeChanges::Left(n, why), crate::net::__PROTO_NodeChanges);
    }
}

pub struct __Handle_NodeChanges {
    inner: std::sync::Arc<std::sync::Mutex<dyn NodeChanges + Send>>,
}

impl Clone for __Handle_NodeChanges {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_NodeChanges {
    pub fn new<__H: NodeChanges + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn NodeChanges + Send>>) -> Self {
        Self { inner }
    }
}

impl NodeChanges for __Handle_NodeChanges {
    fn joined(&mut self, n: Node) {
        self.inner.lock().unwrap().joined(n)
    }
    fn left(&mut self, n: Node, why: String) {
        self.inner.lock().unwrap().left(n, why)
    }
}

pub enum __Msg_NodeChanges {
    Joined(Node),
    Left(Node, String),
}

impl crate::wire::__Wire for __Msg_NodeChanges {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_NodeChanges::Joined(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeChanges::Left(__p0, __p1) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_NodeChanges::Joined(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_NodeChanges::Left(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `NodeChanges`.
pub const __PROTO_NodeChanges: &str = "db3e0aa5831c2e44";

pub struct StaticNodeGroup {
    name: String,
    all: Vec<NodeEndpoint>,
    known: SalvoMap<NodeId, Node>,
    watchers: Vec<usize>,
    __dep_Transport: crate::net::__Handle_Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_StaticNodeGroup>,
}

impl StaticNodeGroup {
    pub fn new(name: String, all: Vec<NodeEndpoint>, __dep_Transport: crate::net::__Handle_Transport) -> Self {
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

impl NodeGroup for StaticNodeGroup {

    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        let mut all_known: Vec<Node> = vec![];
        for mut id in self.known.keys().cloned().collect::<Vec<_>>() {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                all_known.push(n.unwrap().clone());
            }
        }
        crate::scheduler::salvo_reply_wire::<Vec<Node>>(out, all_known);
    }

    fn subscribe(&mut self, w: usize) {
        self.watchers.push(w);
    }

    fn leave(&mut self) {
        crate::scheduler::salvo_leave_group();
    }
}

impl StaticNodeGroup {

    fn init(&mut self) {
        if !(crate::scheduler::salvo_connected()) { panic!("salvo: {} at net:382:9", "a node group starts on a connected node: call connect(me) first".to_string()) };
        let mut me = self.__dep_Transport.local_endpoint();
        crate::scheduler::salvo_set_group((self.name.clone()).clone(), crate::wire::salvo_encode(&me.clone()));
        crate::scheduler::salvo_watch_peers((self.__addr.expect("a handler naming its own address runs as an actor")).clone(), |__n, __ep, __t| Box::new(__Priv_StaticNodeGroup::Hello(NodeId { id: __n as i64 }, crate::wire::salvo_decode::<NodeEndpoint>(__ep).expect("a peer's endpoint"), __t.iter().map(|(a, b)| (a.clone(), b.clone())).collect())), |__n| Box::new(__Priv_StaticNodeGroup::Gone(NodeId { id: __n as i64 })), |__ps| Box::new(__Priv_StaticNodeGroup::Introduced(__ps.iter().filter_map(|__p| crate::wire::salvo_decode::<NodeEndpoint>(__p)).collect())));
        for e in &self.all {
            if !eq(e, &me) {
                let mut _sent = self.__dep_Transport.deliver(&(e.clone()), crate::scheduler::salvo_hello_frame());
            }
        }
    }

    fn hello(&mut self, node: NodeId, at: NodeEndpoint, protocols: Vec<(String, String)>) {
        if self.known.contains_key(&node) {
            return;
        }
        let mut n = Node { id: node.clone(), at: at };
        self.known.insert(node, n.clone());
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Joined(n.clone()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn gone(&mut self, node: NodeId) {
        let mut n = self.known.remove(&node);
        if n.is_none() {
            return;
        }
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Left(n.as_ref().unwrap().clone(), "left".to_string()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn introduced(&mut self, peers: Vec<NodeEndpoint>) {
    }
}

pub enum __Cont_StaticNodeGroup {
    Members,
    Subscribe,
    Hello(NodeId, NodeEndpoint),
    Gone,
    Introduced,
}

pub enum __Priv_StaticNodeGroup {
    Init,
    Hello(NodeId, NodeEndpoint, Vec<(String, String)>),
    Gone(NodeId),
    Introduced(Vec<NodeEndpoint>),
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
            crate::net::__Msg_NodeGroup::Members(out) => crate::net::NodeGroup::members(&mut self.handler, out),
            crate::net::__Msg_NodeGroup::Subscribe(w) => crate::net::NodeGroup::subscribe(&mut self.handler, w),
            crate::net::__Msg_NodeGroup::Leave => crate::net::NodeGroup::leave(&mut self.handler),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_StaticNodeGroup) {
        match msg {
            __Priv_StaticNodeGroup::Init => self.handler.init(),
            __Priv_StaticNodeGroup::Hello(node, at, protocols) => self.handler.hello(node, at, protocols),
            __Priv_StaticNodeGroup::Gone(node) => self.handler.gone(node),
            __Priv_StaticNodeGroup::Introduced(peers) => self.handler.introduced(peers),
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
            __Cont_StaticNodeGroup::Hello(node, at) => self.__dispatch_priv(__Priv_StaticNodeGroup::Hello(node, at, *value.downcast::<Vec<(String, String)>>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Gone => self.__dispatch_priv(__Priv_StaticNodeGroup::Gone(*value.downcast::<NodeId>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Introduced => self.__dispatch_priv(__Priv_StaticNodeGroup::Introduced(*value.downcast::<Vec<NodeEndpoint>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_StaticNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Hello{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<(String, String)>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Gone{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeId>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Introduced{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<NodeEndpoint>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_StaticNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_StaticNodeGroup);
fn __decode_msg_StaticNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_NodeGroup {
            return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct GossipNodeGroup {
    name: String,
    seeds: Vec<NodeEndpoint>,
    known: SalvoMap<NodeId, Node>,
    dialed: SalvoSet<String>,
    watchers: Vec<usize>,
    __dep_Transport: crate::net::__Handle_Transport,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_GossipNodeGroup>,
}

impl GossipNodeGroup {
    pub fn new(name: String, seeds: Vec<NodeEndpoint>, __dep_Transport: crate::net::__Handle_Transport) -> Self {
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

impl NodeGroup for GossipNodeGroup {

    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        let mut all_known: Vec<Node> = vec![];
        for mut id in self.known.keys().cloned().collect::<Vec<_>>() {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                all_known.push(n.unwrap().clone());
            }
        }
        crate::scheduler::salvo_reply_wire::<Vec<Node>>(out, all_known);
    }

    fn subscribe(&mut self, w: usize) {
        self.watchers.push(w);
    }

    fn leave(&mut self) {
        crate::scheduler::salvo_leave_group();
    }
}

impl GossipNodeGroup {

    fn init(&mut self) {
        if !(crate::scheduler::salvo_connected()) { panic!("salvo: {} at net:456:9", "a node group starts on a connected node: call connect(me) first".to_string()) };
        let mut me = self.__dep_Transport.local_endpoint();
        crate::scheduler::salvo_set_group((self.name.clone()).clone(), crate::wire::salvo_encode(&me.clone()));
        crate::scheduler::salvo_watch_peers((self.__addr.expect("a handler naming its own address runs as an actor")).clone(), |__n, __ep, __t| Box::new(__Priv_GossipNodeGroup::Hello(NodeId { id: __n as i64 }, crate::wire::salvo_decode::<NodeEndpoint>(__ep).expect("a peer's endpoint"), __t.iter().map(|(a, b)| (a.clone(), b.clone())).collect())), |__n| Box::new(__Priv_GossipNodeGroup::Gone(NodeId { id: __n as i64 })), |__ps| Box::new(__Priv_GossipNodeGroup::Introduced(__ps.iter().filter_map(|__p| crate::wire::salvo_decode::<NodeEndpoint>(__p)).collect())));
        for e in &self.seeds {
            dial(&mut self.__dep_Transport, &mut self.dialed, e.clone());
        }
    }

    fn hello(&mut self, node: NodeId, at: NodeEndpoint, protocols: Vec<(String, String)>) {
        if self.known.contains_key(&node) {
            return;
        }
        let mut others: Vec<NodeEndpoint> = vec![];
        for mut id in self.known.keys().cloned().collect::<Vec<_>>() {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                others.push(n.unwrap().clone().at.clone());
            }
            crate::scheduler::salvo_introduce((id.clone()).id as u64, &(vec![at.clone()]).iter().map(|__p| crate::wire::salvo_encode(__p)).collect::<Vec<_>>());
        }
        crate::scheduler::salvo_introduce((node.clone()).id as u64, &(others).iter().map(|__p| crate::wire::salvo_encode(__p)).collect::<Vec<_>>());
        self.dialed.insert(to_str__2(&at));
        let mut n = Node { id: node.clone(), at: at };
        self.known.insert(node, n.clone());
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Joined(n.clone()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn gone(&mut self, node: NodeId) {
        let mut n = self.known.remove(&node);
        if n.is_none() {
            return;
        }
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Left(n.as_ref().unwrap().clone(), "left".to_string()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn introduced(&mut self, peers: Vec<NodeEndpoint>) {
        for e in &peers {
            dial(&mut self.__dep_Transport, &mut self.dialed, e.clone());
        }
    }
}

pub enum __Cont_GossipNodeGroup {
    Members,
    Subscribe,
    Hello(NodeId, NodeEndpoint),
    Gone,
    Introduced,
}

pub enum __Priv_GossipNodeGroup {
    Init,
    Hello(NodeId, NodeEndpoint, Vec<(String, String)>),
    Gone(NodeId),
    Introduced(Vec<NodeEndpoint>),
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
            crate::net::__Msg_NodeGroup::Members(out) => crate::net::NodeGroup::members(&mut self.handler, out),
            crate::net::__Msg_NodeGroup::Subscribe(w) => crate::net::NodeGroup::subscribe(&mut self.handler, w),
            crate::net::__Msg_NodeGroup::Leave => crate::net::NodeGroup::leave(&mut self.handler),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_GossipNodeGroup) {
        match msg {
            __Priv_GossipNodeGroup::Init => self.handler.init(),
            __Priv_GossipNodeGroup::Hello(node, at, protocols) => self.handler.hello(node, at, protocols),
            __Priv_GossipNodeGroup::Gone(node) => self.handler.gone(node),
            __Priv_GossipNodeGroup::Introduced(peers) => self.handler.introduced(peers),
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
            __Cont_GossipNodeGroup::Hello(node, at) => self.__dispatch_priv(__Priv_GossipNodeGroup::Hello(node, at, *value.downcast::<Vec<(String, String)>>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Gone => self.__dispatch_priv(__Priv_GossipNodeGroup::Gone(*value.downcast::<NodeId>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Introduced => self.__dispatch_priv(__Priv_GossipNodeGroup::Introduced(*value.downcast::<Vec<NodeEndpoint>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_GossipNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Hello{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<(String, String)>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Gone{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeId>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Introduced{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<NodeEndpoint>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_GossipNodeGroup: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_GossipNodeGroup);
fn __decode_msg_GossipNodeGroup(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_NodeGroup {
            return crate::wire::salvo_decode::<crate::net::__Msg_NodeGroup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn dial(transport: &mut crate::net::__Handle_Transport, dialed: &mut SalvoSet<String>, e: NodeEndpoint) {
    if eq(&e, &(transport.local_endpoint())) || dialed.contains(&to_str__2(&e)) {
        return;
    }
    dialed.insert(to_str__2(&e));
    let mut _sent = transport.deliver(&e, crate::scheduler::salvo_hello_frame());
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

pub trait ActorGroup {
    fn join(&mut self, member: usize);
    fn leave(&mut self, member: usize);
    fn members(&mut self, out: crate::scheduler::SalvoReply);
    fn subscribe(&mut self, w: usize);
}

pub struct __Stub_ActorGroup {
    addr: usize,
}

impl __Stub_ActorGroup {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl ActorGroup for __Stub_ActorGroup {
    fn join(&mut self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Join(member), crate::net::__PROTO_ActorGroup);
    }
    fn leave(&mut self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Leave(member), crate::net::__PROTO_ActorGroup);
    }
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Members(out), crate::net::__PROTO_ActorGroup);
    }
    fn subscribe(&mut self, w: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Subscribe(w), crate::net::__PROTO_ActorGroup);
    }
}

pub struct __Handle_ActorGroup {
    inner: std::sync::Arc<std::sync::Mutex<dyn ActorGroup + Send>>,
}

impl Clone for __Handle_ActorGroup {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_ActorGroup {
    pub fn new<__H: ActorGroup + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn ActorGroup + Send>>) -> Self {
        Self { inner }
    }
}

impl ActorGroup for __Handle_ActorGroup {
    fn join(&mut self, member: usize) {
        self.inner.lock().unwrap().join(member)
    }
    fn leave(&mut self, member: usize) {
        self.inner.lock().unwrap().leave(member)
    }
    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().members(out)
    }
    fn subscribe(&mut self, w: usize) {
        self.inner.lock().unwrap().subscribe(w)
    }
}

pub enum __Msg_ActorGroup {
    Join(usize),
    Leave(usize),
    Members(crate::scheduler::SalvoReply),
    Subscribe(usize),
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
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_ActorGroup::Join(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_ActorGroup::Leave(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_ActorGroup::Members(crate::wire::__Wire::__dec(r)?)),
            3 => Some(__Msg_ActorGroup::Subscribe(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `ActorGroup`.
pub const __PROTO_ActorGroup: &str = "2baf186cc10cfc79";

pub trait ActorChanges {
    fn joined(&mut self, member: usize);
    fn left(&mut self, member: usize);
}

pub struct __Stub_ActorChanges {
    addr: usize,
}

impl __Stub_ActorChanges {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl ActorChanges for __Stub_ActorChanges {
    fn joined(&mut self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorChanges::Joined(member), crate::net::__PROTO_ActorChanges);
    }
    fn left(&mut self, member: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorChanges::Left(member), crate::net::__PROTO_ActorChanges);
    }
}

pub struct __Handle_ActorChanges {
    inner: std::sync::Arc<std::sync::Mutex<dyn ActorChanges + Send>>,
}

impl Clone for __Handle_ActorChanges {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_ActorChanges {
    pub fn new<__H: ActorChanges + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn ActorChanges + Send>>) -> Self {
        Self { inner }
    }
}

impl ActorChanges for __Handle_ActorChanges {
    fn joined(&mut self, member: usize) {
        self.inner.lock().unwrap().joined(member)
    }
    fn left(&mut self, member: usize) {
        self.inner.lock().unwrap().left(member)
    }
}

pub enum __Msg_ActorChanges {
    Joined(usize),
    Left(usize),
}

impl crate::wire::__Wire for __Msg_ActorChanges {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_ActorChanges::Joined(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorChanges::Left(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_ActorChanges::Joined(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_ActorChanges::Left(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `ActorChanges`.
pub const __PROTO_ActorChanges: &str = "9fa424e5858bb3bd";

pub fn open_group(proto: Protocol, nodes: usize) -> usize {
    let mut name = proto.name.clone();
    return open_named_group(name, proto, nodes);
}

pub fn open_named_group(name: String, proto: Protocol, nodes: usize) -> usize {
    let (mut group, mut changes) = ({ let __h = ActorGrouping::new(name, proto); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, Box::new(__Actor_ActorGrouping::new(__h)), __DECODE_ActorGrouping); crate::scheduler::salvo_send(__a, Box::new(__Priv_ActorGrouping::Init)); (__a, __a) });
    crate::scheduler::salvo_send_wire(nodes, crate::net::__Msg_NodeGroup::Subscribe(changes), crate::net::__PROTO_NodeGroup);
    return group;
}

pub fn join(group: &usize, member: usize) {
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

impl ActorGroup for ActorGrouping {

    fn join(&mut self, member: usize) {
        if !admit(&mut self.all, member.clone()) {
            return;
        }
        mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorChanges::Joined(member.clone()), crate::net::__PROTO_ActorChanges);
        }
        for p in &self.peers {
            crate::scheduler::salvo_share_members(&self.name.clone(), (p.clone()).id as u64, &vec![member.clone()]);
        }
    }

    fn leave(&mut self, member: usize) {
        if !withdraw(&mut self.all, member.clone()) {
            return;
        }
        mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorChanges::Left(member.clone()), crate::net::__PROTO_ActorChanges);
        }
    }

    fn members(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_reply_wire::<Vec<usize>>(out, self.all.clone());
    }

    fn subscribe(&mut self, w: usize) {
        self.watchers.push(w);
    }
}

impl NodeChanges for ActorGrouping {

    fn joined(&mut self, n: Node) {
    }

    fn left(&mut self, n: Node, why: String) {
        let mut gone: Vec<usize> = vec![];
        for m in &self.all {
            if eq__2(&(NodeId { id: crate::scheduler::salvo_addr_identity((m.clone()).clone()).node as i64 }), &n.id) {
                gone.push(m.clone());
            }
        }
        for m in &gone {
            if withdraw(&mut self.all, m.clone()) {
                for w in &self.watchers {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorChanges::Left(m.clone()), crate::net::__PROTO_ActorChanges);
                }
            }
        }
        if ((gone.len() as i32) > 0) {
            mirror(&(self.__addr.expect("a handler naming its own address runs as an actor")), &self.all);
        }
    }
}

impl ActorGrouping {

    fn init(&mut self) {
        { let __me = (self.__addr.expect("a handler naming its own address runs as an actor")).clone(); crate::scheduler::salvo_publish((self.name.clone()).clone(), __me, Some((__me, |__n| Box::new(__Priv_ActorGrouping::Peer(NodeId { id: __n as i64 })), |__n, __ids| Box::new(__Priv_ActorGrouping::Merged(NodeId { id: __n as i64 }, __ids.iter().map(|__r| crate::scheduler::salvo_import_addr(*__r)).collect()))))) };
    }

    fn peer(&mut self, node: NodeId) {
        let mut theirs = crate::scheduler::salvo_peer_protocol((node.clone()).id as u64, &self.proto.name.clone());
        if ((theirs.is_none()) || !(&theirs.as_ref().unwrap()[..] == &self.proto.hash[..])) {
            return;
        }
        if contains_node(&self.peers, &(node.clone())) {
            return;
        }
        self.peers.push(node.clone());
        crate::scheduler::salvo_share_members(&self.name.clone(), (node).id as u64, &self.all.clone());
    }

    fn merged(&mut self, from: NodeId, found: Vec<usize>) {
        if !contains_node(&self.peers, &(from.clone())) {
            let mut theirs = crate::scheduler::salvo_peer_protocol((from.clone()).id as u64, &self.proto.name.clone());
            if ((theirs.is_none()) || !(&theirs.as_ref().unwrap()[..] == &self.proto.hash[..])) {
                return;
            }
            self.peers.push(from);
        }
        let mut changed = false;
        for m in &found {
            if admit(&mut self.all, m.clone()) {
                changed = true;
                for w in &self.watchers {
                    crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorChanges::Joined(m.clone()), crate::net::__PROTO_ActorChanges);
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
    Peer,
    Merged(NodeId),
}

pub enum __Priv_ActorGrouping {
    Init,
    Peer(NodeId),
    Merged(NodeId, Vec<usize>),
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
            crate::net::__Msg_ActorGroup::Join(member) => crate::net::ActorGroup::join(&mut self.handler, member),
            crate::net::__Msg_ActorGroup::Leave(member) => crate::net::ActorGroup::leave(&mut self.handler, member),
            crate::net::__Msg_ActorGroup::Members(out) => crate::net::ActorGroup::members(&mut self.handler, out),
            crate::net::__Msg_ActorGroup::Subscribe(w) => crate::net::ActorGroup::subscribe(&mut self.handler, w),
        }
    }
    fn __dispatch_NodeChanges(&mut self, msg: crate::net::__Msg_NodeChanges) {
        match msg {
            crate::net::__Msg_NodeChanges::Joined(n) => crate::net::NodeChanges::joined(&mut self.handler, n),
            crate::net::__Msg_NodeChanges::Left(n, why) => crate::net::NodeChanges::left(&mut self.handler, n, why),
        }
    }
    fn __dispatch_priv(&mut self, msg: __Priv_ActorGrouping) {
        match msg {
            __Priv_ActorGrouping::Init => self.handler.init(),
            __Priv_ActorGrouping::Peer(node) => self.handler.peer(node),
            __Priv_ActorGrouping::Merged(from, found) => self.handler.merged(from, found),
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
        let msg = match msg.downcast::<crate::net::__Msg_NodeChanges>() {
            Ok(__m) => return self.__dispatch_NodeChanges(*__m),
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
            __Cont_ActorGrouping::Joined => self.__dispatch_NodeChanges(crate::net::__Msg_NodeChanges::Joined(*value.downcast::<Node>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Left(n) => self.__dispatch_NodeChanges(crate::net::__Msg_NodeChanges::Left(n, *value.downcast::<String>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Peer => self.__dispatch_priv(__Priv_ActorGrouping::Peer(*value.downcast::<NodeId>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Merged(from) => self.__dispatch_priv(__Priv_ActorGrouping::Merged(from, *value.downcast::<Vec<usize>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ActorGrouping::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Leave{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Joined{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Node>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Left{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Peer{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeId>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Merged{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_ActorGrouping: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_ActorGrouping);
fn __decode_msg_ActorGrouping(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_ActorGroup {
            return crate::wire::salvo_decode::<crate::net::__Msg_ActorGroup>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
        if proto == crate::net::__PROTO_NodeChanges {
            return crate::wire::salvo_decode::<crate::net::__Msg_NodeChanges>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn mirror(group: &usize, members: &Vec<usize>) {
    crate::scheduler::salvo_view_set((group.clone()).clone(), &members.clone());
}

pub fn admit(list: &mut Vec<usize>, a: usize) -> bool {
    for x in &*list {
        if ((crate::scheduler::salvo_addr_identity((x).clone()) == crate::scheduler::salvo_addr_identity((a).clone()))) {
            return false;
        }
    }
    list.push(a);
    return true;
}

pub fn contains_node(list: &Vec<NodeId>, n: &NodeId) -> bool {
    for x in list {
        if eq__2(x, n) {
            return true;
        }
    }
    return false;
}

pub fn withdraw(list: &mut Vec<usize>, a: usize) -> bool {
    let mut mut_index: Option<i32> = None;
    let mut i = 0;
    for x in &*list {
        if ((crate::scheduler::salvo_addr_identity((x).clone()) == crate::scheduler::salvo_addr_identity((a).clone()))) {
            mut_index = Some(i);
        }
        i = i + 1;
    }
    if mut_index.is_none() {
        return false;
    }
    let mut _removed = list.salvo_remove_at(mut_index.unwrap());
    return true;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActorView {
    pub addr: usize,
    pub pending: i32,
    pub local: bool,
}

impl crate::wire::__Wire for ActorView {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.addr, out);
        crate::wire::__Wire::__enc(&self.pending, out);
        crate::wire::__Wire::__enc(&self.local, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            addr: crate::wire::__Wire::__dec(r)?,
            pending: crate::wire::__Wire::__dec(r)?,
            local: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActorGroupView {
    pub actors: Vec<ActorView>,
    pub key: Option<i64>,
}

impl crate::wire::__Wire for ActorGroupView {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.actors, out);
        crate::wire::__Wire::__enc(&self.key, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            actors: crate::wire::__Wire::__dec(r)?,
            key: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub trait Pick {
    fn choose(&mut self, view: ActorGroupView) -> Option<usize>;
}

pub struct __Handle_Pick {
    inner: std::sync::Arc<std::sync::Mutex<dyn Pick + Send>>,
}

impl Clone for __Handle_Pick {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Pick {
    pub fn new<__H: Pick + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Pick + Send>>) -> Self {
        Self { inner }
    }
}

impl Pick for __Handle_Pick {
    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        self.inner.lock().unwrap().choose(view)
    }
}

pub fn route_to(pick: &mut crate::net::__Handle_Pick, group: &usize) -> usize {
    return route_keyed(pick, group, &(None));
}

pub fn route_to__2(pick: &mut crate::net::__Handle_Pick, group: &usize, key: i64) -> usize {
    return route_keyed(pick, group, &(Some(key)));
}

pub fn route_keyed(pick: &mut crate::net::__Handle_Pick, group: &usize, key: &Option<i64>) -> usize {
    loop {
        let mut members = crate::scheduler::salvo_view_members((group.clone()).clone());
        let mut actors: Vec<ActorView> = vec![];
        for m in &members {
            actors.push(ActorView { addr: m.clone(), pending: crate::scheduler::salvo_pending((m.clone()).clone()), local: eq__2(&(NodeId { id: crate::scheduler::salvo_addr_identity((m.clone()).clone()).node as i64 }), &(NodeId { id: crate::scheduler::salvo_here_node() as i64 })) });
        }
        let mut picked = pick.choose(ActorGroupView { actors: actors.clone(), key: key.clone() });
        if !(picked.is_none()) {
            return picked.as_ref().unwrap().clone();
        }
        crate::scheduler::salvo_park_briefly();
    }
    return route_keyed(pick, group, key);
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

impl Pick for LeastLoaded {

    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        let mut best: Option<ActorView> = None;
        for a in &view.actors {
            if best.is_none() {
                best = Some(a.clone());
            } else {
                let mut b: ActorView = best.as_ref().unwrap().clone();
                let mut take = if self.prefer_local && a.local && !b.local {
                    true
                } else if self.prefer_local && !a.local && b.local {
                    false
                } else {
                    a.pending < b.pending
                };
                if take {
                    best = Some(a.clone());
                }
            }
        }
        let mut chosen: ActorView = if best.is_some() { best.as_ref().unwrap().clone() } else { return None };
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

impl Pick for Sharded {

    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        let mut n = (view.actors.len() as i32);
        if n == 0 {
            return None;
        }
        let mut k = if view.key.is_some() { view.key.unwrap() } else { 0i64 };
        let mut magnitude = if k < 0i64 {
            0i64 - k
        } else {
            k
        };
        let mut slot = ((magnitude % ((n) as i64)) as i32);
        let mut picked = { let __pick1 = view.actors.get((slot) as i64 as usize); if __pick1.is_some() { __pick1.unwrap() } else { return None } };
        return Some(picked.addr.clone());
    }
}

pub trait Leader {
    fn leader(&mut self) -> Option<NodeId>;
}

pub struct __Handle_Leader {
    inner: std::sync::Arc<std::sync::Mutex<dyn Leader + Send>>,
}

impl Clone for __Handle_Leader {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Leader {
    pub fn new<__H: Leader + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Leader + Send>>) -> Self {
        Self { inner }
    }
}

impl Leader for __Handle_Leader {
    fn leader(&mut self) -> Option<NodeId> {
        self.inner.lock().unwrap().leader()
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

impl Leader for StaticLeader {

    fn leader(&mut self) -> Option<NodeId> {
        return Some(self.node.clone());
    }
}

#[derive(Clone)]
pub struct Elected {
    __dep_Leader: crate::net::__Handle_Leader,
}

impl Elected {
    pub fn new(__dep_Leader: crate::net::__Handle_Leader) -> Self {
        Self {
            __dep_Leader,
        }
    }
}

impl Pick for Elected {

    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        let mut l = { let __pick2 = self.__dep_Leader.leader(); if __pick2.is_some() { __pick2.as_ref().unwrap().clone() } else { return None } };
        for a in &view.actors {
            if eq__2(&(NodeId { id: crate::scheduler::salvo_addr_identity((a.addr.clone()).clone()).node as i64 }), &l) {
                return Some(a.addr.clone());
            }
        }
        return None;
    }
}

pub trait MemNet {
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

impl MemNet for __Stub_MemNet {
    fn attach(&mut self, at: NodeEndpoint, sink: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Attach(at, sink), crate::net::__PROTO_MemNet);
    }
    fn detach(&mut self, at: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Detach(at), crate::net::__PROTO_MemNet);
    }
    fn route(&mut self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Route(from, to, out), crate::net::__PROTO_MemNet);
    }
    fn partition(&mut self, a: NodeEndpoint, b: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Partition(a, b), crate::net::__PROTO_MemNet);
    }
    fn heal(&mut self, a: NodeEndpoint, b: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Heal(a, b), crate::net::__PROTO_MemNet);
    }
    fn kill(&mut self, node: NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Kill(node), crate::net::__PROTO_MemNet);
    }
    fn delivered(&mut self, out: crate::scheduler::SalvoReply) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_MemNet::Delivered(out), crate::net::__PROTO_MemNet);
    }
}

pub struct __Handle_MemNet {
    inner: std::sync::Arc<std::sync::Mutex<dyn MemNet + Send>>,
}

impl Clone for __Handle_MemNet {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_MemNet {
    pub fn new<__H: MemNet + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn MemNet + Send>>) -> Self {
        Self { inner }
    }
}

impl MemNet for __Handle_MemNet {
    fn attach(&mut self, at: NodeEndpoint, sink: usize) {
        self.inner.lock().unwrap().attach(at, sink)
    }
    fn detach(&mut self, at: NodeEndpoint) {
        self.inner.lock().unwrap().detach(at)
    }
    fn route(&mut self, from: NodeEndpoint, to: NodeEndpoint, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().route(from, to, out)
    }
    fn partition(&mut self, a: NodeEndpoint, b: NodeEndpoint) {
        self.inner.lock().unwrap().partition(a, b)
    }
    fn heal(&mut self, a: NodeEndpoint, b: NodeEndpoint) {
        self.inner.lock().unwrap().heal(a, b)
    }
    fn kill(&mut self, node: NodeEndpoint) {
        self.inner.lock().unwrap().kill(node)
    }
    fn delivered(&mut self, out: crate::scheduler::SalvoReply) {
        self.inner.lock().unwrap().delivered(out)
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

impl MemNet for MemNetwork {

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
            crate::net::__Msg_MemNet::Attach(at, sink) => crate::net::MemNet::attach(&mut self.handler, at, sink),
            crate::net::__Msg_MemNet::Detach(at) => crate::net::MemNet::detach(&mut self.handler, at),
            crate::net::__Msg_MemNet::Route(from, to, out) => crate::net::MemNet::route(&mut self.handler, from, to, out),
            crate::net::__Msg_MemNet::Partition(a, b) => crate::net::MemNet::partition(&mut self.handler, a, b),
            crate::net::__Msg_MemNet::Heal(a, b) => crate::net::MemNet::heal(&mut self.handler, a, b),
            crate::net::__Msg_MemNet::Kill(node) => crate::net::MemNet::kill(&mut self.handler, node),
            crate::net::__Msg_MemNet::Delivered(out) => crate::net::MemNet::delivered(&mut self.handler, out),
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
            __Cont_MemNetwork::Attach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Detach{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Route{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Partition{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Heal{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Kill{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<NodeEndpoint>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_MemNetwork::Delivered{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
        }
    }
}

pub const __DECODE_MemNetwork: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_MemNetwork);
fn __decode_msg_MemNetwork(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        if proto == crate::net::__PROTO_MemNet {
            return crate::wire::salvo_decode::<crate::net::__Msg_MemNet>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
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

impl Transport for MemTransport {

    fn listen(&mut self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        crate::scheduler::salvo_send_wire(self.net.clone(), crate::net::__Msg_MemNet::Attach(at.clone(), sink), crate::net::__PROTO_MemNet);
        return Union2::<(), Union2<Unreachable, WireFailed>>::U1(ok(()));
    }

    fn unlisten(&mut self, at: &NodeEndpoint) {
        crate::scheduler::salvo_send_wire(self.net.clone(), crate::net::__Msg_MemNet::Detach(at.clone()), crate::net::__PROTO_MemNet);
    }

    fn deliver(&mut self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>> {
        let mut sink = {
            let (mut out, __wid) = crate::scheduler::salvo_waiter();
            crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Option<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
            crate::scheduler::salvo_send_wire(self.net.clone(), crate::net::__Msg_MemNet::Route(self.me.clone(), to.clone(), out), crate::net::__PROTO_MemNet);
            *crate::scheduler::salvo_wait(__wid).downcast::<Option<usize>>().expect("the awaited answer")
        };
        if sink.is_none() {
            return Union2::<(), Union2<Unreachable, WireFailed>>::U2(Union2::<Unreachable, WireFailed>::U1(err(Unreachable { to: to.clone() })));
        }
        crate::scheduler::salvo_send_wire(sink.as_ref().unwrap().clone(), crate::net::__Msg_Inbound::ReceiveFrame(self.me.clone(), frame), crate::net::__PROTO_Inbound);
        return Union2::<(), Union2<Unreachable, WireFailed>>::U1(ok(()));
    }

    fn local_endpoint(&mut self) -> NodeEndpoint {
        return self.me.clone();
    }
}

pub fn cut_key(a: &NodeEndpoint, b: &NodeEndpoint) -> String {
    return format!("{}>{}", to_str__2(a), to_str__2(b));
}

pub fn cmp(a: &NodeEndpoint, b: &NodeEndpoint) -> i32 {
    (Ord::cmp(a, b) as i32)
}

pub fn hash(value: &NodeEndpoint) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq(a: &NodeEndpoint, b: &NodeEndpoint) -> bool {
    (a == b)
}

pub fn hash__2(value: &NodeId) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq__2(a: &NodeId, b: &NodeId) -> bool {
    (a == b)
}

pub fn hash__3(value: &Node) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq__3(a: &Node, b: &Node) -> bool {
    (a == b)
}

pub struct __Hash_hash__NodeId_NodeId;
impl SalvoHash<NodeId> for __Hash_hash__NodeId_NodeId {
    fn hash(__v: &NodeId) -> i64 { hash__2(__v) }
}

pub struct __Eq_eq__NodeId_NodeId;
impl SalvoEq<NodeId> for __Eq_eq__NodeId_NodeId {
    fn eq(__a: &NodeId, __b: &NodeId) -> bool { eq__2(__a, __b) }
}

pub struct __Hash_hash__NodeEndpoint_NodeEndpoint;
impl SalvoHash<NodeEndpoint> for __Hash_hash__NodeEndpoint_NodeEndpoint {
    fn hash(__v: &NodeEndpoint) -> i64 { hash(__v) }
}

pub struct __Eq_eq__NodeEndpoint_NodeEndpoint;
impl SalvoEq<NodeEndpoint> for __Eq_eq__NodeEndpoint_NodeEndpoint {
    fn eq(__a: &NodeEndpoint, __b: &NodeEndpoint) -> bool { eq(__a, __b) }
}
