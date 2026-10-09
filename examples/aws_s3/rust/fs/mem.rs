use crate::fs::AlreadyExists;
use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::stream::End;
use crate::fs::FileInfo;
use crate::stream::InStream;
use crate::stream::InvalidUtf8;
use crate::fs::IoError;
use crate::core_map::Map;
use crate::fs::NotADirectory;
use crate::fs::NotFound;
use crate::stream::OutStream;
use crate::stream::Packet;
use crate::fs_path::Path;
use crate::fs::PathEscapes;
use crate::fs::PermissionDenied;
use crate::core_set::Set;
use crate::stream::StreamFailed;
use crate::fs::Streaming;
use crate::core_set::add_platform;
use crate::core_string::byte_size_platform;
use crate::core_checked::checked;
use crate::core_map::contains_key_platform;
use crate::core_result::err;
use crate::stream::fresh_handle;
use crate::core_checked::ignore;
use crate::core_string::index_of_platform;
use crate::core_bytes::mut_bytes;
use crate::core_map::mut_map_of_platform;
use crate::core_set::mut_set_of_platform;
use crate::core_result::ok;
use crate::core_map::put_platform;
use crate::core_map::remove_platform;
use crate::core_bytes::size_platform;
use crate::core_bytes::slice_platform;
use crate::core_list::sort;
use crate::core_string::starts_with_platform;
use crate::core_bytes::str_of_bytes_platform;
use crate::core_string::substr_platform;
use crate::core_bytes::to_bytes_platform;
use crate::core_set::to_list_platform;
use crate::fs_path::to_str;
use crate::core_string::trim_prefix_platform;
use crate::core_string::trim_suffix_platform;


#[derive(Clone, Debug, PartialEq)]
pub struct MemRead {
    pub source: String,
    pub data: crate::core_bytes::Bytes,
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
    pub buffer: crate::core_bytes::Bytes,
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
    files: crate::core_map::Map<String, crate::core_bytes::Bytes>,
    reads: crate::core_map::Map<i64, crate::fs_mem::MemRead>,
    writes: crate::core_map::Map<i64, crate::fs_mem::MemWrite>,
}

impl MemFs {
    pub fn new() -> Self {
        Self {
            files: crate::core_map::mut_map_of_platform::<String, crate::core_bytes::Bytes>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])),
            reads: crate::core_map::mut_map_of_platform::<i64, crate::fs_mem::MemRead>(vec![], &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1))),
            writes: crate::core_map::mut_map_of_platform::<i64, crate::fs_mem::MemWrite>(vec![], &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)))
        }
    }
}

