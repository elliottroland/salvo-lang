// Host implementation of the platform declarations of Salvo module `core.hostfs`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: rustc
// checks it, against the traits the backend generates from the
// `platform effect` and `platform handler` declarations.

use crate::core_fs::*;
use crate::core_hostfs::*;
use crate::unions::*;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

/// The droppable failure kinds, as the generated union.
type Kind = Union8<
    NotFound,
    PermissionDenied,
    AlreadyExists,
    NotADirectory,
    PathEscapes,
    InvalidUtf8,
    StaleHandle,
    IoError,
>;

fn not_found(path: &str) -> Kind {
    Union8::U1(NotFound { path: path.to_string() })
}
fn permission_denied(path: &str) -> Kind {
    Union8::U2(PermissionDenied { path: path.to_string() })
}
fn already_exists(path: &str) -> Kind {
    Union8::U3(AlreadyExists { path: path.to_string() })
}
fn not_a_directory(path: &str) -> Kind {
    Union8::U4(NotADirectory { path: path.to_string() })
}
fn invalid_utf8(path: &str) -> Kind {
    Union8::U6(InvalidUtf8 { path: path.to_string() })
}
fn stale_handle(handle: i64) -> Kind {
    Union8::U7(StaleHandle { path: format!("<handle {handle}>") })
}
fn io_error(path: &str, message: String) -> Kind {
    Union8::U8(IoError { path: path.to_string(), message })
}

/// The one place an `std::io::Error` becomes a Salvo kind. ENOTDIR has no
/// stable `ErrorKind`, so it is read from the raw OS code.
fn kind_of(path: &str, e: &std::io::Error) -> Kind {
    match e.kind() {
        std::io::ErrorKind::NotFound => not_found(path),
        std::io::ErrorKind::PermissionDenied => permission_denied(path),
        std::io::ErrorKind::AlreadyExists => already_exists(path),
        _ if e.raw_os_error() == Some(20) => not_a_directory(path),
        _ => io_error(path, e.to_string()),
    }
}

fn err_long(kind: Kind) -> Union2<i64, Kind> {
    Union2::U2(kind)
}
fn err_unit(kind: Kind) -> Union2<(), Kind> {
    Union2::U2(kind)
}

/// A stream open for reading: the buffer is *bytes*, so the consumed-byte
/// position is exact and UTF-8 is decoded by us, strictly.
struct Reading {
    path: String,
    file: BufReader<std::fs::File>,
    position: i64,
    failed: Option<Kind>,
}

/// A stream open for writing. `position` counts bytes accepted, which is
/// what a later ranged read needs; failures are recorded and surface at
/// flush or close.
struct Writing {
    path: String,
    file: BufWriter<std::fs::File>,
    position: i64,
    failed: Option<Kind>,
}

pub struct HostRawFs {
    next_handle: i64,
    reading: HashMap<i64, Reading>,
    writing: HashMap<i64, Writing>,
}

impl HostRawFs {
    pub fn new() -> Self {
        Self {
            next_handle: 0,
            reading: HashMap::new(),
            writing: HashMap::new(),
        }
    }

    fn mint(&mut self) -> i64 {
        self.next_handle += 1;
        self.next_handle
    }

    /// Decodes strictly, recording `InvalidUtf8` against the stream.
    fn decode(bytes: Vec<u8>, path: &str) -> Result<String, Kind> {
        String::from_utf8(bytes).map_err(|_| invalid_utf8(path))
    }
}

impl crate::core_hostfs::RawFs for HostRawFs {
    fn raw_open_read(&mut self, path: &String) -> Union2<i64, Kind> {
        match std::fs::File::open(path) {
            Ok(file) => {
                let handle = self.mint();
                self.reading.insert(
                    handle,
                    Reading {
                        path: path.clone(),
                        file: BufReader::new(file),
                        position: 0,
                        failed: None,
                    },
                );
                Union2::U1(handle)
            }
            Err(e) => err_long(kind_of(path, &e)),
        }
    }

    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> Union2<i64, Kind> {
        match std::fs::File::open(path) {
            Ok(mut file) => {
                use std::io::Seek;
                if let Err(e) = file.seek(std::io::SeekFrom::Start(offset.max(0) as u64)) {
                    return err_long(kind_of(path, &e));
                }
                let handle = self.mint();
                self.reading.insert(
                    handle,
                    Reading {
                        path: path.clone(),
                        file: BufReader::new(file),
                        position: offset.max(0),
                        failed: None,
                    },
                );
                Union2::U1(handle)
            }
            Err(e) => err_long(kind_of(path, &e)),
        }
    }

