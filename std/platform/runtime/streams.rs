// Host implementation of the platform declarations of Salvo module
// `runtime.streams`: the host stream objects and the leaf operations on them
// (RUNTIME.md §3.4). The table, buffering, positions and failure recording
// are Salvo. Written by hand, against the generated `streams.sv.rs`.

use std::io::{Read as _, Write as _};

/// `linear platform type HostIn canbe Mut`: a host stream to read.
pub struct HostIn {
    pub input: Box<dyn std::io::Read + Send>,
}

/// `linear platform type HostOut canbe Mut`: a host stream to write.
pub struct HostOut {
    pub output: Box<dyn std::io::Write + Send>,
}

pub fn host_read(h: &mut HostIn, max: i32) -> crate::runtime_streams::HostRead {
    let mut buf = vec![0u8; max.max(0) as usize];
    loop {
        match h.input.read(&mut buf) {
            Ok(n) => {
                buf.truncate(n);
                return crate::runtime_streams::HostRead { data: buf, error: None };
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return crate::runtime_streams::HostRead { data: Vec::new(), error: Some(e.to_string()) },
        }
    }
}

pub fn host_close_in(h: HostIn) {
    drop(h);
}

pub fn host_write(h: &mut HostOut, data: &Vec<u8>) -> Option<String> {
    h.output.write_all(data).err().map(|e| e.to_string())
}

pub fn host_flush(h: &mut HostOut) -> Option<String> {
    h.output.flush().err().map(|e| e.to_string())
}

pub fn host_close_out(mut h: HostOut) -> Option<String> {
    h.output.flush().err().map(|e| e.to_string())
}

pub fn host_bytes_in(data: Vec<u8>) -> HostIn {
    HostIn { input: Box::new(std::io::Cursor::new(data)) }
}

/// [stream-provider] A handle the table never minted.
pub fn not_ours(handle: i64) -> ! {
    panic!(
        "salvo: stream handle {handle} is not the host's: a stream belongs to the provider \
         that minted it [stream-provider]"
    )
}
