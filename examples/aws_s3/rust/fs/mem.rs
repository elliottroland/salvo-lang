use crate::collections::*;
use crate::core_actor::*;
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
use crate::fs::*;
use crate::stream::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MemRead {
    pub source: String,
    pub data: Vec<u8>,
    pub at: i32,
    pub failed: bool,
}

impl crate::wire::__Wire for MemRead {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.source, out);
        crate::wire::__Wire::__enc(&self.data, out);
        crate::wire::__Wire::__enc(&self.at, out);
        crate::wire::__Wire::__enc(&self.failed, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            source: crate::wire::__Wire::__dec(r)?,
            data: crate::wire::__Wire::__dec(r)?,
            at: crate::wire::__Wire::__dec(r)?,
            failed: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemWrite {
    pub path: String,
    pub buffer: Vec<u8>,
}

impl crate::wire::__Wire for MemWrite {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.path, out);
        crate::wire::__Wire::__enc(&self.buffer, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            path: crate::wire::__Wire::__dec(r)?,
            buffer: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub struct MemFs {
    files: SalvoMap<String, Vec<u8>>,
    reads: SalvoMap<i64, MemRead>,
    writes: SalvoMap<i64, MemWrite>,
}

impl MemFs {
    pub fn new() -> Self {
        Self {
            files: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            reads: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            writes: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
        }
    }
}

impl crate::fs::__Stateful_Fs for MemFs {

    fn open_read(&mut self, path: &String) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut content = self.files.get(&path);
        if content.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path.clone() }))));
        }
        let mut handle = fresh_handle__2();
        self.reads.insert(handle.clone(), MemRead { source: path.clone(), data: content.unwrap().clone(), at: 0, failed: false });
        return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(InStream { handle: handle.clone() }));
    }

    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut content = self.files.get(&path);
        if content.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path.clone() }))));
        }
        let mut at = ((offset) as i32);
        if at < 0 {
            at = 0;
        }
        let mut end = (content.unwrap().clone().len() as i32);
        if at > end {
            at = end;
        }
        let mut handle = fresh_handle__2();
        self.reads.insert(handle.clone(), MemRead { source: path.clone(), data: content.unwrap().clone(), at: at, failed: false });
        return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(InStream { handle: handle.clone() }));
    }

    fn open_write(&mut self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut handle = fresh_handle__2();
        let mut empty = Vec::<u8>::new();
        self.writes.insert(handle.clone(), MemWrite { path: path.clone(), buffer: empty });
        return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(OutStream { handle: handle.clone() }));
    }

    fn open_append(&mut self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut existing = self.files.get(&path);
        let mut start = Vec::<u8>::new();
        if existing.is_none() {
        } else {
            start.extend_from_slice(&existing.unwrap().clone()[..]);
        }
        let mut handle = fresh_handle__2();
        self.writes.insert(handle.clone(), MemWrite { path: path.clone(), buffer: start });
        return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(OutStream { handle: handle.clone() }));
    }

    fn exists(&mut self, path: &String) -> bool {
        if self.files.contains_key(&path) {
            return true;
        }
        return fs_has_children(&self.files, path);
    }

    fn metadata(&mut self, path: &String) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut content = self.files.get(&path);
        if content.is_none() {
            if fs_has_children(&self.files, path) {
                return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(FileInfo { size: 0i64, is_dir: true }));
            }
            return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path.clone() }))));
        }
        return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(FileInfo { size: (((content.unwrap().clone().len() as i32)) as i64), is_dir: false }));
    }

    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        if self.files.contains_key(&path) {
            return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U4(NotADirectory { path: path.clone() }))));
        }
        if !fs_has_children(&self.files, path) {
            return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path.clone() }))));
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
        return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(sorted));
    }

    fn create_dirs(&mut self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
    }

    fn delete(&mut self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        if self.files.contains_key(&path) {
            self.files.remove(&path);
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
        }
        if fs_has_children(&self.files, path) {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U6(IoError { path: path.clone(), message: "directory not empty".to_string() }))));
        }
        return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path.clone() }))));
    }

    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut content = self.files.get(&from);
        if content.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: from.clone() }))));
        }
        let mut bytes: Vec<u8> = content.unwrap().clone();
        self.files.remove(&from);
        self.files.insert(to.clone(), bytes);
        return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
    }
}

