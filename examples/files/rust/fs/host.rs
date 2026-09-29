use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_result::*;
use crate::core_string::*;
use crate::fs::*;
use crate::stream::*;
use crate::unions::*;

pub trait __Stateless_RawFs: Send + Sync {
    fn raw_open_read(&self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_open_read_at(&self, path: &String, offset: i64) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_open_write(&self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_open_append(&self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_exists(&self, path: &String) -> bool;
    fn raw_metadata(&self, path: &String) -> Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_list_dir(&self, path: &String) -> Union2<Vec<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_create_dirs(&self, path: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_delete(&self, path: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_rename_path(&self, from: &String, to: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
}

pub trait __Stateful_RawFs: Send {
    fn raw_open_read(&mut self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_open_write(&mut self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_open_append(&mut self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_exists(&mut self, path: &String) -> bool;
    fn raw_metadata(&mut self, path: &String) -> Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_list_dir(&mut self, path: &String) -> Union2<Vec<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_create_dirs(&mut self, path: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_delete(&mut self, path: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
    fn raw_rename_path(&mut self, from: &String, to: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>;
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
    pub fn raw_open_read(&self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read(path),
        }
    }
    pub fn raw_open_read_at(&self, path: &String, offset: i64) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_read_at(path, offset),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_read_at(path, offset),
        }
    }
    pub fn raw_open_write(&self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_open_write(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_open_write(path),
        }
    }
    pub fn raw_open_append(&self, path: &String) -> Union2<i64, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
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
    pub fn raw_metadata(&self, path: &String) -> Union2<FileInfo, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_metadata(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_metadata(path),
        }
    }
    pub fn raw_list_dir(&self, path: &String) -> Union2<Vec<String>, Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_list_dir(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_list_dir(path),
        }
    }
    pub fn raw_create_dirs(&self, path: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_create_dirs(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_create_dirs(path),
        }
    }
    pub fn raw_delete(&self, path: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_delete(path),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_delete(path),
        }
    }
    pub fn raw_rename_path(&self, from: &String, to: &String) -> Union2<(), Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
        match &self.inner {
            __Inner_RawFs::Shared(h) => h.raw_rename_path(from, to),
            __Inner_RawFs::Locked(h) => h.lock().unwrap().raw_rename_path(from, to),
        }
    }
}

#[derive(Clone)]
pub struct DefaultFs {
    __dep_RawFs: crate::fs_host::RawFs,
    __dep_Streams: crate::stream::Streams,
}

impl DefaultFs {
    pub fn new(__dep_RawFs: crate::fs_host::RawFs, __dep_Streams: crate::stream::Streams) -> Self {
        Self {
            __dep_RawFs,
            __dep_Streams,
        }
    }
}

impl crate::fs::__Stateless_Fs for DefaultFs {

    fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_open_read(path);
        match r {
            Union2::U1(_) => {
                return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(InStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_open_read_at(path, offset);
        match r {
            Union2::U1(_) => {
                return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(InStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_open_write(path);
        match r {
            Union2::U1(_) => {
                return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(OutStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_open_append(path);
        match r {
            Union2::U1(_) => {
                return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(OutStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn exists(&self, path: &String) -> bool {
        return self.__dep_RawFs.raw_exists(path);
    }

    fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_metadata(path);
        match r {
            Union2::U1(_) => {
                return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_list_dir(path);
        match r {
            Union2::U1(_) => {
                return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_create_dirs(path);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn delete(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_delete(path);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut r = self.__dep_RawFs.raw_rename_path(from, to);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }
}
