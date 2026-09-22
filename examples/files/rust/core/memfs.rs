use crate::collections::*;
use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_fs::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_nonempty::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MemRead {
    pub path: String,
    pub at: i32,
    pub failed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemWrite {
    pub path: String,
    pub buffer: Vec<u8>,
}

pub struct MemFs {
    files: SalvoMap<String, Vec<u8>>,
    reads: SalvoMap<i64, MemRead>,
    writes: SalvoMap<i64, MemWrite>,
    next_handle: i64,
}

impl MemFs {
    pub fn new() -> Self {
        Self {
            files: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            reads: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            writes: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            next_handle: 0i64,
        }
    }
}

impl Fs for MemFs {

    fn open_read(&mut self, path: &String) -> Union2<InStream, FsError> {
        if !self.files.contains_key(&path) {
            return Union2::<InStream, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
        }
        self.next_handle = self.next_handle + ((1) as i64);
        self.reads.insert(self.next_handle.clone(), MemRead { path: path.clone(), at: 0, failed: false });
        return Union2::<InStream, FsError>::U1(ok(InStream { handle: self.next_handle.clone() }));
    }

    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, FsError> {
        let mut content = self.files.get(&path);
        if content.is_none() {
            return Union2::<InStream, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
        }
        let mut at = ((offset) as i32);
        if at < 0 {
            at = 0;
        }
        let mut end = (content.unwrap().clone().len() as i32);
        if at > end {
            at = end;
        }
        self.next_handle = self.next_handle + ((1) as i64);
        self.reads.insert(self.next_handle.clone(), MemRead { path: path.clone(), at: at, failed: false });
        return Union2::<InStream, FsError>::U1(ok(InStream { handle: self.next_handle.clone() }));
    }

    fn open_write(&mut self, path: &String) -> Union2<OutStream, FsError> {
        self.next_handle = self.next_handle + ((1) as i64);
        let mut empty = Vec::<u8>::new();
        self.writes.insert(self.next_handle.clone(), MemWrite { path: path.clone(), buffer: empty });
        return Union2::<OutStream, FsError>::U1(ok(OutStream { handle: self.next_handle.clone() }));
    }

    fn open_append(&mut self, path: &String) -> Union2<OutStream, FsError> {
        let mut existing = self.files.get(&path);
        let mut start = Vec::<u8>::new();
        if existing.is_none() {
        } else {
            start.extend_from_slice(&existing.unwrap().clone()[..]);
        }
        self.next_handle = self.next_handle + ((1) as i64);
        self.writes.insert(self.next_handle.clone(), MemWrite { path: path.clone(), buffer: start });
        return Union2::<OutStream, FsError>::U1(ok(OutStream { handle: self.next_handle.clone() }));
    }

    fn exists(&mut self, path: &String) -> bool {
        if self.files.contains_key(&path) {
            return true;
        }
        return fs_has_children(&self.files, path);
    }

    fn metadata(&mut self, path: &String) -> Union2<FileInfo, FsError> {
        let mut content = self.files.get(&path);
        if content.is_none() {
            if fs_has_children(&self.files, path) {
                return Union2::<FileInfo, FsError>::U1(ok(FileInfo { size: 0i64, is_dir: true }));
            }
            return Union2::<FileInfo, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
        }
        return Union2::<FileInfo, FsError>::U1(ok(FileInfo { size: (((content.unwrap().clone().len() as i32)) as i64), is_dir: false }));
    }

    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, FsError> {
        if self.files.contains_key(&path) {
            return Union2::<Vec<String>, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U4(NotADirectory { path: path.clone() }) }));
        }
        if !fs_has_children(&self.files, path) {
            return Union2::<Vec<String>, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
        }
        let mut names: SalvoSet<String> = SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]);
        let mut prefix = format!("{}/", path.clone());
        for mut key in self.files.clone().keys().cloned().collect::<Vec<_>>() {
            if key.starts_with(&prefix[..]) {
                let mut rest = { let __s = &key[..]; __s.strip_prefix(&prefix[..]).unwrap_or(__s).to_string() };
                let mut cut = { let __s = &rest[..]; __s.find(&"/".to_string()[..]).map(|__b| __s[..__b].chars().count() as i32) };
                let mut name = rest.clone();
                if cut.is_some() {
                    let mut head = { let __s = &rest[..]; let __i = 0; let __j = cut.unwrap(); let __n = __s.chars().count() as i32; if __i >= 0 && __j >= __i && __j <= __n { Some(__s.chars().skip(__i as usize).take((__j - __i) as usize).collect::<String>()) } else { None } };
                    if head.is_some() {
                        name = head.as_ref().unwrap().clone();
                    }
                }
                Some(names.insert(name.clone()));
            }
        }
        let mut sorted: Vec<String> = sort::<String>(&(names.iter().cloned().collect::<Vec<_>>()), &mut |__i0, __i1| (Ord::cmp(&__i0[..], &__i1[..]) as i32));
        return Union2::<Vec<String>, FsError>::U1(ok(sorted));
    }

    fn create_dirs(&mut self, path: &String) -> Union2<(), FsError> {
        return Union2::<(), FsError>::U1(ok(()));
    }

    fn delete(&mut self, path: &String) -> Union2<(), FsError> {
        if self.files.contains_key(&path) {
            self.files.remove(&path);
            return Union2::<(), FsError>::U1(ok(()));
        }
        if fs_has_children(&self.files, path) {
            return Union2::<(), FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U8(IoError { path: path.clone(), message: "directory not empty".to_string() }) }));
        }
        return Union2::<(), FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
    }

    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), FsError> {
        let mut content = self.files.get(&from);
        if content.is_none() {
            return Union2::<(), FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: from.clone() }) }));
        }
        let mut bytes: Vec<u8> = content.unwrap().clone();
        self.files.remove(&from);
        self.files.insert(to.clone(), bytes);
        return Union2::<(), FsError>::U1(ok(()));
    }

    fn read_line(&mut self, s: &InStream) -> Option<String> {
        return mem_read_line(&mut self.reads, &self.files, s.handle);
    }

    fn read_all(&mut self, s: &InStream) -> Union2<String, FsError> {
        return mem_read_all(&mut self.reads, &self.files, s.handle);
    }

    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, FsError> {
        return mem_read_bytes(&mut self.reads, &self.files, s.handle, max);
    }

    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, FsError> {
        let mut got = mem_read_bytes(&mut self.reads, &self.files, s.handle, max);
        if matches!(got, Union2::U2(_)) {
            return Union2::<i32, FsError>::U2(got.u2().clone());
        }
        let mut data: Vec<u8> = got.u1().clone();
        buf.extend_from_slice(&data[..]);
        return Union2::<i32, FsError>::U1(ok((data.len() as i32)));
    }

    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, FsError> {
        let mut got = mem_read_all(&mut self.reads, &self.files, s.handle);
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, FsError>::U2(got.u2().clone());
        }
        let mut text: String = got.u1().clone();
        buf.push_str(&text[..]);
        return Union2::<i64, FsError>::U1(ok((text.len() as i64)));
    }

    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        let mut line = mem_read_line(&mut self.reads, &self.files, s.handle);
        match line {
            Some(_) => {
                buf.push_str(&line.as_ref().unwrap().clone()[..]);
                return true;
            }
            None => {
                return false;
            }
        }
    }

    fn position(&mut self, s: &InStream) -> i64 {
        let mut open = self.reads.get(&s.handle);
        if open.is_none() {
            return 0i64;
        }
        return ((open.unwrap().clone().at) as i64);
    }

    fn close(&mut self, s: InStream) -> Union2<(), FsError> {
        let mut open = self.reads.get(&s.handle);
        let mut failed = false;
        let mut path = "<stream>".to_string();
        if open.is_some() {
            failed = open.unwrap().clone().failed;
            path = open.unwrap().clone().path.clone();
        }
        self.reads.remove(&s.handle);
        drop(s);
        if failed {
            return Union2::<(), FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U6(InvalidUtf8 { path: path }) }));
        }
        return Union2::<(), FsError>::U1(ok(()));
    }

    fn write(&mut self, s: &OutStream, text: &String) -> i64 {
        return mem_append(&mut self.writes, s.handle, &(text.as_bytes().to_vec()));
    }

    fn write_line(&mut self, s: &OutStream, text: &String) -> i64 {
        return mem_append(&mut self.writes, s.handle, &(format!("{}\n", text.clone()).as_bytes().to_vec()));
    }

    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64 {
        return mem_append(&mut self.writes, s.handle, &(data.clone()));
    }

    fn position__2(&mut self, s: &OutStream) -> i64 {
        let mut open = self.writes.get(&s.handle);
        if open.is_none() {
            return 0i64;
        }
        return (((open.unwrap().clone().buffer.len() as i32)) as i64);
    }

    fn flush(&mut self, s: &OutStream) -> Union2<(), FsError> {
        let mut open = self.writes.get(&s.handle);
        if open.is_none() {
            return Union2::<(), FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U7(StaleHandle { path: "<stream>".to_string() }) }));
        }
        self.files.insert(open.unwrap().clone().path.clone(), open.unwrap().clone().buffer.clone());
        return Union2::<(), FsError>::U1(ok(()));
    }

    fn close__2(&mut self, s: OutStream) -> Union2<(), FsError> {
        let mut open = self.writes.get(&s.handle);
        if open.is_none() {
            drop(s);
            return Union2::<(), FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U7(StaleHandle { path: "<stream>".to_string() }) }));
        }
        self.files.insert(open.unwrap().clone().path.clone(), open.unwrap().clone().buffer.clone());
        self.writes.remove(&s.handle);
        drop(s);
        return Union2::<(), FsError>::U1(ok(()));
    }
}

