use crate::core_bytes::*;
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

#[derive(Clone, Debug, PartialEq)]
pub struct PermissionDenied {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlreadyExists {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotADirectory {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathEscapes {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidUtf8 {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaleHandle {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IoError {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FsError {
    pub kind: Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>,
}

pub fn ignore__2(e: FsError) {
    drop(e);
}

pub fn detach(e: FsError) -> Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError> {
    let mut kind = e.kind.clone();
    drop(e);
    return kind;
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

pub fn to_str__2(e: &FsError) -> String {
    return to_str(&e.kind);
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileInfo {
    pub size: i64,
    pub is_dir: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InStream {
    pub handle: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutStream {
    pub handle: i64,
}

pub trait Fs {
    fn open_read(&mut self, path: &String) -> Union2<InStream, FsError>;
    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, FsError>;
    fn open_write(&mut self, path: &String) -> Union2<OutStream, FsError>;
    fn open_append(&mut self, path: &String) -> Union2<OutStream, FsError>;
    fn exists(&mut self, path: &String) -> bool;
    fn metadata(&mut self, path: &String) -> Union2<FileInfo, FsError>;
    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, FsError>;
    fn create_dirs(&mut self, path: &String) -> Union2<(), FsError>;
    fn delete(&mut self, path: &String) -> Union2<(), FsError>;
    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), FsError>;
    fn read_line(&mut self, s: &InStream) -> Option<String>;
    fn read_all(&mut self, s: &InStream) -> Union2<String, FsError>;
    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, FsError>;
    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, FsError>;
    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, FsError>;
    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool;
    fn position(&mut self, s: &InStream) -> i64;
    fn close(&mut self, s: InStream) -> Union2<(), FsError>;
    fn write(&mut self, s: &OutStream, text: &String) -> i64;
    fn write_line(&mut self, s: &OutStream, text: &String) -> i64;
    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64;
    fn position__2(&mut self, s: &OutStream) -> i64;
    fn flush(&mut self, s: &OutStream) -> Union2<(), FsError>;
    fn close__2(&mut self, s: OutStream) -> Union2<(), FsError>;
}

pub trait __Share_Fs: Fs + Send {
    fn __clone_box(&self) -> Box<dyn __Share_Fs>;
}

impl<__H: Fs + Clone + Send + 'static> __Share_Fs for __H {
    fn __clone_box(&self) -> Box<dyn __Share_Fs> {
        Box::new(self.clone())
    }
}

pub struct __Mon_Fs {
    inner: Box<dyn __Share_Fs>,
}

impl Clone for __Mon_Fs {
    fn clone(&self) -> Self {
        Self { inner: self.inner.__clone_box() }
    }
}

impl __Mon_Fs {
    pub fn new(inner: Box<dyn __Share_Fs>) -> Self {
        Self { inner }
    }
}

impl Fs for __Mon_Fs {
    fn open_read(&mut self, path: &String) -> Union2<InStream, FsError> {
        self.inner.open_read(path)
    }
    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, FsError> {
        self.inner.open_read_at(path, offset)
    }
    fn open_write(&mut self, path: &String) -> Union2<OutStream, FsError> {
        self.inner.open_write(path)
    }
    fn open_append(&mut self, path: &String) -> Union2<OutStream, FsError> {
        self.inner.open_append(path)
    }
    fn exists(&mut self, path: &String) -> bool {
        self.inner.exists(path)
    }
    fn metadata(&mut self, path: &String) -> Union2<FileInfo, FsError> {
        self.inner.metadata(path)
    }
    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, FsError> {
        self.inner.list_dir(path)
    }
    fn create_dirs(&mut self, path: &String) -> Union2<(), FsError> {
        self.inner.create_dirs(path)
    }
    fn delete(&mut self, path: &String) -> Union2<(), FsError> {
        self.inner.delete(path)
    }
    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), FsError> {
        self.inner.rename_path(from, to)
    }
    fn read_line(&mut self, s: &InStream) -> Option<String> {
        self.inner.read_line(s)
    }
    fn read_all(&mut self, s: &InStream) -> Union2<String, FsError> {
        self.inner.read_all(s)
    }
    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, FsError> {
        self.inner.read_bytes(s, max)
    }
    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, FsError> {
        self.inner.read_to(s, buf, max)
    }
    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, FsError> {
        self.inner.read_to__2(s, buf)
    }
    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        self.inner.read_line_to(s, buf)
    }
    fn position(&mut self, s: &InStream) -> i64 {
        self.inner.position(s)
    }
    fn close(&mut self, s: InStream) -> Union2<(), FsError> {
        self.inner.close(s)
    }
    fn write(&mut self, s: &OutStream, text: &String) -> i64 {
        self.inner.write(s, text)
    }
    fn write_line(&mut self, s: &OutStream, text: &String) -> i64 {
        self.inner.write_line(s, text)
    }
    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64 {
        self.inner.write_bytes(s, data)
    }
    fn position__2(&mut self, s: &OutStream) -> i64 {
        self.inner.position__2(s)
    }
    fn flush(&mut self, s: &OutStream) -> Union2<(), FsError> {
        self.inner.flush(s)
    }
    fn close__2(&mut self, s: OutStream) -> Union2<(), FsError> {
        self.inner.close__2(s)
    }
}

pub struct __Lock_Fs<H> {
    inner: std::sync::Arc<std::sync::Mutex<H>>,
}

impl<H> Clone for __Lock_Fs<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<H> __Lock_Fs<H> {
    pub fn new(inner: H) -> Self {
        Self { inner: std::sync::Arc::new(std::sync::Mutex::new(inner)) }
    }
}

impl<H: Fs + Send> Fs for __Lock_Fs<H> {
    fn open_read(&mut self, path: &String) -> Union2<InStream, FsError> {
        self.inner.lock().unwrap().open_read(path)
    }
    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, FsError> {
        self.inner.lock().unwrap().open_read_at(path, offset)
    }
    fn open_write(&mut self, path: &String) -> Union2<OutStream, FsError> {
        self.inner.lock().unwrap().open_write(path)
    }
    fn open_append(&mut self, path: &String) -> Union2<OutStream, FsError> {
        self.inner.lock().unwrap().open_append(path)
    }
    fn exists(&mut self, path: &String) -> bool {
        self.inner.lock().unwrap().exists(path)
    }
    fn metadata(&mut self, path: &String) -> Union2<FileInfo, FsError> {
        self.inner.lock().unwrap().metadata(path)
    }
    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, FsError> {
        self.inner.lock().unwrap().list_dir(path)
    }
    fn create_dirs(&mut self, path: &String) -> Union2<(), FsError> {
        self.inner.lock().unwrap().create_dirs(path)
    }
    fn delete(&mut self, path: &String) -> Union2<(), FsError> {
        self.inner.lock().unwrap().delete(path)
    }
    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), FsError> {
        self.inner.lock().unwrap().rename_path(from, to)
    }
    fn read_line(&mut self, s: &InStream) -> Option<String> {
        self.inner.lock().unwrap().read_line(s)
    }
    fn read_all(&mut self, s: &InStream) -> Union2<String, FsError> {
        self.inner.lock().unwrap().read_all(s)
    }
    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, FsError> {
        self.inner.lock().unwrap().read_bytes(s, max)
    }
    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, FsError> {
        self.inner.lock().unwrap().read_to(s, buf, max)
    }
    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, FsError> {
        self.inner.lock().unwrap().read_to__2(s, buf)
    }
    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        self.inner.lock().unwrap().read_line_to(s, buf)
    }
    fn position(&mut self, s: &InStream) -> i64 {
        self.inner.lock().unwrap().position(s)
    }
    fn close(&mut self, s: InStream) -> Union2<(), FsError> {
        self.inner.lock().unwrap().close(s)
    }
    fn write(&mut self, s: &OutStream, text: &String) -> i64 {
        self.inner.lock().unwrap().write(s, text)
    }
    fn write_line(&mut self, s: &OutStream, text: &String) -> i64 {
        self.inner.lock().unwrap().write_line(s, text)
    }
    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64 {
        self.inner.lock().unwrap().write_bytes(s, data)
    }
    fn position__2(&mut self, s: &OutStream) -> i64 {
        self.inner.lock().unwrap().position__2(s)
    }
    fn flush(&mut self, s: &OutStream) -> Union2<(), FsError> {
        self.inner.lock().unwrap().flush(s)
    }
    fn close__2(&mut self, s: OutStream) -> Union2<(), FsError> {
        self.inner.lock().unwrap().close__2(s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lines {
    pub s: InStream,
}

pub fn lines(s: InStream) -> Lines {
    return Lines { s: s };
}

pub fn next__3(fs: &mut dyn Fs, p: &mut Lines) -> Union2<String, Finished> {
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

pub fn close(fs: &mut dyn Fs, p: Lines) -> Union2<(), FsError> {
    return fs.close(p.s);
}

pub fn open_lines(fs: &mut dyn Fs, path: &String) -> Union2<Lines, FsError> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Lines, FsError>::U2(opened.u2().clone());
    }
    return Union2::<Lines, FsError>::U1(ok(lines(opened.u1().clone())));
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunks {
    pub s: InStream,
    pub size: i32,
}

pub fn chunks(s: InStream, size: i32) -> Chunks {
    return Chunks { s: s, size: size };
}

pub fn next__4(fs: &mut dyn Fs, p: &mut Chunks) -> Union2<Vec<u8>, Finished> {
    let mut got = fs.read_bytes(&p.s, p.size);
    if matches!(got, Union2::U2(_)) {
        ignore__2(got.u2().clone());
        return Union2::<Vec<u8>, Finished>::U2(finished());
    }
    let mut data: Vec<u8> = got.u1().clone();
    if ((data.len() as i32) == 0) {
        return Union2::<Vec<u8>, Finished>::U2(finished());
    }
    return Union2::<Vec<u8>, Finished>::U1(emitted(data));
}

pub fn close__2(fs: &mut dyn Fs, p: Chunks) -> Union2<(), FsError> {
    return fs.close(p.s);
}

pub fn open_chunks(fs: &mut dyn Fs, path: &String, size: i32) -> Union2<Chunks, FsError> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Chunks, FsError>::U2(opened.u2().clone());
    }
    return Union2::<Chunks, FsError>::U1(ok(chunks(opened.u1().clone(), size)));
}

pub fn read_to_str(fs: &mut dyn Fs, path: &String) -> Union2<String, FsError> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<String, FsError>::U2(opened.u2().clone());
    }
    let mut s: InStream = opened.u1().clone();
    let mut content = fs.read_all(&s);
    if matches!(content, Union2::U2(_)) {
        let mut closed = fs.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore__2(closed.u2().clone());
        }
        return Union2::<String, FsError>::U2(content.u2().clone());
    }
    let mut closed = fs.close(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<String, FsError>::U2(closed.u2().clone());
    }
    return Union2::<String, FsError>::U1(content.u1().clone());
}

pub fn read_lines(fs: &mut dyn Fs, path: &String) -> Union2<Vec<String>, FsError> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Vec<String>, FsError>::U2(opened.u2().clone());
    }
    let mut p = lines(opened.u1().clone());
    let mut out: Vec<String> = vec![];
    while let Union2::U1(mut line) = next__3(fs, &mut p) {
        out.push(line);
    }
    let mut closed = close(fs, p);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<Vec<String>, FsError>::U2(closed.u2().clone());
    }
    let mut done: Vec<String> = out;
    return Union2::<Vec<String>, FsError>::U1(ok(done));
}

