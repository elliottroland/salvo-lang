use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::runtime_streams::*;
use crate::stream::*;
use crate::unions::*;

pub trait __Stateless_RawStreams: Send + Sync {
    fn raw_read_line(&self, handle: i64) -> Option<String>;
    fn raw_read_all(&self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Bytes, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Bytes, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&self, handle: i64) -> i64;
    fn raw_close_read(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_write(&self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&self, handle: i64, data: &Bytes) -> i64;
    fn raw_write_position(&self, handle: i64) -> i64;
    fn raw_flush(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_close_write(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&self, data: Bytes) -> i64;
}

pub trait __Stateful_RawStreams: Send {
    fn raw_read_line(&mut self, handle: i64) -> Option<String>;
    fn raw_read_all(&mut self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> Union2<Bytes, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut Bytes, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&mut self, handle: i64) -> i64;
    fn raw_close_read(&mut self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_write(&mut self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&mut self, handle: i64, data: &Bytes) -> i64;
    fn raw_write_position(&mut self, handle: i64) -> i64;
    fn raw_flush(&mut self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_close_write(&mut self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_receive(&mut self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&mut self, data: Bytes) -> i64;
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
    pub fn raw_read_all(&self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_all(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_all(handle),
        }
    }
    pub fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Bytes, Union2<InvalidUtf8, StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_bytes(handle, max),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_bytes(handle, max),
        }
    }
    pub fn raw_read_to_bytes(&self, handle: i64, buf: &mut Bytes, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_to_bytes(handle, buf, max),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_to_bytes(handle, buf, max),
        }
    }
    pub fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>> {
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
    pub fn raw_close_read(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
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
    pub fn raw_write_bytes(&self, handle: i64, data: &Bytes) -> i64 {
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
    pub fn raw_flush(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_flush(handle),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_flush(handle),
        }
    }
    pub fn raw_close_write(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
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
    pub fn raw_from_bytes(&self, data: Bytes) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_from_bytes(data),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_from_bytes(data),
        }
    }
}

pub fn host_received(reply: crate::scheduler::SalvoReply, handle: i64, got: Union3<Bytes, End, Union2<InvalidUtf8, StreamFailed>>) {
    match got {
        Union3::U1(_) => {
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(Packet { bytes: got.u1().clone(), stream: InStream { handle: handle } }))));
        }
        Union3::U2(_) => {
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(got.u2().clone())));
        }
        Union3::U3(_) => {
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U3(err(checked(got.u3().clone())))));
        }
    }
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
        return next_line(handle);
    }

    fn raw_read_all(&self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_in(handle.clone());
        let mut r = read_all(&mut e);
        let mut source = e.source.clone();
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap().clone();
            checkin_in(handle, e);
            return Union2::<String, Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        let mut text = decode(&mut e, &r.data);
        checkin_in(handle, e);
        if text.is_some() {
            let mut t = text.as_ref().unwrap().clone();
            return Union2::<String, Union2<InvalidUtf8, StreamFailed>>::U1(ok(t));
        }
        return Union2::<String, Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &(crate::runtime_streams::Fault { utf8: true, message: "".to_string() }))));
    }

    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Bytes, Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_in(handle.clone());
        let mut r = read_up_to(&mut e, max);
        let mut source = e.source.clone();
        checkin_in(handle, e);
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap().clone();
            return Union2::<Bytes, Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        return Union2::<Bytes, Union2<InvalidUtf8, StreamFailed>>::U1(ok(r.data.clone()));
    }

    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Bytes, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_in(handle.clone());
        let mut r = read_up_to(&mut e, max);
        let mut source = e.source.clone();
        checkin_in(handle, e);
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap().clone();
            return Union2::<i32, Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        append_platform(buf, &r.data);
        return Union2::<i32, Union2<InvalidUtf8, StreamFailed>>::U1(ok(size_platform(&r.data)));
    }

    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_in(handle.clone());
        let mut r = read_all(&mut e);
        let mut source = e.source.clone();
        if r.fault.is_some() {
            let mut f = r.fault.as_ref().unwrap().clone();
            checkin_in(handle, e);
            return Union2::<i64, Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        let mut count = ((size_platform(&r.data)) as i64);
        let mut text = decode(&mut e, &r.data);
        checkin_in(handle, e);
        if text.is_some() {
            let mut t = text.as_ref().unwrap().clone();
            buf.push_str(&t[..]);
            return Union2::<i64, Union2<InvalidUtf8, StreamFailed>>::U1(ok(count));
        }
        return Union2::<i64, Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &(crate::runtime_streams::Fault { utf8: true, message: "".to_string() }))));
    }

    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool {
        let mut line = next_line(handle);
        if line.is_some() {
            let mut t = line.as_ref().unwrap().clone();
            buf.push_str(&t[..]);
            return true;
        }
        return false;
    }

    fn raw_read_position(&self, handle: i64) -> i64 {
        let mut e = checkout_in(handle.clone());
        let mut at = e.position;
        checkin_in(handle, e);
        return at;
    }

    fn raw_close_read(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_in(handle.clone());
        let mut source = e.source.clone();
        let mut failed = close_in(handle, e);
        if failed.is_some() {
            let mut f = failed.as_ref().unwrap().clone();
            return Union2::<(), Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        return Union2::<(), Union2<InvalidUtf8, StreamFailed>>::U1(ok(()));
    }

    fn raw_write(&self, handle: i64, text: &String) -> i64 {
        let mut e = checkout_out(handle.clone());
        let mut n = write(&mut e, &(to_bytes_platform(text)));
        checkin_out(handle, e);
        return n;
    }

    fn raw_write_bytes(&self, handle: i64, data: &Bytes) -> i64 {
        let mut e = checkout_out(handle.clone());
        let mut n = write(&mut e, data);
        checkin_out(handle, e);
        return n;
    }

    fn raw_write_position(&self, handle: i64) -> i64 {
        let mut e = checkout_out(handle.clone());
        let mut at = e.position;
        checkin_out(handle, e);
        return at;
    }

    fn raw_flush(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_out(handle.clone());
        let mut source = e.source.clone();
        let mut failed = flush__2(&mut e);
        checkin_out(handle, e);
        if failed.is_some() {
            let mut f = failed.as_ref().unwrap().clone();
            return Union2::<(), Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        return Union2::<(), Union2<InvalidUtf8, StreamFailed>>::U1(ok(()));
    }

    fn raw_close_write(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        let mut e = checkout_out(handle.clone());
        let mut source = e.source.clone();
        let mut failed = close_out(handle, e);
        if failed.is_some() {
            let mut f = failed.as_ref().unwrap().clone();
            return Union2::<(), Union2<InvalidUtf8, StreamFailed>>::U2(err(kind(source, &f)));
        }
        return Union2::<(), Union2<InvalidUtf8, StreamFailed>>::U1(ok(()));
    }

    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply) {
        receive(handle, ({ let __c0 = reply; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), std::boxed::Box::new(move |__v| chunk_received(__c0, *__v.downcast::<Chunk>().expect("the awaited answer"))), (|__b: &[u8]| crate::wire::salvo_decode::<Chunk>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))) }));
    }

    fn raw_from_bytes(&self, data: Bytes) -> i64 {
        return register_bytes(data);
    }
}

