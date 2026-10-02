// Host implementation of the platform declarations of Salvo module `runtime`:
// the primitives only the host can provide (RUNTIME.md §11.3). Written by
// hand, against the generated `runtime.sv.rs`.

use crate::runtime::*;

// `threadsafe platform type Parker` [runtime-parker]: a thread's handle.
// `std::thread::Thread` is `Clone + Send + Sync`, and its `unpark` keeps a
// token, which is the contract `Parker` states.
#[derive(Clone)]
pub struct Parker {
    thread: std::thread::Thread,
}

pub fn this_parker() -> Parker {
    Parker { thread: std::thread::current() }
}

fn own(p: &Parker) {
    if p.thread.id() != std::thread::current().id() {
        panic!("salvo: a thread parked on another thread's parker [runtime-parker]");
    }
}

pub fn park(p: &Parker) {
    own(p);
    std::thread::park();
}

pub fn park_nanos(p: &Parker, nanos: i64) {
    own(p);
    std::thread::park_timeout(std::time::Duration::from_nanos(nanos.max(0) as u64));
}

pub fn unpark(p: &Parker) {
    p.thread.unpark();
}
