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
use crate::unions::*;

pub fn fresh_handle__2() -> i64 {
    return fresh_handle();
}

pub type StreamError = Union2<InvalidUtf8, StreamFailed>;

/// Factories for the host: one per arm of the union [platform-factory].
impl StreamError {
    pub fn invalid_utf8(value: InvalidUtf8) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn stream_failed(value: StreamFailed) -> Self {
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

pub fn to_str__4(kind: &Union2<InvalidUtf8, StreamFailed>) -> String {
    match kind {
        Union2::U1(_) => {
            return format!("not valid UTF-8: {}", kind.u1().clone().source.clone());
        }
        Union2::U2(_) => {
            return format!("stream failed: {}: {}", kind.u2().clone().source.clone(), kind.u2().clone().message.clone());
        }
    }
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
    pub bytes: Vec<u8>,
    pub stream: InStream,
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

pub fn close(streams: &crate::stream::Streams, p: Packet) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let __destructured1 = p;
    let mut bytes = __destructured1.bytes;
    let mut stream = __destructured1.stream;
    return streams.close(stream);
}

pub trait __Stateless_Streams: Send + Sync {
    fn read_line(&self, s: &InStream) -> Option<String>;
    fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool;
    fn position(&self, s: &InStream) -> i64;
    fn close(&self, s: InStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn write(&self, s: &OutStream, text: &String) -> i64;
    fn write_line(&self, s: &OutStream, text: &String) -> i64;
    fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64;
    fn position__2(&self, s: &OutStream) -> i64;
    fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn receive(&self, s: InStream, reply: crate::scheduler::SalvoReply);
    fn from_bytes(&self, data: Vec<u8>) -> InStream;
}

pub trait __Stateful_Streams: Send {
    fn read_line(&mut self, s: &InStream) -> Option<String>;
    fn read_all(&mut self, s: &InStream) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool;
    fn position(&mut self, s: &InStream) -> i64;
    fn close(&mut self, s: InStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn write(&mut self, s: &OutStream, text: &String) -> i64;
    fn write_line(&mut self, s: &OutStream, text: &String) -> i64;
    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64;
    fn position__2(&mut self, s: &OutStream) -> i64;
    fn flush(&mut self, s: &OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn close__2(&mut self, s: OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>>;
    fn receive(&mut self, s: InStream, reply: crate::scheduler::SalvoReply);
    fn from_bytes(&mut self, data: Vec<u8>) -> InStream;
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
    pub fn read_line(&self, s: &InStream) -> Option<String> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_line(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_line(s),
        }
    }
    pub fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_all(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_all(s),
        }
    }
    pub fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_bytes(s, max),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_bytes(s, max),
        }
    }
    pub fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_to(s, buf, max),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_to(s, buf, max),
        }
    }
    pub fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_to__2(s, buf),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_to__2(s, buf),
        }
    }
    pub fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.read_line_to(s, buf),
            __Inner_Streams::Locked(h) => h.lock().unwrap().read_line_to(s, buf),
        }
    }
    pub fn position(&self, s: &InStream) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.position(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().position(s),
        }
    }
    pub fn close(&self, s: InStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.close(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().close(s),
        }
    }
    pub fn write(&self, s: &OutStream, text: &String) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.write(s, text),
            __Inner_Streams::Locked(h) => h.lock().unwrap().write(s, text),
        }
    }
    pub fn write_line(&self, s: &OutStream, text: &String) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.write_line(s, text),
            __Inner_Streams::Locked(h) => h.lock().unwrap().write_line(s, text),
        }
    }
    pub fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.write_bytes(s, data),
            __Inner_Streams::Locked(h) => h.lock().unwrap().write_bytes(s, data),
        }
    }
    pub fn position__2(&self, s: &OutStream) -> i64 {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.position__2(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().position__2(s),
        }
    }
    pub fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.flush(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().flush(s),
        }
    }
    pub fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.close__2(s),
            __Inner_Streams::Locked(h) => h.lock().unwrap().close__2(s),
        }
    }
    pub fn receive(&self, s: InStream, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.receive(s, reply),
            __Inner_Streams::Locked(h) => h.lock().unwrap().receive(s, reply),
        }
    }
    pub fn from_bytes(&self, data: Vec<u8>) -> InStream {
        match &self.inner {
            __Inner_Streams::Shared(h) => h.from_bytes(data),
            __Inner_Streams::Locked(h) => h.lock().unwrap().from_bytes(data),
        }
    }
}

