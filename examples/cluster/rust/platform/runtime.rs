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

// `threadsafe platform handler HostRuntime` [runtime-host]: stateless, so
// sharing it across every thread needs no synchronization.
pub struct HostRuntime;

impl HostRuntime {
    pub fn new() -> Self {
        HostRuntime
    }
}

impl crate::runtime::RuntimeHostPlatformSync for HostRuntime {
    // [addr-capability] OS entropy: `/dev/urandom` where there is one, and
    // the standard library's per-process random hasher keys otherwise.
    fn secure_bits(&self) -> i64 {
        use std::io::Read;
        let mut b = [0u8; 8];
        if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok() {
            return i64::from_ne_bytes(b);
        }
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0));
        h.finish() as i64
    }

    fn report(&self, line: &String) {
        eprintln!("{line}");
    }

    // [time-timer] The clock `time.tick()` reads.
    fn mono_nanos(&self) -> i64 {
        crate::hosttime::salvo_mono_nanos()
    }
}