impl crate::fs::__Stateful_Fs for MemFs {
    fn open_read(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut content: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if content.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U1(crate::fs::NotFound { path: (path_text).clone() }))));
        };
        let mut handle: i64 = crate::stream::fresh_handle();
        let mut content_1 = content.unwrap();
        crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut self.reads, handle, crate::fs_mem::MemRead { source: (path_text).clone(), data: (content_1).clone(), at: 0i32, failed: false }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::InStream>(crate::stream::InStream { handle: handle }));
    }
    fn open_read_at(&mut self, path: &crate::fs_path::Path, offset: i64) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut content: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if content.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U1(crate::fs::NotFound { path: (path_text).clone() }))));
        };
        let mut at: i32 = ((offset) as i32);
        if (at < 0i32) {
            at = 0i32;
        };
        let mut content_1 = content.unwrap();
        let mut end: i32 = crate::core_bytes::size_platform(content_1);
        if (at > end) {
            at = end;
        };
        let mut handle: i64 = crate::stream::fresh_handle();
        crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut self.reads, handle, crate::fs_mem::MemRead { source: (path_text).clone(), data: (content_1).clone(), at: at, failed: false }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::InStream>(crate::stream::InStream { handle: handle }));
    }
    fn open_write(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut handle: i64 = crate::stream::fresh_handle();
        let mut empty: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
        crate::core_map::put_platform::<i64, crate::fs_mem::MemWrite>(&mut self.writes, handle, crate::fs_mem::MemWrite { path: (path_text).clone(), buffer: empty.clone() }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::OutStream>(crate::stream::OutStream { handle: handle }));
    }
    fn open_append(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut existing: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        let mut start: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
        if existing.is_none() {
        } else {
            let mut existing_1 = existing.unwrap();
            crate::core_bytes::append_platform(&mut start, existing_1);
        };
        let mut handle: i64 = crate::stream::fresh_handle();
        crate::core_map::put_platform::<i64, crate::fs_mem::MemWrite>(&mut self.writes, handle, crate::fs_mem::MemWrite { path: (path_text).clone(), buffer: start.clone() }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return crate::unions::Union2::U1(crate::core_result::ok::<crate::stream::OutStream>(crate::stream::OutStream { handle: handle }));
    }
    fn exists(&mut self, path: &crate::fs_path::Path) -> bool {
        let mut path_text: String = crate::fs_path::to_str(path);
        if crate::core_map::contains_key_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])) {
            return true;
        };
        return crate::fs_mem::fs_has_children(&self.files, &path_text);
    }
    fn metadata(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut content: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if content.is_none() {
            if crate::fs_mem::fs_has_children(&self.files, &path_text) {
                return crate::unions::Union2::U1(crate::core_result::ok::<crate::fs::FileInfo>(crate::fs::FileInfo { size: 0i64, is_dir: true }));
            };
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U1(crate::fs::NotFound { path: (path_text).clone() }))));
        };
        let mut content_1 = content.unwrap();
        return crate::unions::Union2::U1(crate::core_result::ok::<crate::fs::FileInfo>(crate::fs::FileInfo { size: ((crate::core_bytes::size_platform(content_1)) as i64), is_dir: false }));
    }
    fn list_dir(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        if crate::core_map::contains_key_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])) {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U4(crate::fs::NotADirectory { path: (path_text).clone() }))));
        };
        if !(crate::fs_mem::fs_has_children(&self.files, &path_text)) {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U1(crate::fs::NotFound { path: (path_text).clone() }))));
        };
        let mut names: crate::core_set::Set<String> = crate::core_set::mut_set_of_platform::<String>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        let mut prefix: String = format!("{}/", path_text);
        for mut key in crate::platform_core_map::each(&self.files).map(|__x| __x.clone()) {
            if crate::core_string::starts_with_platform(&key, &prefix) {
                let mut rest: String = crate::core_string::trim_prefix_platform(&key, &prefix);
                let mut cut: Option<i32> = crate::core_string::index_of_platform(&rest, &String::from("/"));
                let mut name: String = rest.clone();
                if cut.is_some() {
                    let mut cut_1 = cut.unwrap();
                    let mut head: Option<String> = crate::core_string::substr_platform(&rest, 0i32, cut_1);
                    if head.is_some() {
                        let mut head_2 = head.as_ref().unwrap();
                        name = (head_2).clone();
                    };
                };
                Some(crate::core_set::add_platform::<String>(&mut names, (name).clone(), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])));
            };
        }
        let mut sorted: Vec<String> = crate::core_list::sort::<String>(&crate::core_set::to_list_platform::<String>(&names), &mut |__a0: &String, __a1: &String| (Ord::cmp(&__a0[..], &__a1[..]) as i32));
        return crate::unions::Union2::U1(crate::core_result::ok::<Vec<String>>(sorted));
    }
    fn create_dirs(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn delete(&mut self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        if crate::core_map::contains_key_platform::<String, crate::core_bytes::Bytes>(&self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..])) {
            crate::core_map::remove_platform::<String, crate::core_bytes::Bytes>(&mut self.files, &path_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
            return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
        };
        if crate::fs_mem::fs_has_children(&self.files, &path_text) {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U6(crate::fs::IoError { path: (path_text).clone(), message: String::from("directory not empty") }))));
        };
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U1(crate::fs::NotFound { path: (path_text).clone() }))));
    }
    fn rename_path(&mut self, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut from_text: String = crate::fs_path::to_str(from);
        let mut to_text: String = crate::fs_path::to_str(to);
        let mut content: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<String, crate::core_bytes::Bytes>(&self.files, &from_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if content.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U1(crate::fs::NotFound { path: (from_text).clone() }))));
        };
        let mut content_1 = content.unwrap();
        let mut bytes: crate::core_bytes::Bytes = (content_1).clone();
        crate::core_map::remove_platform::<String, crate::core_bytes::Bytes>(&mut self.files, &from_text, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::core_map::put_platform::<String, crate::core_bytes::Bytes>(&mut self.files, (to_text).clone(), bytes, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
}

impl crate::stream::__Stateful_Streams for MemFs {
    fn read_line(&mut self, s: &crate::stream::InStream) -> Option<String> {
        return crate::fs_mem::mem_read_line(&mut self.reads, s.handle);
    }
    fn read_all(&mut self, s: &crate::stream::InStream) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        return crate::fs_mem::mem_read_all(&mut self.reads, s.handle);
    }
    fn read_bytes(&mut self, s: &crate::stream::InStream, max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        return crate::fs_mem::mem_read_bytes(&mut self.reads, s.handle, max);
    }
    fn read_to__InStream_Bytes_Int(&mut self, s: &crate::stream::InStream, buf: &mut crate::core_bytes::Bytes, max: i32) -> crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut got: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::fs_mem::mem_read_bytes(&mut self.reads, s.handle, max);
        if matches!(got, crate::unions::Union2::U2(_)) {
            let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(got_1);
        };
        let mut got_2 = match &got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut data = got_2;
        crate::core_bytes::append_platform(&mut *buf, data);
        return crate::unions::Union2::U1(crate::core_result::ok::<i32>(crate::core_bytes::size_platform(data)));
    }
    fn read_to__InStream_Str(&mut self, s: &crate::stream::InStream, buf: &mut String) -> crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut got: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::fs_mem::mem_read_all(&mut self.reads, s.handle);
        if matches!(got, crate::unions::Union2::U2(_)) {
            let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            return crate::unions::Union2::U2(got_1);
        };
        let mut got_2 = match &got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut text = got_2;
        crate::core_string::append_platform(&mut *buf, text);
        return crate::unions::Union2::U1(crate::core_result::ok::<i64>(crate::core_string::byte_size_platform(text)));
    }
    fn read_line_to(&mut self, s: &crate::stream::InStream, buf: &mut String) -> bool {
        let mut line: Option<String> = crate::fs_mem::mem_read_line(&mut self.reads, s.handle);
        return if line.is_some() {
            let mut line_1 = line.as_ref().unwrap();
            crate::core_string::append_platform(&mut *buf, line_1);
            return true;
        } else {
            return false;
        };
    }
    fn position__InStream(&mut self, s: &crate::stream::InStream) -> i64 {
        return (({
            let mut __proj_1: crate::fs_mem::MemRead = crate::fs_mem::mem_read_state(&self.reads, s.handle);
            __proj_1.at
        }) as i64);
    }
    fn close__InStream(&mut self, s: crate::stream::InStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut open: crate::fs_mem::MemRead = crate::fs_mem::mem_read_state(&self.reads, s.handle);
        let mut failed: bool = open.failed;
        let mut source: String = (open.source).clone();
        crate::core_map::remove_platform::<i64, crate::fs_mem::MemRead>(&mut self.reads, &s.handle, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        std::mem::drop(s);
        if failed {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U1(crate::stream::InvalidUtf8 { source: source.clone() }))));
        };
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn write(&mut self, s: &crate::stream::OutStream, text: &String) -> i64 {
        return crate::fs_mem::mem_append(&mut self.writes, s.handle, &crate::core_bytes::to_bytes_platform(text));
    }
    fn write_line(&mut self, s: &crate::stream::OutStream, text: &String) -> i64 {
        return crate::fs_mem::mem_append(&mut self.writes, s.handle, &crate::core_bytes::to_bytes_platform(&format!("{}\n", text)));
    }
    fn write_bytes(&mut self, s: &crate::stream::OutStream, data: &crate::core_bytes::Bytes) -> i64 {
        return crate::fs_mem::mem_append(&mut self.writes, s.handle, &(data).clone());
    }
    fn position__OutStream(&mut self, s: &crate::stream::OutStream) -> i64 {
        return ((crate::core_bytes::size_platform(&{
            let mut __proj_1: crate::fs_mem::MemWrite = crate::fs_mem::mem_write_state(&self.writes, s.handle);
            __proj_1.buffer
        })) as i64);
    }
    fn flush(&mut self, s: &crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut open: crate::fs_mem::MemWrite = crate::fs_mem::mem_write_state(&self.writes, s.handle);
        crate::core_map::put_platform::<String, crate::core_bytes::Bytes>(&mut self.files, (open.path).clone(), (open.buffer).clone(), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn close__OutStream(&mut self, s: crate::stream::OutStream) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
        let mut open: crate::fs_mem::MemWrite = crate::fs_mem::mem_write_state(&self.writes, s.handle);
        crate::core_map::put_platform::<String, crate::core_bytes::Bytes>(&mut self.files, (open.path).clone(), (open.buffer).clone(), &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::core_map::remove_platform::<i64, crate::fs_mem::MemWrite>(&mut self.writes, &s.handle, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        std::mem::drop(s);
        return crate::unions::Union2::U1(crate::core_result::ok::<()>(()));
    }
    fn receive(&mut self, s: crate::stream::InStream, reply: crate::scheduler::SalvoReply) {
        let mut got: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::fs_mem::mem_read_bytes(&mut self.reads, s.handle, 65536i32);
        if matches!(got, crate::unions::Union2::U2(_)) {
            let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            let mut open: crate::fs_mem::MemRead = crate::fs_mem::mem_read_state(&self.reads, s.handle);
            crate::core_map::remove_platform::<i64, crate::fs_mem::MemRead>(&mut self.reads, &s.handle, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
            std::mem::drop(s);
            (reply).send(std::boxed::Box::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>::new(crate::unions::Union3::U3(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U2(crate::stream::StreamFailed { source: (open.source).clone(), message: String::from("read failed") }))))));
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(got_1);
            return;
        };
        let mut got_2 = match got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut data: crate::core_bytes::Bytes = got_2;
        if (crate::core_bytes::size_platform(&data) == 0i32) {
            crate::core_map::remove_platform::<i64, crate::fs_mem::MemRead>(&mut self.reads, &s.handle, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
            std::mem::drop(s);
            (reply).send(std::boxed::Box::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>::new(crate::unions::Union3::U2(crate::stream::End {})));
            return;
        };
        (reply).send(std::boxed::Box::<crate::unions::Union3<crate::stream::Packet, crate::stream::End, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>::new(crate::unions::Union3::U1(crate::core_result::ok::<crate::stream::Packet>(crate::stream::Packet { bytes: data.clone(), stream: s }))));
    }
    fn from_bytes(&mut self, data: crate::core_bytes::Bytes) -> crate::stream::InStream {
        let mut handle: i64 = crate::stream::fresh_handle();
        crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut self.reads, handle, crate::fs_mem::MemRead { source: String::from("<bytes>"), data: data.clone(), at: 0i32, failed: false }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return crate::stream::InStream { handle: handle };
    }
}

pub fn fs_has_children(files: &crate::core_map::Map<String, crate::core_bytes::Bytes>, path: &String) -> bool {
    let mut prefix: String = format!("{}/", path);
    for mut key in crate::platform_core_map::each(files).map(|__x| __x.clone()) {
        if crate::core_string::starts_with_platform(&key, &prefix) {
            return true;
        };
    }
    return false;
}

pub fn mem_find_newline(data: &crate::core_bytes::Bytes, mut from: i32) -> i32 {
    let mut end: i32 = crate::core_bytes::size_platform(data);
    let mut i: i32 = from;
    loop {
        if !((i < end)) {
            break;
        };
        if ((({
            let mut __nn_1: Option<u8> = crate::core_bytes::get_platform(data, i);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at fs.mem:323:19");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        }) as i32) == 10i32) {
            return i;
        };
        i = i32::wrapping_add(i, 1i32);
    }
    return end;
}

pub fn mem_append(writes: &mut crate::core_map::Map<i64, crate::fs_mem::MemWrite>, mut handle: i64, data: &crate::core_bytes::Bytes) -> i64 {
    let mut open: crate::fs_mem::MemWrite = crate::fs_mem::mem_write_state(&*writes, handle);
    let mut grown: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![open.buffer.clone()]);
    crate::core_bytes::append_platform(&mut grown, data);
    let mut buffer: crate::core_bytes::Bytes = grown.clone();
    crate::core_map::put_platform::<i64, crate::fs_mem::MemWrite>(&mut *writes, handle, crate::fs_mem::MemWrite { path: (open.path).clone(), buffer: buffer.clone() }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    return ((crate::core_bytes::size_platform(data)) as i64);
}

pub fn mem_read_state(reads: &crate::core_map::Map<i64, crate::fs_mem::MemRead>, mut handle: i64) -> crate::fs_mem::MemRead {
    let mut open: Option<&crate::fs_mem::MemRead> = crate::core_map::get_platform::<i64, crate::fs_mem::MemRead>(reads, &handle, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    if open.is_none() {
        panic!("salvo: {} at fs.mem:350:9", format!("stream handle {} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]", handle));
    };
    let mut open_1 = open.unwrap();
    return crate::fs_mem::MemRead { source: (open_1.source).clone(), data: (open_1.data).clone(), at: open_1.at, failed: open_1.failed };
}

pub fn mem_write_state(writes: &crate::core_map::Map<i64, crate::fs_mem::MemWrite>, mut handle: i64) -> crate::fs_mem::MemWrite {
    let mut open: Option<&crate::fs_mem::MemWrite> = crate::core_map::get_platform::<i64, crate::fs_mem::MemWrite>(writes, &handle, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    if open.is_none() {
        panic!("salvo: {} at fs.mem:358:9", format!("stream handle {} was not opened by this MemFs: a stream belongs to the provider that minted it [stream-provider]", handle));
    };
    let mut open_1 = open.unwrap();
    return crate::fs_mem::MemWrite { path: (open_1.path).clone(), buffer: (open_1.buffer).clone() };
}

pub fn mem_read_line(reads: &mut crate::core_map::Map<i64, crate::fs_mem::MemRead>, mut handle: i64) -> Option<String> {
    let mut open: crate::fs_mem::MemRead = crate::fs_mem::mem_read_state(&*reads, handle);
    if open.failed {
        return None;
    };
    let mut at: i32 = open.at;
    let mut bytes: crate::core_bytes::Bytes = (open.data).clone();
    let mut end: i32 = crate::core_bytes::size_platform(&bytes);
    if (at >= end) {
        return None;
    };
    let mut stop: i32 = crate::fs_mem::mem_find_newline(&bytes, at);
    let mut line: crate::core_bytes::Bytes = {
        let mut __nn_1: Option<crate::core_bytes::Bytes> = crate::core_bytes::slice_platform(&bytes, at, stop);
        if __nn_1.is_none() {
            panic!("salvo: value is absent at fs.mem:382:16");
        } else {
            let mut __some_2 = __nn_1.unwrap();
            __some_2
        }
    };
    let mut next_at: i32 = stop;
    if (stop < end) {
        next_at = i32::wrapping_add(stop, 1i32);
    };
    let mut text: Option<String> = crate::core_bytes::str_of_bytes_platform(&line);
    if text.is_none() {
        crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut *reads, handle, crate::fs_mem::MemRead { source: (open.source).clone(), data: bytes.clone(), at: next_at, failed: true }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return None;
    };
    crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut *reads, handle, crate::fs_mem::MemRead { source: (open.source).clone(), data: bytes.clone(), at: next_at, failed: false }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    let mut text_3 = text.as_ref().unwrap();
    return Some(crate::core_string::trim_suffix_platform(text_3, &String::from("\r")));
}

pub fn mem_read_all(reads: &mut crate::core_map::Map<i64, crate::fs_mem::MemRead>, mut handle: i64) -> crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut open: crate::fs_mem::MemRead = crate::fs_mem::mem_read_state(&*reads, handle);
    let mut source: String = (open.source).clone();
    if open.failed {
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U1(crate::stream::InvalidUtf8 { source: source.clone() }))));
    };
    let mut bytes: crate::core_bytes::Bytes = (open.data).clone();
    let mut end: i32 = crate::core_bytes::size_platform(&bytes);
    let mut rest: crate::core_bytes::Bytes = {
        let mut __nn_1: Option<crate::core_bytes::Bytes> = crate::core_bytes::slice_platform(&bytes, open.at, end);
        if __nn_1.is_none() {
            panic!("salvo: value is absent at fs.mem:406:16");
        } else {
            let mut __some_2 = __nn_1.unwrap();
            __some_2
        }
    };
    let mut text: Option<String> = crate::core_bytes::str_of_bytes_platform(&rest);
    if text.is_none() {
        crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut *reads, handle, crate::fs_mem::MemRead { source: (source).clone(), data: bytes.clone(), at: end, failed: true }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U1(crate::stream::InvalidUtf8 { source: (source).clone() }))));
    };
    crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut *reads, handle, crate::fs_mem::MemRead { source: (source).clone(), data: bytes.clone(), at: end, failed: false }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    let mut text_3 = text.unwrap();
    return crate::unions::Union2::U1(crate::core_result::ok::<String>(text_3));
}

