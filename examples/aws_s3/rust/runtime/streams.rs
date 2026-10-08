use crate::core_bytes::Bytes;
use crate::runtime::Parker;
use crate::core_list::add_platform;
use crate::core_bytes::append_platform;
use crate::core_bytes::bytes_of;
use crate::runtime::external_begin;
use crate::runtime::external_end;
use crate::core_bytes::index_of_platform;
use crate::core_bytes::mut_bytes;
use crate::runtime::park_platform;
use crate::core_list::remove_at_platform;
use crate::core_bytes::slice_platform;
use crate::runtime::start_thread_platform;
use crate::core_bytes::str_of_bytes_platform;
use crate::runtime::this_parker_platform;
use crate::runtime::unpark_platform;


pub fn __module_use0() -> &'static std::sync::Arc<std::sync::Mutex<crate::runtime_streams::Streams>> {
    static CELL: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<crate::runtime_streams::Streams>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Arc::new(std::sync::Mutex::new(crate::runtime_streams::Streams::new())))
}

pub fn __module_use0_0() -> &'static crate::runtime_streams::StreamTable {
    static CELL: std::sync::OnceLock<crate::runtime_streams::StreamTable> = std::sync::OnceLock::new();
    CELL.get_or_init(|| crate::runtime_streams::StreamTable::share_locked(crate::runtime_streams::__module_use0().clone()))
}

/// [platform-type] The host's `HostIn`.
pub use crate::platform_runtime_streams::HostIn;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<HostIn>(); };

/// [platform-type] The host's `HostOut`.
pub use crate::platform_runtime_streams::HostOut;
const _: fn() = || { fn __contract<T: Send + 'static>() {} __contract::<HostOut>(); };

#[derive(Clone, Debug, PartialEq)]
pub struct HostRead {
    pub data: crate::core_bytes::Bytes,
    pub error: Option<String>,
}

impl crate::wire::__Wire for HostRead {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.data, out);
        crate::wire::__Wire::__enc(&self.error, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            data: crate::wire::__Wire::__dec(r)?,
            error: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn host_read_platform(h: &mut crate::runtime_streams::HostIn, mut max: i32) -> crate::runtime_streams::HostRead {
    crate::platform_runtime_streams::host_read(h, max)
}

pub fn host_close_in_platform(mut h: crate::runtime_streams::HostIn) {
    crate::platform_runtime_streams::host_close_in(h)
}

pub fn host_write_platform(h: &mut crate::runtime_streams::HostOut, data: &crate::core_bytes::Bytes) -> Option<String> {
    crate::platform_runtime_streams::host_write(h, data)
}

pub fn host_flush_platform(h: &mut crate::runtime_streams::HostOut) -> Option<String> {
    crate::platform_runtime_streams::host_flush(h)
}

pub fn host_close_out_platform(mut h: crate::runtime_streams::HostOut) -> Option<String> {
    crate::platform_runtime_streams::host_close_out(h)
}

pub fn host_bytes_in_platform(mut data: crate::core_bytes::Bytes) -> crate::runtime_streams::HostIn {
    crate::platform_runtime_streams::host_bytes_in(data)
}

pub fn not_ours_platform(mut handle: i64) {
    crate::platform_runtime_streams::not_ours(handle)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub utf8: bool,
    pub message: String,
}

impl crate::wire::__Wire for Fault {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.utf8, out);
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            utf8: crate::wire::__Wire::__dec(r)?,
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub struct InEntry {
    pub source: String,
    pub host: crate::runtime_streams::HostIn,
    pub position: i64,
    pub ahead: crate::core_bytes::Bytes,
    pub failed: Option<crate::runtime_streams::Fault>,
}

impl std::fmt::Debug for InEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InEntry")
            .field("source", &self.source)
            .field("host", &"<opaque>")
            .field("position", &self.position)
            .field("ahead", &self.ahead)
            .field("failed", &self.failed)
            .finish()
    }
}

pub struct OutEntry {
    pub source: String,
    pub host: crate::runtime_streams::HostOut,
    pub position: i64,
    pub failed: Option<crate::runtime_streams::Fault>,
}

