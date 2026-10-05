use crate::collections::*;
use crate::unions::*;
use crate::core_actor::__Stateful_Faults as _;
use crate::core_actor::__Stateless_Faults as _;
use crate::core_bytes::Bytes;
use crate::core_bytes::mut_bytes;
use crate::core_checked::Checked;
use crate::core_checked::checked;
use crate::core_checked::ignore;
use crate::core_list::at;
use crate::core_list::sort;
use crate::core_result::err;
use crate::core_result::ok;
use crate::core_sorted::max;
use crate::fs::AlreadyExists;
use crate::fs::FileInfo;
use crate::fs::IoError;
use crate::fs::NotADirectory;
use crate::fs::NotFound;
use crate::fs::PathEscapes;
use crate::fs::PermissionDenied;
use crate::fs::Streaming;
use crate::fs::__Stateful_Fs as _;
use crate::fs::__Stateless_Fs as _;
use crate::fs_path::Path;
use crate::fs_path::path;
use crate::fs_path::to_str;
use crate::stream::End;
use crate::stream::InStream;
use crate::stream::InvalidUtf8;
use crate::stream::OutStream;
use crate::stream::Packet;
use crate::stream::StreamFailed;
use crate::stream::__Stateful_Streams as _;
use crate::stream::__Stateless_Streams as _;
use crate::stream::fresh_handle;

#[derive(Clone, Debug, PartialEq)]
pub struct MemRead {
    pub source: String,
    pub data: Bytes,
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
    pub buffer: Bytes,
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
    files: SalvoMap<String, Bytes>,
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