pub fn mem_read_bytes(reads: &mut crate::core_map::Map<i64, crate::fs_mem::MemRead>, mut handle: i64, mut max: i32) -> crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    let mut open: crate::fs_mem::MemRead = crate::fs_mem::mem_read_state(&*reads, handle);
    let mut source: String = (open.source).clone();
    if open.failed {
        return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>(crate::core_checked::checked::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(crate::unions::Union2::U1(crate::stream::InvalidUtf8 { source: source.clone() }))));
    };
    let mut bytes: crate::core_bytes::Bytes = (open.data).clone();
    let mut stop: i32 = i32::wrapping_add(open.at, max);
    if (max < 0i32) {
        stop = open.at;
    };
    let mut end: i32 = crate::core_bytes::size_platform(&bytes);
    if (stop > end) {
        stop = end;
    };
    let mut taken: crate::core_bytes::Bytes = {
        let mut __nn_1: Option<crate::core_bytes::Bytes> = crate::core_bytes::slice_platform(&bytes, open.at, stop);
        if __nn_1.is_none() {
            panic!("salvo: value is absent at fs.mem:432:17");
        } else {
            let mut __some_2 = __nn_1.unwrap();
            __some_2
        }
    };
    crate::core_map::put_platform::<i64, crate::fs_mem::MemRead>(&mut *reads, handle, crate::fs_mem::MemRead { source: (source).clone(), data: bytes.clone(), at: stop, failed: false }, &mut |__a0: &i64| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&(*__a0), &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &i64, __a1: &i64| ((*__a0) == (*__a1)));
    return crate::unions::Union2::U1(crate::core_result::ok::<crate::core_bytes::Bytes>(taken));
}