impl std::fmt::Debug for OutEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutEntry")
            .field("source", &self.source)
            .field("host", &"<opaque>")
            .field("position", &self.position)
            .field("failed", &self.failed)
            .finish()
    }
}

pub fn drop_in_entry(mut e: crate::runtime_streams::InEntry) {
    let mut __destructured_1: crate::runtime_streams::InEntry = e;
    let mut source: String = __destructured_1.source;
    let mut host: crate::runtime_streams::HostIn = __destructured_1.host;
    let mut position: i64 = __destructured_1.position;
    let mut ahead: crate::core_bytes::Bytes = __destructured_1.ahead;
    let mut failed: Option<crate::runtime_streams::Fault> = __destructured_1.failed;
    crate::runtime_streams::host_close_in_platform(host);
}

pub fn drop_out_entry(mut e: crate::runtime_streams::OutEntry) {
    let mut __destructured_1: crate::runtime_streams::OutEntry = e;
    let mut source: String = __destructured_1.source;
    let mut host: crate::runtime_streams::HostOut = __destructured_1.host;
    let mut position: i64 = __destructured_1.position;
    let mut failed: Option<crate::runtime_streams::Fault> = __destructured_1.failed;
    let mut _closed: Option<String> = crate::runtime_streams::host_close_out_platform(host);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Busy {
}

impl crate::wire::__Wire for Busy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unknown {
}

impl crate::wire::__Wire for Unknown {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub struct Pending {
    pub handle: i64,
    pub done: crate::scheduler::SalvoReply,
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending")
            .field("handle", &self.handle)
            .field("done", &"<opaque>")
            .finish()
    }
}

impl crate::wire::__Wire for Pending {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.handle, out);
        crate::wire::__Wire::__enc(&self.done, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            handle: crate::wire::__Wire::__dec(r)?,
            done: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn drop_pending(mut p: crate::runtime_streams::Pending) {
    let mut __destructured_1: crate::runtime_streams::Pending = p;
    let mut handle: i64 = __destructured_1.handle;
    let mut done: crate::scheduler::SalvoReply = __destructured_1.done;
    crate::scheduler::salvo_reply_wire::<crate::runtime_streams::Chunk>(done, crate::runtime_streams::Chunk { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: None, source: String::from("") });
}

pub trait __Stateless_StreamTable: Send + Sync {
    fn next_handle(&self) -> i64;
    fn put_in(&self, handle: i64, e: crate::runtime_streams::InEntry);
    fn put_out(&self, handle: i64, e: crate::runtime_streams::OutEntry);
    fn take_in(&self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::InEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown>;
    fn take_out(&self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::OutEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown>;
    fn forget(&self, handle: i64);
    fn add_pending(&self, p: crate::runtime_streams::Pending);
    fn take_pending(&self, handle: i64) -> Option<crate::runtime_streams::Pending>;
}

pub trait __Stateful_StreamTable: Send {
    fn next_handle(&mut self) -> i64;
    fn put_in(&mut self, handle: i64, e: crate::runtime_streams::InEntry);
    fn put_out(&mut self, handle: i64, e: crate::runtime_streams::OutEntry);
    fn take_in(&mut self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::InEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown>;
    fn take_out(&mut self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::OutEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown>;
    fn forget(&mut self, handle: i64);
    fn add_pending(&mut self, p: crate::runtime_streams::Pending);
    fn take_pending(&mut self, handle: i64) -> Option<crate::runtime_streams::Pending>;
}

pub struct StreamTable {
    inner: __Inner_StreamTable,
}

pub enum __Inner_StreamTable {
    Shared(std::sync::Arc<dyn __Stateless_StreamTable>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_StreamTable>>),
}

impl Clone for StreamTable {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_StreamTable::Shared(h) => __Inner_StreamTable::Shared(h.clone()),
            __Inner_StreamTable::Locked(h) => __Inner_StreamTable::Locked(h.clone()),
        } }
    }
}

impl StreamTable {
    pub fn shared<__H: __Stateless_StreamTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_StreamTable::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_StreamTable>) -> Self {
        Self { inner: __Inner_StreamTable::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_StreamTable + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_StreamTable::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_StreamTable>>) -> Self {
        Self { inner: __Inner_StreamTable::Locked(inner) }
    }
    pub fn next_handle(&self) -> i64 {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.next_handle(),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().next_handle(),
        }
    }
    pub fn put_in(&self, handle: i64, e: crate::runtime_streams::InEntry) {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.put_in(handle, e),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().put_in(handle, e),
        }
    }
    pub fn put_out(&self, handle: i64, e: crate::runtime_streams::OutEntry) {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.put_out(handle, e),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().put_out(handle, e),
        }
    }
    pub fn take_in(&self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::InEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown> {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.take_in(handle, me),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().take_in(handle, me),
        }
    }
    pub fn take_out(&self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::OutEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown> {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.take_out(handle, me),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().take_out(handle, me),
        }
    }
    pub fn forget(&self, handle: i64) {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.forget(handle),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().forget(handle),
        }
    }
    pub fn add_pending(&self, p: crate::runtime_streams::Pending) {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.add_pending(p),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().add_pending(p),
        }
    }
    pub fn take_pending(&self, handle: i64) -> Option<crate::runtime_streams::Pending> {
        match &self.inner {
            __Inner_StreamTable::Shared(h) => h.take_pending(handle),
            __Inner_StreamTable::Locked(h) => h.lock().unwrap().take_pending(handle),
        }
    }
}