    fn raw_open_write(&mut self, path: &String) -> Union2<i64, Kind> {
        match std::fs::File::create(path) {
            Ok(file) => {
                let handle = self.mint();
                self.writing.insert(
                    handle,
                    Writing {
                        path: path.clone(),
                        file: BufWriter::new(file),
                        position: 0,
                        failed: None,
                    },
                );
                Union2::U1(handle)
            }
            Err(e) => err_long(kind_of(path, &e)),
        }
    }

    fn raw_open_append(&mut self, path: &String) -> Union2<i64, Kind> {
        match std::fs::OpenOptions::new().append(true).create(true).open(path) {
            Ok(file) => {
                let position = file.metadata().map(|m| m.len() as i64).unwrap_or(0);
                let handle = self.mint();
                self.writing.insert(
                    handle,
                    Writing {
                        path: path.clone(),
                        file: BufWriter::new(file),
                        position,
                        failed: None,
                    },
                );
                Union2::U1(handle)
            }
            Err(e) => err_long(kind_of(path, &e)),
        }
    }

    fn raw_exists(&mut self, path: &String) -> bool {
        std::path::Path::new(path).exists()
    }

    fn raw_metadata(&mut self, path: &String) -> Union2<FileInfo, Kind> {
        match std::fs::metadata(path) {
            Ok(m) => Union2::U1(FileInfo {
                size: m.len() as i64,
                is_dir: m.is_dir(),
            }),
            Err(e) => Union2::U2(kind_of(path, &e)),
        }
    }

