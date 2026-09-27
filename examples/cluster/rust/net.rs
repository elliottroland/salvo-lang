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
    fn frame(&mut self, from: NodeEndpoint, data: Vec<u8>);
}

pub trait __Has_Inbound {
    fn __get_Inbound(&mut self) -> &mut dyn Inbound;
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
    fn frame(&mut self, from: NodeEndpoint, data: Vec<u8>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Inbound::Frame(from, data), crate::net::__PROTO_Inbound);
    }
}

pub enum __Msg_Inbound {
    Frame(NodeEndpoint, Vec<u8>),
}

impl crate::wire::__Wire for __Msg_Inbound {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Inbound::Frame(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Inbound::Frame(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Inbound`.
pub const __PROTO_Inbound: &str = "d9080cf2876be4b1";

pub trait Transport {
    fn listen(&mut self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn unlisten(&mut self, at: &NodeEndpoint);
    fn deliver(&mut self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn local_endpoint(&mut self) -> NodeEndpoint;
}

pub trait __Has_Transport {
    fn __get_Transport(&mut self) -> &mut dyn Transport;
}

pub trait __Share_Transport: Transport + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Transport>;
}

impl<__H: Transport + Clone + Send + 'static> __Share_Transport for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Transport> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Transport {
    inner: Box<dyn __Share_Transport>,
}

impl Clone for __Mon_Transport {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Transport {
    pub fn new(inner: Box<dyn __Share_Transport>) -> Self {
        Self { inner }
    }
}

impl Transport for __Mon_Transport {
    fn listen(&mut self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        self.inner.listen(at, sink)
    }
    fn unlisten(&mut self, at: &NodeEndpoint) {
        self.inner.unlisten(at)
    }
    fn deliver(&mut self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>> {
        self.inner.deliver(to, frame)
    }
    fn local_endpoint(&mut self) -> NodeEndpoint {
        self.inner.local_endpoint()
    }
}

pub struct __Lock_Transport<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Transport<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Transport<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Transport + Send> Transport for __Lock_Transport<H> {
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

impl __Has_Transport for __Mon_Transport {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        self
    }
}

pub trait Outbound {
    fn frame(&mut self, to: NodeEndpoint, data: Vec<u8>);
}

pub trait __Has_Outbound {
    fn __get_Outbound(&mut self) -> &mut dyn Outbound;
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
    fn frame(&mut self, to: NodeEndpoint, data: Vec<u8>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Outbound::Frame(to, data), crate::net::__PROTO_Outbound);
    }
}

pub enum __Msg_Outbound {
    Frame(NodeEndpoint, Vec<u8>),
}

impl crate::wire::__Wire for __Msg_Outbound {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Outbound::Frame(__p0, __p1) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Outbound::Frame(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Outbound`.
pub const __PROTO_Outbound: &str = "d9080cf2876be4b1";

pub struct Sending {
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_Sending>,
}

impl Sending {
    pub fn new() -> Self {
        Self {
            __mailbox_capacity: 256,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_Sending<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Transport + ?Sized> __Has_Transport for __Deps_Sending<'a, __P> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__p)
    }
}

pub trait __Impl_Sending {

    fn frame<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, to: NodeEndpoint, data: Vec<u8>);
}

impl __Impl_Sending for Sending {

    fn frame<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, to: NodeEndpoint, data: Vec<u8>) {
        let mut _sent = __Has_Transport::__get_Transport(&mut *__fx).deliver(&to, data);
    }
}

pub enum __Cont_Sending {
    Frame(NodeEndpoint),
}

pub struct __Prov_Sending<__D0> {
    pub __d0: __D0,
}

impl<__D0: Transport> __Has_Transport for __Prov_Sending<__D0> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        &mut self.__d0
    }
}

pub struct __Actor_Sending<__D0> {
    handler: Sending,
    prov: __Prov_Sending<__D0>,
}