pub fn pipe(streams: &crate::stream::Streams, from: InStream, to: OutStream, done: crate::scheduler::SalvoReply) {
    { let __a1 = ({ let __e0 = streams.clone(); let __c0 = to; let __c1 = done; let __c2 = 0i64; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), Box::new(move |__v| pipe_step(__e0.clone(), __c0, __c1, __c2, *__v.downcast::<Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>>().expect("the awaited answer"))), (|_| None)) }); streams.receive(from, __a1) };
}

pub fn pipe_step(mut streams: crate::stream::Streams, to: OutStream, done: crate::scheduler::SalvoReply, moved: i64, got: Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>) {
    match got {
        Union3::U1(_) => {
            let __destructured2 = (match got { Union3::U1(__v) => __v, _ => unreachable!() });
            let mut bytes = __destructured2.bytes;
            let mut stream = __destructured2.stream;
            let mut written = streams.write_bytes(&to, &bytes);
            { let __a1 = ({ let __e0 = streams.clone(); let __c0 = to; let __c1 = done; let __c2 = moved + written; crate::scheduler::salvo_mint_task(crate::scheduler::salvo_current_pool(), Box::new(move |__v| pipe_step(__e0.clone(), __c0, __c1, __c2, *__v.downcast::<Union3<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>>().expect("the awaited answer"))), (|_| None)) }); streams.receive(stream, __a1) };
        }
        Union3::U2(_) => {
            let mut closed = streams.close__2(to);
            match closed {
                Union2::U1(_) => {
                    crate::scheduler::salvo_reply_wire::<Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>>(done, Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(moved)));
                }
                Union2::U2(_) => {
                    crate::scheduler::salvo_reply_wire::<Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>>(done, Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match closed { Union2::U2(__v) => __v, _ => unreachable!() })));
                }
            }
        }
        Union3::U3(_) => {
            let mut closed = streams.close__2(to);
            match closed {
                Union2::U1(_) => {
                }
                Union2::U2(_) => {
                    ignore((match closed { Union2::U2(__v) => __v, _ => unreachable!() }));
                }
            }
            crate::scheduler::salvo_reply_wire::<Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>>(done, Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union3::U3(__v) => __v, _ => unreachable!() })));
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lines {
    pub s: InStream,
}

pub fn lines__2(s: InStream) -> Lines {
    return Lines { s: s };
}

pub fn next__19(streams: &crate::stream::Streams, p: &mut Lines) -> Union2<String, Finished> {
    let mut line = streams.read_line(&p.s);
    match line {
        Some(_) => {
            return Union2::<String, Finished>::U1(emitted(line.as_ref().unwrap().clone()));
        }
        None => {
            return Union2::<String, Finished>::U2(finished());
        }
    }
}

pub fn close__2(streams: &crate::stream::Streams, p: Lines) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(p.s);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunks {
    pub s: InStream,
    pub size: i32,
}

pub fn chunks(s: InStream, size: i32) -> Chunks {
    return Chunks { s: s, size: size };
}

pub fn next__20(streams: &crate::stream::Streams, p: &mut Chunks) -> Union2<Vec<u8>, Finished> {
    let mut got = streams.read_bytes(&p.s, p.size);
    if matches!(got, Union2::U2(_)) {
        ignore((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        return Union2::<Vec<u8>, Finished>::U2(finished());
    }
    let mut data: Vec<u8> = got.u1().clone();
    if ((data.len() as i32) == 0) {
        return Union2::<Vec<u8>, Finished>::U2(finished());
    }
    return Union2::<Vec<u8>, Finished>::U1(emitted(data));
}

pub fn close__3(streams: &crate::stream::Streams, p: Chunks) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(p.s);
}

pub fn stream_chunk_size() -> i32 {
    return 65536;
}

pub fn fill_from(streams: &crate::stream::Streams, s: &InStream, buf: &mut Vec<u8>) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let mut total: i64 = 0i64;
    let mut reading = true;
    while reading {
        let mut got = streams.read_to(s, buf, stream_chunk_size());
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        let mut n: i32 = *got.u1();
        total = total + ((n) as i64);
        if n == 0 {
            reading = false;
        }
    }
    return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(total));
}

pub fn copy_stream(streams: &crate::stream::Streams, s: &InStream, w: &OutStream) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let mut buf = Vec::<u8>::new();
    let mut total: i64 = 0i64;
    let mut copying = true;
    while copying {
        buf.clear();
        let mut got = streams.read_to(s, &mut buf, stream_chunk_size());
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        let mut n: i32 = *got.u1();
        if n == 0 {
            copying = false;
        } else {
            total = total + streams.write_bytes(w, &buf);
        }
    }
    return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(total));
}
