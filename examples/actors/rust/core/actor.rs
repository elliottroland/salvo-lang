use crate::core_iterator::*;
use crate::core_string::*;

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

pub trait Faults {
    fn faulted(&mut self, fault: Fault);
}

pub struct __Stub_Faults {
    addr: usize,
}

impl __Stub_Faults {
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }
}

impl Faults for __Stub_Faults {
    fn faulted(&mut self, fault: Fault) {
        crate::scheduler::salvo_send_wire(self.addr, __Msg_Faults::Faulted(fault), crate::core_actor::__PROTO_Faults);
    }
}

pub struct __Handle_Faults {
    inner: std::sync::Arc<std::sync::Mutex<dyn Faults + Send>>,
}

impl Clone for __Handle_Faults {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl __Handle_Faults {
    pub fn new<__H: Faults + Send + 'static>(inner: __H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
    pub fn share(inner: std::sync::Arc<std::sync::Mutex<dyn Faults + Send>>) -> Self {
        Self { inner }
    }
}

impl Faults for __Handle_Faults {
    fn faulted(&mut self, fault: Fault) {
        self.inner.lock().unwrap().faulted(fault)
    }
}

pub enum __Msg_Faults {
    Faulted(Fault),
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
