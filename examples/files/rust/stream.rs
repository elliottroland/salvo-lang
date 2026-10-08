use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::core_iterator::Finished;
use crate::core_bytes::add_platform;
use crate::core_checked::checked;
use crate::core_bytes::clear_platform;
use crate::core_iterator::emitted;
use crate::core_result::err;
use crate::core_iterator::finished;
use crate::core_bytes::get_platform;
use crate::core_checked::ignore;
use crate::core_bytes::mut_bytes;
use crate::core_result::ok;
use crate::core_bytes::size_platform;


pub fn fresh_handle() -> i64 {
    return crate::runtime_streams::fresh_handle();
}

pub type StreamError = crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>;

/// Factories for the host: one per arm of the union [platform-factory].
impl StreamError {
    pub fn invalid_utf8(value: crate::stream::InvalidUtf8) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn stream_failed(value: crate::stream::StreamFailed) -> Self {
        crate::unions::Union2::U2(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidUtf8 {
    pub source: String,
}

impl crate::wire::__Wire for InvalidUtf8 {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.source, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            source: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamFailed {
    pub source: String,
    pub message: String,
}

impl crate::wire::__Wire for StreamFailed {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.source, out);
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            source: crate::wire::__Wire::__dec(r)?,
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn to_str(kind: &crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>) -> String {
    return if matches!(kind, crate::unions::Union2::U1(_)) {
        let mut kind_1 = match &kind { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return format!("not valid UTF-8: {}", kind_1.source);
    } else {
        let mut kind_2 = match &kind { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return format!("stream failed: {}: {}", kind_2.source, kind_2.message);
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct InStream {
    pub handle: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutStream {
    pub handle: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub bytes: crate::core_bytes::Bytes,
    pub stream: crate::stream::InStream,
}

#[derive(Clone, Debug, PartialEq)]
pub struct End {
}

impl crate::wire::__Wire for End {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub fn close__Packet(streams: &crate::stream::Streams, mut p: crate::stream::Packet) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut __destructured_1: crate::stream::Packet = p;
    let mut bytes: crate::core_bytes::Bytes = __destructured_1.bytes;
    let mut stream: crate::stream::InStream = __destructured_1.stream;
    return streams.close__InStream(stream);
}

pub trait __Stateless_Streams: Send + Sync {
    fn read_line(&self, s: &crate::stream::InStream) -> Option<String>;
    fn read_all(&self, s: &crate::stream::InStream) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_bytes(&self, s: &crate::stream::InStream, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_to__InStream_Bytes_Int(&self, s: &crate::stream::InStream, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_to__InStream_Str(&self, s: &crate::stream::InStream, buf: &mut String) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_line_to(&self, s: &crate::stream::InStream, buf: &mut String) -> bool;
    fn position__InStream(&self, s: &crate::stream::InStream) -> i64;
    fn close__InStream(&self, s: crate::stream::InStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn write(&self, s: &crate::stream::OutStream, text: &String) -> i64;
    fn write_line(&self, s: &crate::stream::OutStream, text: &String) -> i64;
    fn write_bytes(&self, s: &crate::stream::OutStream, data: &crate::core_bytes::Bytes) -> i64;
    fn position__OutStream(&self, s: &crate::stream::OutStream) -> i64;
    fn flush(&self, s: &crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn close__OutStream(&self, s: crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn receive(&self, s: crate::stream::InStream, reply: crate::scheduler::SalvoReply);
    fn from_bytes(&self, data: crate::core_bytes::Bytes) -> crate::stream::InStream;
}

pub trait __Stateful_Streams: Send {
    fn read_line(&mut self, s: &crate::stream::InStream) -> Option<String>;
    fn read_all(&mut self, s: &crate::stream::InStream) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_bytes(&mut self, s: &crate::stream::InStream, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_to__InStream_Bytes_Int(&mut self, s: &crate::stream::InStream, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_to__InStream_Str(&mut self, s: &crate::stream::InStream, buf: &mut String) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn read_line_to(&mut self, s: &crate::stream::InStream, buf: &mut String) -> bool;
    fn position__InStream(&mut self, s: &crate::stream::InStream) -> i64;
    fn close__InStream(&mut self, s: crate::stream::InStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn write(&mut self, s: &crate::stream::OutStream, text: &String) -> i64;
    fn write_line(&mut self, s: &crate::stream::OutStream, text: &String) -> i64;
    fn write_bytes(&mut self, s: &crate::stream::OutStream, data: &crate::core_bytes::Bytes) -> i64;
    fn position__OutStream(&mut self, s: &crate::stream::OutStream) -> i64;
    fn flush(&mut self, s: &crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn close__OutStream(&mut self, s: crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>;
    fn receive(&mut self, s: crate::stream::InStream, reply: crate::scheduler::SalvoReply);
    fn from_bytes(&mut self, data: crate::core_bytes::Bytes) -> crate::stream::InStream;
}

pub struct Streams {
    inner: __Inner_Streams,
}

pub enum __Inner_Streams {
    Shared(std::sync::Arc<dyn __Stateless_Streams>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Streams>>),
}

impl Clone for Streams {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Streams::Shared(h) => __Inner_Streams::Shared(h.clone()),
            __Inner_Streams::Locked(h) => __Inner_Streams::Locked(h.clone()),
        } }
    }
}

impl Streams {
    pub fn shared<__H: __Stateless_Streams + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Streams::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Streams>) -> Self {
        Self { inner: __Inner_Streams::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Streams + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Streams::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Streams>>) -> Self {
        Self { inner: __Inner_Streams::Locked(inner) }
    }
    pub fn read_line(&self, s: &crate::stream::InStream) -> Option<String> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_line(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_line(s),
        }
    }
    pub fn read_all(&self, s: &crate::stream::InStream) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_all(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_all(s),
        }
    }
    pub fn read_bytes(&self, s: &crate::stream::InStream, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_bytes(s, max),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_bytes(s, max),
        }
    }
    pub fn read_to__InStream_Bytes_Int(&self, s: &crate::stream::InStream, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_to__InStream_Bytes_Int(s, buf, max),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_to__InStream_Bytes_Int(s, buf, max),
        }
    }
    pub fn read_to__InStream_Str(&self, s: &crate::stream::InStream, buf: &mut String) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_to__InStream_Str(s, buf),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_to__InStream_Str(s, buf),
        }
    }
    pub fn read_line_to(&self, s: &crate::stream::InStream, buf: &mut String) -> bool {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_line_to(s, buf),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_line_to(s, buf),
        }
    }
    pub fn position__InStream(&self, s: &crate::stream::InStream) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.position__InStream(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().position__InStream(s),
        }
    }
    pub fn close__InStream(&self, s: crate::stream::InStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.close__InStream(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().close__InStream(s),
        }
    }
    pub fn write(&self, s: &crate::stream::OutStream, text: &String) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.write(s, text),
            __Inner_Streams::Locked(h) => h.lock().unwrap().write(s, text),
        }
    }
    pub fn write_line(&self, s: &crate::stream::OutStream, text: &String) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.write_line(s, text),
            __Inner_Streams::Locked(h) => h.lock().unwrap().write_line(s, text),
        }
    }
    pub fn write_bytes(&self, s: &crate::stream::OutStream, data: &crate::core_bytes::Bytes) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.write_bytes(s, data),
            __Inner_Streams::Locked(h) => h.lock().unwrap().write_bytes(s, data),
        }
    }
    pub fn position__OutStream(&self, s: &crate::stream::OutStream) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.position__OutStream(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().position__OutStream(s),
        }
    }
    pub fn flush(&self, s: &crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.flush(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().flush(s),
        }
    }
    pub fn close__OutStream(&self, s: crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.close__OutStream(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().close__OutStream(s),
        }
    }
    pub fn receive(&self, s: crate::stream::InStream, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.receive(s, reply),
            __Inner_Streams::Locked(h) => h.lock().unwrap().receive(s, reply),
        }
    }
    pub fn from_bytes(&self, data: crate::core_bytes::Bytes) -> crate::stream::InStream {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.from_bytes(data),
            __Inner_Streams::Locked(h) => h.lock().unwrap().from_bytes(data),
        }
    }
}

