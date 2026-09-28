use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq)]
pub struct NotFound {
    pub path: String,
}

impl crate::wire::__Wire for NotFound {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PermissionDenied {
    pub path: String,
}

impl crate::wire::__Wire for PermissionDenied {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlreadyExists {
    pub path: String,
}

impl crate::wire::__Wire for AlreadyExists {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotADirectory {
    pub path: String,
}

impl crate::wire::__Wire for NotADirectory {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathEscapes {
    pub path: String,
}

impl crate::wire::__Wire for PathEscapes {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidUtf8 {
    pub path: String,
}

impl crate::wire::__Wire for InvalidUtf8 {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaleHandle {
    pub path: String,
}

impl crate::wire::__Wire for StaleHandle {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IoError {
    pub path: String,
    pub message: String,
}

impl crate::wire::__Wire for IoError {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn to_str(kind: &Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>) -> String {
    match kind {
        Union8::U1(_) => {
            return format!("no such file or directory: {}", kind.u1().clone().path.clone());
        }
        Union8::U2(_) => {
            return format!("permission denied: {}", kind.u2().clone().path.clone());
        }
        Union8::U3(_) => {
            return format!("already exists: {}", kind.u3().clone().path.clone());
        }
        Union8::U4(_) => {
            return format!("not a directory: {}", kind.u4().clone().path.clone());
        }
        Union8::U5(_) => {
            return format!("path escapes the root: {}", kind.u5().clone().path.clone());
        }
        Union8::U6(_) => {
            return format!("not valid UTF-8: {}", kind.u6().clone().path.clone());
        }
        Union8::U7(_) => {
            return format!("stale stream token: {}", kind.u7().clone().path.clone());
        }
        Union8::U8(_) => {
            return format!("io error: {}: {}", kind.u8().clone().path.clone(), kind.u8().clone().message.clone());
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileInfo {
    pub size: i64,
    pub is_dir: bool,
}

impl crate::wire::__Wire for FileInfo {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.size, out);
        crate::wire::__Wire::__enc(&self.is_dir, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            size: crate::wire::__Wire::__dec(r)?,
            is_dir: crate::wire::__Wire::__dec(r)?,
        })
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

pub trait __Stateless_Fs: Send + Sync {
    fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn exists(&self, path: &String) -> bool;
    fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn delete(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_line(&self, s: &InStream) -> Option<String>;
    fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool;
    fn position(&self, s: &InStream) -> i64;
    fn close(&self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn write(&self, s: &OutStream, text: &String) -> i64;
    fn write_line(&self, s: &OutStream, text: &String) -> i64;
    fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64;
    fn position__2(&self, s: &OutStream) -> i64;
    fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
}

pub trait __Stateful_Fs: Send {
    fn open_read(&mut self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn open_write(&mut self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn open_append(&mut self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn exists(&mut self, path: &String) -> bool;
    fn metadata(&mut self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn create_dirs(&mut self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn delete(&mut self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_line(&mut self, s: &InStream) -> Option<String>;
    fn read_all(&mut self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool;
    fn position(&mut self, s: &InStream) -> i64;
    fn close(&mut self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn write(&mut self, s: &OutStream, text: &String) -> i64;
    fn write_line(&mut self, s: &OutStream, text: &String) -> i64;
    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64;
    fn position__2(&mut self, s: &OutStream) -> i64;
    fn flush(&mut self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
    fn close__2(&mut self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>;
}

pub struct Fs {
    inner: __Inner_Fs,
}

pub enum __Inner_Fs {
    Shared(std::sync::Arc<dyn __Stateless_Fs>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_Fs>>),
}

impl Clone for Fs {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_Fs::Shared(h) => __Inner_Fs::Shared(h.clone()),
            __Inner_Fs::Locked(h) => __Inner_Fs::Locked(h.clone()),
        } }
    }
}

impl Fs {
    pub fn shared<__H: __Stateless_Fs + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Fs::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_Fs>) -> Self {
        Self { inner: __Inner_Fs::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_Fs + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_Fs::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_Fs>>) -> Self {
        Self { inner: __Inner_Fs::Locked(inner) }
    }
    pub fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_read(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_read(path),
        }
    }
    pub fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_read_at(path, offset),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_read_at(path, offset),
        }
    }
    pub fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_write(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_write(path),
        }
    }
    pub fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_append(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_append(path),
        }
    }
    pub fn exists(&self, path: &String) -> bool {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.exists(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().exists(path),
        }
    }
    pub fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.metadata(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().metadata(path),
        }
    }
    pub fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.list_dir(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().list_dir(path),
        }
    }
    pub fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.create_dirs(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().create_dirs(path),
        }
    }
    pub fn delete(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.delete(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().delete(path),
        }
    }
    pub fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.rename_path(from, to),
            __Inner_Fs::Locked(h) => h.lock().unwrap().rename_path(from, to),
        }
    }
    pub fn read_line(&self, s: &InStream) -> Option<String> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.read_line(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().read_line(s),
        }
    }
    pub fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.read_all(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().read_all(s),
        }
    }
    pub fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.read_bytes(s, max),
            __Inner_Fs::Locked(h) => h.lock().unwrap().read_bytes(s, max),
        }
    }
    pub fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.read_to(s, buf, max),
            __Inner_Fs::Locked(h) => h.lock().unwrap().read_to(s, buf, max),
        }
    }
    pub fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.read_to__2(s, buf),
            __Inner_Fs::Locked(h) => h.lock().unwrap().read_to__2(s, buf),
        }
    }
    pub fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.read_line_to(s, buf),
            __Inner_Fs::Locked(h) => h.lock().unwrap().read_line_to(s, buf),
        }
    }
    pub fn position(&self, s: &InStream) -> i64 {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.position(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().position(s),
        }
    }
    pub fn close(&self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.close(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().close(s),
        }
    }
    pub fn write(&self, s: &OutStream, text: &String) -> i64 {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.write(s, text),
            __Inner_Fs::Locked(h) => h.lock().unwrap().write(s, text),
        }
    }
    pub fn write_line(&self, s: &OutStream, text: &String) -> i64 {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.write_line(s, text),
            __Inner_Fs::Locked(h) => h.lock().unwrap().write_line(s, text),
        }
    }
    pub fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64 {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.write_bytes(s, data),
            __Inner_Fs::Locked(h) => h.lock().unwrap().write_bytes(s, data),
        }
    }
    pub fn position__2(&self, s: &OutStream) -> i64 {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.position__2(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().position__2(s),
        }
    }
    pub fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.flush(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().flush(s),
        }
    }
    pub fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.close__2(s),
            __Inner_Fs::Locked(h) => h.lock().unwrap().close__2(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lines {
    pub s: InStream,
}

pub fn lines(s: InStream) -> Lines {
    return Lines { s: s };
}

pub fn next__13(fs: &crate::fs::Fs, p: &mut Lines) -> Union2<String, Finished> {
    let mut line = fs.read_line(&p.s);
    match line {
        Some(_) => {
            return Union2::<String, Finished>::U1(emitted(line.as_ref().unwrap().clone()));
        }
        None => {
            return Union2::<String, Finished>::U2(finished());
        }
    }
}

pub fn close(fs: &crate::fs::Fs, p: Lines) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    return fs.close(p.s);
}

