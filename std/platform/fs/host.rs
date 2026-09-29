// Host implementation of the platform declarations of Salvo module `fs.host`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: rustc
// checks it, against the traits the backend generates from the
// `platform effect` and `platform handler` declarations.

use crate::fs::*;
use crate::fs_host::*;
use crate::unions::*;

use std::io::{BufReader, BufWriter};

/// The droppable failure kinds, as the generated union (`fs.FsError`; its
/// `Streaming` arm is never the host's to report — a stream failure surfaces
/// through `stream.host`).
type Kind = Union7<
    NotFound,
    PermissionDenied,
    AlreadyExists,
    NotADirectory,
    PathEscapes,
    IoError,
    Streaming,
>;

fn not_found(path: &str) -> Kind {
    Union7::U1(NotFound { path: path.to_string() })
}
fn permission_denied(path: &str) -> Kind {
    Union7::U2(PermissionDenied { path: path.to_string() })
}
fn already_exists(path: &str) -> Kind {
    Union7::U3(AlreadyExists { path: path.to_string() })
}
fn not_a_directory(path: &str) -> Kind {
    Union7::U4(NotADirectory { path: path.to_string() })
}
fn io_error(path: &str, message: String) -> Kind {
    Union7::U6(IoError { path: path.to_string(), message })
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

// `platform handler HostRawFs` — the compiler SERIALIZES this instance; it
// holds nothing of its own now. What it opens goes into the runtime's stream
// table [stream-table], where `stream.host`'s `HostRawStreams` reads it.
pub struct HostRawFs {}

impl HostRawFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl crate::fs_host::__Stateful_RawFs for HostRawFs {
    fn raw_open_read(&mut self, path: &String) -> Union2<i64, Kind> {
        match std::fs::File::open(path) {
            Ok(file) => Union2::U1(crate::scheduler::salvo_stream_register_in(
                path.clone(),
                Box::new(BufReader::new(file)),
                0,
            )),
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
                Union2::U1(crate::scheduler::salvo_stream_register_in(
                    path.clone(),
                    Box::new(BufReader::new(file)),
                    offset.max(0),
                ))
            }
            Err(e) => err_long(kind_of(path, &e)),
        }
    }

    fn raw_open_write(&mut self, path: &String) -> Union2<i64, Kind> {
        match std::fs::File::create(path) {
            Ok(file) => Union2::U1(crate::scheduler::salvo_stream_register_out(
                path.clone(),
                Box::new(BufWriter::new(file)),
                0,
            )),
            Err(e) => err_long(kind_of(path, &e)),
        }
    }

    fn raw_open_append(&mut self, path: &String) -> Union2<i64, Kind> {
        match std::fs::OpenOptions::new().append(true).create(true).open(path) {
            Ok(file) => {
                let position = file.metadata().map(|m| m.len() as i64).unwrap_or(0);
                Union2::U1(crate::scheduler::salvo_stream_register_out(
                    path.clone(),
                    Box::new(BufWriter::new(file)),
                    position,
                ))
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
}
