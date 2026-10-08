use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::stream::Chunks;
use crate::core_iterator::Finished;
use crate::stream::InStream;
use crate::stream::InvalidUtf8;
use crate::stream::Lines;
use crate::stream::OutStream;
use crate::fs_path::Path;
use crate::stream::StreamFailed;
use crate::stream::Streams;
use crate::core_list::add_platform;
use crate::core_checked::checked;
use crate::stream::chunks;
use crate::stream::close__Lines;
use crate::stream::copy_stream;
use crate::core_checked::detach;
use crate::core_result::err;
use crate::stream::fill_from;
use crate::core_checked::ignore;
use crate::stream::lines;
use crate::core_bytes::mut_bytes;
use crate::stream::next__Lines;
use crate::core_result::ok;


pub type FsError = crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>;

/// Factories for the host: one per arm of the union [platform-factory].
impl FsError {
    pub fn not_found(value: crate::fs::NotFound) -> Self {
        crate::unions::Union7::U1(value)
    }
    pub fn permission_denied(value: crate::fs::PermissionDenied) -> Self {
        crate::unions::Union7::U2(value)
    }
    pub fn already_exists(value: crate::fs::AlreadyExists) -> Self {
        crate::unions::Union7::U3(value)
    }
    pub fn not_a_directory(value: crate::fs::NotADirectory) -> Self {
        crate::unions::Union7::U4(value)
    }
    pub fn path_escapes(value: crate::fs::PathEscapes) -> Self {
        crate::unions::Union7::U5(value)
    }
    pub fn io_error(value: crate::fs::IoError) -> Self {
        crate::unions::Union7::U6(value)
    }
    pub fn streaming(value: crate::fs::Streaming) -> Self {
        crate::unions::Union7::U7(value)
    }
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct Streaming {
    pub error: crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>,
}

impl crate::wire::__Wire for Streaming {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.error, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            error: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn to_str(kind: &crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> String {
    return if matches!(kind, crate::unions::Union7::U1(_)) {
        let mut kind_1 = match &kind { crate::unions::Union7::U1(__v) => __v, _ => unreachable!() };
        return format!("no such file or directory: {}", kind_1.path);
    } else if matches!(kind, crate::unions::Union7::U2(_)) {
        let mut kind_2 = match &kind { crate::unions::Union7::U2(__v) => __v, _ => unreachable!() };
        return format!("permission denied: {}", kind_2.path);
    } else if matches!(kind, crate::unions::Union7::U3(_)) {
        let mut kind_3 = match &kind { crate::unions::Union7::U3(__v) => __v, _ => unreachable!() };
        return format!("already exists: {}", kind_3.path);
    } else if matches!(kind, crate::unions::Union7::U4(_)) {
        let mut kind_4 = match &kind { crate::unions::Union7::U4(__v) => __v, _ => unreachable!() };
        return format!("not a directory: {}", kind_4.path);
    } else if matches!(kind, crate::unions::Union7::U5(_)) {
        let mut kind_5 = match &kind { crate::unions::Union7::U5(__v) => __v, _ => unreachable!() };
        return format!("path escapes the root: {}", kind_5.path);
    } else if matches!(kind, crate::unions::Union7::U6(_)) {
        let mut kind_6 = match &kind { crate::unions::Union7::U6(__v) => __v, _ => unreachable!() };
        return format!("io error: {}: {}", kind_6.path, kind_6.message);
    } else {
        let mut kind_7 = match &kind { crate::unions::Union7::U7(__v) => __v, _ => unreachable!() };
        return crate::stream::to_str(&kind_7.error);
    };
}

pub fn fs_stream_error(mut e: crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>) -> crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
    return crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U7(crate::fs::Streaming { error: crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(e) }));
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

pub trait __Stateless_Fs: Send + Sync {
    fn open_read(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn open_read_at(&self, path: &crate::fs_path::Path, offset: i64) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn open_write(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn open_append(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn exists(&self, path: &crate::fs_path::Path) -> bool;
    fn metadata(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn list_dir(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn create_dirs(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn delete(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn rename_path(&self, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
}

pub trait __Stateful_Fs: Send {
    fn open_read(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn open_read_at(&mut self, path: &crate::fs_path::Path, offset: i64) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn open_write(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn open_append(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn exists(&mut self, path: &crate::fs_path::Path) -> bool;
    fn metadata(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn list_dir(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn create_dirs(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn delete(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
    fn rename_path(&mut self, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>;
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
    pub fn open_read(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_read(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_read(path),
        }
    }
    pub fn open_read_at(&self, path: &crate::fs_path::Path, offset: i64) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_read_at(path, offset),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_read_at(path, offset),
        }
    }
    pub fn open_write(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_write(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_write(path),
        }
    }
    pub fn open_append(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_append(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_append(path),
        }
    }
    pub fn exists(&self, path: &crate::fs_path::Path) -> bool {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.exists(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().exists(path),
        }
    }
    pub fn metadata(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.metadata(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().metadata(path),
        }
    }
    pub fn list_dir(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.list_dir(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().list_dir(path),
        }
    }
    pub fn create_dirs(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.create_dirs(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().create_dirs(path),
        }
    }
    pub fn delete(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.delete(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().delete(path),
        }
    }
    pub fn rename_path(&self, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.rename_path(from, to),
            __Inner_Fs::Locked(h) => h.lock().unwrap().rename_path(from, to),
        }
    }
}

pub fn open_lines(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::Lines, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::Lines>(crate::stream::lines(opened_2)));
}

pub fn open_chunks(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path, mut size: i32) -> crate::unions::Union2<crate::stream::Chunks, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::Chunks>(crate::stream::chunks(opened_2, size)));
}

pub fn read_to_str(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut s: crate::stream::InStream = opened_2;
    let mut content: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_all(&s);
    if matches!(content, crate::unions::Union2::U2(_)) {
        let mut content_3 = match content { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_4 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_4);
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(content_3)));
    };
    let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
    if matches!(closed, crate::unions::Union2::U2(_)) {
        let mut closed_5 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(closed_5)));
    };
    let mut content_6 = match content { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    return crate::unions::Union2::U1(crate::core_result::ok::<String>(content_6));
}

pub fn read_lines(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut p: crate::stream::Lines = crate::stream::lines(opened_2);
    let mut out: Vec<String> = vec![];
    loop {
        let mut __step_4: crate::unions::Union2<String, crate::core_iterator::Finished> = crate::stream::next__Lines(streams, &mut p);
        if matches!(__step_4, crate::unions::Union2::U1(_)) {
            let mut __emitted_5 = match __step_4 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut line: String = __emitted_5;
            crate::core_list::add_platform::<String>(&mut out, line);
        } else {
            break;
        };
    }
    let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::close__Lines(streams, p);
    if matches!(closed, crate::unions::Union2::U2(_)) {
        let mut closed_6 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(closed_6)));
    };
    let mut done: Vec<String> = out.clone();
    return crate::unions::Union2::U1(crate::core_result::ok::<Vec<String>>(done));
}

pub fn write_str(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path, content: &String) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_write(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut s: crate::stream::OutStream = opened_2;
    let mut written: i64 = streams.write(&s, content);
    let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(s);
    if matches!(closed, crate::unions::Union2::U2(_)) {
        let mut closed_3 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(closed_3)));
    };
    return crate::unions::Union2::U1(crate::core_result::ok::<i64>(written));
}

pub fn read_to_bytes(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut s: crate::stream::InStream = opened_2;
    let mut buf: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
    let mut filling: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::fill_from(streams, &s, &mut buf);
    if matches!(filling, crate::unions::Union2::U2(_)) {
        let mut filling_3 = match filling { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_4 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_4);
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(filling_3)));
    };
    let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
    if matches!(closed, crate::unions::Union2::U2(_)) {
        let mut closed_5 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(closed_5)));
    };
    let mut done: crate::core_bytes::Bytes = buf.clone();
    return crate::unions::Union2::U1(crate::core_result::ok::<crate::core_bytes::Bytes>(done));
}

pub fn write_bytes_to(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path, data: &crate::core_bytes::Bytes) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_write(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut s: crate::stream::OutStream = opened_2;
    let mut written: i64 = streams.write_bytes(&s, data);
    let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(s);
    if matches!(closed, crate::unions::Union2::U2(_)) {
        let mut closed_3 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(closed_3)));
    };
    return crate::unions::Union2::U1(crate::core_result::ok::<i64>(written));
}