impl<__D0> __Actor_Sending<__D0> {
    pub fn new(handler: Sending, prov: __Prov_Sending<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Transport> __Actor_Sending<__D0> {
    fn __dispatch(&mut self, msg: crate::net::__Msg_Outbound) {
        let mut __deps = __Deps_Sending{ __p: &mut self.prov };
        match msg {
            crate::net::__Msg_Outbound::Frame(to, data) => __Impl_Sending::frame(&mut self.handler, &mut __deps, to, data),
        }
    }
}

impl<__D0: Transport + Send + 'static> crate::scheduler::SalvoActor for __Actor_Sending<__D0> {
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
            __Cont_Sending::Frame(to) => self.__dispatch(crate::net::__Msg_Outbound::Frame(to, *value.downcast::<Vec<u8>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Sending::Frame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<u8>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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

    fn frame(&mut self, from: NodeEndpoint, data: Vec<u8>) {
        let mut _delivered = crate::scheduler::salvo_deliver_frame(&data);
    }
}

pub enum __Cont_Receiving {
    Frame(NodeEndpoint),
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
            crate::net::__Msg_Inbound::Frame(from, data) => crate::net::Inbound::frame(&mut self.handler, from, data),
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
            __Cont_Receiving::Frame(from) => self.__dispatch(crate::net::__Msg_Inbound::Frame(from, *value.downcast::<Vec<u8>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_Receiving::Frame{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<u8>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Node {
    pub id: i64,
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
    fn join(&mut self, events: usize);
    fn members(&mut self, out: crate::scheduler::SalvoReply);
    fn subscribe(&mut self, w: usize);
    fn leave(&mut self);
}

pub trait __Has_NodeGroup {
    fn __get_NodeGroup(&mut self) -> &mut dyn NodeGroup;
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
    fn join(&mut self, events: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_NodeGroup::Join(events), crate::net::__PROTO_NodeGroup);
    }
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

pub enum __Msg_NodeGroup {
    Join(usize),
    Members(crate::scheduler::SalvoReply),
    Subscribe(usize),
    Leave,
}

impl crate::wire::__Wire for __Msg_NodeGroup {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_NodeGroup::Join(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeGroup::Members(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeGroup::Subscribe(__p0) => {
                out.push(2);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_NodeGroup::Leave => out.push(3),
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_NodeGroup::Join(crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_NodeGroup::Members(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_NodeGroup::Subscribe(crate::wire::__Wire::__dec(r)?)),
            3 => Some(__Msg_NodeGroup::Leave),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `NodeGroup`.
pub const __PROTO_NodeGroup: &str = "233fd4f252e83877";

pub trait NodeChanges {
    fn joined(&mut self, n: Node);
    fn left(&mut self, n: Node, why: String);
}

pub trait __Has_NodeChanges {
    fn __get_NodeChanges(&mut self) -> &mut dyn NodeChanges;
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
pub const __PROTO_NodeChanges: &str = "44acce8053e58038";

pub trait PeerEvents {
    fn hello(&mut self, node: i64, at: NodeEndpoint, protocols: Vec<(String, String)>);
    fn gone(&mut self, node: i64);
    fn introduced(&mut self, peers: Vec<NodeEndpoint>);
}

pub trait __Has_PeerEvents {
    fn __get_PeerEvents(&mut self) -> &mut dyn PeerEvents;
}

pub struct __Stub_PeerEvents {
    addr: usize,
}

impl __Stub_PeerEvents {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl PeerEvents for __Stub_PeerEvents {
    fn hello(&mut self, node: i64, at: NodeEndpoint, protocols: Vec<(String, String)>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_PeerEvents::Hello(node, at, protocols), crate::net::__PROTO_PeerEvents);
    }
    fn gone(&mut self, node: i64) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_PeerEvents::Gone(node), crate::net::__PROTO_PeerEvents);
    }
    fn introduced(&mut self, peers: Vec<NodeEndpoint>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_PeerEvents::Introduced(peers), crate::net::__PROTO_PeerEvents);
    }
}

pub enum __Msg_PeerEvents {
    Hello(i64, NodeEndpoint, Vec<(String, String)>),
    Gone(i64),
    Introduced(Vec<NodeEndpoint>),
}

impl crate::wire::__Wire for __Msg_PeerEvents {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_PeerEvents::Hello(__p0, __p1, __p2) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
                crate::wire::__Wire::__enc(__p2, out);
            }
            __Msg_PeerEvents::Gone(__p0) => {
                out.push(1);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_PeerEvents::Introduced(__p0) => {
                out.push(2);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_PeerEvents::Hello(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            1 => Some(__Msg_PeerEvents::Gone(crate::wire::__Wire::__dec(r)?)),
            2 => Some(__Msg_PeerEvents::Introduced(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `PeerEvents`.
pub const __PROTO_PeerEvents: &str = "c95a1117ed864d0b";

pub fn start_group(faces: (usize, usize)) -> usize {
    let (mut group, mut events) = faces;
    crate::scheduler::salvo_send_wire(group, crate::net::__Msg_NodeGroup::Join(events), crate::net::__PROTO_NodeGroup);
    return group;
}

pub struct StaticNodeGroup {
    name: String,
    me: NodeEndpoint,
    all: Vec<NodeEndpoint>,
    known: SalvoMap<i64, Node>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_StaticNodeGroup>,
}

impl StaticNodeGroup {
    pub fn new(name: String, me: NodeEndpoint, all: Vec<NodeEndpoint>) -> Self {
        Self {
            name,
            me,
            all,
            known: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            watchers: vec![],
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_StaticNodeGroup<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Transport + ?Sized> __Has_Transport for __Deps_StaticNodeGroup<'a, __P> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__p)
    }
}

pub trait __Impl_StaticNodeGroup {

    fn join<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, events: usize);

    fn members<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, out: crate::scheduler::SalvoReply);

    fn subscribe<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, w: usize);

    fn leave<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx);

    fn hello<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64, at: NodeEndpoint, protocols: Vec<(String, String)>);

    fn gone<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64);

    fn introduced<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, peers: Vec<NodeEndpoint>);
}

impl __Impl_StaticNodeGroup for StaticNodeGroup {

    fn join<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, events: usize) {
        crate::scheduler::salvo_set_group((self.name.clone()).clone(), crate::wire::salvo_encode(&self.me.clone()));
        crate::scheduler::salvo_watch_peers((events).clone(), |__n, __ep, __t| Box::new(crate::net::__Msg_PeerEvents::Hello(__n as i64, crate::wire::salvo_decode::<NodeEndpoint>(__ep).expect("a peer's endpoint"), __t.iter().map(|(a, b)| (a.clone(), b.clone())).collect())), |__n| Box::new(crate::net::__Msg_PeerEvents::Gone(__n as i64)), |__ps| Box::new(crate::net::__Msg_PeerEvents::Introduced(__ps.iter().filter_map(|__p| crate::wire::salvo_decode::<NodeEndpoint>(__p)).collect())));
        for e in &self.all {
            if !eq(e, &self.me) {
                let mut _sent = __Has_Transport::__get_Transport(&mut *__fx).deliver(&(e.clone()), crate::scheduler::salvo_hello_frame());
            }
        }
    }

    fn members<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, out: crate::scheduler::SalvoReply) {
        let mut all_known: Vec<Node> = vec![];
        for mut id in self.known.keys().cloned().collect::<Vec<_>>() {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                all_known.push(n.unwrap().clone());
            }
        }
        crate::scheduler::salvo_reply_wire::<Vec<Node>>(out, all_known);
    }

    fn subscribe<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, w: usize) {
        self.watchers.push(w);
    }

    fn leave<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx) {
        crate::scheduler::salvo_leave_group();
    }

    fn hello<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64, at: NodeEndpoint, protocols: Vec<(String, String)>) {
        if self.known.contains_key(&node) {
            return;
        }
        let mut n = Node { id: node.clone(), at: at };
        self.known.insert(node, n.clone());
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Joined(n.clone()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn gone<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64) {
        let mut n = self.known.remove(&node);
        if n.is_none() {
            return;
        }
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Left(n.as_ref().unwrap().clone(), "left".to_string()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn introduced<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, peers: Vec<NodeEndpoint>) {
    }
}

pub enum __Cont_StaticNodeGroup {
    Join,
    Members,
    Subscribe,
    Hello(i64, NodeEndpoint),
    Gone,
    Introduced,
}

pub struct __Prov_StaticNodeGroup<__D0> {
    pub __d0: __D0,
}

impl<__D0: Transport> __Has_Transport for __Prov_StaticNodeGroup<__D0> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        &mut self.__d0
    }
}

pub struct __Actor_StaticNodeGroup<__D0> {
    handler: StaticNodeGroup,
    prov: __Prov_StaticNodeGroup<__D0>,
}

impl<__D0> __Actor_StaticNodeGroup<__D0> {
    pub fn new(handler: StaticNodeGroup, prov: __Prov_StaticNodeGroup<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Transport> __Actor_StaticNodeGroup<__D0> {
    fn __dispatch_NodeGroup(&mut self, msg: crate::net::__Msg_NodeGroup) {
        let mut __deps = __Deps_StaticNodeGroup{ __p: &mut self.prov };
        match msg {
            crate::net::__Msg_NodeGroup::Join(events) => __Impl_StaticNodeGroup::join(&mut self.handler, &mut __deps, events),
            crate::net::__Msg_NodeGroup::Members(out) => __Impl_StaticNodeGroup::members(&mut self.handler, &mut __deps, out),
            crate::net::__Msg_NodeGroup::Subscribe(w) => __Impl_StaticNodeGroup::subscribe(&mut self.handler, &mut __deps, w),
            crate::net::__Msg_NodeGroup::Leave => __Impl_StaticNodeGroup::leave(&mut self.handler, &mut __deps),
        }
    }
    fn __dispatch_PeerEvents(&mut self, msg: crate::net::__Msg_PeerEvents) {
        let mut __deps = __Deps_StaticNodeGroup{ __p: &mut self.prov };
        match msg {
            crate::net::__Msg_PeerEvents::Hello(node, at, protocols) => __Impl_StaticNodeGroup::hello(&mut self.handler, &mut __deps, node, at, protocols),
            crate::net::__Msg_PeerEvents::Gone(node) => __Impl_StaticNodeGroup::gone(&mut self.handler, &mut __deps, node),
            crate::net::__Msg_PeerEvents::Introduced(peers) => __Impl_StaticNodeGroup::introduced(&mut self.handler, &mut __deps, peers),
        }
    }
}

impl<__D0: Transport + Send + 'static> crate::scheduler::SalvoActor for __Actor_StaticNodeGroup<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::net::__Msg_NodeGroup>() {
            Ok(__m) => return self.__dispatch_NodeGroup(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<crate::net::__Msg_PeerEvents>() {
            Ok(__m) => return self.__dispatch_PeerEvents(*__m),
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
            __Cont_StaticNodeGroup::Join => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Join(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Members => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Subscribe => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Hello(node, at) => self.__dispatch_PeerEvents(crate::net::__Msg_PeerEvents::Hello(node, at, *value.downcast::<Vec<(String, String)>>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Gone => self.__dispatch_PeerEvents(crate::net::__Msg_PeerEvents::Gone(*value.downcast::<i64>().expect("the awaited answer"))),
            __Cont_StaticNodeGroup::Introduced => self.__dispatch_PeerEvents(crate::net::__Msg_PeerEvents::Introduced(*value.downcast::<Vec<NodeEndpoint>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_StaticNodeGroup::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Hello{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<(String, String)>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_StaticNodeGroup::Gone{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<i64>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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
        if proto == crate::net::__PROTO_PeerEvents {
            return crate::wire::salvo_decode::<crate::net::__Msg_PeerEvents>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub struct GossipNodeGroup {
    name: String,
    me: NodeEndpoint,
    seeds: Vec<NodeEndpoint>,
    known: SalvoMap<i64, Node>,
    dialed: SalvoSet<String>,
    watchers: Vec<usize>,
    pub __mailbox_capacity: i32,
    __addr: Option<usize>,
    __parked: std::collections::HashMap<u64, __Cont_GossipNodeGroup>,
}

impl GossipNodeGroup {
    pub fn new(name: String, me: NodeEndpoint, seeds: Vec<NodeEndpoint>) -> Self {
        Self {
            name,
            me,
            seeds,
            known: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            dialed: SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]),
            watchers: vec![],
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

pub struct __Deps_GossipNodeGroup<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Transport + ?Sized> __Has_Transport for __Deps_GossipNodeGroup<'a, __P> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        __Has_Transport::__get_Transport(&mut *self.__p)
    }
}

pub trait __Impl_GossipNodeGroup {

    fn join<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, events: usize);

    fn members<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, out: crate::scheduler::SalvoReply);

    fn subscribe<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, w: usize);

    fn leave<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx);

    fn hello<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64, at: NodeEndpoint, protocols: Vec<(String, String)>);

    fn gone<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64);

    fn introduced<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, peers: Vec<NodeEndpoint>);
}

impl __Impl_GossipNodeGroup for GossipNodeGroup {

    fn join<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, events: usize) {
        crate::scheduler::salvo_set_group((self.name.clone()).clone(), crate::wire::salvo_encode(&self.me.clone()));
        crate::scheduler::salvo_watch_peers((events).clone(), |__n, __ep, __t| Box::new(crate::net::__Msg_PeerEvents::Hello(__n as i64, crate::wire::salvo_decode::<NodeEndpoint>(__ep).expect("a peer's endpoint"), __t.iter().map(|(a, b)| (a.clone(), b.clone())).collect())), |__n| Box::new(crate::net::__Msg_PeerEvents::Gone(__n as i64)), |__ps| Box::new(crate::net::__Msg_PeerEvents::Introduced(__ps.iter().filter_map(|__p| crate::wire::salvo_decode::<NodeEndpoint>(__p)).collect())));
        for e in &self.seeds {
            dial(&mut *__fx, &self.me, &mut self.dialed, e.clone());
        }
    }

    fn members<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, out: crate::scheduler::SalvoReply) {
        let mut all_known: Vec<Node> = vec![];
        for mut id in self.known.keys().cloned().collect::<Vec<_>>() {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                all_known.push(n.unwrap().clone());
            }
        }
        crate::scheduler::salvo_reply_wire::<Vec<Node>>(out, all_known);
    }

    fn subscribe<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, w: usize) {
        self.watchers.push(w);
    }

    fn leave<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx) {
        crate::scheduler::salvo_leave_group();
    }

    fn hello<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64, at: NodeEndpoint, protocols: Vec<(String, String)>) {
        if self.known.contains_key(&node) {
            return;
        }
        let mut others: Vec<NodeEndpoint> = vec![];
        for mut id in self.known.keys().cloned().collect::<Vec<_>>() {
            let mut n = self.known.get(&id);
            if !(n.is_none()) {
                others.push(n.unwrap().clone().at.clone());
            }
            crate::scheduler::salvo_introduce((id.clone()) as u64, &(vec![at.clone()]).iter().map(|__p| crate::wire::salvo_encode(__p)).collect::<Vec<_>>());
        }
        crate::scheduler::salvo_introduce((node.clone()) as u64, &(others).iter().map(|__p| crate::wire::salvo_encode(__p)).collect::<Vec<_>>());
        self.dialed.insert(to_str__2(&at));
        let mut n = Node { id: node.clone(), at: at };
        self.known.insert(node, n.clone());
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Joined(n.clone()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn gone<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, node: i64) {
        let mut n = self.known.remove(&node);
        if n.is_none() {
            return;
        }
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_NodeChanges::Left(n.as_ref().unwrap().clone(), "left".to_string()), crate::net::__PROTO_NodeChanges);
        }
    }