pub fn write_str(fs: &mut dyn Fs, path: &String, content: &String) -> Union2<i64, FsError> {
    let mut opened = fs.open_write(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, FsError>::U2(opened.u2().clone());
    }
    let mut s: OutStream = opened.u1().clone();
    let mut written = fs.write(&s, content);
    let mut closed = fs.close__2(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<i64, FsError>::U2(closed.u2().clone());
    }
    return Union2::<i64, FsError>::U1(ok(written));
}

pub fn read_to_bytes(fs: &mut dyn Fs, path: &String) -> Union2<Vec<u8>, FsError> {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<Vec<u8>, FsError>::U2(opened.u2().clone());
    }
    let mut s: InStream = opened.u1().clone();
    let mut buf = Vec::<u8>::new();
    let mut filling = fill_from(fs, &s, &mut buf);
    if matches!(filling, Union2::U2(_)) {
        let mut closed = fs.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore__2(closed.u2().clone());
        }
        return Union2::<Vec<u8>, FsError>::U2(filling.u2().clone());
    }
    let mut closed = fs.close(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<Vec<u8>, FsError>::U2(closed.u2().clone());
    }
    let mut done: Vec<u8> = buf;
    return Union2::<Vec<u8>, FsError>::U1(ok(done));
}

pub fn write_bytes_to(fs: &mut dyn Fs, path: &String, data: &Vec<u8>) -> Union2<i64, FsError> {
    let mut opened = fs.open_write(path);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, FsError>::U2(opened.u2().clone());
    }
    let mut s: OutStream = opened.u1().clone();
    let mut written = fs.write_bytes(&s, data);
    let mut closed = fs.close__2(s);
    if matches!(closed, Union2::U2(_)) {
        return Union2::<i64, FsError>::U2(closed.u2().clone());
    }
    return Union2::<i64, FsError>::U1(ok(written));
}

