use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::stream::*;
use crate::unions::*;

pub type FsError = Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>;

/// Factories for the host: one per arm of the union [platform-factory].
impl FsError {
    pub fn not_found(value: NotFound) -> Self {
        crate::unions::Union7::U1(value)
    }
    pub fn permission_denied(value: PermissionDenied) -> Self {
        crate::unions::Union7::U2(value)
    }
    pub fn already_exists(value: AlreadyExists) -> Self {
        crate::unions::Union7::U3(value)
    }
    pub fn not_a_directory(value: NotADirectory) -> Self {
        crate::unions::Union7::U4(value)
    }
    pub fn path_escapes(value: PathEscapes) -> Self {
        crate::unions::Union7::U5(value)
    }
    pub fn io_error(value: IoError) -> Self {
        crate::unions::Union7::U6(value)
    }
    pub fn streaming(value: Streaming) -> Self {
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
    pub error: Union2<InvalidUtf8, StreamFailed>,
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

pub fn to_str(kind: &Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>) -> String {
    match kind {
        Union7::U1(_) => {
            return format!("no such file or directory: {}", kind.u1().clone().path.clone());
        }
        Union7::U2(_) => {
            return format!("permission denied: {}", kind.u2().clone().path.clone());
        }
        Union7::U3(_) => {
            return format!("already exists: {}", kind.u3().clone().path.clone());
        }
        Union7::U4(_) => {
            return format!("not a directory: {}", kind.u4().clone().path.clone());
        }
        Union7::U5(_) => {
            return format!("path escapes the root: {}", kind.u5().clone().path.clone());
        }
        Union7::U6(_) => {
            return format!("io error: {}: {}", kind.u6().clone().path.clone(), kind.u6().clone().message.clone());
        }
        Union7::U7(_) => {
            return to_str__4(&kind.u7().clone().error);
        }
    }
}

pub fn fs_stream_error(e: Checked<Union2<InvalidUtf8, StreamFailed>>) -> Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U7(Streaming { error: detach(e) }));
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
    fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn exists(&self, path: &String) -> bool;
    fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn delete(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
}

pub trait __Stateful_Fs: Send {
    fn open_read(&mut self, path: &String) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn open_write(&mut self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn open_append(&mut self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn exists(&mut self, path: &String) -> bool;
    fn metadata(&mut self, path: &String) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn create_dirs(&mut self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn delete(&mut self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>;
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
    pub fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_read(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_read(path),
        }
    }
    pub fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_read_at(path, offset),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_read_at(path, offset),
        }
    }
    pub fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.open_write(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().open_write(path),
        }
    }
    pub fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
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
    pub fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.metadata(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().metadata(path),
        }
    }
    pub fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.list_dir(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().list_dir(path),
        }
    }
    pub fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.create_dirs(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().create_dirs(path),
        }
    }
    pub fn delete(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.delete(path),
            __Inner_Fs::Locked(h) => h.lock().unwrap().delete(path),
        }
    }
    pub fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        match &self.inner {
            __Inner_Fs::Shared(h) => h.rename_path(from, to),
            __Inner_Fs::Locked(h) => h.lock().unwrap().rename_path(from, to),
        }
    }
}

pub fn open_lines(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String) -> Union2<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    return Union2::<Lines, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(lines((match opened { Union2::U1(__v) => __v, _ => unreachable!() }))));
}

pub fn open_chunks(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String, size: i32) -> Union2<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    return Union2::<Chunks, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(chunks((match opened { Union2::U1(__v) => __v, _ => unreachable!() }), size)));
}

pub fn read_to_str(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String) -> Union2<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut s: InStream = opened.u1().clone();
    let mut content = streams.read_all(&s);
    if matches!(content, Union2::U2(_)) {
        let mut closed = streams.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore((match closed { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        return Union2::<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match content { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    let mut closed = streams.close(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match closed { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    return Union2::<String, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(content.u1().clone()));
}

pub fn read_lines(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut p = lines((match opened { Union2::U1(__v) => __v, _ => unreachable!() }));
    let mut out: Vec<String> = vec![];
    while let Union2::U1(mut line) = next__15(streams, &mut p) {
        out.push(line);
    }
    let mut closed = close__2(streams, p);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match closed { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    let mut done: Vec<String> = out;
    return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(done));
}

pub fn write_str(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String, content: &String) -> Union2<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_write(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut s: OutStream = opened.u1().clone();
    let mut written = streams.write(&s, content);
    let mut closed = streams.close__2(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match closed { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(written));
}

pub fn read_to_bytes(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String) -> Union2<Vec<u8>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Vec<u8>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut s: InStream = opened.u1().clone();
    let mut buf = Vec::<u8>::new();
    let mut filling = fill_from(streams, &s, &mut buf);
    if matches!(filling, Union2::U2(_)) {
        let mut closed = streams.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore((match closed { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        return Union2::<Vec<u8>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match filling { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    let mut closed = streams.close(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<Vec<u8>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match closed { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    let mut done: Vec<u8> = buf;
    return Union2::<Vec<u8>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(done));
}

pub fn write_bytes_to(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String, data: &Vec<u8>) -> Union2<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_write(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut s: OutStream = opened.u1().clone();
    let mut written = streams.write_bytes(&s, data);
    let mut closed = streams.close__2(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match closed { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(written));
}

pub fn copy_file(fs: &crate::fs::Fs, streams: &crate::stream::Streams, from: &String, to: &String) -> Union2<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
    let mut opened = fs.open_read(from);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match opened { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut s: InStream = opened.u1().clone();
    let mut created = fs.open_write(to);
    if matches!(created, Union2::U2(_)) {
        let mut closed = streams.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore((match closed { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2((match created { Union2::U2(__v) => __v, _ => unreachable!() }));
    }
    let mut w: OutStream = created.u1().clone();
    let mut moved = copy_stream(streams, &s, &w);
    let mut shut_w = streams.close__2(w);
    let mut shut_s = streams.close(s);
    if matches!(moved, Union2::U2(_)) {
        if matches!(shut_w, Union2::U2(_)) {
            ignore((match shut_w { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        if matches!(shut_s, Union2::U2(_)) {
            ignore((match shut_s { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match moved { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    if matches!(shut_w, Union2::U2(_)) {
        if matches!(shut_s, Union2::U2(_)) {
            ignore((match shut_s { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match shut_w { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    if matches!(shut_s, Union2::U2(_)) {
        return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_stream_error((match shut_s { Union2::U2(__v) => __v, _ => unreachable!() }))));
    }
    return Union2::<i64, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(*moved.u1()));
}