pub fn open_lines(fs: &crate::fs::Fs, path: &String) -> Union2<Lines, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Lines, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    return Union2::<Lines, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(lines(opened.u1().clone())));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunks {
    pub s: InStream,
    pub size: i32,
}

pub fn chunks(s: InStream, size: i32) -> Chunks {
    return Chunks { s: s, size: size };
}

pub fn next__14(fs: &crate::fs::Fs, p: &mut Chunks) -> Union2<Vec<u8>, Finished> {
    let mut got = fs.read_bytes(&p.s, p.size);
    if matches!(got, Union2::U2(_)) {
        ignore(got.u2().clone());
        return Union2::<Vec<u8>, Finished>::U2(finished());
    }
    let mut data: Vec<u8> = got.u1().clone();
    if ((data.len() as i32) == 0) {
        return Union2::<Vec<u8>, Finished>::U2(finished());
    }
    return Union2::<Vec<u8>, Finished>::U1(emitted(data));
}

pub fn close__2(fs: &crate::fs::Fs, p: Chunks) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    return fs.close(p.s);
}

pub fn open_chunks(fs: &crate::fs::Fs, path: &String, size: i32) -> Union2<Chunks, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Chunks, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    return Union2::<Chunks, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(chunks(opened.u1().clone(), size)));
}

