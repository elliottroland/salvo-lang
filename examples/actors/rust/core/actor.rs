use crate::core_iterator::*;
use crate::core_string::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Mailbox {
    pub capacity: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub reason: String,
}

pub trait Faults {
    fn faulted(&mut self, fault: Fault);
}

pub trait __Has_Faults {
    fn __get_Faults(&mut self) -> &mut dyn Faults;
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
        crate::scheduler::salvo_send(self.addr, Box::new(__Msg_Faults::Faulted(fault)));
    }
}

pub enum __Msg_Faults {
    Faulted(Fault),
}

pub enum __Cont_Faults {
    Faulted,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exit {
    pub reason: String,
}
