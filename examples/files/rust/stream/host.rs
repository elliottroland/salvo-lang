use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_result::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::stream::*;
use crate::unions::*;

pub trait __Stateless_RawStreams: Send + Sync {
    fn raw_read_line(&self, handle: i64) -> Option<String>;
    fn raw_read_all(&self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&self, handle: i64) -> i64;
    fn raw_close_read(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_write(&self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64;
    fn raw_write_position(&self, handle: i64) -> i64;
    fn raw_flush(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_close_write(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&self, data: Vec<u8>) -> i64;
}

pub trait __Stateful_RawStreams: Send {
    fn raw_read_line(&mut self, handle: i64) -> Option<String>;
    fn raw_read_all(&mut self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> Union2<Vec<u8>, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&mut self, handle: i64) -> i64;
    fn raw_close_read(&mut self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_write(&mut self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&mut self, handle: i64, data: &Vec<u8>) -> i64;
    fn raw_write_position(&mut self, handle: i64) -> i64;
    fn raw_flush(&mut self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_close_write(&mut self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_receive(&mut self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&mut self, data: Vec<u8>) -> i64;
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
    pub fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Union2<InvalidUtf8, StreamFailed>> {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_read_bytes(handle, max),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_read_bytes(handle, max),
        }
    }
    pub fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>> {
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
    pub fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64 {
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
    pub fn raw_from_bytes(&self, data: Vec<u8>) -> i64 {
        match &self.inner {
            __Inner_RawStreams::Shared(h) => h.raw_from_bytes(data),
            __Inner_RawStreams::Locked(h) => h.lock().unwrap().raw_from_bytes(data),
        }
    }
}

/// The adapter a `use` of a platform handler of `RawStreams` constructs [platform-abi].
pub struct __Platform_RawStreams<T>(pub T);

/// What a `threadsafe platform handler` of `RawStreams` implements [platform-abi].
pub trait RawStreamsPlatformSync: Send + Sync {
    fn raw_read_line(&self, handle: i64) -> Option<String>;
    fn raw_read_all(&self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>>;
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&self, handle: i64) -> i64;
    fn raw_close_read(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_write(&self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64;
    fn raw_write_position(&self, handle: i64) -> i64;
    fn raw_flush(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_close_write(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>>;
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply);
    fn raw_from_bytes(&self, data: Vec<u8>) -> i64;
}

impl<T: RawStreamsPlatformSync> __Stateless_RawStreams for __Platform_RawStreams<T> {
    fn raw_read_line(&self, handle: i64) -> Option<String> {
        self.0.raw_read_line(handle)
    }
    fn raw_read_all(&self, handle: i64) -> Union2<String, Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_read_all(handle)
    }
    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_read_bytes(handle, max)
    }
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_read_to_bytes(handle, buf, max)
    }
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_read_to_str(handle, buf)
    }
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool {
        self.0.raw_read_line_to_str(handle, buf)
    }
    fn raw_read_position(&self, handle: i64) -> i64 {
        self.0.raw_read_position(handle)
    }
    fn raw_close_read(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_close_read(handle)
    }
    fn raw_write(&self, handle: i64, text: &String) -> i64 {
        self.0.raw_write(handle, text)
    }
    fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64 {
        self.0.raw_write_bytes(handle, data)
    }
    fn raw_write_position(&self, handle: i64) -> i64 {
        self.0.raw_write_position(handle)
    }
    fn raw_flush(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_flush(handle)
    }
    fn raw_close_write(&self, handle: i64) -> Union2<(), Union2<InvalidUtf8, StreamFailed>> {
        self.0.raw_close_write(handle)
    }
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply) {
        self.0.raw_receive(handle, reply)
    }
    fn raw_from_bytes(&self, data: Vec<u8>) -> i64 {
        self.0.raw_from_bytes(data)
    }
}

pub fn host_received(reply: crate::scheduler::SalvoReply, handle: i64, got: Union3<Vec<u8>, End, Union2<InvalidUtf8, StreamFailed>>) {
    match got {
        Union3::U1(_) => {
            (reply).send(Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(Packet { bytes: got.u1().clone(), stream: InStream { handle: handle } }))));
        }
        Union3::U2(_) => {
            (reply).send(Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(got.u2().clone())));
        }
        Union3::U3(_) => {
            (reply).send(Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U3(err(checked(got.u3().clone())))));
        }
    }
}

pub type __Platform_HostRawStreams = crate::stream_host::__Platform_RawStreams<crate::platform_stream_host::HostRawStreams>;

impl __Platform_HostRawStreams {
    pub fn new() -> Self {
        crate::stream_host::__Platform_RawStreams(crate::platform_stream_host::HostRawStreams::new())
    }
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

    fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut r = self.__dep_RawStreams.raw_read_bytes(s.handle, max);
        match r {
            Union2::U1(_) => {
                return Union2::<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>> {
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
        self.__dep_RawStreams.raw_receive(handle.clone(), ({ let __c0 = reply; let __c1 = handle; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), Box::new(move |__v| host_received(__c0, __c1, *__v.downcast::<Union3<Vec<u8>, End, Union2<InvalidUtf8, StreamFailed>>>().expect("the awaited answer"))), (|__b: &[u8]| crate::wire::salvo_decode::<Union3<Vec<u8>, End, Union2<InvalidUtf8, StreamFailed>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg))) }));
    }

    fn from_bytes(&self, data: Vec<u8>) -> InStream {
        return InStream { handle: self.__dep_RawStreams.raw_from_bytes(data) };
    }

    fn write(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_RawStreams.raw_write(s.handle, text);
    }

    fn write_line(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_RawStreams.raw_write(s.handle, &(format!("{}\n", text.clone())));
    }

    fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64 {
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