    fn introduced<__Fx: __Has_Transport>(&mut self, __fx: &mut __Fx, peers: Vec<NodeEndpoint>) {
        for e in &peers {
            dial(&mut *__fx, &self.me, &mut self.dialed, e.clone());
        }
    }
}

pub enum __Cont_GossipNodeGroup {
    Join,
    Members,
    Subscribe,
    Hello(i64, NodeEndpoint),
    Gone,
    Introduced,
}

pub struct __Prov_GossipNodeGroup<__D0> {
    pub __d0: __D0,
}

impl<__D0: Transport> __Has_Transport for __Prov_GossipNodeGroup<__D0> {
    fn __get_Transport(&mut self) -> &mut dyn Transport {
        &mut self.__d0
    }
}

pub struct __Actor_GossipNodeGroup<__D0> {
    handler: GossipNodeGroup,
    prov: __Prov_GossipNodeGroup<__D0>,
}

impl<__D0> __Actor_GossipNodeGroup<__D0> {
    pub fn new(handler: GossipNodeGroup, prov: __Prov_GossipNodeGroup<__D0>) -> Self {
        Self { handler, prov }
    }
}

impl<__D0: Transport> __Actor_GossipNodeGroup<__D0> {
    fn __dispatch_NodeGroup(&mut self, msg: crate::net::__Msg_NodeGroup) {
        let mut __deps = __Deps_GossipNodeGroup{ __p: &mut self.prov };
        match msg {
            crate::net::__Msg_NodeGroup::Join(events) => __Impl_GossipNodeGroup::join(&mut self.handler, &mut __deps, events),
            crate::net::__Msg_NodeGroup::Members(out) => __Impl_GossipNodeGroup::members(&mut self.handler, &mut __deps, out),
            crate::net::__Msg_NodeGroup::Subscribe(w) => __Impl_GossipNodeGroup::subscribe(&mut self.handler, &mut __deps, w),
            crate::net::__Msg_NodeGroup::Leave => __Impl_GossipNodeGroup::leave(&mut self.handler, &mut __deps),
        }
    }
    fn __dispatch_PeerEvents(&mut self, msg: crate::net::__Msg_PeerEvents) {
        let mut __deps = __Deps_GossipNodeGroup{ __p: &mut self.prov };
        match msg {
            crate::net::__Msg_PeerEvents::Hello(node, at, protocols) => __Impl_GossipNodeGroup::hello(&mut self.handler, &mut __deps, node, at, protocols),
            crate::net::__Msg_PeerEvents::Gone(node) => __Impl_GossipNodeGroup::gone(&mut self.handler, &mut __deps, node),
            crate::net::__Msg_PeerEvents::Introduced(peers) => __Impl_GossipNodeGroup::introduced(&mut self.handler, &mut __deps, peers),
        }
    }
}