pub fn chunk_received(reply: crate::scheduler::SalvoReply, c: Chunk) {
    if c.fault.is_some() {
        let mut f = c.fault.as_ref().unwrap().clone();
        crate::scheduler::salvo_reply_wire::<Union3<Bytes, End, Union2<InvalidUtf8, StreamFailed>>>(reply, Union3::<Bytes, End, Union2<InvalidUtf8, StreamFailed>>::U3(err(kind(c.source.clone(), &f))));
    } else if c.end {
        crate::scheduler::salvo_reply_wire::<Union3<Bytes, End, Union2<InvalidUtf8, StreamFailed>>>(reply, Union3::<Bytes, End, Union2<InvalidUtf8, StreamFailed>>::U2(End {  }));
    } else {
        crate::scheduler::salvo_reply_wire::<Union3<Bytes, End, Union2<InvalidUtf8, StreamFailed>>>(reply, Union3::<Bytes, End, Union2<InvalidUtf8, StreamFailed>>::U1(ok(c.data.clone())));
    }
}

pub fn next_line(handle: i64) -> Option<String> {
    let mut e = checkout_in(handle.clone());
    let mut r = read_line(&mut e);
    let mut text: Option<String> = None;
    if !r.end && (r.fault.is_none()) {
        text = decode(&mut e, &r.data);
    }
    checkin_in(handle, e);
    return text;
}