pub struct Streams {
    next: i64,
    in_keys: Vec<i64>,
    ins: Vec<crate::runtime_streams::InEntry>,
    out_keys: Vec<i64>,
    outs: Vec<crate::runtime_streams::OutEntry>,
    busy: Vec<i64>,
    waiting: Vec<crate::runtime::Parker>,
    pending: Vec<crate::runtime_streams::Pending>,
}

impl Streams {
    pub fn new() -> Self {
        Self {
            next: 0i64,
            in_keys: vec![],
            ins: vec![],
            out_keys: vec![],
            outs: vec![],
            busy: vec![],
            waiting: vec![],
            pending: vec![]
        }
    }
}

impl crate::runtime_streams::__Stateful_StreamTable for Streams {
    fn next_handle(&mut self) -> i64 {
        self.next = i64::wrapping_add(self.next, 1i64);
        return self.next;
    }
    fn put_in(&mut self, handle: i64, e: crate::runtime_streams::InEntry) {
        crate::runtime_streams::unbusy(&mut self.busy, &mut self.waiting, handle);
        crate::core_list::add_platform::<i64>(&mut self.in_keys, handle);
        crate::core_list::add_platform::<crate::runtime_streams::InEntry>(&mut self.ins, e);
    }
    fn put_out(&mut self, handle: i64, e: crate::runtime_streams::OutEntry) {
        crate::runtime_streams::unbusy(&mut self.busy, &mut self.waiting, handle);
        crate::core_list::add_platform::<i64>(&mut self.out_keys, handle);
        crate::core_list::add_platform::<crate::runtime_streams::OutEntry>(&mut self.outs, e);
    }
    fn take_in(&mut self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::InEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown> {
        let mut at: i32 = crate::runtime_streams::index_in(&self.in_keys, handle);
        if (at >= 0i32) {
            let mut _k: Option<i64> = crate::core_list::remove_at_platform::<i64>(&mut self.in_keys, at);
            crate::core_list::add_platform::<i64>(&mut self.busy, handle);
            return crate::unions::Union3::U1({
                let mut __nn_1: Option<crate::runtime_streams::InEntry> = crate::core_list::remove_at_platform::<crate::runtime_streams::InEntry>(&mut self.ins, at);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.streams:159:20");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            });
        };
        if (crate::runtime_streams::index_in(&self.busy, handle) >= 0i32) {
            crate::core_list::add_platform::<crate::runtime::Parker>(&mut self.waiting, me);
            return crate::unions::Union3::U2(crate::runtime_streams::Busy {});
        };
        return crate::unions::Union3::U3(crate::runtime_streams::Unknown {});
    }
    fn take_out(&mut self, handle: i64, me: crate::runtime::Parker) -> crate::unions::Union3<crate::runtime_streams::OutEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown> {
        let mut at: i32 = crate::runtime_streams::index_in(&self.out_keys, handle);
        if (at >= 0i32) {
            let mut _k: Option<i64> = crate::core_list::remove_at_platform::<i64>(&mut self.out_keys, at);
            crate::core_list::add_platform::<i64>(&mut self.busy, handle);
            return crate::unions::Union3::U1({
                let mut __nn_1: Option<crate::runtime_streams::OutEntry> = crate::core_list::remove_at_platform::<crate::runtime_streams::OutEntry>(&mut self.outs, at);
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.streams:173:20");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            });
        };
        if (crate::runtime_streams::index_in(&self.busy, handle) >= 0i32) {
            crate::core_list::add_platform::<crate::runtime::Parker>(&mut self.waiting, me);
            return crate::unions::Union3::U2(crate::runtime_streams::Busy {});
        };
        return crate::unions::Union3::U3(crate::runtime_streams::Unknown {});
    }
    fn forget(&mut self, handle: i64) {
        crate::runtime_streams::unbusy(&mut self.busy, &mut self.waiting, handle);
    }
    fn add_pending(&mut self, p: crate::runtime_streams::Pending) {
        crate::core_list::add_platform::<crate::runtime_streams::Pending>(&mut self.pending, p);
    }
    fn take_pending(&mut self, handle: i64) -> Option<crate::runtime_streams::Pending> {
        let mut i: i32 = 0i32;
        loop {
            if !((i < crate::core_list::size_platform::<crate::runtime_streams::Pending>(&self.pending))) {
                break;
            };
            if (({
                let mut __proj_3: &crate::runtime_streams::Pending = {
                    let mut __nn_1: Option<&crate::runtime_streams::Pending> = crate::core_list::get_platform::<crate::runtime_streams::Pending>(&self.pending, i);
                    if __nn_1.is_none() {
                        panic!("salvo: value is absent at runtime.streams:193:16");
                    } else {
                        let mut __some_2 = __nn_1.unwrap();
                        __some_2
                    }
                };
                __proj_3.handle
            }) == (handle)) {
                return crate::core_list::remove_at_platform::<crate::runtime_streams::Pending>(&mut self.pending, i);
            };
            i = i32::wrapping_add(i, 1i32);
        }
        return None;
    }
}

pub fn index_in(keys: &Vec<i64>, mut handle: i64) -> i32 {
    let mut i: i32 = 0i32;
    loop {
        if !((i < crate::core_list::size_platform::<i64>(keys))) {
            break;
        };
        if (({
            let mut __nn_1: Option<i64> = crate::core_list::get_platform::<i64>(keys, i).copied();
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime.streams:207:12");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }) == (handle)) {
            return i;
        };
        i = i32::wrapping_add(i, 1i32);
    }
    return i32::wrapping_neg(1i32);
}

pub fn unbusy(busy: &mut Vec<i64>, waiting: &mut Vec<crate::runtime::Parker>, mut handle: i64) {
    let mut at: i32 = crate::runtime_streams::index_in(&*busy, handle);
    if (at >= 0i32) {
        let mut _h: Option<i64> = crate::core_list::remove_at_platform::<i64>(&mut *busy, at);
    };
    loop {
        if !((crate::core_list::size_platform::<crate::runtime::Parker>(&*waiting) > 0i32)) {
            break;
        };
        { let __arg1 = {
            let mut __nn_1: Option<crate::runtime::Parker> = crate::core_list::remove_at_platform::<crate::runtime::Parker>(&mut *waiting, 0i32);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at runtime.streams:224:16");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }; crate::runtime::unpark_platform(&__arg1) };
    }
}

pub fn fresh_handle() -> i64 {
    return crate::runtime_streams::__module_use0_0().next_handle();
}

pub fn register_in(mut source: String, mut host: crate::runtime_streams::HostIn, mut position: i64) -> i64 {
    let mut handle: i64 = crate::runtime_streams::fresh_handle();
    crate::runtime_streams::__module_use0_0().put_in(handle, crate::runtime_streams::InEntry { source: source.clone(), host: host, position: position, ahead: crate::core_bytes::mut_bytes(vec![]), failed: None });
    return handle;
}

pub fn register_out(mut source: String, mut host: crate::runtime_streams::HostOut, mut position: i64) -> i64 {
    let mut handle: i64 = crate::runtime_streams::fresh_handle();
    crate::runtime_streams::__module_use0_0().put_out(handle, crate::runtime_streams::OutEntry { source: source.clone(), host: host, position: position, failed: None });
    return handle;
}

pub fn register_bytes(mut data: crate::core_bytes::Bytes) -> i64 {
    return crate::runtime_streams::register_in(String::from("<bytes>"), crate::runtime_streams::host_bytes_in_platform(data), 0i64);
}

pub fn checkout_in(mut handle: i64) -> crate::runtime_streams::InEntry {
    loop {
        if !(true) {
            break;
        };
        let mut got: crate::unions::Union3<crate::runtime_streams::InEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown> = crate::runtime_streams::__module_use0_0().take_in(handle, crate::runtime::this_parker_platform());
        if matches!(got, crate::unions::Union3::U1(_)) {
            let mut e = match got { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
            return e;
        };
        if matches!(got, crate::unions::Union3::U3(_)) {
            let mut got_1 = match &got { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
            crate::runtime_streams::not_ours_platform(handle);
        };
        crate::runtime::park_platform(&crate::runtime::this_parker_platform());
    }
    return crate::runtime_streams::checkout_in(handle);
}

pub fn checkout_out(mut handle: i64) -> crate::runtime_streams::OutEntry {
    loop {
        if !(true) {
            break;
        };
        let mut got: crate::unions::Union3<crate::runtime_streams::OutEntry, crate::runtime_streams::Busy, crate::runtime_streams::Unknown> = crate::runtime_streams::__module_use0_0().take_out(handle, crate::runtime::this_parker_platform());
        if matches!(got, crate::unions::Union3::U1(_)) {
            let mut e = match got { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
            return e;
        };
        if matches!(got, crate::unions::Union3::U3(_)) {
            let mut got_1 = match &got { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
            crate::runtime_streams::not_ours_platform(handle);
        };
        crate::runtime::park_platform(&crate::runtime::this_parker_platform());
    }
    return crate::runtime_streams::checkout_out(handle);
}

pub fn checkin_in(mut handle: i64, mut e: crate::runtime_streams::InEntry) {
    crate::runtime_streams::__module_use0_0().put_in(handle, e);
}

pub fn checkin_out(mut handle: i64, mut e: crate::runtime_streams::OutEntry) {
    crate::runtime_streams::__module_use0_0().put_out(handle, e);
}

pub fn close_in(mut handle: i64, mut e: crate::runtime_streams::InEntry) -> Option<crate::runtime_streams::Fault> {
    crate::runtime_streams::__module_use0_0().forget(handle);
    let mut __destructured_1: crate::runtime_streams::InEntry = e;
    let mut source: String = __destructured_1.source;
    let mut host: crate::runtime_streams::HostIn = __destructured_1.host;
    let mut position: i64 = __destructured_1.position;
    let mut ahead: crate::core_bytes::Bytes = __destructured_1.ahead;
    let mut failed: Option<crate::runtime_streams::Fault> = __destructured_1.failed;
    crate::runtime_streams::host_close_in_platform(host);
    return failed;
}

pub fn close_out(mut handle: i64, mut e: crate::runtime_streams::OutEntry) -> Option<crate::runtime_streams::Fault> {
    crate::runtime_streams::__module_use0_0().forget(handle);
    let mut __destructured_1: crate::runtime_streams::OutEntry = e;
    let mut source: String = __destructured_1.source;
    let mut host: crate::runtime_streams::HostOut = __destructured_1.host;
    let mut position: i64 = __destructured_1.position;
    let mut failed: Option<crate::runtime_streams::Fault> = __destructured_1.failed;
    let mut flushing: Option<String> = crate::runtime_streams::host_flush_platform(&mut host);
    let mut closing: Option<String> = crate::runtime_streams::host_close_out_platform(host);
    if failed.is_some() {
        let mut f = failed.as_ref().unwrap();
        return Some(f.clone());
    };
    if flushing.is_some() {
        let mut message = flushing.as_ref().unwrap();
        return Some(crate::runtime_streams::Fault { utf8: false, message: message.clone() });
    };
    if closing.is_some() {
        let mut message = closing.as_ref().unwrap();
        return Some(crate::runtime_streams::Fault { utf8: false, message: message.clone() });
    };
    return None;
}

pub fn record(e: &mut crate::runtime_streams::InEntry, mut message: String) -> crate::runtime_streams::Fault {
    let mut f: crate::runtime_streams::Fault = crate::runtime_streams::Fault { utf8: false, message: message.clone() };
    e.failed = Some((f).clone());
    return f;
}

pub fn take_ahead(e: &mut crate::runtime_streams::InEntry, mut n: i32) -> crate::core_bytes::Bytes {
    let mut all: i32 = crate::core_bytes::size_platform(&e.ahead);
    let mut front: crate::core_bytes::Bytes = {
        let mut __elv_1: Option<crate::core_bytes::Bytes> = crate::core_bytes::slice_platform(&e.ahead, 0i32, n);
        if __elv_1.is_none() {
            crate::core_bytes::bytes_of(vec![])
        } else {
            let mut __some_2 = __elv_1.unwrap();
            __some_2
        }
    };
    let mut rest: crate::core_bytes::Bytes = {
        let mut __elv_3: Option<crate::core_bytes::Bytes> = crate::core_bytes::slice_platform(&e.ahead, n, all);
        if __elv_3.is_none() {
            crate::core_bytes::bytes_of(vec![])
        } else {
            let mut __some_4 = __elv_3.unwrap();
            __some_4
        }
    };
    e.ahead = crate::core_bytes::mut_bytes(vec![rest.clone()]);
    e.position = i64::wrapping_add(e.position, ((n) as i64));
    return front;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub data: crate::core_bytes::Bytes,
    pub end: bool,
    pub fault: Option<crate::runtime_streams::Fault>,
}

impl crate::wire::__Wire for Read {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.data, out);
        crate::wire::__Wire::__enc(&self.end, out);
        crate::wire::__Wire::__enc(&self.fault, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            data: crate::wire::__Wire::__dec(r)?,
            end: crate::wire::__Wire::__dec(r)?,
            fault: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn read_line(e: &mut crate::runtime_streams::InEntry) -> crate::runtime_streams::Read {
    if e.failed.is_some() {
        let mut f = e.failed.as_ref().unwrap();
        return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: Some((f).clone()) };
    };
    loop {
        if !(true) {
            break;
        };
        let mut at: Option<i32> = crate::core_bytes::index_of_platform(&e.ahead, (((10i32) as i32) as u8));
        if at.is_some() {
            let mut i = at.unwrap();
            let mut line: crate::core_bytes::Bytes = crate::runtime_streams::take_ahead(&mut *e, i32::wrapping_add(i, 1i32));
            let mut n: i32 = i32::wrapping_sub(crate::core_bytes::size_platform(&line), 1i32);
            if ((n > 0i32) && (((({
                let mut __nn_1: Option<u8> = crate::core_bytes::get_platform(&line, i32::wrapping_sub(n, 1i32));
                if __nn_1.is_none() {
                    panic!("salvo: value is absent at runtime.streams:362:32");
                } else {
                    let mut __some_2 = __nn_1.unwrap();
                    __some_2
                }
            }) as i32)) == (13i32))) {
                n = i32::wrapping_sub(n, 1i32);
            };
            return crate::runtime_streams::Read { data: {
                let mut __elv_3: Option<crate::core_bytes::Bytes> = crate::core_bytes::slice_platform(&line, 0i32, n);
                if __elv_3.is_none() {
                    crate::core_bytes::bytes_of(vec![])
                } else {
                    let mut __some_4 = __elv_3.unwrap();
                    __some_4
                }
            }, end: false, fault: None };
        };
        let mut got: crate::runtime_streams::HostRead = crate::runtime_streams::host_read_platform(&mut e.host, 8192i32);
        if got.error.is_some() {
            let mut message = got.error.as_ref().unwrap();
            return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: Some(crate::runtime_streams::record(&mut *e, (message).clone())) };
        };
        if ((crate::core_bytes::size_platform(&got.data)) == (0i32)) {
            if ((crate::core_bytes::size_platform(&e.ahead)) == (0i32)) {
                return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: None };
            };
            let mut rest: crate::core_bytes::Bytes = { let __arg1 = crate::core_bytes::size_platform(&e.ahead); crate::runtime_streams::take_ahead(&mut *e, __arg1) };
            return crate::runtime_streams::Read { data: rest.clone(), end: false, fault: None };
        };
        crate::core_bytes::append_platform(&mut e.ahead, &got.data);
    }
    return crate::runtime_streams::read_line(&mut *e);
}

pub fn read_all(e: &mut crate::runtime_streams::InEntry) -> crate::runtime_streams::Read {
    if e.failed.is_some() {
        let mut f = e.failed.as_ref().unwrap();
        return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: Some((f).clone()) };
    };
    let mut out: crate::core_bytes::Bytes = { let __arg2 = vec![{ let __arg1 = crate::core_bytes::size_platform(&e.ahead); crate::runtime_streams::take_ahead(&mut *e, __arg1) }]; crate::core_bytes::mut_bytes(__arg2) };
    loop {
        if !(true) {
            break;
        };
        let mut got: crate::runtime_streams::HostRead = crate::runtime_streams::host_read_platform(&mut e.host, 65536i32);
        if got.error.is_some() {
            let mut message = got.error.as_ref().unwrap();
            return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: Some(crate::runtime_streams::record(&mut *e, (message).clone())) };
        };
        if ((crate::core_bytes::size_platform(&got.data)) == (0i32)) {
            return crate::runtime_streams::Read { data: (out).clone(), end: true, fault: None };
        };
        e.position = i64::wrapping_add(e.position, ((crate::core_bytes::size_platform(&got.data)) as i64));
        crate::core_bytes::append_platform(&mut out, &got.data);
    }
    return crate::runtime_streams::read_all(&mut *e);
}

pub fn read_up_to(e: &mut crate::runtime_streams::InEntry, mut max: i32) -> crate::runtime_streams::Read {
    if e.failed.is_some() {
        let mut f = e.failed.as_ref().unwrap();
        return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: Some((f).clone()) };
    };
    if (max <= 0i32) {
        return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: false, fault: None };
    };
    if (crate::core_bytes::size_platform(&e.ahead) > 0i32) {
        let mut n: i32 = max;
        if (crate::core_bytes::size_platform(&e.ahead) < n) {
            n = crate::core_bytes::size_platform(&e.ahead);
        };
        return crate::runtime_streams::Read { data: crate::runtime_streams::take_ahead(&mut *e, n), end: false, fault: None };
    };
    let mut got: crate::runtime_streams::HostRead = crate::runtime_streams::host_read_platform(&mut e.host, max);
    if got.error.is_some() {
        let mut message = got.error.as_ref().unwrap();
        return crate::runtime_streams::Read { data: crate::core_bytes::bytes_of(vec![]), end: true, fault: Some(crate::runtime_streams::record(&mut *e, (message).clone())) };
    };
    e.position = i64::wrapping_add(e.position, ((crate::core_bytes::size_platform(&got.data)) as i64));
    return crate::runtime_streams::Read { data: (got.data).clone(), end: ((crate::core_bytes::size_platform(&got.data)) == (0i32)), fault: None };
}

pub fn decode(e: &mut crate::runtime_streams::InEntry, data: &crate::core_bytes::Bytes) -> Option<String> {
    let mut text: Option<String> = crate::core_bytes::str_of_bytes_platform(data);
    if text.is_none() {
        e.failed = Some(crate::runtime_streams::Fault { utf8: true, message: String::from("") });
    };
    return text;
}

pub fn record_out(e: &mut crate::runtime_streams::OutEntry, mut message: String) {
    e.failed = Some(crate::runtime_streams::Fault { utf8: false, message: message.clone() });
}

pub fn write(e: &mut crate::runtime_streams::OutEntry, data: &crate::core_bytes::Bytes) -> i64 {
    if e.failed.is_some() {
        let mut earlier = e.failed.as_ref().unwrap();
        return 0i64;
    };
    let mut failed: Option<String> = crate::runtime_streams::host_write_platform(&mut e.host, data);
    if failed.is_some() {
        let mut message = failed.as_ref().unwrap();
        crate::runtime_streams::record_out(&mut *e, (message).clone());
        return 0i64;
    };
    e.position = i64::wrapping_add(e.position, ((crate::core_bytes::size_platform(data)) as i64));
    return ((crate::core_bytes::size_platform(data)) as i64);
}

pub fn flush(e: &mut crate::runtime_streams::OutEntry) -> Option<crate::runtime_streams::Fault> {
    let mut failed: Option<String> = crate::runtime_streams::host_flush_platform(&mut e.host);
    if failed.is_some() {
        let mut message = failed.as_ref().unwrap();
        e.failed = Some(crate::runtime_streams::Fault { utf8: false, message: (message).clone() });
    };
    let mut f: Option<crate::runtime_streams::Fault> = (e.failed).clone();
    e.failed = None;
    return f;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub data: crate::core_bytes::Bytes,
    pub end: bool,
    pub fault: Option<crate::runtime_streams::Fault>,
    pub source: String,
}

impl crate::wire::__Wire for Chunk {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.data, out);
        crate::wire::__Wire::__enc(&self.end, out);
        crate::wire::__Wire::__enc(&self.fault, out);
        crate::wire::__Wire::__enc(&self.source, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            data: crate::wire::__Wire::__dec(r)?,
            end: crate::wire::__Wire::__dec(r)?,
            fault: crate::wire::__Wire::__dec(r)?,
            source: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn receive(mut handle: i64, mut done: crate::scheduler::SalvoReply) {
    crate::runtime_streams::__module_use0_0().add_pending(crate::runtime_streams::Pending { handle: handle, done: done });
    crate::runtime::external_begin();
    crate::runtime_streams::start_reader(handle);
}

pub fn start_reader(mut handle: i64) {
    crate::runtime::start_thread_platform(std::boxed::Box::new({ let mut handle = handle.clone(); move || {
        crate::runtime_streams::read_and_answer(handle);
    } }));
}

pub fn read_and_answer(mut handle: i64) {
    let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
    let mut got: crate::runtime_streams::Read = crate::runtime_streams::read_up_to(&mut e, 65536i32);
    let mut source: String = (e.source).clone();
    if (got.end || !(got.fault.is_none())) {
        let mut _closed: Option<crate::runtime_streams::Fault> = crate::runtime_streams::close_in(handle, e);
    } else {
        crate::runtime_streams::checkin_in(handle, e);
    };
    let mut p: Option<crate::runtime_streams::Pending> = crate::runtime_streams::__module_use0_0().take_pending(handle);
    if p.is_some() {
        let mut found = p.unwrap();
        let mut __destructured_1: crate::runtime_streams::Pending = found;
        let mut _h: i64 = __destructured_1.handle;
        let mut done: crate::scheduler::SalvoReply = __destructured_1.done;
        crate::scheduler::salvo_reply_wire::<crate::runtime_streams::Chunk>(done, crate::runtime_streams::Chunk { data: got.data.clone(), end: got.end, fault: got.fault.clone(), source: source.clone() });
    };
    crate::runtime::external_end();
}

pub fn take_for_host(mut handle: i64) -> crate::runtime_streams::InEntry {
    let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
    crate::runtime_streams::__module_use0_0().forget(handle);
    return e;
}