pub fn fs_has_children(files: &SalvoMap<String, Vec<u8>>, path: &String) -> bool {
    let mut prefix = format!("{}/", path.clone());
    for mut key in files.clone().keys().cloned().collect::<Vec<_>>() {
        if key.starts_with(&prefix[..]) {
            return true;
        }
    }
    return false;
}

pub fn mem_find_newline(data: &Vec<u8>, from: i32) -> i32 {
    let mut end = (data.len() as i32);
    let mut i = from;
    while i < end {
        if (((data.get((i) as usize).copied().unwrap()) as i32) == 10) {
            return i;
        }
        i = i + 1;
    }
    return end;
}

pub fn mem_append(writes: &mut SalvoMap<i64, MemWrite>, handle: i64, data: &Vec<u8>) -> i64 {
    let mut open = writes.get(&handle);
    if open.is_none() {
        return 0i64;
    }
    let mut grown = [open.unwrap().clone().buffer.clone()].iter().flat_map(|__p| __p.iter().copied()).collect::<Vec<u8>>();
    grown.extend_from_slice(&data[..]);
    let mut buffer: Vec<u8> = grown;
    writes.insert(handle.clone(), MemWrite { path: open.unwrap().clone().path.clone(), buffer: buffer });
    return (((data.len() as i32)) as i64);
}