    fn open_read(&mut self, path: &Path) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut content = crate::core_map::get_platform(&self.files, &path_text);
        if content.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path_text.clone() }))));
        }
        let mut handle = fresh_handle();
        crate::core_map::put_platform(&mut self.reads, handle.clone(), MemRead { source: path_text.clone(), data: content.unwrap().clone(), at: 0, failed: false });
        return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(InStream { handle: handle.clone() }));
    }

    fn open_read_at(&mut self, path: &Path, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut content = crate::core_map::get_platform(&self.files, &path_text);
        if content.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path_text.clone() }))));
        }
        let mut at = ((offset) as i32);
        if at < 0 {
            at = 0;
        }
        let mut end = crate::core_bytes::size_platform(content.unwrap());
        if at > end {
            at = end;
        }
        let mut handle = fresh_handle();
        crate::core_map::put_platform(&mut self.reads, handle.clone(), MemRead { source: path_text.clone(), data: content.unwrap().clone(), at: at, failed: false });
        return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(InStream { handle: handle.clone() }));
    }

    fn open_write(&mut self, path: &Path) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut handle = fresh_handle();
        let mut empty = mut_bytes(vec![]);
        crate::core_map::put_platform(&mut self.writes, handle.clone(), MemWrite { path: path_text.clone(), buffer: empty });
        return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(OutStream { handle: handle.clone() }));
    }

    fn open_append(&mut self, path: &Path) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut existing = crate::core_map::get_platform(&self.files, &path_text);
        let mut start = mut_bytes(vec![]);
        if existing.is_none() {
        } else {
            crate::core_bytes::append_platform(&mut start, existing.unwrap());
        }
        let mut handle = fresh_handle();
        crate::core_map::put_platform(&mut self.writes, handle.clone(), MemWrite { path: path_text.clone(), buffer: start });
        return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(OutStream { handle: handle.clone() }));
    }

    fn exists(&mut self, path: &Path) -> bool {
        let mut path_text = to_str(path);
        if crate::core_map::contains_key_platform(&self.files, &path_text) {
            return true;
        }
        return fs_has_children(&self.files, &path_text);
    }

    fn metadata(&mut self, path: &Path) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut content = crate::core_map::get_platform(&self.files, &path_text);
        if content.is_none() {
            if fs_has_children(&self.files, &path_text) {
                return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(FileInfo { size: 0i64, is_dir: true }));
            }
            return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path_text.clone() }))));
        }
        return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(FileInfo { size: ((crate::core_bytes::size_platform(content.unwrap())) as i64), is_dir: false }));
    }

    fn list_dir(&mut self, path: &Path) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        if crate::core_map::contains_key_platform(&self.files, &path_text) {
            return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U4(NotADirectory { path: path_text.clone() }))));
        }
        if !fs_has_children(&self.files, &path_text) {
            return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path_text.clone() }))));
        }
        let mut names: SalvoSet<String> = SalvoSet::from_elements::<HostHash, HostEq, _>(vec![]);
        let mut prefix = format!("{}/", path_text);
        for key in crate::platform_core_map::each(&self.files) {
            if crate::core_string::starts_with_platform(key, &prefix) {
                let mut rest = crate::core_string::trim_prefix_platform(key, &prefix);
                let mut cut = crate::core_string::index_of_platform(&rest, &("/".to_string()));
                let mut name = rest.clone();
                if cut.is_some() {
                    let mut head = crate::core_string::substr_platform(&rest, 0, cut.unwrap());
                    if head.is_some() {
                        name = head.as_ref().unwrap().clone();
                    }
                }
                Some(crate::core_set::add_platform(&mut names, name.clone()));
            }
        }
        let mut sorted: Vec<String> = sort::<String>(&(crate::core_set::to_list_platform(&names)), &mut |__i0, __i1| (Ord::cmp(&__i0[..], &__i1[..]) as i32));
        return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(sorted));
    }

    fn create_dirs(&mut self, path: &Path) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
    }

    fn delete(&mut self, path: &Path) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        if crate::core_map::contains_key_platform(&self.files, &path_text) {
            crate::core_map::remove_platform(&mut self.files, &path_text);
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U1(ok(()));
        }
        if fs_has_children(&self.files, &path_text) {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U6(IoError { path: path_text.clone(), message: "directory not empty".to_string() }))));
        }
        return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: path_text.clone() }))));
    }

    fn rename_path(&mut self, from: &Path, to: &Path) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut from_text = to_str(from);
        let mut to_text = to_str(to);
        let mut content = crate::core_map::get_platform(&self.files, &from_text);
        if content.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U1(NotFound { path: from_text.clone() }))));
        }
        let mut bytes: Bytes = content.unwrap().clone();
        crate::core_map::remove_platform(&mut self.files, &from_text);
        crate::core_map::put_platform(&mut self.files, to_text.clone(), bytes);
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

    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        return mem_read_bytes(&mut self.reads, s.handle, max);
    }

    fn read_to__InStream_Bytes_Int(&mut self, s: &InStream, buf: &mut Bytes, max: i32) -> Union2<i32, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut got = mem_read_bytes(&mut self.reads, s.handle, max);
        if matches!(got, Union2::U2(_)) {
            return Union2::<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        let mut data: Bytes = got.u1().clone();
        crate::core_bytes::append_platform(buf, &data);
        return Union2::<i32, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(crate::core_bytes::size_platform(&data)));
    }

    fn read_to__InStream_Str(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut got = mem_read_all(&mut self.reads, s.handle);
        if matches!(got, Union2::U2(_)) {
            return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        let mut text: String = got.u1().clone();
        crate::core_string::append_platform(buf, &text);
        return Union2::<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(crate::core_string::byte_size_platform(&text)));
    }

    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        let mut line = mem_read_line(&mut self.reads, s.handle);
        match line {
            Some(_) => {
                crate::core_string::append_platform(buf, line.as_ref().unwrap());
                return true;
            }
            None => {
                return false;
            }
        }
    }

    fn position__InStream(&mut self, s: &InStream) -> i64 {
        return ((mem_read_state(&self.reads, s.handle).at) as i64);
    }

    fn close__InStream(&mut self, s: InStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut open = mem_read_state(&self.reads, s.handle);
        let mut failed = open.failed;
        let mut source = open.source.clone();
        crate::core_map::remove_platform(&mut self.reads, &s.handle);
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
            crate::core_map::remove_platform(&mut self.reads, &s.handle);
            drop(s);
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U3(err(checked(Union2::<InvalidUtf8, StreamFailed>::U2(StreamFailed { source: open.source.clone(), message: "read failed".to_string() }))))));
            ignore((match got { Union2::U2(__v) => __v, _ => unreachable!() }));
            return;
        }
        let mut data: Bytes = got.u1().clone();
        if crate::core_bytes::size_platform(&data) == 0 {
            crate::core_map::remove_platform(&mut self.reads, &s.handle);
            drop(s);
            (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(End {  })));
            return;
        }
        (reply).send(std::boxed::Box::new(Union3::<Packet, End, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(Packet { bytes: data, stream: s }))));
    }

    fn from_bytes(&mut self, data: Bytes) -> InStream {
        let mut handle = fresh_handle();
        crate::core_map::put_platform(&mut self.reads, handle.clone(), MemRead { source: "<bytes>".to_string(), data: data, at: 0, failed: false });
        return InStream { handle: handle.clone() };
    }

    fn write(&mut self, s: &OutStream, text: &String) -> i64 {
        return mem_append(&mut self.writes, s.handle, &(crate::core_bytes::to_bytes_platform(text)));
    }

    fn write_line(&mut self, s: &OutStream, text: &String) -> i64 {
        return mem_append(&mut self.writes, s.handle, &(crate::core_bytes::to_bytes_platform(&(format!("{}\n", text.clone())))));
    }

    fn write_bytes(&mut self, s: &OutStream, data: &Bytes) -> i64 {
        return mem_append(&mut self.writes, s.handle, &(data.clone()));
    }

    fn position__OutStream(&mut self, s: &OutStream) -> i64 {
        return ((crate::core_bytes::size_platform(&mem_write_state(&self.writes, s.handle).buffer)) as i64);
    }

    fn flush(&mut self, s: &OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut open = mem_write_state(&self.writes, s.handle);
        crate::core_map::put_platform(&mut self.files, open.path.clone(), open.buffer.clone());
        return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
    }

    fn close__OutStream(&mut self, s: OutStream) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
        let mut open = mem_write_state(&self.writes, s.handle);
        crate::core_map::put_platform(&mut self.files, open.path.clone(), open.buffer.clone());
        crate::core_map::remove_platform(&mut self.writes, &s.handle);
        drop(s);
        return Union2::<(), Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(()));
    }
}

