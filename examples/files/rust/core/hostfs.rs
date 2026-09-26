use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_fs::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_nonempty::*;
use crate::core_result::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

pub trait RawFs {
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

pub trait __Has_RawFs {
    fn __get_RawFs(&mut self) -> &mut dyn RawFs;
}

pub trait __Share_RawFs: RawFs + Send {
    fn __clone_box(&self) -> Box<dyn __Share_RawFs>;
}

impl<__H: RawFs + Clone + Send + 'static> __Share_RawFs for __H {
    fn __clone_box(&self) -> Box<dyn __Share_RawFs> {
        Box::new(self.clone())
    }
}

pub struct __Mon_RawFs {
    inner: Box<dyn __Share_RawFs>,
}

impl Clone for __Mon_RawFs {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_RawFs {
    pub fn new(inner: Box<dyn __Share_RawFs>) -> Self {
        Self { inner }
    }
}

impl RawFs for __Mon_RawFs {
    fn raw_open_read(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_open_read(path)
    }
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_open_read_at(path, offset)
    }
    fn raw_open_write(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_open_write(path)
    }
    fn raw_open_append(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_open_append(path)
    }
    fn raw_exists(&mut self, path: &String) -> bool {
        self.inner.raw_exists(path)
    }
    fn raw_metadata(&mut self, path: &String) -> Union2<FileInfo, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_metadata(path)
    }
    fn raw_list_dir(&mut self, path: &String) -> Union2<Vec<String>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_list_dir(path)
    }
    fn raw_create_dirs(&mut self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_create_dirs(path)
    }
    fn raw_delete(&mut self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_delete(path)
    }
    fn raw_rename_path(&mut self, from: &String, to: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_rename_path(from, to)
    }
    fn raw_read_line(&mut self, handle: i64) -> Option<String> {
        self.inner.raw_read_line(handle)
    }
    fn raw_read_all(&mut self, handle: i64) -> Union2<String, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_read_all(handle)
    }
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> Union2<Vec<u8>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_read_bytes(handle, max)
    }
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_read_to_bytes(handle, buf, max)
    }
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_read_to_str(handle, buf)
    }
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool {
        self.inner.raw_read_line_to_str(handle, buf)
    }
    fn raw_read_position(&mut self, handle: i64) -> i64 {
        self.inner.raw_read_position(handle)
    }
    fn raw_close_read(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_close_read(handle)
    }
    fn raw_write(&mut self, handle: i64, text: &String) -> i64 {
        self.inner.raw_write(handle, text)
    }
    fn raw_write_bytes(&mut self, handle: i64, data: &Vec<u8>) -> i64 {
        self.inner.raw_write_bytes(handle, data)
    }
    fn raw_write_position(&mut self, handle: i64) -> i64 {
        self.inner.raw_write_position(handle)
    }
    fn raw_flush(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_flush(handle)
    }
    fn raw_close_write(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.raw_close_write(handle)
    }
}

pub struct __Lock_RawFs<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_RawFs<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_RawFs<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: RawFs + Send> RawFs for __Lock_RawFs<H> {
    fn raw_open_read(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_open_read(path)
    }
    fn raw_open_read_at(&mut self, path: &String, offset: i64) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_open_read_at(path, offset)
    }
    fn raw_open_write(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_open_write(path)
    }
    fn raw_open_append(&mut self, path: &String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_open_append(path)
    }
    fn raw_exists(&mut self, path: &String) -> bool {
        self.inner.lock().unwrap().raw_exists(path)
    }
    fn raw_metadata(&mut self, path: &String) -> Union2<FileInfo, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_metadata(path)
    }
    fn raw_list_dir(&mut self, path: &String) -> Union2<Vec<String>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_list_dir(path)
    }
    fn raw_create_dirs(&mut self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_create_dirs(path)
    }
    fn raw_delete(&mut self, path: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_delete(path)
    }
    fn raw_rename_path(&mut self, from: &String, to: &String) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_rename_path(from, to)
    }
    fn raw_read_line(&mut self, handle: i64) -> Option<String> {
        self.inner.lock().unwrap().raw_read_line(handle)
    }
    fn raw_read_all(&mut self, handle: i64) -> Union2<String, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_read_all(handle)
    }
    fn raw_read_bytes(&mut self, handle: i64, max: i32) -> Union2<Vec<u8>, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_read_bytes(handle, max)
    }
    fn raw_read_to_bytes(&mut self, handle: i64, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_read_to_bytes(handle, buf, max)
    }
    fn raw_read_to_str(&mut self, handle: i64, buf: &mut String) -> Union2<i64, Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_read_to_str(handle, buf)
    }
    fn raw_read_line_to_str(&mut self, handle: i64, buf: &mut String) -> bool {
        self.inner.lock().unwrap().raw_read_line_to_str(handle, buf)
    }
    fn raw_read_position(&mut self, handle: i64) -> i64 {
        self.inner.lock().unwrap().raw_read_position(handle)
    }
    fn raw_close_read(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_close_read(handle)
    }
    fn raw_write(&mut self, handle: i64, text: &String) -> i64 {
        self.inner.lock().unwrap().raw_write(handle, text)
    }
    fn raw_write_bytes(&mut self, handle: i64, data: &Vec<u8>) -> i64 {
        self.inner.lock().unwrap().raw_write_bytes(handle, data)
    }
    fn raw_write_position(&mut self, handle: i64) -> i64 {
        self.inner.lock().unwrap().raw_write_position(handle)
    }
    fn raw_flush(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_flush(handle)
    }
    fn raw_close_write(&mut self, handle: i64) -> Union2<(), Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
        self.inner.lock().unwrap().raw_close_write(handle)
    }
}