pub fn fs_chunk_size() -> i32 {
    return 65536;
}

pub fn fill_from(fs: &mut dyn Fs, s: &InStream, buf: &mut Vec<u8>) -> Union2<i64, FsError> {
    let mut total: i64 = 0i64;
    let mut reading = true;
    while reading {
        let mut got = fs.read_to(s, buf, fs_chunk_size());
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, FsError>::U2(got.u2().clone());
        }
        let mut n: i32 = *got.u1();
        total = total + ((n) as i64);
        if n == 0 {
            reading = false;
        }
    }
    return Union2::<i64, FsError>::U1(ok(total));
}

pub fn copy_stream(fs: &mut dyn Fs, s: &InStream, w: &OutStream) -> Union2<i64, FsError> {
    let mut buf = Vec::<u8>::new();
    let mut total: i64 = 0i64;
    let mut copying = true;
    while copying {
        buf.clear();
        let mut got = fs.read_to(s, &mut buf, fs_chunk_size());
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, FsError>::U2(got.u2().clone());
        }
        let mut n: i32 = *got.u1();
        if n == 0 {
            copying = false;
        } else {
            total = total + fs.write_bytes(w, &buf);
        }
    }
    return Union2::<i64, FsError>::U1(ok(total));
}

pub fn copy_file(fs: &mut dyn Fs, from: &String, to: &String) -> Union2<i64, FsError> {
    let mut opened = fs.open_read(from);
    if matches!(opened, Union2::U2(_)) {
        return Union2::<i64, FsError>::U2(opened.u2().clone());
    }
    let mut s: InStream = opened.u1().clone();
    let mut created = fs.open_write(to);
    if matches!(created, Union2::U2(_)) {
        let mut closed = fs.close(s);
        if matches!(closed, Union2::U2(_)) {
            ignore__2(closed.u2().clone());
        }
        return Union2::<i64, FsError>::U2(created.u2().clone());
    }
    let mut w: OutStream = created.u1().clone();
    let mut moved = copy_stream(fs, &s, &w);
    let mut shut_w = fs.close__2(w);
    let mut shut_s = fs.close(s);
    if matches!(moved, Union2::U2(_)) {
        if matches!(shut_w, Union2::U2(_)) {
            ignore__2(shut_w.u2().clone());
        }
        if matches!(shut_s, Union2::U2(_)) {
            ignore__2(shut_s.u2().clone());
        }
        return Union2::<i64, FsError>::U2(moved.u2().clone());
    }
    if matches!(shut_w, Union2::U2(_)) {
        if matches!(shut_s, Union2::U2(_)) {
            ignore__2(shut_s.u2().clone());
        }
        return Union2::<i64, FsError>::U2(shut_w.u2().clone());
    }
    if matches!(shut_s, Union2::U2(_)) {
        return Union2::<i64, FsError>::U2(shut_s.u2().clone());
    }
    return Union2::<i64, FsError>::U1(*moved.u1());
}
