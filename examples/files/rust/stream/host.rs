use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::runtime_streams::Chunk;
use crate::stream::End;
use crate::runtime_streams::Fault;
use crate::runtime_streams::InEntry;
use crate::stream::InStream;
use crate::stream::InvalidUtf8;
use crate::runtime_streams::OutEntry;
use crate::stream::OutStream;
use crate::stream::Packet;
use crate::runtime_streams::Read;
use crate::stream::StreamFailed;
use crate::core_checked::checked;
use crate::runtime_streams::checkin_in;
use crate::runtime_streams::checkin_out;
use crate::runtime_streams::checkout_in;
use crate::runtime_streams::checkout_out;
use crate::runtime_streams::close_in;
use crate::runtime_streams::close_out;
use crate::runtime_streams::decode;
use crate::core_result::err;
use crate::runtime_streams::flush;
use crate::core_result::ok;
use crate::runtime_streams::read_all;
use crate::runtime_streams::read_line;
use crate::runtime_streams::read_up_to;
use crate::runtime_streams::receive;
use crate::runtime_streams::register_bytes;
use crate::core_bytes::size_platform;
use crate::core_bytes::to_bytes_platform;
use crate::runtime_streams::write;


pub trait __Stateless_RawStreams: Send + Sync {
    fn raw_read_line(&self, handle: i64) -> Option<String>;
    fn raw_read_all(&self, handle: i64) -> crate::unions::Union2<String, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_bytes(&self, handle: i64, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> crate::unions::Union2<i64, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&self, handle: i64) -> i64;
    fn raw_close_read(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_write(&self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&self, handle: i64, data: &crate::core_bytes::Bytes) -> i64;
    fn raw_write_position(&self, handle: i64) -> i64;
    fn raw_flush(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_close_write(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&self, data: crate::core_bytes::Bytes) -> i64;
}

pub trait __Stateful_RawStreams: Send {
    fn raw_read_line(&mut self, handle: i64) -> Option<String>;
    fn raw_read_all(&mut self, handle: i64) -> crate::unions::Union2<String, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> crate::unions::Union2<i64, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&mut self, handle: i64) -> i64;
    fn raw_close_read(&mut self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_write(&mut self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&mut self, handle: i64, data: &crate::core_bytes::Bytes) -> i64;
    fn raw_write_position(&mut self, handle: i64) -> i64;
    fn raw_flush(&mut self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_close_write(&mut self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>;
    fn raw_receive(&mut self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&mut self, data: crate::core_bytes::Bytes) -> i64;
}

pub struct RawStreams {
    inner: __Inner_RawStreams,
}

pub enum __Inner_RawStreams {
    Shared(std::sync::Arc<dyn __Stateless_RawStreams>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_RawStreams>>),
}

impl Clone for RawStreams {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_RawStreams::Shared(h) => __Inner_RawStreams::Shared(h.clone()),
            __Inner_RawStreams::Locked(h) => __Inner_RawStreams::Locked(h.clone()),
        } }
    }
}

impl RawStreams {
    pub fn shared<__H: __Stateless_RawStreams + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RawStreams::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_RawStreams>) -> Self {
        Self { inner: __Inner_RawStreams::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_RawStreams + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RawStreams::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_RawStreams>>) -> Self {
        Self { inner: __Inner_RawStreams::Locked(inner) }
    }
    pub fn raw_read_line(&self, handle: i64) -> Option<String> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_line(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_line(handle),
        }
    }
    pub fn raw_read_all(&self, handle: i64) -> crate::unions::Union2<String, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_all(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_all(handle),
        }
    }
    pub fn raw_read_bytes(&self, handle: i64, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_bytes(handle, max),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_bytes(handle, max),
        }
    }
    pub fn raw_read_to_bytes(&self, handle: i64, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_to_bytes(handle, buf, max),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_to_bytes(handle, buf, max),
        }
    }
    pub fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> crate::unions::Union2<i64, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_to_str(handle, buf),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_to_str(handle, buf),
        }
    }
    pub fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_line_to_str(handle, buf),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_line_to_str(handle, buf),
        }
    }
    pub fn raw_read_position(&self, handle: i64) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_position(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_position(handle),
        }
    }
    pub fn raw_close_read(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_close_read(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_close_read(handle),
        }
    }
    pub fn raw_write(&self, handle: i64, text: &String) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_write(handle, text),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_write(handle, text),
        }
    }
    pub fn raw_write_bytes(&self, handle: i64, data: &crate::core_bytes::Bytes) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_write_bytes(handle, data),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_write_bytes(handle, data),
        }
    }
    pub fn raw_write_position(&self, handle: i64) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_write_position(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_write_position(handle),
        }
    }
    pub fn raw_flush(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_flush(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_flush(handle),
        }
    }
    pub fn raw_close_write(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_close_write(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_close_write(handle),
        }
    }
    pub fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_receive(handle, reply),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_receive(handle, reply),
        }
    }
    pub fn raw_from_bytes(&self, data: crate::core_bytes::Bytes) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_from_bytes(data),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_from_bytes(data),
        }
    }
}