impl __Has_RawFs for __Mon_RawFs {
    fn __get_RawFs(&mut self) -> &mut dyn RawFs {
        self
    }
}

#[derive(Clone)]
pub struct DefaultFs {
    __dep_RawFs: crate::core_hostfs::__Mon_RawFs,
}

impl DefaultFs {
    pub fn new(__dep_RawFs: crate::core_hostfs::__Mon_RawFs) -> Self {
        Self {
            __dep_RawFs,
        }
    }
}

impl Fs for DefaultFs {

    fn open_read(&mut self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_open_read(path);
        match r {
            Union2::U1(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(InStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_open_read_at(path, offset);
        match r {
            Union2::U1(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(InStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_write(&mut self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_open_write(path);
        match r {
            Union2::U1(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(OutStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn open_append(&mut self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_open_append(path);
        match r {
            Union2::U1(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(OutStream { handle: *r.u1() }));
            }
            Union2::U2(_) => {
                return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn exists(&mut self, path: &String) -> bool {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_exists(path);
    }

    fn metadata(&mut self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_metadata(path);
        match r {
            Union2::U1(_) => {
                return Union2::<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_list_dir(path);
        match r {
            Union2::U1(_) => {
                return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn create_dirs(&mut self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_create_dirs(path);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn delete(&mut self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_delete(path);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_rename_path(from, to);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_line(&mut self, s: &InStream) -> Option<String> {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_line(s.handle);
    }

    fn read_all(&mut self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_all(s.handle);
        match r {
            Union2::U1(_) => {
                return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_bytes(s.handle, max);
        match r {
            Union2::U1(_) => {
                return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(r.u1().clone()));
            }
            Union2::U2(_) => {
                return Union2::<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_to_bytes(s.handle, buf, max);
        match r {
            Union2::U1(_) => {
                return Union2::<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(*r.u1()));
            }
            Union2::U2(_) => {
                return Union2::<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_to_str(s.handle, buf);
        match r {
            Union2::U1(_) => {
                return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(*r.u1()));
            }
            Union2::U2(_) => {
                return Union2::<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_line_to_str(s.handle, buf);
    }

    fn position(&mut self, s: &InStream) -> i64 {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_read_position(s.handle);
    }

    fn close(&mut self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_close_read(s.handle);
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

    fn write(&mut self, s: &OutStream, text: &String) -> i64 {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_write(s.handle, text);
    }

    fn write_line(&mut self, s: &OutStream, text: &String) -> i64 {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_write(s.handle, &(format!("{}\n", text.clone())));
    }

    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64 {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_write_bytes(s.handle, data);
    }

    fn position__2(&mut self, s: &OutStream) -> i64 {
        return __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_write_position(s.handle);
    }

    fn flush(&mut self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_flush(s.handle);
        match r {
            Union2::U1(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U1(ok(()));
            }
            Union2::U2(_) => {
                return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(checked(r.u2().clone())));
            }
        }
    }

    fn close__2(&mut self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut r = __Has_RawFs::__get_RawFs(&mut self.__dep_RawFs).raw_close_write(s.handle);
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