pub fn read_to_str(fs: &crate::fs::Fs, path: &String) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    let mut s: InStream = opened.u1().clone();
    let mut content = fs.read_all(&s);
    if matches!(content, Union2::U2(_)) {
        let mut closed = fs.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore(closed.u2().clone());
        }
        return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(content.u2().clone());
    }
    let mut closed = fs.close(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(closed.u2().clone());
    }
    return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(content.u1().clone());
}

pub fn read_lines(fs: &crate::fs::Fs, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    let mut p = lines(opened.u1().clone());
    let mut out: Vec<String> = vec![];
    while let Union2::U1(mut line) = next__13(fs, &mut p) {
        out.push(line);
    }
    let mut closed = close(fs, p);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(closed.u2().clone());
    }
    let mut done: Vec<String> = out;
    return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(done));
}

pub fn write_str(fs: &crate::fs::Fs, path: &String, content: &String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_write(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    let mut s: OutStream = opened.u1().clone();
    let mut written = fs.write(&s, content);
    let mut closed = fs.close__2(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(closed.u2().clone());
    }
    return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(written));
}

pub fn read_to_bytes(fs: &crate::fs::Fs, path: &String) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    let mut s: InStream = opened.u1().clone();
    let mut buf = Vec::<u8>::new();
    let mut filling = fill_from(fs, &s, &mut buf);
    if matches!(filling, Union2::U2(_)) {
        let mut closed = fs.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore(closed.u2().clone());
        }
        return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(filling.u2().clone());
    }
    let mut closed = fs.close(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(closed.u2().clone());
    }
    let mut done: Vec<u8> = buf;
    return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(done));
}

pub fn write_bytes_to(fs: &crate::fs::Fs, path: &String, data: &Vec<u8>) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_write(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    let mut s: OutStream = opened.u1().clone();
    let mut written = fs.write_bytes(&s, data);
    let mut closed = fs.close__2(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(closed.u2().clone());
    }
    return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(written));
}

pub fn fs_chunk_size() -> i32 {
    return 65536;
}

pub fn fill_from(fs: &crate::fs::Fs, s: &InStream, buf: &mut Vec<u8>) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut total: i64 = 0i64;
    let mut reading = true;
    while reading {
        let mut got = fs.read_to(s, buf, fs_chunk_size());
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(got.u2().clone());
        }
        let mut n: i32 = *got.u1();
        total = total + ((n) as i64);
        if n == 0 {
            reading = false;
        }
    }
    return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(total));
}

pub fn copy_stream(fs: &crate::fs::Fs, s: &InStream, w: &OutStream) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut buf = Vec::<u8>::new();
    let mut total: i64 = 0i64;
    let mut copying = true;
    while copying {
        buf.clear();
        let mut got = fs.read_to(s, &mut buf, fs_chunk_size());
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(got.u2().clone());
        }
        let mut n: i32 = *got.u1();
        if n == 0 {
            copying = false;
        } else {
            total = total + fs.write_bytes(w, &buf);
        }
    }
    return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(total));
}

pub fn copy_file(fs: &crate::fs::Fs, from: &String, to: &String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
    let mut opened = fs.open_read(from);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(opened.u2().clone());
    }
    let mut s: InStream = opened.u1().clone();
    let mut created = fs.open_write(to);
    if matches!(created, Union2::U2(_)) {
        let mut closed = fs.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore(closed.u2().clone());
        }
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(created.u2().clone());
    }
    let mut w: OutStream = created.u1().clone();
    let mut moved = copy_stream(fs, &s, &w);
    let mut shut_w = fs.close__2(w);
    let mut shut_s = fs.close(s);
    if matches!(moved, Union2::U2(_)) {
        if matches!(shut_w, Union2::U2(_)) {
            ignore(shut_w.u2().clone());
        }
        if matches!(shut_s, Union2::U2(_)) {
            ignore(shut_s.u2().clone());
        }
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(moved.u2().clone());
    }
    if matches!(shut_w, Union2::U2(_)) {
        if matches!(shut_s, Union2::U2(_)) {
            ignore(shut_s.u2().clone());
        }
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(shut_w.u2().clone());
    }
    if matches!(shut_s, Union2::U2(_)) {
        return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(shut_s.u2().clone());
    }
    return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(*moved.u1());
}