pub fn mem_read_line(reads: &mut SalvoMap<i64, MemRead>, files: &SalvoMap<String, Vec<u8>>, handle: i64) -> Option<String> {
    let mut open = reads.get(&handle);
    if open.is_none() {
        return None;
    }
    if open.unwrap().clone().failed {
        return None;
    }
    let mut at = open.unwrap().clone().at;
    let mut path: String = open.unwrap().clone().path.clone();
    let mut content = files.get(&path);
    if content.is_none() {
        return None;
    }
    let mut bytes: Vec<u8> = content.unwrap().clone();
    let mut end = (bytes.len() as i32);
    if at >= end {
        return None;
    }
    let mut stop = mem_find_newline(&bytes, at);
    let mut line = { let __d = &bytes; let __i = at; let __j = stop; if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() { Some(__d[(__i as usize)..(__j as usize)].to_vec()) } else { None } }.unwrap();
    let mut next_at = stop;
    if stop < end {
        next_at = stop + 1;
    }
    let mut text = String::from_utf8(line.clone()).ok();
    if text.is_none() {
        reads.insert(handle.clone(), MemRead { path: path.clone(), at: next_at, failed: true });
        return None;
    }
    reads.insert(handle.clone(), MemRead { path: path.clone(), at: next_at, failed: false });
    return Some({ let __s = &text.as_ref().unwrap().clone()[..]; __s.strip_suffix(&"\r".to_string()[..]).unwrap_or(__s).to_string() });
}