pub fn fs_has_children(files: &SalvoMap<String, Bytes>, path: &String) -> bool {
    let mut prefix = format!("{}/", path.clone());
    for key in crate::platform_core_map::each(files) {
        if crate::core_string::starts_with_platform(key, &prefix) {
            return true;
        }
    }
    return false;
}

pub fn mem_find_newline(data: &Bytes, from: i32) -> i32 {
    let mut end = crate::core_bytes::size_platform(data);
    let mut i = from;
    while i < end {
        if (((crate::core_bytes::get_platform(data, i).expect("salvo: value is absent at fs.mem:323:19")) as i32) == 10) {
            return i;
        }
        i = i + 1;
    }
    return end;
}

pub fn mem_append(writes: &mut SalvoMap<i64, MemWrite>, handle: i64, data: &Bytes) -> i64 {
    let mut open = mem_write_state(writes, handle);
    let mut grown = mut_bytes(vec![open.buffer.clone()]);
    crate::core_bytes::append_platform(&mut grown, data);
    let mut buffer: Bytes = grown;
    crate::core_map::put_platform(writes, handle.clone(), MemWrite { path: open.path.clone(), buffer: buffer });
    return ((crate::core_bytes::size_platform(data)) as i64);
}

pub fn mem_read_state(reads: &SalvoMap<i64, MemRead>, handle: i64) -> MemRead {
    let mut open = crate::core_map::get_platform(reads, &handle);
    if open.is_none() {
        panic!("salvo: {} at fs.mem:350:9", format!("stream handle {} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]", handle));
    }
    return MemRead { source: open.unwrap().clone().source.clone(), data: open.unwrap().clone().data.clone(), at: open.unwrap().clone().at, failed: open.unwrap().clone().failed };
}

