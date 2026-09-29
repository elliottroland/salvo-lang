// Host implementation of the platform declarations of Salvo module `stream.host`.
//
// Generated once by `salvo platform generate`, then written by hand: the
// process's stream table lives in the runtime (`crate::scheduler`,
// [stream-table]), and this class reads and writes whatever is registered
// there — a file `HostRawFs` opened, a network body, a buffer.

use crate::stream::*;
use crate::unions::*;

use crate::scheduler::{SalvoFault, SalvoIn};

/// `stream.StreamError`, as the generated union.
type Kind = Union2<InvalidUtf8, StreamFailed>;

fn kind(source: &str, fault: SalvoFault) -> Kind {
    match fault {
        SalvoFault::Utf8 => Union2::U1(InvalidUtf8 { source: source.to_string() }),
        SalvoFault::Failed(message) => Union2::U2(StreamFailed { source: source.to_string(), message }),
    }
}

/// Decodes strictly, recording `InvalidUtf8` against the stream.
fn decode(stream: &mut SalvoIn, bytes: Vec<u8>) -> Result<String, Kind> {
    String::from_utf8(bytes).map_err(|_| {
        let f = stream.utf8_failed();
        kind(&stream.source, f)
    })
}

// `threadsafe platform handler HostRawStreams` — THE CONTRACT: every member
// below is safe to run concurrently with every other. The state is the
// runtime's table, which locks per stream, so there is nothing here to guard.
pub struct HostRawStreams {}

impl HostRawStreams {
    pub fn new() -> Self {
        Self {}
    }
}

impl crate::stream_host::__Stateless_RawStreams for HostRawStreams {
    fn raw_read_line(&self, handle: i64) -> Option<String> {
        let slot = crate::scheduler::salvo_stream_in(handle);
        let mut stream = slot.lock().unwrap();
        match stream.read_line_bytes() {
            Ok(Some(bytes)) => decode(&mut stream, bytes).ok(),
            Ok(None) | Err(_) => None,
        }
    }

    fn raw_read_all(&self, handle: i64) -> Union2<String, Kind> {
        let slot = crate::scheduler::salvo_stream_in(handle);
        let mut stream = slot.lock().unwrap();
        match stream.read_all_bytes() {
            Ok(bytes) => match decode(&mut stream, bytes) {
                Ok(text) => Union2::U1(text),
                Err(k) => Union2::U2(k),
            },
            Err(f) => Union2::U2(kind(&stream.source, f)),
        }
    }

    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Kind> {
        let mut out = Vec::new();
        match self.raw_read_to_bytes(handle, &mut out, max) {
            Union2::U1(_) => Union2::U1(out),
            Union2::U2(k) => Union2::U2(k),
        }
    }

    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Kind> {
        let slot = crate::scheduler::salvo_stream_in(handle);
        let mut stream = slot.lock().unwrap();
        match stream.read_up_to(buf, max.max(0) as usize) {
            Ok(n) => Union2::U1(n as i32),
            Err(f) => Union2::U2(kind(&stream.source, f)),
        }
    }

    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Kind> {
        match self.raw_read_all(handle) {
            Union2::U1(text) => {
                let count = text.len() as i64;
                buf.push_str(&text);
                Union2::U1(count)
            }
            Union2::U2(k) => Union2::U2(k),
        }
    }

    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool {
        match self.raw_read_line(handle) {
            Some(line) => {
                buf.push_str(&line);
                true
            }
            None => false,
        }
    }

    fn raw_read_position(&self, handle: i64) -> i64 {
        crate::scheduler::salvo_stream_in(handle).lock().unwrap().position
    }

    fn raw_close_read(&self, handle: i64) -> Union2<(), Kind> {
        let slot = crate::scheduler::salvo_stream_take_in(handle);
        let stream = slot.lock().unwrap();
        match stream.failed.clone() {
            Some(f) => Union2::U2(kind(&stream.source, f)),
            None => Union2::U1(()),
        }
    }

    fn raw_write(&self, handle: i64, text: &String) -> i64 {
        let slot = crate::scheduler::salvo_stream_out(handle);
        let mut stream = slot.lock().unwrap();
        stream.write_data(text.as_bytes())
    }

    fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64 {
        let slot = crate::scheduler::salvo_stream_out(handle);
        let mut stream = slot.lock().unwrap();
        stream.write_data(data)
    }

    fn raw_write_position(&self, handle: i64) -> i64 {
        crate::scheduler::salvo_stream_out(handle).lock().unwrap().position
    }

    fn raw_flush(&self, handle: i64) -> Union2<(), Kind> {
        let slot = crate::scheduler::salvo_stream_out(handle);
        let mut stream = slot.lock().unwrap();
        match stream.flush_data() {
            Ok(()) => Union2::U1(()),
            Err(f) => Union2::U2(kind(&stream.source, f)),
        }
    }

    /// [stream-receive] Reads on a thread of its own and completes `reply`
    /// from there [platform-reply]: the caller's worker never waits. At the
    /// end, or on a failure, the stream is released before answering, so the
    /// caller has nothing left to close.
    fn raw_receive(&self, handle: i64, reply: crate::scheduler::SalvoReply) {
        let reply = reply.hosted();
        let slot = crate::scheduler::salvo_stream_in(handle);
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let got = {
                let mut stream = slot.lock().unwrap();
                match stream.read_up_to(&mut buf, 65536) {
                    Ok(_) => Ok(()),
                    Err(f) => Err(kind(&stream.source, f)),
                }
            };
            let answer: Union3<Vec<u8>, End, Kind> = match got {
                Ok(()) if !buf.is_empty() => Union3::U1(buf),
                Ok(()) => {
                    crate::scheduler::salvo_stream_take_in(handle);
                    Union3::U2(End {})
                }
                Err(k) => {
                    crate::scheduler::salvo_stream_take_in(handle);
                    Union3::U3(k)
                }
            };
            reply.send(answer);
        });
    }

    /// [stream-from-bytes] A readable stream over `data`, registered in the
    /// process's table.
    fn raw_from_bytes(&self, data: Vec<u8>) -> i64 {
        crate::scheduler::salvo_stream_register_in(
            "<bytes>".to_string(),
            Box::new(std::io::Cursor::new(data)),
            0,
        )
    }

    fn raw_close_write(&self, handle: i64) -> Union2<(), Kind> {
        let slot = crate::scheduler::salvo_stream_take_out(handle);
        let mut stream = slot.lock().unwrap();
        match stream.flush_data() {
            Ok(()) => Union2::U1(()),
            Err(f) => Union2::U2(kind(&stream.source, f)),
        }
    }
}