impl<__D0: Transport + Send + 'static> crate::scheduler::SalvoActor for __Actor_GossipNodeGroup<__D0> {
    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {
        self.handler.__addr = Some(_ctx.addr);
        let msg = match msg.downcast::<crate::net::__Msg_NodeGroup>() {
            Ok(__m) => return self.__dispatch_NodeGroup(*__m),
            Err(__m) => __m,
        };
        let msg = match msg.downcast::<crate::net::__Msg_PeerEvents>() {
            Ok(__m) => return self.__dispatch_PeerEvents(*__m),
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
            __Cont_GossipNodeGroup::Join => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Join(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Members => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Members(*value.downcast::<crate::scheduler::SalvoReply>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Subscribe => self.__dispatch_NodeGroup(crate::net::__Msg_NodeGroup::Subscribe(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Hello(node, at) => self.__dispatch_PeerEvents(crate::net::__Msg_PeerEvents::Hello(node, at, *value.downcast::<Vec<(String, String)>>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Gone => self.__dispatch_PeerEvents(crate::net::__Msg_PeerEvents::Gone(*value.downcast::<i64>().expect("the awaited answer"))),
            __Cont_GossipNodeGroup::Introduced => self.__dispatch_PeerEvents(crate::net::__Msg_PeerEvents::Introduced(*value.downcast::<Vec<NodeEndpoint>>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_GossipNodeGroup::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Hello{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<(String, String)>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_GossipNodeGroup::Gone{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<i64>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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
        if proto == crate::net::__PROTO_PeerEvents {
            return crate::wire::salvo_decode::<crate::net::__Msg_PeerEvents>(payload)
                .map(|__m| Box::new(__m) as crate::scheduler::SalvoMsg);
        }
    None
}

pub fn dial<__Fx: __Has_Transport>(__fx: &mut __Fx, me: &NodeEndpoint, dialed: &mut SalvoSet<String>, e: NodeEndpoint) {
    if eq(&e, me) || dialed.contains(&to_str__2(&e)) {
        return;
    }
    dialed.insert(to_str__2(&e));
    let mut _sent = __Has_Transport::__get_Transport(&mut *__fx).deliver(&e, crate::scheduler::salvo_hello_frame());
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
    fn peer(&mut self, node: i64);
    fn merged(&mut self, from: i64, found: Vec<usize>);
    fn start(&mut self, me: usize);
}

pub trait __Has_ActorGroup {
    fn __get_ActorGroup(&mut self) -> &mut dyn ActorGroup;
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
    fn peer(&mut self, node: i64) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Peer(node), crate::net::__PROTO_ActorGroup);
    }
    fn merged(&mut self, from: i64, found: Vec<usize>) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Merged(from, found), crate::net::__PROTO_ActorGroup);
    }
    fn start(&mut self, me: usize) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_ActorGroup::Start(me), crate::net::__PROTO_ActorGroup);
    }
}

pub enum __Msg_ActorGroup {
    Join(usize),
    Leave(usize),
    Members(crate::scheduler::SalvoReply),
    Subscribe(usize),
    Peer(i64),
    Merged(i64, Vec<usize>),
    Start(usize),
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
            __Msg_ActorGroup::Peer(__p0) => {
                out.push(4);
                crate::wire::__Wire::__enc(__p0, out);
            }
            __Msg_ActorGroup::Merged(__p0, __p1) => {
                out.push(5);
                crate::wire::__Wire::__enc(__p0, out);
                crate::wire::__Wire::__enc(__p1, out);
            }
            __Msg_ActorGroup::Start(__p0) => {
                out.push(6);
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
            4 => Some(__Msg_ActorGroup::Peer(crate::wire::__Wire::__dec(r)?)),
            5 => Some(__Msg_ActorGroup::Merged(crate::wire::__Wire::__dec(r)?, crate::wire::__Wire::__dec(r)?)),
            6 => Some(__Msg_ActorGroup::Start(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `ActorGroup`.
pub const __PROTO_ActorGroup: &str = "97d578baed604429";

pub trait ActorChanges {
    fn joined(&mut self, member: usize);
    fn left(&mut self, member: usize);
}

pub trait __Has_ActorChanges {
    fn __get_ActorChanges(&mut self) -> &mut dyn ActorChanges;
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

pub fn attach(name: String, proto: Protocol, nodes: usize) -> usize {
    let (mut group, mut changes) = ({ let __h = ActorGrouping::new(name, proto); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn(crate::scheduler::salvo_pool(((1) as usize)), __cap as usize, Box::new(__Actor_ActorGrouping::new(__h)), __DECODE_ActorGrouping); (__a, __a) });
    crate::scheduler::salvo_send_wire(nodes, crate::net::__Msg_NodeGroup::Subscribe(changes), crate::net::__PROTO_NodeGroup);
    crate::scheduler::salvo_send_wire(group, crate::net::__Msg_ActorGroup::Start(group.clone()), crate::net::__PROTO_ActorGroup);
    return group;
}

pub fn join(group: &usize, member: usize) {
    crate::scheduler::salvo_send_wire(group.clone(), crate::net::__Msg_ActorGroup::Join(member), crate::net::__PROTO_ActorGroup);
}

pub fn attach__2(proto: Protocol, nodes: usize) -> usize {
    let mut name = proto.name.clone();
    return attach(name, proto, nodes);
}

pub struct ActorGrouping {
    name: String,
    proto: Protocol,
    all: Vec<usize>,
    peers: Vec<i64>,
    watchers: Vec<usize>,
    self_addr: Option<usize>,
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
            self_addr: None,
            __mailbox_capacity: 64,
            __addr: None,
            __parked: std::collections::HashMap::new(),
        }
    }
}

impl ActorGroup for ActorGrouping {

    fn start(&mut self, me: usize) {
        self.self_addr = Some(me.clone());
        { let __me = (me).clone(); crate::scheduler::salvo_publish((self.name.clone()).clone(), __me, Some((__me, |__n| Box::new(crate::net::__Msg_ActorGroup::Peer(__n as i64)), |__n, __ids| Box::new(crate::net::__Msg_ActorGroup::Merged(__n as i64, __ids.iter().map(|__r| crate::scheduler::salvo_import_addr(*__r)).collect()))))) };
    }

    fn join(&mut self, member: usize) {
        if !admit(&mut self.all, member.clone()) {
            return;
        }
        mirror(&self.self_addr, &self.all);
        for w in &self.watchers {
            crate::scheduler::salvo_send_wire(w.clone(), crate::net::__Msg_ActorChanges::Joined(member.clone()), crate::net::__PROTO_ActorChanges);
        }
        for p in &self.peers {
            crate::scheduler::salvo_share_members(&self.name.clone(), (p.clone()) as u64, &vec![member.clone()]);
        }
    }

    fn leave(&mut self, member: usize) {
        if !withdraw(&mut self.all, member.clone()) {
            return;
        }
        mirror(&self.self_addr, &self.all);
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

    fn peer(&mut self, node: i64) {
        let mut theirs = crate::scheduler::salvo_peer_protocol((node.clone()) as u64, &self.proto.name.clone());
        if ((theirs.is_none()) || !(&theirs.as_ref().unwrap()[..] == &self.proto.hash[..])) {
            return;
        }
        if contains_node(&self.peers, node.clone()) {
            return;
        }
        self.peers.push(node.clone());
        crate::scheduler::salvo_share_members(&self.name.clone(), (node) as u64, &self.all.clone());
    }

    fn merged(&mut self, from: i64, found: Vec<usize>) {
        if !contains_node(&self.peers, from.clone()) {
            let mut theirs = crate::scheduler::salvo_peer_protocol((from.clone()) as u64, &self.proto.name.clone());
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
            mirror(&self.self_addr, &self.all);
        }
    }
}

impl NodeChanges for ActorGrouping {

    fn joined(&mut self, n: Node) {
    }

    fn left(&mut self, n: Node, why: String) {
        let mut gone: Vec<usize> = vec![];
        for m in &self.all {
            if ((((crate::scheduler::salvo_addr_identity((m.clone()).clone()).node as i64)) == (n.id))) {
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
            mirror(&self.self_addr, &self.all);
        }
    }
}

pub enum __Cont_ActorGrouping {
    Start,
    Join,
    Leave,
    Joined,
    Left(Node),
    Members,
    Subscribe,
    Peer,
    Merged(i64),
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
            crate::net::__Msg_ActorGroup::Peer(node) => crate::net::ActorGroup::peer(&mut self.handler, node),
            crate::net::__Msg_ActorGroup::Merged(from, found) => crate::net::ActorGroup::merged(&mut self.handler, from, found),
            crate::net::__Msg_ActorGroup::Start(me) => crate::net::ActorGroup::start(&mut self.handler, me),
        }
    }
    fn __dispatch_NodeChanges(&mut self, msg: crate::net::__Msg_NodeChanges) {
        match msg {
            crate::net::__Msg_NodeChanges::Joined(n) => crate::net::NodeChanges::joined(&mut self.handler, n),
            crate::net::__Msg_NodeChanges::Left(n, why) => crate::net::NodeChanges::left(&mut self.handler, n, why),
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
            __Cont_ActorGrouping::Peer => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Peer(*value.downcast::<i64>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Merged(from) => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Merged(from, *value.downcast::<Vec<usize>>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Start => self.__dispatch_ActorGroup(crate::net::__Msg_ActorGroup::Start(*value.downcast::<usize>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Joined => self.__dispatch_NodeChanges(crate::net::__Msg_NodeChanges::Joined(*value.downcast::<Node>().expect("the awaited answer"))),
            __Cont_ActorGrouping::Left(n) => self.__dispatch_NodeChanges(crate::net::__Msg_NodeChanges::Left(n, *value.downcast::<String>().expect("the awaited answer"))),
        }
    }

    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {
        match self.handler.__parked.get(&slot)? {
            __Cont_ActorGrouping::Join{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Leave{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Members{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<crate::scheduler::SalvoReply>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Subscribe{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Peer{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<i64>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Merged{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Vec<usize>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Start{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<usize>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Joined{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<Node>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
            __Cont_ActorGrouping::Left{ .. } => (|__b: &[u8]| crate::wire::salvo_decode::<String>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))(payload),
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

pub fn mirror(group: &Option<usize>, members: &Vec<usize>) {
    if group.is_none() {
        return;
    }
    crate::scheduler::salvo_view_set((group.as_ref().unwrap()).clone(), &members.clone());
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

pub fn contains_node(list: &Vec<i64>, n: i64) -> bool {
    for x in list {
        if (((*x) == (n))) {
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

pub trait __Has_Pick {
    fn __get_Pick(&mut self) -> &mut dyn Pick;
}

pub trait __Share_Pick: Pick + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Pick>;
}

impl<__H: Pick + Clone + Send + 'static> __Share_Pick for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Pick> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Pick {
    inner: Box<dyn __Share_Pick>,
}

impl Clone for __Mon_Pick {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Pick {
    pub fn new(inner: Box<dyn __Share_Pick>) -> Self {
        Self { inner }
    }
}

impl Pick for __Mon_Pick {
    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        self.inner.choose(view)
    }
}

pub struct __Lock_Pick<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Pick<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Pick<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Pick + Send> Pick for __Lock_Pick<H> {
    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        self.inner.lock().unwrap().choose(view)
    }
}

impl __Has_Pick for __Mon_Pick {
    fn __get_Pick(&mut self) -> &mut dyn Pick {
        self
    }
}

pub fn route_to<__Fx: __Has_Pick>(__fx: &mut __Fx, group: &usize) -> usize {
    return route_keyed(&mut *__fx, group, &(None));
}

pub fn route_to__2<__Fx: __Has_Pick>(__fx: &mut __Fx, group: &usize, key: i64) -> usize {
    return route_keyed(&mut *__fx, group, &(Some(key)));
}

pub fn route_keyed<__Fx: __Has_Pick>(__fx: &mut __Fx, group: &usize, key: &Option<i64>) -> usize {
    loop {
        let mut members = crate::scheduler::salvo_view_members((group.clone()).clone());
        let mut actors: Vec<ActorView> = vec![];
        for m in &members {
            actors.push(ActorView { addr: m.clone(), pending: crate::scheduler::salvo_pending((m.clone()).clone()), local: (((crate::scheduler::salvo_addr_identity((m.clone()).clone()).node as i64)) == ((crate::scheduler::salvo_here_node() as i64))) });
        }
        let mut picked = __Has_Pick::__get_Pick(&mut *__fx).choose(ActorGroupView { actors: actors.clone(), key: key.clone() });
        if !(picked.is_none()) {
            return picked.as_ref().unwrap().clone();
        }
        crate::scheduler::salvo_park_briefly();
    }
    return route_keyed(&mut *__fx, group, key);
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
    fn leader(&mut self) -> Option<i64>;
}

pub trait __Has_Leader {
    fn __get_Leader(&mut self) -> &mut dyn Leader;
}

pub trait __Share_Leader: Leader + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Leader>;
}

impl<__H: Leader + Clone + Send + 'static> __Share_Leader for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Leader> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Leader {
    inner: Box<dyn __Share_Leader>,
}

impl Clone for __Mon_Leader {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Leader {
    pub fn new(inner: Box<dyn __Share_Leader>) -> Self {
        Self { inner }
    }
}

impl Leader for __Mon_Leader {
    fn leader(&mut self) -> Option<i64> {
        self.inner.leader()
    }
}

pub struct __Lock_Leader<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Leader<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Leader<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Leader + Send> Leader for __Lock_Leader<H> {
    fn leader(&mut self) -> Option<i64> {
        self.inner.lock().unwrap().leader()
    }
}

impl __Has_Leader for __Mon_Leader {
    fn __get_Leader(&mut self) -> &mut dyn Leader {
        self
    }
}

#[derive(Clone)]
pub struct StaticLeader {
    node: i64,
}

impl StaticLeader {
    pub fn new(node: i64) -> Self {
        Self {
            node,
        }
    }
}

impl Leader for StaticLeader {

    fn leader(&mut self) -> Option<i64> {
        return Some(self.node.clone());
    }
}

#[derive(Clone)]
pub struct Elected {
    __dep_Leader: crate::net::__Mon_Leader,
}

impl Elected {
    pub fn new(__dep_Leader: crate::net::__Mon_Leader) -> Self {
        Self {
            __dep_Leader,
        }
    }
}

impl Pick for Elected {

    fn choose(&mut self, view: ActorGroupView) -> Option<usize> {
        let mut l = { let __pick2 = __Has_Leader::__get_Leader(&mut self.__dep_Leader).leader(); if __pick2.is_some() { __pick2.unwrap() } else { return None } };
        for a in &view.actors {
            if ((((crate::scheduler::salvo_addr_identity((a.addr.clone()).clone()).node as i64)) == (l))) {
                return Some(a.addr.clone());
            }
        }
        return None;
    }
}

// [threadsafe-platform] [rs-platform-handler] The `&self` twin of `Transport`
// that the threadsafe host `HostTcpTransport` implements: the host synchronizes
// internally, and rustc checks that what it holds is `Sync`.
pub trait __Shared_HostTcpTransport: Send + Sync {
    fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn unlisten(&self, at: &NodeEndpoint);
    fn deliver(&self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>>;
    fn local_endpoint(&self) -> NodeEndpoint;
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

pub trait __Has_MemNet {
    fn __get_MemNet(&mut self) -> &mut dyn MemNet;
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
        crate::scheduler::salvo_send_wire(sink.as_ref().unwrap().clone(), crate::net::__Msg_Inbound::Frame(self.me.clone(), frame), crate::net::__PROTO_Inbound);
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

pub fn hash__2(value: &Node) -> i64 {
    let mut __h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(value, &mut __h);
    (std::hash::Hasher::finish(&__h) as i64)
}

pub fn eq__2(a: &Node, b: &Node) -> bool {
    (a == b)
}

pub struct __Hash_hash__NodeEndpoint_NodeEndpoint;
impl SalvoHash<NodeEndpoint> for __Hash_hash__NodeEndpoint_NodeEndpoint {
    fn hash(__v: &NodeEndpoint) -> i64 { hash(__v) }
}

pub struct __Eq_eq__NodeEndpoint_NodeEndpoint;
impl SalvoEq<NodeEndpoint> for __Eq_eq__NodeEndpoint_NodeEndpoint {
    fn eq(__a: &NodeEndpoint, __b: &NodeEndpoint) -> bool { eq(__a, __b) }
}
