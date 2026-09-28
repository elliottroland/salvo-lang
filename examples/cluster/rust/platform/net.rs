// Host implementation of the platform declarations of Salvo module `net`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: rustc
// checks it, against the traits the backend generates from the
// `platform effect` and `platform handler` declarations.

use crate::net::*;
use crate::unions::*;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

// `threadsafe platform handler HostTcpTransport` — THE CONTRACT YOU ARE SIGNING:
// this instance is shared across every thread of the program with NO
// lock around it. Every member below may run concurrently with every
// other, so the receivers are `&self`, any mutable state needs its own
// synchronization (`Mutex`, `RwLock`, atomics — a `RefCell` will not
// compile), and rustc enforces `Send + Sync` on this struct. If the
// host cannot promise that, delete `threadsafe` from the Salvo
// declaration: the compiler then serializes the instance for you and
// the receivers become `&mut self` [threadsafe-platform].
//
// Signed 2026-09-26: every table below is behind its own `Mutex`, held only
// for the table operation — never across a socket write — and a connection
// is written under the per-peer lock inside the table entry.
//
// **The wire.** One TCP connection per (this node, peer), opened lazily by
// the first `deliver` and kept until it fails. A connection opens with a
// **hello**: the sender's own endpoint (`bind`), so the receiver can label
// every frame's `from` without a lookup. After the hello, frames: a 4-byte
// big-endian length followed by that many bytes. A `listen` spawns one
// acceptor thread per endpoint, and one reader thread per accepted
// connection; a reader turns each frame into a send on the registered
// `Inbound` addr through the generated forwarding stub — the same door the
// scheduler's own timer uses to re-enter [time-timer].
pub struct HostTcpTransport {
    bind: NodeEndpoint,
    /// Outbound connections, one per peer, each behind its own lock so two
    /// pools delivering to two peers never wait on each other.
    peers: Mutex<HashMap<NodeEndpoint, Arc<Mutex<TcpStream>>>>,
    /// Inbound listeners by endpoint: the sink each acceptor forwards to,
    /// shared with the acceptor so `listen` twice replaces the sink and
    /// `unlisten` stops the acceptor at its next accept.
    listening: Mutex<HashMap<NodeEndpoint, Arc<Mutex<Option<usize>>>>>,
}

impl HostTcpTransport {
    pub fn new(bind: NodeEndpoint) -> Self {
        Self {
            bind,
            peers: Mutex::new(HashMap::new()),
            listening: Mutex::new(HashMap::new()),
        }
    }

    fn addr_of(e: &NodeEndpoint) -> String {
        format!("{}:{}", e.host, e.port)
    }

    fn unreachable(to: &NodeEndpoint) -> Union2<(), Union2<Unreachable, WireFailed>> {
        Union2::U2(Union2::U1(Unreachable { to: to.clone() }))
    }

    fn wire_failed(
        to: &NodeEndpoint,
        reason: String,
    ) -> Union2<(), Union2<Unreachable, WireFailed>> {
        Union2::U2(Union2::U2(WireFailed { to: to.clone(), reason }))
    }

    /// Writes one length-prefixed frame.
    fn write_frame(stream: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
        let len = (bytes.len() as u32).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(bytes)?;
        stream.flush()
    }

    /// Reads one length-prefixed frame; `None` at a clean close.
    fn read_frame(stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
        let mut len = [0u8; 4];
        match stream.read_exact(&mut len) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let n = u32::from_be_bytes(len) as usize;
        let mut buf = vec![0u8; n];
        stream.read_exact(&mut buf)?;
        Ok(Some(buf))
    }

    /// The hello: the sender's endpoint as `host\nport`.
    fn encode_hello(e: &NodeEndpoint) -> Vec<u8> {
        format!("{}\n{}", e.host, e.port).into_bytes()
    }

