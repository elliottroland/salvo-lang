use crate::fs::AlreadyExists;
use crate::core_checked::Checked;
use crate::fs::FileInfo;
use crate::stream::InStream;
use crate::fs::IoError;
use crate::fs::NotADirectory;
use crate::fs::NotFound;
use crate::stream::OutStream;
use crate::fs_path::Path;
use crate::fs::PathEscapes;
use crate::fs::PermissionDenied;
use crate::fs::Streaming;
use crate::stream::Streams;
use crate::core_checked::checked;
use crate::fs_path::to_str;


pub trait __Stateless_RawFs: Send + Sync {
    fn raw_open_read(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_read_at(&self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_write(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_append(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_exists(&self, path: &String) -> bool;
    fn raw_metadata(&self, path: &String) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_list_dir(&self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_create_dirs(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_delete(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_rename_path(&self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
}

pub trait __Stateful_RawFs: Send {
    fn raw_open_read(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_write(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_append(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_exists(&mut self, path: &String) -> bool;
    fn raw_metadata(&mut self, path: &String) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_list_dir(&mut self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_create_dirs(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_delete(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_rename_path(&mut self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
}

pub struct RawFs {
    inner: __Inner_RawFs,
}

pub enum __Inner_RawFs {
    Shared(std::sync::Arc<dyn __Stateless_RawFs>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_RawFs>>),
}

impl Clone for RawFs {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_RawFs::Shared(h) => __Inner_RawFs::Shared(h.clone()),
            __Inner_RawFs::Locked(h) => __Inner_RawFs::Locked(h.clone()),
        } }
    }
}

impl RawFs {
    pub fn shared<__H: __Stateless_RawFs + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RawFs::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_RawFs>) -> Self {
        Self { inner: __Inner_RawFs::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_RawFs + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_RawFs::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_RawFs>>) -> Self {
        Self { inner: __Inner_RawFs::Locked(inner) }
    }
    pub fn raw_open_read(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read(path),
        }
    }
    pub fn raw_open_read_at(&self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read_at(path, offset),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read_at(path, offset),
        }
    }
    pub fn raw_open_write(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_write(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_write(path),
        }
    }
    pub fn raw_open_append(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_append(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_append(path),
        }
    }
    pub fn raw_exists(&self, path: &String) -> bool {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_exists(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_exists(path),
        }
    }
    pub fn raw_metadata(&self, path: &String) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_metadata(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_metadata(path),
        }
    }
    pub fn raw_list_dir(&self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_list_dir(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_list_dir(path),
        }
    }
    pub fn raw_create_dirs(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_create_dirs(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_create_dirs(path),
        }
    }
    pub fn raw_delete(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_delete(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_delete(path),
        }
    }
    pub fn raw_rename_path(&self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_rename_path(from, to),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_rename_path(from, to),
        }
    }
}

/// The adapter a `use` of a platform handler of `RawFs` constructs [platform-abi].
pub struct __Platform_RawFs<T>(pub T);

/// What a `platform handler` of `RawFs` implements [platform-abi].
pub trait RawFsPlatform: Send {
    fn raw_open_read(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_write(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_open_append(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_exists(&mut self, path: &String) -> bool;
    fn raw_metadata(&mut self, path: &String) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_list_dir(&mut self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_create_dirs(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_delete(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
    fn raw_rename_path(&mut self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>;
}

impl<T: RawFsPlatform> __Stateful_RawFs for __Platform_RawFs<T> {
    fn raw_open_read(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_open_read(path)
    }
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_open_read_at(path, offset)
    }
    fn raw_open_write(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_open_write(path)
    }
    fn raw_open_append(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_open_append(path)
    }
    fn raw_exists(&mut self, path: &String) -> bool {
        self.0.raw_exists(path)
    }
    fn raw_metadata(&mut self, path: &String) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_metadata(path)
    }
    fn raw_list_dir(&mut self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_list_dir(path)
    }
    fn raw_create_dirs(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_create_dirs(path)
    }
    fn raw_delete(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_delete(path)
    }
    fn raw_rename_path(&mut self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        self.0.raw_rename_path(from, to)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenRead;

impl RawOpenRead {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenReadAt;

impl RawOpenReadAt {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenWrite;

impl RawOpenWrite {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenAppend;

impl RawOpenAppend {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawMetadata;

impl RawMetadata {
    pub fn ok(value: crate::fs::FileInfo) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawListDir;

impl RawListDir {
    pub fn ok(value: Vec<String>) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawCreateDirs;

impl RawCreateDirs {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawDelete;

impl RawDelete {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawRenamePath;

impl RawRenamePath {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

pub type __Platform_HostRawFs = crate::fs_host::__Platform_RawFs<crate::platform_fs_host::HostRawFs>;

impl __Platform_HostRawFs {
    pub fn new() -> Self {
        crate::fs_host::__Platform_RawFs(crate::platform_fs_host::HostRawFs::new())
    }
}

#[derive(Clone)]
pub struct DefaultFs {
    __dep0: crate::fs_host::RawFs,
    __dep1: crate::stream::Streams,
}

impl DefaultFs {
    pub fn new(__dep0: crate::fs_host::RawFs, __dep1: crate::stream::Streams) -> Self {
        Self {
            __dep0,
            __dep1
        }
    }
}

impl crate::fs::__Stateless_Fs for DefaultFs {
    fn open_read(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_open_read(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::InStream>(crate::stream::InStream { handle: r_1 }));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn open_read_at(&self, path: &crate::fs_path::Path, offset: i64) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_open_read_at(&path_text, offset);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::InStream>(crate::stream::InStream { handle: r_1 }));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn open_write(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_open_write(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::OutStream>(crate::stream::OutStream { handle: r_1 }));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn open_append(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_open_append(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::OutStream>(crate::stream::OutStream { handle: r_1 }));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn exists(&self, path: &crate::fs_path::Path) -> bool {
        let mut path_text: String = crate::fs_path::to_str(path);
        return self.__dep0.raw_exists(&path_text);
    }
    fn metadata(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<crate::fs::FileInfo, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_metadata(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<crate::fs::FileInfo>(r_1));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn list_dir(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_list_dir(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<Vec<String>>(r_1));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn create_dirs(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_create_dirs(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn delete(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut r: crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_delete(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
    fn rename_path(&self, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut from_text: String = crate::fs_path::to_str(from);
        let mut to_text: String = crate::fs_path::to_str(to);
        let mut r: crate::unions::Union2<(), crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = self.__dep0.raw_rename_path(&from_text, &to_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        } else {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(r_2)));
        };
    }
}
