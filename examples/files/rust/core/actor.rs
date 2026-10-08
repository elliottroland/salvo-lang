use crate::runtime::Token;
use crate::runtime_routing::adopt;
use crate::runtime::new_pool_of;
use crate::runtime_routing::same_actor;


pub fn eq(mut a: usize, mut b: usize) -> bool {
    return crate::runtime_routing::same_actor((((a).clone()) as i32), (((b).clone()) as i32));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mailbox {
    pub capacity: i32,
}

impl crate::wire::__Wire for Mailbox {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.capacity, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            capacity: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn pool(mut size: i32) -> usize {
    let mut p: i32 = crate::runtime::new_pool_of(size, i32::wrapping_neg(1i32));
    crate::runtime_routing::adopt(p);
    return ((p) as usize);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub reason: String,
}

impl crate::wire::__Wire for Fault {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.reason, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            reason: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub trait __Stateless_Faults: Send + Sync {
    fn faulted(&self, fault: crate::core_actor::Fault);
}

pub trait __Stateful_Faults: Send {
    fn faulted(&mut self, fault: crate::core_actor::Fault);
}

pub struct Faults {
    inner: __Inner_Faults,
}

pub enum __Inner_Faults {
    Shared(std::sync::Arc<dyn __Stateless_Faults>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Faults>>),
}

impl Clone for Faults {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Faults::Shared(h) => __Inner_Faults::Shared(h.clone()),
            __Inner_Faults::Locked(h) => __Inner_Faults::Locked(h.clone()),
        } }
    }
}

impl Faults {
    pub fn shared<__H: __Stateless_Faults + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Faults::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Faults>) -> Self {
        Self { inner: __Inner_Faults::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Faults + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Faults::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Faults>>) -> Self {
        Self { inner: __Inner_Faults::Locked(inner) }
    }
    pub fn faulted(&self, fault: crate::core_actor::Fault) {
        match &self.inner {
            __Inner_Faults::Shared(h) => h.faulted(fault),
            __Inner_Faults::Locked(h) => h.lock().unwrap().faulted(fault),
        }
    }
}

pub enum __Msg_Faults {
    Faulted(crate::core_actor::Fault),
}

impl crate::wire::__Wire for __Msg_Faults {
    fn __enc(&self, out: &mut Vec<u8>) {
        match self {
            __Msg_Faults::Faulted(__p0) => {
                out.push(0);
                crate::wire::__Wire::__enc(__p0, out);
            }
        }
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        match r.u8()? {
            0 => Some(__Msg_Faults::Faulted(crate::wire::__Wire::__dec(r)?)),
            _ => None,
        }
    }
}

/// [protocol-hash] The canonical hash of `Faults`.
pub const __PROTO_Faults: &str = "b2ab28f759af3855";

pub struct __Stub_Faults {
    addr: usize,
}

impl __Stub_Faults {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl __Stateless_Faults for __Stub_Faults {
    fn faulted(&self, fault: crate::core_actor::Fault) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Faults::Faulted(fault), crate::core_actor::__PROTO_Faults);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exit {
    pub reason: String,
}

impl crate::wire::__Wire for Exit {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.reason, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            reason: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn watch(mut target: usize, mut on_exit: crate::scheduler::SalvoReply) {
    let mut __subject_1: Option<crate::runtime::Token> = (on_exit).take_local();
    if __subject_1.is_some() {
        let mut t = __subject_1.unwrap();
        crate::runtime::watch((((target).clone()) as i32), t);
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Idle {
    pub parked_gates: i32,
    pub parked_tokens: i32,
}

impl crate::wire::__Wire for Idle {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.parked_gates, out);
        crate::wire::__Wire::__enc(&self.parked_tokens, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            parked_gates: crate::wire::__Wire::__dec(r)?,
            parked_tokens: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn on_idle(mut p: usize, mut notify: crate::scheduler::SalvoReply) {
    let mut __subject_1: Option<crate::runtime::Token> = (notify).take_local();
    if __subject_1.is_some() {
        let mut t = __subject_1.unwrap();
        crate::runtime::on_idle((((p).clone()) as i32), t);
    };
}