    fn decode_hello(bytes: &[u8]) -> Option<NodeEndpoint> {
        let text = std::str::from_utf8(bytes).ok()?;
        let (host, port) = text.split_once('\n')?;
        Some(NodeEndpoint {
            host: host.to_string(),
            port: port.parse().ok()?,
        })
    }

    /// One accepted connection: read the hello, then forward every frame to
    /// the listener's current sink until the peer closes or `unlisten` ran.
    fn serve(mut stream: TcpStream, sink: Arc<Mutex<Option<usize>>>) {
        let from = match Self::read_frame(&mut stream) {
            Ok(Some(hello)) => match Self::decode_hello(&hello) {
                Some(e) => e,
                None => return,
            },
            _ => return,
        };
        loop {
            let frame = match Self::read_frame(&mut stream) {
                Ok(Some(f)) => f,
                _ => return,
            };
            let Some(addr) = *sink.lock().unwrap() else {
                return;
            };
            let mut stub = __Stub_Inbound::new(addr);
            stub.receive_frame(from.clone(), frame);
        }
    }
}

impl crate::net::__Stateless_Transport for HostTcpTransport {
    fn listen(&self, at: &NodeEndpoint, sink: usize) -> Union2<(), Union2<Unreachable, WireFailed>> {
        let mut listening = self.listening.lock().unwrap();
        if let Some(slot) = listening.get(at) {
            // A second `listen` at the same endpoint replaces the sink; the
            // acceptor already running keeps forwarding, now to the new one.
            *slot.lock().unwrap() = Some(sink);
            return Union2::U1(());
        }
        let listener = match TcpListener::bind(Self::addr_of(at)) {
            Ok(l) => l,
            Err(e) => return Self::wire_failed(at, e.to_string()),
        };
        let slot = Arc::new(Mutex::new(Some(sink)));
        listening.insert(at.clone(), slot.clone());
        // An open listener is a source of work the scheduler cannot see, so
        // it must not declare the program idle or deadlocked while one is
        // open: tell it [threadsafe-platform] [actor-on-idle].
        crate::scheduler::salvo_external_begin();
        std::thread::spawn(move || {
            for accepted in listener.incoming() {
                if slot.lock().unwrap().is_none() {
                    return;
                }
                let Ok(stream) = accepted else { continue };
                let per_conn = slot.clone();
                std::thread::spawn(move || Self::serve(stream, per_conn));
            }
        });
        Union2::U1(())
    }

    fn unlisten(&self, at: &NodeEndpoint) {
        if let Some(slot) = self.listening.lock().unwrap().remove(at) {
            *slot.lock().unwrap() = None;
            // Wake the acceptor so it sees the cleared slot and exits.
            let _ = TcpStream::connect(Self::addr_of(at));
            crate::scheduler::salvo_external_end();
        }
    }

    fn deliver(&self, to: &NodeEndpoint, frame: Vec<u8>) -> Union2<(), Union2<Unreachable, WireFailed>> {
        let conn = {
            let mut peers = self.peers.lock().unwrap();
            match peers.get(to) {
                Some(c) => c.clone(),
                None => {
                    let mut stream = match TcpStream::connect(Self::addr_of(to)) {
                        Ok(s) => s,
                        Err(_) => return Self::unreachable(to),
                    };
                    if let Err(e) = Self::write_frame(&mut stream, &Self::encode_hello(&self.bind)) {
                        return Self::wire_failed(to, e.to_string());
                    }
                    let c = Arc::new(Mutex::new(stream));
                    peers.insert(to.clone(), c.clone());
                    c
                }
            }
        };
        let result = Self::write_frame(&mut conn.lock().unwrap(), &frame);
        match result {
            Ok(()) => Union2::U1(()),
            Err(e) => {
                // A failed connection is dropped; the next `deliver` reconnects.
                self.peers.lock().unwrap().remove(to);
                Self::wire_failed(to, e.to_string())
            }
        }
    }

    fn local_endpoint(&self) -> NodeEndpoint {
        self.bind.clone()
    }
}