pub fn host_received(mut reply: crate::scheduler::SalvoReply, mut handle: i64, mut got: crate::unions::Union3<crate::core_bytes::Bytes, crate::stream::End, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>) {
    if matches!(got, crate::unions::Union3::U1(_)) {
        let mut got_1 = match &got { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        (reply).send(std::boxed::Box::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>::new(crate::unions::Union3::U1(crate::core_result::ok::<crate::stream::Packet>(crate::stream::Packet { bytes: got_1.clone(), stream: crate::stream::InStream { handle: handle } }))));
    } else if matches!(got, crate::unions::Union3::U2(_)) {
        let mut got_2 = match &got { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        (reply).send(std::boxed::Box::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>::new(crate::unions::Union3::U2(got_2.clone())));
    } else {
        let mut got_3 = match got { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        (reply).send(std::boxed::Box::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>::new(crate::unions::Union3::U3(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(got_3)))));
    };
}

#[derive(Clone)]
pub struct HostRawStreams {
}

impl HostRawStreams {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl crate::stream_host::__Stateless_RawStreams for HostRawStreams {
    fn raw_read_line(&self, handle: i64) -> Option<String> {
        return crate::stream_host::next_line(handle);
    }
    fn raw_read_all(&self, handle: i64) -> crate::unions::Union2<String, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
        let mut r: crate::runtime_streams::Read = crate::runtime_streams::read_all(&mut e);
        let mut source: String = (e.source).clone();
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap();
            crate::runtime_streams::checkin_in(handle, e);
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        let mut text: Option<String> = crate::runtime_streams::decode(&mut e, &r.data);
        crate::runtime_streams::checkin_in(handle, e);
        if text.is_some() {
            let mut t = text.unwrap();
            return crate::unions::Union2::U1(crate::core_result::ok::<String>(t));
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, &crate::runtime_streams::Fault { utf8: true, message: String::from("") })));
    }
    fn raw_read_bytes(&self, handle: i64, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
        let mut r: crate::runtime_streams::Read = crate::runtime_streams::read_up_to(&mut e, max);
        let mut source: String = (e.source).clone();
        crate::runtime_streams::checkin_in(handle, e);
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap();
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        return crate::unions::Union2::U1(crate::core_result::ok::<crate::core_bytes::Bytes>(r.data.clone()));
    }
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
        let mut r: crate::runtime_streams::Read = crate::runtime_streams::read_up_to(&mut e, max);
        let mut source: String = (e.source).clone();
        crate::runtime_streams::checkin_in(handle, e);
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap();
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        crate::core_bytes::append_platform(&mut *buf, &r.data);
        return crate::unions::Union2::U1(crate::core_result::ok::<i32>(crate::core_bytes::size_platform(&r.data)));
    }
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> crate::unions::Union2<i64, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
        let mut r: crate::runtime_streams::Read = crate::runtime_streams::read_all(&mut e);
        let mut source: String = (e.source).clone();
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap();
            crate::runtime_streams::checkin_in(handle, e);
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        let mut count: i64 = ((crate::core_bytes::size_platform(&r.data)) as i64);
        let mut text: Option<String> = crate::runtime_streams::decode(&mut e, &r.data);
        crate::runtime_streams::checkin_in(handle, e);
        if text.is_some() {
            let mut t = text.as_ref().unwrap();
            crate::core_string::append_platform(&mut *buf, t);
            return crate::unions::Union2::U1(crate::core_result::ok::<i64>(count));
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, &crate::runtime_streams::Fault { utf8: true, message: String::from("") })));
    }
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool {
        let mut line: Option<String> = crate::stream_host::next_line(handle);
        if line.is_some() {
            let mut t = line.as_ref().unwrap();
            crate::core_string::append_platform(&mut *buf, t);
            return true;
        };
        return false;
    }
    fn raw_read_position(&self, handle: i64) -> i64 {
        let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
        let mut at: i64 = e.position;
        crate::runtime_streams::checkin_in(handle, e);
        return at;
    }
    fn raw_close_read(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
        let mut source: String = (e.source).clone();
        let mut failed: Option<crate::runtime_streams::Fault> = crate::runtime_streams::close_in(handle, e);
        if failed.is_some() {
            let mut f = failed.as_ref().unwrap();
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn raw_write(&self, handle: i64, text: &String) -> i64 {
        let mut e: crate::runtime_streams::OutEntry = crate::runtime_streams::checkout_out(handle);
        let mut n: i64 = crate::runtime_streams::write(&mut e, &crate::core_bytes::to_bytes_platform(text));
        crate::runtime_streams::checkin_out(handle, e);
        return n;
    }
    fn raw_write_bytes(&self, handle: i64, data: &crate::core_bytes::Bytes) -> i64 {
        let mut e: crate::runtime_streams::OutEntry = crate::runtime_streams::checkout_out(handle);
        let mut n: i64 = crate::runtime_streams::write(&mut e, data);
        crate::runtime_streams::checkin_out(handle, e);
        return n;
    }
    fn raw_write_position(&self, handle: i64) -> i64 {
        let mut e: crate::runtime_streams::OutEntry = crate::runtime_streams::checkout_out(handle);
        let mut at: i64 = e.position;
        crate::runtime_streams::checkin_out(handle, e);
        return at;
    }
    fn raw_flush(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::OutEntry = crate::runtime_streams::checkout_out(handle);
        let mut source: String = (e.source).clone();
        let mut failed: Option<crate::runtime_streams::Fault> = crate::runtime_streams::flush(&mut e);
        crate::runtime_streams::checkin_out(handle, e);
        if failed.is_some() {
            let mut f = failed.as_ref().unwrap();
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn raw_close_write(&self, handle: i64) -> crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> {
        let mut e: crate::runtime_streams::OutEntry = crate::runtime_streams::checkout_out(handle);
        let mut source: String = (e.source).clone();
        let mut failed: Option<crate::runtime_streams::Fault> = crate::runtime_streams::close_out(handle, e);
        if failed.is_some() {
            let mut f = failed.as_ref().unwrap();
            return crate::unions::Union2::U2(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind(source, f)));
        };
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply) {
        crate::runtime_streams::receive(handle, ({ let __c0 = reply; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), std::boxed::Box::new(move |__v| crate::stream_host::chunk_received(__c0, *__v.downcast::<crate::runtime_streams::Chunk>().expect("the awaited answer"))), (|__b: &[u8]| crate::wire::salvo_decode::<crate::runtime_streams::Chunk>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))) }));
    }
    fn raw_from_bytes(&self, data: crate::core_bytes::Bytes) -> i64 {
        return crate::runtime_streams::register_bytes(data);
    }
}