    fn raw_list_dir(&mut self, path: &String) -> Union2<Vec<String>, Kind> {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => return Union2::U2(kind_of(path, &e)),
        };
        let mut names = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => names.push(entry.file_name().to_string_lossy().to_string()),
                Err(e) => return Union2::U2(kind_of(path, &e)),
            }
        }
        // Directory order is the host's; sorting makes the two backends agree.
        names.sort();
        Union2::U1(names)
    }

    fn raw_create_dirs(&mut self, path: &String) -> Union2<(), Kind> {
        match std::fs::create_dir_all(path) {
            Ok(()) => Union2::U1(()),
            Err(e) => err_unit(kind_of(path, &e)),
        }
    }

    fn raw_delete(&mut self, path: &String) -> Union2<(), Kind> {
        let is_dir = std::fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false);
        let result = if is_dir {
            std::fs::remove_dir(path)
        } else {
            std::fs::remove_file(path)
        };
        match result {
            Ok(()) => Union2::U1(()),
            Err(e) => err_unit(kind_of(path, &e)),
        }
    }

    fn raw_rename_path(&mut self, from: &String, to: &String) -> Union2<(), Kind> {
        match std::fs::rename(from, to) {
            Ok(()) => Union2::U1(()),
            Err(e) => err_unit(kind_of(from, &e)),
        }
    }

    fn raw_read_line(&mut self, handle: i64) -> Option<String> {
        let stream = self.reading.get_mut(&handle)?;
        if stream.failed.is_some() {
            return None;
        }
        let mut bytes = Vec::new();
        match stream.file.read_until(b'\n', &mut bytes) {
            Ok(0) => None,
            Ok(read) => {
                stream.position += read as i64;
                // Neither terminator is part of the line [fs-line-terminators].
                if bytes.last() == Some(&b'\n') {
                    bytes.pop();
                    if bytes.last() == Some(&b'\r') {
                        bytes.pop();
                    }
                }
                match Self::decode(bytes, &stream.path) {
                    Ok(line) => Some(line),
                    Err(kind) => {
                        stream.failed = Some(kind);
                        None
                    }
                }
            }
            Err(e) => {
                stream.failed = Some(kind_of(&stream.path, &e));
                None
            }
        }
    }

    fn raw_read_all(&mut self, handle: i64) -> Union2<String, Kind> {
        let Some(stream) = self.reading.get_mut(&handle) else {
            return Union2::U2(stale_handle(handle));
        };
        if let Some(kind) = stream.failed.clone() {
            return Union2::U2(kind);
        }
        let mut bytes = Vec::new();
        match stream.file.read_to_end(&mut bytes) {
            Ok(read) => {
                stream.position += read as i64;
                match Self::decode(bytes, &stream.path) {
                    Ok(text) => Union2::U1(text),
                    Err(kind) => {
                        stream.failed = Some(kind.clone());
                        Union2::U2(kind)
                    }
                }
            }
            Err(e) => {
                let kind = kind_of(&stream.path, &e);
                stream.failed = Some(kind.clone());
                Union2::U2(kind)
            }
        }
    }

    /// Up to `max` bytes, undecoded — the byte side of the read surface. The
    /// buffer below is bytes already, so this is the plainest read here: no
    /// decoding, so no way to fail on content.
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> Union2<Vec<u8>, Kind> {
        let mut out = Vec::new();
        match self.raw_read_to_bytes(handle, &mut out, max) {
            Union2::U1(_) => Union2::U1(out),
            Union2::U2(kind) => Union2::U2(kind),
        }
    }

    /// The fill-a-buffer read: up to `max` bytes **appended** to `buf`,
    /// answering how many. The one the others are written in terms of, because
    /// it is the one that does not allocate a payload — the `Vec` keeps the
    /// room it had, so a copy loop allocates once.
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Kind> {
        let Some(stream) = self.reading.get_mut(&handle) else {
            return Union2::U2(stale_handle(handle));
        };
        if let Some(kind) = stream.failed.clone() {
            return Union2::U2(kind);
        }
        if max <= 0 {
            return Union2::U1(0);
        }
        let start = buf.len();
        buf.resize(start + max as usize, 0);
        match stream.file.read(&mut buf[start..]) {
            Ok(read) => {
                stream.position += read as i64;
                buf.truncate(start + read);
                Union2::U1(read as i32)
            }
            Err(e) => {
                buf.truncate(start);
                let kind = kind_of(&stream.path, &e);
                stream.failed = Some(kind.clone());
                Union2::U2(kind)
            }
        }
    }

    /// `raw_read_all` with the destination handed in.
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> Union2<i64, Kind> {
        match self.raw_read_all(handle) {
            Union2::U1(text) => {
                let count = text.len() as i64;
                buf.push_str(&text);
                Union2::U1(count)
            }
            Union2::U2(kind) => Union2::U2(kind),
        }
    }

    /// `raw_read_line` with the destination handed in; no string per line.
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool {
        match self.raw_read_line(handle) {
            Some(line) => {
                buf.push_str(&line);
                true
            }
            None => false,
        }
    }

    fn raw_read_position(&mut self, handle: i64) -> i64 {
        self.reading.get(&handle).map(|s| s.position).unwrap_or(0)
    }

    fn raw_close_read(&mut self, handle: i64) -> Union2<(), Kind> {
        match self.reading.remove(&handle) {
            Some(stream) => match stream.failed {
                Some(kind) => err_unit(kind),
                None => Union2::U1(()),
            },
            None => err_unit(stale_handle(handle)),
        }
    }

    fn raw_write(&mut self, handle: i64, text: &String) -> i64 {
        let Some(stream) = self.writing.get_mut(&handle) else {
            return 0;
        };
        if stream.failed.is_some() {
            return 0;
        }
        let bytes = text.as_bytes();
        match stream.file.write_all(bytes) {
            Ok(()) => {
                stream.position += bytes.len() as i64;
                bytes.len() as i64
            }
            Err(e) => {
                stream.failed = Some(kind_of(&stream.path, &e));
                0
            }
        }
    }

    /// Bytes as they are: the write side's byte counterpart, nothing encoded.
    fn raw_write_bytes(&mut self, handle: i64, data: &Vec<u8>) -> i64 {
        let Some(stream) = self.writing.get_mut(&handle) else {
            return 0;
        };
        if stream.failed.is_some() {
            return 0;
        }
        match stream.file.write_all(data) {
            Ok(()) => {
                stream.position += data.len() as i64;
                data.len() as i64
            }
            Err(e) => {
                stream.failed = Some(kind_of(&stream.path, &e));
                0
            }
        }
    }

    fn raw_write_position(&mut self, handle: i64) -> i64 {
        self.writing.get(&handle).map(|s| s.position).unwrap_or(0)
    }

    fn raw_flush(&mut self, handle: i64) -> Union2<(), Kind> {
        let Some(stream) = self.writing.get_mut(&handle) else {
            return err_unit(stale_handle(handle));
        };
        if let Err(e) = stream.file.flush() {
            stream.failed = Some(kind_of(&stream.path, &e));
        }
        match stream.failed.take() {
            Some(kind) => err_unit(kind),
            None => Union2::U1(()),
        }
    }

    fn raw_close_write(&mut self, handle: i64) -> Union2<(), Kind> {
        match self.writing.remove(&handle) {
            Some(mut stream) => {
                if let Err(e) = stream.file.flush() {
                    stream.failed = Some(kind_of(&stream.path, &e));
                }
                match stream.failed {
                    Some(kind) => err_unit(kind),
                    None => Union2::U1(()),
                }
            }
            None => err_unit(stale_handle(handle)),
        }
    }
}