pub fn copy_file(fs: &crate::fs::Fs, streams: &crate::stream::Streams, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(from);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(opened_1);
    };
    let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut s: crate::stream::InStream = opened_2;
    let mut created: crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_write(to);
    if matches!(created, crate::unions::Union2::U2(_)) {
        let mut created_3 = match created { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_4 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_4);
        };
        return crate::unions::Union2::U2(created_3);
    };
    let mut created_5 = match created { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut w: crate::stream::OutStream = created_5;
    let mut moved: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::copy_stream(streams, &s, &w);
    let mut shut_w: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(w);
    let mut shut_s: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
    if matches!(moved, crate::unions::Union2::U2(_)) {
        let mut moved_6 = match moved { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        if matches!(shut_w, crate::unions::Union2::U2(_)) {
            let mut shut_w_7 = match shut_w { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(shut_w_7);
        };
        if matches!(shut_s, crate::unions::Union2::U2(_)) {
            let mut shut_s_8 = match shut_s { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(shut_s_8);
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(moved_6)));
    };
    if matches!(shut_w, crate::unions::Union2::U2(_)) {
        let mut shut_w_9 = match shut_w { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        if matches!(shut_s, crate::unions::Union2::U2(_)) {
            let mut shut_s_10 = match shut_s { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(shut_s_10);
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(shut_w_9)));
    };
    if matches!(shut_s, crate::unions::Union2::U2(_)) {
        let mut shut_s_11 = match shut_s { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs::fs_stream_error(shut_s_11)));
    };
    let mut moved_12 = match &moved { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
    return crate::unions::Union2::U1(crate::core_result::ok::<i64>(moved_12));
}