pub fn chunk_received(mut reply: crate::scheduler::SalvoReply, mut c: crate::runtime_streams::Chunk) {
    if c.fault.is_some() {
        let mut f = c.fault.as_ref().unwrap();
        crate::scheduler::salvo_reply_wire::<crate::unions::Union3<crate::core_bytes::Bytes, crate::stream::End, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(reply, crate::unions::Union3::U3(crate::core_result::err::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::stream_host::kind((c.source).clone(), f))));
    } else if c.end {
        crate::scheduler::salvo_reply_wire::<crate::unions::Union3<crate::core_bytes::Bytes, crate::stream::End, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(reply, crate::unions::Union3::U2(crate::stream::End {}));
    } else {
        crate::scheduler::salvo_reply_wire::<crate::unions::Union3<crate::core_bytes::Bytes, crate::stream::End, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(reply, crate::unions::Union3::U1(crate::core_result::ok::<crate::core_bytes::Bytes>((c.data).clone())));
    };
}

pub fn next_line(mut handle: i64) -> Option<String> {
    let mut e: crate::runtime_streams::InEntry = crate::runtime_streams::checkout_in(handle);
    let mut r: crate::runtime_streams::Read = crate::runtime_streams::read_line(&mut e);
    let mut text: Option<String> = None;
    if (!(r.end) && r.fault.is_none()) {
        text = crate::runtime_streams::decode(&mut e, &r.data);
    };
    crate::runtime_streams::checkin_in(handle, e);
    return text;
}

