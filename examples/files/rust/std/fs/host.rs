use crate::std_fs::AlreadyExists;
use crate::std_core_checked::Checked;
use crate::std_fs::FileInfo;
use crate::std_stream::InStream;
use crate::std_fs::IoError;
use crate::std_fs::NotADirectory;
use crate::std_fs::NotFound;
use crate::std_stream::OutStream;
use crate::std_fs_path::Path;
use crate::std_fs::PathEscapes;
use crate::std_fs::PermissionDenied;
use crate::std_fs::Streaming;
use crate::std_stream::Streams;
use crate::std_core_checked::checked;
use crate::std_fs_path::to_str;


pub trait __Stateless_RawFs: Send + Sync {
    fn raw_open_read(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_read_at(&self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_write(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_append(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_exists(&self, path: &String) -> bool;
    fn raw_metadata(&self, path: &String) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_list_dir(&self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_create_dirs(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_delete(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_rename_path(&self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
}

pub trait __Stateful_RawFs: Send {
    fn raw_open_read(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_write(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_append(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_exists(&mut self, path: &String) -> bool;
    fn raw_metadata(&mut self, path: &String) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_list_dir(&mut self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_create_dirs(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_delete(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_rename_path(&mut self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
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
    pub fn raw_open_read(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read(path),
        }
    }
    pub fn raw_open_read_at(&self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read_at(path, offset),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read_at(path, offset),
        }
    }
    pub fn raw_open_write(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_write(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_write(path),
        }
    }
    pub fn raw_open_append(&self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
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
    pub fn raw_metadata(&self, path: &String) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_metadata(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_metadata(path),
        }
    }
    pub fn raw_list_dir(&self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_list_dir(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_list_dir(path),
        }
    }
    pub fn raw_create_dirs(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_create_dirs(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_create_dirs(path),
        }
    }
    pub fn raw_delete(&self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_delete(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_delete(path),
        }
    }
    pub fn raw_rename_path(&self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
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
    fn raw_open_read(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_write(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_open_append(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_exists(&mut self, path: &String) -> bool;
    fn raw_metadata(&mut self, path: &String) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_list_dir(&mut self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_create_dirs(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_delete(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
    fn raw_rename_path(&mut self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>;
}

impl<T: RawFsPlatform> __Stateful_RawFs for __Platform_RawFs<T> {
    fn raw_open_read(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_open_read(path)
    }
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_open_read_at(path, offset)
    }
    fn raw_open_write(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_open_write(path)
    }
    fn raw_open_append(&mut self, path: &String) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_open_append(path)
    }
    fn raw_exists(&mut self, path: &String) -> bool {
        self.0.raw_exists(path)
    }
    fn raw_metadata(&mut self, path: &String) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_metadata(path)
    }
    fn raw_list_dir(&mut self, path: &String) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_list_dir(path)
    }
    fn raw_create_dirs(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_create_dirs(path)
    }
    fn raw_delete(&mut self, path: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_delete(path)
    }
    fn raw_rename_path(&mut self, from: &String, to: &String) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        self.0.raw_rename_path(from, to)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenRead;

impl RawOpenRead {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenReadAt;

impl RawOpenReadAt {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenWrite;

impl RawOpenWrite {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawOpenAppend;

impl RawOpenAppend {
    pub fn ok(value: i64) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawMetadata;

impl RawMetadata {
    pub fn ok(value: crate::std_fs::FileInfo) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawListDir;

impl RawListDir {
    pub fn ok(value: Vec<String>) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawCreateDirs;

impl RawCreateDirs {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawDelete;

impl RawDelete {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

/// Factories for the host: one per arm of the union [platform-factory].
pub struct RawRenamePath;

impl RawRenamePath {
    pub fn ok(value: ()) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U1(value)
    }
    pub fn err(value: crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>) -> crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
        crate::unions::Union2::U2(value)
    }
}

pub type __Platform_HostRawFs = crate::std_fs_host::__Platform_RawFs<crate::platform_std_fs_host::HostRawFs>;

impl __Platform_HostRawFs {
    pub fn new() -> Self {
        crate::std_fs_host::__Platform_RawFs(crate::platform_std_fs_host::HostRawFs::new())
    }
}

#[derive(Clone)]
pub struct DefaultFs {
    __dep0: crate::std_fs_host::RawFs,
    __dep1: crate::std_stream::Streams,
}

impl DefaultFs {
    pub fn new(__dep0: crate::std_fs_host::RawFs, __dep1: crate::std_stream::Streams) -> Self {
        Self {
            __dep0,
            __dep1
        }
    }
}

impl crate::std_fs::__Stateless_Fs for DefaultFs {
    fn open_read(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_open_read(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<crate::std_stream::InStream>(crate::std_stream::InStream { handle: r_1 }));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:63:9");
        };
    }
    fn open_read_at(&self, path: &crate::std_fs_path::Path, offset: i64) -> crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_open_read_at(&path_text, offset);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<crate::std_stream::InStream>(crate::std_stream::InStream { handle: r_1 }));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:72:9");
        };
    }
    fn open_write(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_stream::OutStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_open_write(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<crate::std_stream::OutStream>(crate::std_stream::OutStream { handle: r_1 }));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:81:9");
        };
    }
    fn open_append(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_stream::OutStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<i64, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_open_append(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<crate::std_stream::OutStream>(crate::std_stream::OutStream { handle: r_1 }));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:90:9");
        };
    }
    fn exists(&self, path: &crate::std_fs_path::Path) -> bool {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        return self.__dep0.raw_exists(&path_text);
    }
    fn metadata(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<crate::std_fs::FileInfo, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_metadata(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<crate::std_fs::FileInfo>(r_1));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:104:9");
        };
    }
    fn list_dir(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<Vec<String>, crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_list_dir(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<Vec<String>>(r_1));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:113:9");
        };
    }
    fn create_dirs(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_create_dirs(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<()>(()));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:122:9");
        };
    }
    fn delete(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut r: crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_delete(&path_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<()>(()));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:131:9");
        };
    }
    fn rename_path(&self, from: &crate::std_fs_path::Path, to: &crate::std_fs_path::Path) -> crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut from_text: String = crate::std_fs_path::to_str(from);
        let mut to_text: String = crate::std_fs_path::to_str(to);
        let mut r: crate::unions::Union2<(), crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = self.__dep0.raw_rename_path(&from_text, &to_text);
        return if matches!(r, crate::unions::Union2::U1(_)) {
            let mut r_1 = match &r { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U1(crate::std_core_result::ok::<()>(()));
        } else if matches!(r, crate::unions::Union2::U2(_)) {
            let mut r_2 = match r { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(r_2)));
        } else {
            panic!("salvo: unhandled union arm at std.fs.host:141:9");
        };
    }
}
