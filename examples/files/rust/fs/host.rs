use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_result::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::fs::*;
use crate::unions::*;

pub trait __Stateless_RawFs: Send + Sync {
    fn raw_open_read(&self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_open_read_at(&self, path: &String, offset: i64) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_open_write(&self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_open_append(&self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_exists(&self, path: &String) -> bool;
    fn raw_metadata(&self, path: &String) -> Union2<FileInfo, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_list_dir(&self, path: &String) -> Union2<Vec<String>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_create_dirs(&self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_delete(&self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_rename_path(&self, from: &String, to: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_line(&self, handle: i64) -> Option<String>;
    fn raw_read_all(&self, handle: i64) -> Union2<String, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&self, handle: i64) -> i64;
    fn raw_close_read(&self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_write(&self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64;
    fn raw_write_position(&self, handle: i64) -> i64;
    fn raw_flush(&self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_close_write(&self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
}

pub trait __Stateful_RawFs: Send {
    fn raw_open_read(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_open_write(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_open_append(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_exists(&mut self, path: &String) -> bool;
    fn raw_metadata(&mut self, path: &String) -> Union2<FileInfo, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_list_dir(&mut self, path: &String) -> Union2<Vec<String>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_create_dirs(&mut self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_delete(&mut self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_rename_path(&mut self, from: &String, to: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_line(&mut self, handle: i64) -> Option<String>;
    fn raw_read_all(&mut self, handle: i64) -> Union2<String, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> Union2<Vec<u8>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool;
    fn raw_read_position(&mut self, handle: i64) -> i64;
    fn raw_close_read(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_write(&mut self, handle: i64, text: &String) -> i64;
    fn raw_write_bytes(&mut self, handle: i64, data: &Vec<u8>) -> i64;
    fn raw_write_position(&mut self, handle: i64) -> i64;
    fn raw_flush(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
    fn raw_close_write(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>;
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
    pub fn raw_open_read(&self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read(path),
        }
    }
    pub fn raw_open_read_at(&self, path: &String, offset: i64) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read_at(path, offset),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read_at(path, offset),
        }
    }
    pub fn raw_open_write(&self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_write(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_write(path),
        }
    }
    pub fn raw_open_append(&self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
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
    pub fn raw_metadata(&self, path: &String) -> Union2<FileInfo, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_metadata(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_metadata(path),
        }
    }
    pub fn raw_list_dir(&self, path: &String) -> Union2<Vec<String>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_list_dir(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_list_dir(path),
        }
    }
    pub fn raw_create_dirs(&self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_create_dirs(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_create_dirs(path),
        }
    }
    pub fn raw_delete(&self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_delete(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_delete(path),
        }
    }
    pub fn raw_rename_path(&self, from: &String, to: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_rename_path(from, to),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_rename_path(from, to),
        }
    }
    pub fn raw_read_line(&self, handle: i64) -> Option<String> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_line(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_line(handle),
        }
    }
    pub fn raw_read_all(&self, handle: i64) -> Union2<String, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_all(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_all(handle),
        }
    }
    pub fn raw_read_bytes(&self, handle: i64, max: i32) -> Union2<Vec<u8>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_bytes(handle, max),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_bytes(handle, max),
        }
    }
    pub fn raw_read_to_bytes(&self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_to_bytes(handle, buf, max),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_to_bytes(handle, buf, max),
        }
    }
    pub fn raw_read_to_str(&self, handle: i64, buf: &mut String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_to_str(handle, buf),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_to_str(handle, buf),
        }
    }
    pub fn raw_read_line_to_str(&self, handle: i64, buf: &mut String) -> bool {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_line_to_str(handle, buf),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_line_to_str(handle, buf),
        }
    }
    pub fn raw_read_position(&self, handle: i64) -> i64 {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_read_position(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_read_position(handle),
        }
    }
    pub fn raw_close_read(&self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_close_read(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_close_read(handle),
        }
    }
    pub fn raw_write(&self, handle: i64, text: &String) -> i64 {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_write(handle, text),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_write(handle, text),
        }
    }
    pub fn raw_write_bytes(&self, handle: i64, data: &Vec<u8>) -> i64 {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_write_bytes(handle, data),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_write_bytes(handle, data),
        }
    }
    pub fn raw_write_position(&self, handle: i64) -> i64 {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_write_position(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_write_position(handle),
        }
    }
    pub fn raw_flush(&self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_flush(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_flush(handle),
        }
    }
    pub fn raw_close_write(&self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_close_write(handle),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_close_write(handle),
        }
    }
}

#[derive(Clone)]
pub struct DefaultFs {
    __dep_RawFs: crate::fs_host::RawFs,
}

impl DefaultFs {
    pub fn new(__dep_RawFs: crate::fs_host::RawFs) -> Self {
        Self {
            __dep_RawFs,
        }
    }
}

impl crate::fs::__Stateless_Fs for DefaultFs {

    fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_open_read(path);
        match r {
            Union2::U1(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(InStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_open_read_at(path, offset);
        match r {
            Union2::U1(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(InStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_open_write(path);
        match r {
            Union2::U1(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(OutStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_open_append(path);
        match r {
            Union2::U1(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(OutStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn exists(&self, path: &String) -> bool {
        return self.__dep_RawFs.raw_exists(path);
    }

    fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_metadata(path);
        match r {
            Union2::U1(_) => {
                return Union2::<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_list_dir(path);
        match r {
            Union2::U1(_) => {
                return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_create_dirs(path);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn delete(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_delete(path);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_rename_path(from, to);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_line(&self, s: &InStream) -> Option<String> {
        return self.__dep_RawFs.raw_read_line(s.handle);
    }

    fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_read_all(s.handle);
        match r {
            Union2::U1(_) => {
                return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_read_bytes(s.handle, max);
        match r {
            Union2::U1(_) => {
                return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_read_to_bytes(s.handle, buf, max);
        match r {
            Union2::U1(_) => {
                return Union2::<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(*r.u1()));
            }
            Union2::U2(_) => {
                return Union2::<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_read_to_str(s.handle, buf);
        match r {
            Union2::U1(_) => {
                return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(*r.u1()));
            }
            Union2::U2(_) => {
                return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool {
        return self.__dep_RawFs.raw_read_line_to_str(s.handle, buf);
    }

    fn position(&self, s: &InStream) -> i64 {
        return self.__dep_RawFs.raw_read_position(s.handle);
    }

    fn close(&self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_close_read(s.handle);
        drop(s);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn write(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_RawFs.raw_write(s.handle, text);
    }

    fn write_line(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_RawFs.raw_write(s.handle, &(format!("{}\n", text.clone())));
    }

    fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64 {
        return self.__dep_RawFs.raw_write_bytes(s.handle, data);
    }

    fn position__2(&self, s: &OutStream) -> i64 {
        return self.__dep_RawFs.raw_write_position(s.handle);
    }

    fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_flush(s.handle);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = self.__dep_RawFs.raw_close_write(s.handle);
        drop(s);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }
}