pub fn kind(mut source: String, f: &crate::runtime_streams::Fault) -> crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed> {
    if f.utf8 {
        return crate::unions::Union2::U1(crate::stream::InvalidUtf8 { source: source.clone() });
    };
    return crate::unions::Union2::U2(crate::stream::StreamFailed { source: source.clone(), message: (f.message).clone() });
}

#[derive(Clone)]
pub struct DefaultStreams {
    __dep0: crate::stream_host::RawStreams,
}

impl DefaultStreams {
    pub fn new(__dep0: crate::stream_host::RawStreams) -> Self {
        Self {
            __dep0
        }
    }
}

impl crate::stream::__Stateless_Streams for DefaultStreams {
    fn read_line(&self, s: &crate::stream::InStream) -> Option<String> {
        return self.__dep0.raw_read_line(s.handle);
    }
    fn read_all(&self, s: &crate::stream::InStream) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<String, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_read_all(s.handle);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<String>(r_1));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn read_bytes(&self, s: &crate::stream::InStream, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<crate::core_bytes::Bytes, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_read_bytes(s.handle, max);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<crate::core_bytes::Bytes>(r_1));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn read_to__InStream_Bytes_Int(&self, s: &crate::stream::InStream, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<i32, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_read_to_bytes(s.handle, &mut *buf, max);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<i32>(r_1));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn read_to__InStream_Str(&self, s: &crate::stream::InStream, buf: &mut String) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<i64, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_read_to_str(s.handle, &mut *buf);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<i64>(r_1));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn read_line_to(&self, s: &crate::stream::InStream, buf: &mut String) -> bool {
        return self.__dep0.raw_read_line_to_str(s.handle, &mut *buf);
    }
    fn position__InStream(&self, s: &crate::stream::InStream) -> i64 {
        return self.__dep0.raw_read_position(s.handle);
    }
    fn close__InStream(&self, s: crate::stream::InStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_close_read(s.handle);
        std::mem::drop(s);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn write(&self, s: &crate::stream::OutStream, text: &String) -> i64 {
        return self.__dep0.raw_write(s.handle, text);
    }
    fn write_line(&self, s: &crate::stream::OutStream, text: &String) -> i64 {
        return self.__dep0.raw_write(s.handle, &format!("{}\n", text));
    }
    fn write_bytes(&self, s: &crate::stream::OutStream, data: &crate::core_bytes::Bytes) -> i64 {
        return self.__dep0.raw_write_bytes(s.handle, data);
    }
    fn position__OutStream(&self, s: &crate::stream::OutStream) -> i64 {
        return self.__dep0.raw_write_position(s.handle);
    }
    fn flush(&self, s: &crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_flush(s.handle);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn close__OutStream(&self, s: crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut r: crate::unions::Union2<(), crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>> = self.__dep0.raw_close_write(s.handle);
        std::mem::drop(s);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(r_2)));
        };
    }
    fn receive(&self, s: crate::stream::InStream, reply: crate::scheduler::SalvoReply) {
        let mut handle: i64 = s.handle;
        std::mem::drop(s);
        self.__dep0.raw_receive(handle, ({ let __c0 = reply; let __c1 = handle; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), std::boxed::Box::new(move |__v| crate::stream_host::host_received(__c0, __c1, *__v.downcast::<crate::unions::Union3<crate::core_bytes::Bytes, crate::stream::End, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>().expect("the awaited answer"))), (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union3<crate::core_bytes::Bytes, crate::stream::End, crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))) }));
    }
    fn from_bytes(&self, data: crate::core_bytes::Bytes) -> crate::stream::InStream {
        return crate::stream::InStream { handle: self.__dep0.raw_from_bytes(data) };
    }
}