pub fn pipe(streams: &crate::stream::Streams, mut from: crate::stream::InStream, mut to: crate::stream::OutStream, mut done: crate::scheduler::SalvoReply) {
    streams.receive(from, ({ let __e0 = streams.clone(); let __c0 = to; let __c1 = done; let __c2 = 0i64; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), std::boxed::Box::new(move |__v| crate::stream::pipe_step(&__e0, __c0, __c1, __c2, *__v.downcast::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>().expect("the awaited answer"))), (|_| None)) }));
}

pub fn pipe_step(streams: &crate::stream::Streams, mut to: crate::stream::OutStream, mut done: crate::scheduler::SalvoReply, mut moved: i64, mut got: crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>) {
    if matches!(got, crate::unions::Union3::U1(_)) {
        let mut got_1 = match got { crate::unions::Union3::U1(__v) => __v, _ => unreachable!() };
        let mut __destructured_2: crate::stream::Packet = got_1;
        let mut bytes: crate::core_bytes::Bytes = __destructured_2.bytes;
        let mut stream: crate::stream::InStream = __destructured_2.stream;
        let mut written: i64 = streams.write_bytes(&to, &bytes);
        streams.receive(stream, ({ let __e0 = streams.clone(); let __c0 = to; let __c1 = done; let __c2 = i64::wrapping_add(moved, written); crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), std::boxed::Box::new(move |__v| crate::stream::pipe_step(&__e0, __c0, __c1, __c2, *__v.downcast::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>().expect("the awaited answer"))), (|_| None)) }));
    } else if matches!(got, crate::unions::Union3::U2(_)) {
        let mut got_3 = match &got { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(to);
        if matches!(closed, crate::unions::Union2::U1(_)) {
            let mut closed_4 = match &closed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>(done, crate::unions::Union2::U1(crate::core_result::ok::<i64>(moved)));
        } else {
            let mut closed_5 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>(done, crate::unions::Union2::U2(closed_5));
        };
    } else {
        let mut got_6 = match got { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(to);
        if matches!(closed, crate::unions::Union2::U1(_)) {
            let mut closed_7 = match &closed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        } else {
            let mut closed_8 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_8);
        };
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>(done, crate::unions::Union2::U2(got_6));
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lines {
    pub s: crate::stream::InStream,
}

pub fn lines(mut s: crate::stream::InStream) -> crate::stream::Lines {
    return crate::stream::Lines { s: s };
}

pub fn next__Lines(streams: &crate::stream::Streams, p: &mut crate::stream::Lines) -> crate::unions::Union2<String, crate::core_iterator::Finished> {
    let mut line: Option<String> = streams.read_line(&p.s);
    return if line.is_some() {
        let mut line_1 = line.unwrap();
        return crate::unions::Union2::U1(crate::core_iterator::emitted::<String>(line_1));
    } else {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
}

pub fn close__Lines(streams: &crate::stream::Streams, mut p: crate::stream::Lines) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    return streams.close__InStream(p.s);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunks {
    pub s: crate::stream::InStream,
    pub size: i32,
}

pub fn chunks(mut s: crate::stream::InStream, mut size: i32) -> crate::stream::Chunks {
    return crate::stream::Chunks { s: s, size: size };
}

pub fn next__Chunks(streams: &crate::stream::Streams, p: &mut crate::stream::Chunks) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_iterator::Finished> {
    let mut got: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_bytes(&p.s, p.size);
    if matches!(got, crate::unions::Union2::U2(_)) {
        let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(got_1);
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    let mut got_2 = match got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut data: crate::core_bytes::Bytes = got_2;
    if ((crate::core_bytes::size_platform(&data)) == (0i32)) {
        return crate::unions::Union2::U2(crate::core_iterator::finished());
    };
    return crate::unions::Union2::U1(crate::core_iterator::emitted::<crate::core_bytes::Bytes>(data));
}

pub fn close__Chunks(streams: &crate::stream::Streams, mut p: crate::stream::Chunks) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    return streams.close__InStream(p.s);
}

pub fn stream_chunk_size() -> i32 {
    return 65536i32;
}

pub fn fill_from(streams: &crate::stream::Streams, s: &crate::stream::InStream, buf: &mut crate::core_bytes::Bytes) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut total: i64 = 0i64;
    let mut reading: bool = true;
    loop {
        if !(reading) {
            break;
        };
        let mut got: crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_to__InStream_Bytes_Int(s, &mut *buf, crate::stream::stream_chunk_size());
        if matches!(got, crate::unions::Union2::U2(_)) {
            let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(got_1);
        };
        let mut got_2 = match &got { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        let mut n: i32 = got_2;
        total = i64::wrapping_add(total, ((n) as i64));
        if ((n) == (0i32)) {
            reading = false;
        };
    }
    return crate::unions::Union2::U1(crate::core_result::ok::<i64>(total));
}

pub fn copy_stream(streams: &crate::stream::Streams, s: &crate::stream::InStream, w: &crate::stream::OutStream) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut buf: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
    let mut total: i64 = 0i64;
    let mut copying: bool = true;
    loop {
        if !(copying) {
            break;
        };
        crate::core_bytes::clear_platform(&mut buf);
        let mut got: crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_to__InStream_Bytes_Int(s, &mut buf, crate::stream::stream_chunk_size());
        if matches!(got, crate::unions::Union2::U2(_)) {
            let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(got_1);
        };
        let mut got_2 = match &got { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        let mut n: i32 = got_2;
        if ((n) == (0i32)) {
            copying = false;
        } else {
            total = i64::wrapping_add(total, streams.write_bytes(w, &buf));
        };
    }
    return crate::unions::Union2::U1(crate::core_result::ok::<i64>(total));
}

pub fn write_int(streams: &crate::stream::Streams, s: &crate::stream::OutStream, mut v: i32) -> i64 {
    return streams.write_bytes(s, &crate::stream::fixed_bytes(((v) as i64), 4i32));
}

pub fn write_long(streams: &crate::stream::Streams, s: &crate::stream::OutStream, mut v: i64) -> i64 {
    return streams.write_bytes(s, &crate::stream::fixed_bytes(v, 8i32));
}

pub fn write_value<T: Clone>(streams: &crate::stream::Streams, s: &crate::stream::OutStream, mut v: T, encode: &mut dyn FnMut(T) -> crate::core_bytes::Bytes) -> i64 {
    let mut data: crate::core_bytes::Bytes = encode(v);
    let mut n: i64 = crate::stream::write_int(streams, s, crate::core_bytes::size_platform(&data));
    return i64::wrapping_add(n, streams.write_bytes(s, &data));
}

pub fn read_int(streams: &crate::stream::Streams, s: &crate::stream::InStream) -> crate::unions::Union3<i32, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut r: crate::unions::Union3<i64, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::read_fixed(streams, s, 4i32);
    return if matches!(r, crate::unions::Union3::U1(_)) {
        let mut r_1 = match &r { crate::unions::Union3::U1(__v) => *__v, _ => unreachable!() };
        return crate::unions::Union3::U1(crate::core_result::ok::<i32>(((r_1) as i32)));
    } else if matches!(r, crate::unions::Union3::U2(_)) {
        let mut r_2 = match &r { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union3::U2(crate::stream::End {});
    } else {
        let mut r_3 = match r { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        return crate::unions::Union3::U3(r_3);
    };
}

pub fn read_long(streams: &crate::stream::Streams, s: &crate::stream::InStream) -> crate::unions::Union3<i64, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    return crate::stream::read_fixed(streams, s, 8i32);
}

pub fn read_value<T: Clone>(streams: &crate::stream::Streams, s: &crate::stream::InStream, decode: &mut dyn FnMut(&crate::core_bytes::Bytes) -> Option<T>) -> crate::unions::Union3<T, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut len: crate::unions::Union3<i32, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::read_int(streams, s);
    if matches!(len, crate::unions::Union3::U2(_)) {
        let mut len_1 = match &len { crate::unions::Union3::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union3::U2(crate::stream::End {});
    };
    if matches!(len, crate::unions::Union3::U3(_)) {
        let mut len_2 = match len { crate::unions::Union3::U3(__v) => __v, _ => unreachable!() };
        return crate::unions::Union3::U3(len_2);
    };
    let mut len_3 = match &len { crate::unions::Union3::U1(__v) => *__v, _ => unreachable!() };
    let mut n: i32 = len_3;
    let mut data: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_bytes(s, n);
    if matches!(data, crate::unions::Union2::U2(_)) {
        let mut data_4 = match data { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union3::U3(data_4);
    };
    let mut data_5 = match &data { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut bytes = data_5;
    if (crate::core_bytes::size_platform(bytes) < n) {
        return crate::unions::Union3::U3(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U2(crate::stream::StreamFailed { source: String::from("read_value"), message: String::from("the stream ended inside a value") }))));
    };
    let mut v: Option<T> = decode(bytes);
    if v.is_none() {
        return crate::unions::Union3::U3(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U2(crate::stream::StreamFailed { source: String::from("read_value"), message: String::from("the bytes are not a value of the type asked for") }))));
    };
    let mut v_6 = v.unwrap();
    return crate::unions::Union3::U1(crate::core_result::ok::<T>(v_6));
}

pub fn fixed_bytes(mut v: i64, mut width: i32) -> crate::core_bytes::Bytes {
    let mut out: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
    let mut rest: i64 = v;
    let mut i: i32 = 0i32;
    loop {
        if !((i < width)) {
            break;
        };
        let mut low: i64 = i64::wrapping_rem(rest, 256i64);
        if (low < 0i64) {
            low = i64::wrapping_add(low, 256i64);
        };
        crate::core_bytes::add_platform(&mut out, (((((low) as i32)) as i32) as u8));
        rest = i64::wrapping_div(i64::wrapping_sub(rest, low), 256i64);
        i = i32::wrapping_add(i, 1i32);
    }
    let mut back: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
    let mut j: i32 = i32::wrapping_sub(width, 1i32);
    loop {
        if !((j >= 0i32)) {
            break;
        };
        crate::core_bytes::add_platform(&mut back, {
            let mut __nn_1: Option<u8> = crate::core_bytes::get_platform(&out, j);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at stream:421:19");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        });
        j = i32::wrapping_sub(j, 1i32);
    }
    return back;
}

pub fn read_fixed(streams: &crate::stream::Streams, s: &crate::stream::InStream, mut width: i32) -> crate::unions::Union3<i64, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut r: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_bytes(s, width);
    if matches!(r, crate::unions::Union2::U2(_)) {
        let mut r_1 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union3::U3(r_1);
    };
    let mut r_2 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut data = r_2;
    if ((crate::core_bytes::size_platform(data)) == (0i32)) {
        return crate::unions::Union3::U2(crate::stream::End {});
    };
    if (crate::core_bytes::size_platform(data) < width) {
        return crate::unions::Union3::U3(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U2(crate::stream::StreamFailed { source: String::from("read"), message: String::from("the stream ended inside a number") }))));
    };
    let mut first: i64 = (((({
        let mut __nn_3: Option<u8> = crate::core_bytes::get_platform(data, 0i32);
        if __nn_3.is_none() {
            panic!("salvo: value is absent at stream:440:32");
        } else {
            let mut __some_4 = __nn_3.unwrap();
            __some_4
        }
    }) as i32)) as i64);
    if (first >= 128i64) {
        first = i64::wrapping_sub(first, 256i64);
    };
    let mut v: i64 = first;
    let mut i: i32 = 1i32;
    loop {
        if !((i < width)) {
            break;
        };
        v = i64::wrapping_add(i64::wrapping_mul(v, 256i64), (((({
            let mut __nn_5: Option<u8> = crate::core_bytes::get_platform(data, i);
            if __nn_5.is_none() {
                panic!("salvo: value is absent at stream:447:39");
            } else {
                let mut __some_6 = __nn_5.unwrap();
                __some_6
            }
        }) as i32)) as i64));
        i = i32::wrapping_add(i, 1i32);
    }
    return crate::unions::Union3::U1(crate::core_result::ok::<i64>(v));
}