impl crate::stream::__Stateful_Streams for MemFs {

    fn read_line(&mut self, s: &InStream) -> Option<String> {
        return mem_read_line(&mut self.reads, s.handle);
    }

    fn read_all(&mut self, s: &InStream) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return mem_read_all(&mut self.reads, s.handle);
    }

    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return mem_read_bytes(&mut self.reads, s.handle, max);
    }

    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut got = mem_read_bytes(&mut self.reads, s.handle, max);
        if matches!(got, Union2::U2(_)) {
            return Union2::<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        let mut data: Vec<u8> = got.u1().clone();
        buf.extend_from_slice(&data[..]);
        return Union2::<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok((data.len() as i32)));
    }

    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut got = mem_read_all(&mut self.reads, s.handle);
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        let mut text: String = got.u1().clone();
        buf.push_str(&text[..]);
        return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok((text.len() as i64)));
    }

    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        let mut line = mem_read_line(&mut self.reads, s.handle);
        match line {
            Some(_) => {
                buf.push_str(&line.as_ref().unwrap()[..]);
                return true;
            }
            None => {
                return false;
            }
        }
    }

    fn position(&mut self, s: &InStream) -> i64 {
        return ((mem_read_state(&self.reads, s.handle).at) as i64);
    }

    fn close(&mut self, s: InStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut open = mem_read_state(&self.reads, s.handle);
        let mut failed = open.failed;
        let mut source = open.source.clone();
        self.reads.remove(&s.handle);
        drop(s);
        if failed {
            return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source }))));
        }
        return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
    }

    fn receive(&mut self, s: InStream, reply: crate::scheduler::SalvoReply) {
        let mut got = mem_read_bytes(&mut self.reads, s.handle, 65536);
        if matches!(got, Union2::U2(_)) {
            let mut open = mem_read_state(&self.reads, s.handle);
            self.reads.remove(&s.handle);
            drop(s);
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U3(err(checked(Union2::<InvalidUtf8, StreamFailed>::U2(StreamFailed { source: open.source.clone(), message: "read failed".to_string() }))))));
            ignore((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
            return;
        }
        let mut data: Vec<u8> = got.u1().clone();
        if ((data.len() as i32) == 0) {
            self.reads.remove(&s.handle);
            drop(s);
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(End {  })));
            return;
        }
        (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(Packet { bytes: data, stream: s }))));
    }

    fn from_bytes(&mut self, data: Vec<u8>) -> InStream {
        let mut handle = fresh_handle__2();
        self.reads.insert(handle.clone(), MemRead { source: "<bytes>".to_string(), data: data, at: 0, failed: false });
        return InStream { handle: handle.clone() };
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
        return (((mem_write_state(&self.writes, s.handle).buffer.len() as i32)) as i64);
    }

    fn flush(&mut self, s: &OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut open = mem_write_state(&self.writes, s.handle);
        self.files.insert(open.path.clone(), open.buffer.clone());
        return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
    }

    fn close__2(&mut self, s: OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut open = mem_write_state(&self.writes, s.handle);
        self.files.insert(open.path.clone(), open.buffer.clone());
        self.writes.remove(&s.handle);
        drop(s);
        return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
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
        if (((data.get((i) as i64 as usize).copied().expect("salvo: value is absent at fs.mem:311:19")) as i32) == 10) {
            return i;
        }
        i = i + 1;
    }
    return end;
}

pub fn mem_append(writes: &mut SalvoMap<i64, MemWrite>, handle: i64, data: &Vec<u8>) -> i64 {
    let mut open = mem_write_state(writes, handle);
    let mut grown = [open.buffer.clone()].iter().flat_map(|__p| __p.iter().copied()).collect::<Vec<u8>>();
    grown.extend_from_slice(&data[..]);
    let mut buffer: Vec<u8> = grown;
    writes.insert(handle.clone(), MemWrite { path: open.path.clone(), buffer: buffer });
    return (((data.len() as i32)) as i64);
}