pub fn kind(source: String, f: &crate::runtime_streams::Fault) -> Union2<InvalidUtf8, StreamFailed> {
    if f.utf8 {
        return Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source });
    }
    return Union2::<InvalidUtf8, StreamFailed>::U2(StreamFailed { source: source, message: f.message.clone() });
}

#[derive(Clone)]
pub struct DefaultStreams {
    __dep_RawStreams: crate::stream_host::RawStreams,
}

impl DefaultStreams {
    pub fn new(__dep_RawStreams: crate::stream_host::RawStreams) -> Self {
        Self {
            __dep_RawStreams,
        }
    }
}

impl crate::stream::__Stateless_Streams for DefaultStreams {

    fn read_line(&self, s: &InStream) -> Option<String> {
        return self.__dep_RawStreams.raw_read_line(s.handle);
    }

    fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_read_all(s.handle);
        match r {
            Union2::U1(_) => {
                return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_read_bytes(s.handle, max);
        match r {
            Union2::U1(_) => {
                return Union2::<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to(&self, s: &InStream, buf: &mut Bytes, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_read_to_bytes(s.handle, buf, max);
        match r {
            Union2::U1(_) => {
                return Union2::<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(*r.u1()));
            }
            Union2::U2(_) => {
                return Union2::<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_read_to_str(s.handle, buf);
        match r {
            Union2::U1(_) => {
                return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(*r.u1()));
            }
            Union2::U2(_) => {
                return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool {
        return self.__dep_RawStreams.raw_read_line_to_str(s.handle, buf);
    }

    fn position(&self, s: &InStream) -> i64 {
        return self.__dep_RawStreams.raw_read_position(s.handle);
    }

    fn close(&self, s: InStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_close_read(s.handle);
        drop(s);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn receive(&self, s: InStream, reply: crate::scheduler::SalvoReply) {
        let mut handle = s.handle;
        drop(s);
        self.__dep_RawStreams.raw_receive(handle.clone(), ({ let __c0 = reply; let __c1 = handle; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), std::boxed::Box::new(move |__v| host_received(__c0, __c1, *__v.downcast::<Union3<Bytes, End, Union2<InvalidUtf8, StreamFailed>>>().expect("the awaited answer"))), (|__b: &[u8]| crate::wire::salvo_decode::<Union3<Bytes, End, Union2<InvalidUtf8, StreamFailed>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))) }));
    }

    fn from_bytes(&self, data: Bytes) -> InStream {
        return InStream { handle: self.__dep_RawStreams.raw_from_bytes(data) };
    }

    fn write(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_RawStreams.raw_write(s.handle, text);
    }

    fn write_line(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_RawStreams.raw_write(s.handle, &(format!("{}\n", text.clone())));
    }

    fn write_bytes(&self, s: &OutStream, data: &Bytes) -> i64 {
        return self.__dep_RawStreams.raw_write_bytes(s.handle, data);
    }

    fn position__2(&self, s: &OutStream) -> i64 {
        return self.__dep_RawStreams.raw_write_position(s.handle);
    }

    fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_flush(s.handle);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_close_write(s.handle);
        drop(s);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }
}