pub fn mem_write_state(writes: &SalvoMap<i64, MemWrite>, handle: i64) -> MemWrite {
    let mut open = crate::core_map::get_platform(writes, &handle);
    if open.is_none() {
        panic!("salvo: {} at fs.mem:358:9", format!("stream handle {} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]", handle));
    }
    return MemWrite { path: open.unwrap().clone().path.clone(), buffer: open.unwrap().clone().buffer.clone() };
}

pub fn mem_read_line(reads: &mut SalvoMap<i64, MemRead>, handle: i64) -> Option<String> {
    let mut open = mem_read_state(reads, handle);
    if open.failed {
        return None;
    }
    let mut at = &open.at;
    let mut bytes: Bytes = open.data.clone();
    let mut end = crate::core_bytes::size_platform(&bytes);
    if *at >= end {
        return None;
    }
    let mut stop = mem_find_newline(&bytes, *at);
    let mut line = crate::core_bytes::slice_platform(&bytes, *at, stop).expect("salvo: value is absent at fs.mem:382:16");
    let mut next_at = stop;
    if stop < end {
        next_at = stop + 1;
    }
    let mut text = crate::core_bytes::str_of_bytes_platform(&line);
    if text.is_none() {
        crate::core_map::put_platform(reads, handle.clone(), MemRead { source: open.source.clone(), data: bytes, at: next_at, failed: true });
        return None;
    }
    crate::core_map::put_platform(reads, handle.clone(), MemRead { source: open.source.clone(), data: bytes, at: next_at, failed: false });
    return Some(crate::core_string::trim_suffix_platform(text.as_ref().unwrap(), &("\r".to_string())));
}

pub fn mem_read_all(reads: &mut SalvoMap<i64, MemRead>, handle: i64) -> Union2<String, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let mut open = mem_read_state(reads, handle);
    let mut source: String = open.source.clone();
    if open.failed {
        return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source }))));
    }
    let mut bytes: Bytes = open.data.clone();
    let mut end = crate::core_bytes::size_platform(&bytes);
    let mut rest = crate::core_bytes::slice_platform(&bytes, open.at, end).expect("salvo: value is absent at fs.mem:406:16");
    let mut text = crate::core_bytes::str_of_bytes_platform(&rest);
    if text.is_none() {
        crate::core_map::put_platform(reads, handle.clone(), MemRead { source: source.clone(), data: bytes, at: end, failed: true });
        return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source.clone() }))));
    }
    crate::core_map::put_platform(reads, handle.clone(), MemRead { source: source.clone(), data: bytes, at: end, failed: false });
    return Union2::<String, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(text.as_ref().unwrap().clone()));
}

pub fn mem_read_bytes(reads: &mut SalvoMap<i64, MemRead>, handle: i64, max: i32) -> Union2<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    let mut open = mem_read_state(reads, handle);
    let mut source: String = open.source.clone();
    if open.failed {
        return Union2::<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>::U2(err(checked(Union2::<InvalidUtf8, StreamFailed>::U1(InvalidUtf8 { source: source }))));
    }
    let mut bytes: Bytes = open.data.clone();
    let mut stop = open.at + max;
    if max < 0 {
        stop = open.at;
    }
    let mut end = crate::core_bytes::size_platform(&bytes);
    if stop > end {
        stop = end;
    }
    let mut taken = crate::core_bytes::slice_platform(&bytes, open.at, stop).expect("salvo: value is absent at fs.mem:432:17");
    crate::core_map::put_platform(reads, handle.clone(), MemRead { source: source.clone(), data: bytes, at: stop, failed: false });
    return Union2::<Bytes, Checked<Union2<InvalidUtf8, StreamFailed>>>::U1(ok(taken));
}