pub fn mem_read_state(reads: &SalvoMap<i64, MemRead>, handle: i64) -> MemRead {
    let mut open = reads.get(&handle);
    if open.is_none() {
        panic!("salvo: {} at fs.mem:338:9", format!("stream handle {} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]", handle));
    }
    return MemRead { source: open.unwrap().clone().source.clone(), data: open.unwrap().clone().data.clone(), at: open.unwrap().clone().at, failed: open.unwrap().clone().failed };
}

pub fn mem_write_state(writes: &SalvoMap<i64, MemWrite>, handle: i64) -> MemWrite {
    let mut open = writes.get(&handle);
    if open.is_none() {
        panic!("salvo: {} at fs.mem:346:9", format!("stream handle {} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]", handle));
    }
    return MemWrite { path: open.unwrap().clone().path.clone(), buffer: open.unwrap().clone().buffer.clone() };
}

pub fn mem_read_line(reads: &mut SalvoMap<i64, MemRead>, handle: i64) -> Option<String> {
    let mut open = mem_read_state(reads, handle);
    if open.failed {
        return None;
    }
    let mut at = &open.at;
    let mut bytes: Vec<u8> = open.data.clone();
    let mut end = (bytes.len() as i32);
    if *at >= end {
        return None;
    }
    let mut stop = mem_find_newline(&bytes, *at);
    let mut line = { let __d = &bytes; let __i = *at; let __j = stop; if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() { Some(__d[(__i as usize)..(__j as usize)].to_vec()) } else { None } }.expect("salvo: value is absent at fs.mem:370:16");
    let mut next_at = stop;
    if stop < end {
        next_at = stop + 1;
    }
    let mut text = String::from_utf8(line.clone()).ok();
    if text.is_none() {
        reads.insert(handle.clone(), MemRead { source: open.source.clone(), data: bytes, at: next_at, failed: true });
        return None;
    }
    reads.insert(handle.clone(), MemRead { source: open.source.clone(), data: bytes, at: next_at, failed: false });
    return Some({ let __s = &text.as_ref().unwrap()[..]; __s.strip_suffix(&"\r".to_string()[..]).unwrap_or(__s).to_string() });
}

pub fn mem_read_all(reads: &mut SalvoMap<i64, MemRead>, handle: i64) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let mut open = mem_read_state(reads, handle);
    let mut source: String = open.source.clone();
    if open.failed {
        return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source }))));
    }
    let mut bytes: Vec<u8> = open.data.clone();
    let mut end = (bytes.len() as i32);
    let mut rest = { let __d = &bytes; let __i = open.at; let __j = end; if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() { Some(__d[(__i as usize)..(__j as usize)].to_vec()) } else { None } }.expect("salvo: value is absent at fs.mem:394:16");
    let mut text = String::from_utf8(rest.clone()).ok();
    if text.is_none() {
        reads.insert(handle.clone(), MemRead { source: source.clone(), data: bytes, at: end, failed: true });
        return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source.clone() }))));
    }
    reads.insert(handle.clone(), MemRead { source: source.clone(), data: bytes, at: end, failed: false });
    return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(text.as_ref().unwrap().clone()));
}

pub fn mem_read_bytes(reads: &mut SalvoMap<i64, MemRead>, handle: i64, max: i32) -> Union2<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let mut open = mem_read_state(reads, handle);
    let mut source: String = open.source.clone();
    if open.failed {
        return Union2::<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source }))));
    }
    let mut bytes: Vec<u8> = open.data.clone();
    let mut stop = open.at + max;
    if max < 0 {
        stop = open.at;
    }
    let mut end = (bytes.len() as i32);
    if stop > end {
        stop = end;
    }
    let mut taken = { let __d = &bytes; let __i = open.at; let __j = stop; if __i >= 0 && __j >= __i && (__j as usize) <= __d.len() { Some(__d[(__i as usize)..(__j as usize)].to_vec()) } else { None } }.expect("salvo: value is absent at fs.mem:420:17");
    reads.insert(handle.clone(), MemRead { source: source.clone(), data: bytes, at: stop, failed: false });
    return Union2::<Vec<u8>, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(taken));
}