pub fn mem_read_all(reads: &mut SalvoMap<i64, MemRead>, files: &SalvoMap<String, Vec<u8>>, handle: i64) -> Union2<String, FsError> {
    let mut open = reads.get(&handle);
    if open.is_none() {
        return Union2::<String, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U7(StaleHandle { path: "<stream>".to_string() }) }));
    }
    let mut path: String = open.unwrap().clone().path.clone();
    if open.unwrap().clone().failed {
        return Union2::<String, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U6(InvalidUtf8 { path: path }) }));
    }
    let mut content = files.get(&path);
    if content.is_none() {
        return Union2::<String, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
    }
    let mut bytes: Vec<u8> = content.unwrap().clone();
    let mut end = (bytes.len() as i32);
    let mut rest = { let __d = &bytes; let __i = open.unwrap().clone().at; let __j = end; if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() { Some(__d[(__i as usize)..(__j as usize)].to_vec()) } else { None } }.unwrap();
    let mut text = String::from_utf8(rest.clone()).ok();
    if text.is_none() {
        reads.insert(handle.clone(), MemRead { path: path.clone(), at: end, failed: true });
        return Union2::<String, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U6(InvalidUtf8 { path: path.clone() }) }));
    }
    reads.insert(handle.clone(), MemRead { path: path.clone(), at: end, failed: false });
    return Union2::<String, FsError>::U1(ok(text.as_ref().unwrap().clone()));
}

pub fn mem_read_bytes(reads: &mut SalvoMap<i64, MemRead>, files: &SalvoMap<String, Vec<u8>>, handle: i64, max: i32) -> Union2<Vec<u8>, FsError> {
    let mut open = reads.get(&handle);
    if open.is_none() {
        return Union2::<Vec<u8>, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U7(StaleHandle { path: "<stream>".to_string() }) }));
    }
    let mut path: String = open.unwrap().clone().path.clone();
    if open.unwrap().clone().failed {
        return Union2::<Vec<u8>, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U6(InvalidUtf8 { path: path }) }));
    }
    let mut content = files.get(&path);
    if content.is_none() {
        return Union2::<Vec<u8>, FsError>::U2(err(FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U1(NotFound { path: path.clone() }) }));
    }
    let mut bytes: Vec<u8> = content.unwrap().clone();
    let mut stop = open.unwrap().clone().at + max;
    if max < 0 {
        stop = open.unwrap().clone().at;
    }
    let mut end = (bytes.len() as i32);
    if stop > end {
        stop = end;
    }
    let mut taken = { let __d = &bytes; let __i = open.unwrap().clone().at; let __j = stop; if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() { Some(__d[(__i as usize)..(__j as usize)].to_vec()) } else { None } }.unwrap();
    reads.insert(handle.clone(), MemRead { path: path.clone(), at: stop, failed: false });
    return Union2::<Vec<u8>, FsError>::U1(ok(taken));
}
