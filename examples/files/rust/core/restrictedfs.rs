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

pub fn fs_resolve(root: &String, path: &String) -> Option<String> {
    if path.starts_with(&"/".to_string()[..]) {
        return None;
    }
    let mut segs = path.split(&"/".to_string()[..]).map(|__p| __p.to_string()).collect::<Vec<String>>();
    let mut kept: Vec<String> = vec![];
    let mut skip = 0;
    let mut i = (segs.len() as i32) - 1;
    while i >= 0 {
        let mut seg = segs.get((i) as usize).unwrap();
        if seg.clone() == "..".to_string() {
            skip = skip + 1;
        } else {
            if ((seg.chars().count() as i32) == 0 || seg.clone() == ".".to_string()) {
            } else {
                if skip > 0 {
                    skip = skip - 1;
                } else {
                    kept.push(seg.clone());
                }
            }
        }
        i = i - 1;
    }
    if skip > 0 {
        return None;
    }
    let mut parts: Vec<String> = vec![];
    let mut j = (kept.len() as i32) - 1;
    while j >= 0 {
        parts.push(kept.get((j) as usize).unwrap().clone());
        j = j - 1;
    }
    let mut rel = parts.join(&"/".to_string()[..]);
    if ((rel.chars().count() as i32) == 0) {
        return Some(root.clone());
    }
    return Some(format!("{}/{}", root.clone(), rel));
}

pub fn fs_escaped(path: &String) -> FsError {
    return FsError { kind: Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U5(PathEscapes { path: path.clone() }) };
}

pub struct RestrictedFs {
    root: String,
}

impl RestrictedFs {
    pub fn new(root: String) -> Self {
        Self {
            root,
        }
    }
}

pub struct __Deps_RestrictedFs<'a, __P: ?Sized> {
    pub __p: &'a mut __P,
}

impl<'a, __P: __Has_Fs + ?Sized> __Has_Fs for __Deps_RestrictedFs<'a, __P> {
    fn __get_Fs(&mut self) -> &mut dyn Fs {
        __Has_Fs::__get_Fs(&mut *self.__p)
    }
}

pub trait __Impl_RestrictedFs {

    fn open_read<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<InStream, FsError>;

    fn open_read_at<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String, offset: i64) -> Union2<InStream, FsError>;

    fn open_write<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<OutStream, FsError>;

    fn open_append<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<OutStream, FsError>;

    fn exists<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> bool;

    fn metadata<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<FileInfo, FsError>;

    fn list_dir<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<Vec<String>, FsError>;

    fn create_dirs<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<(), FsError>;

    fn delete<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<(), FsError>;

    fn rename_path<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, from: &String, to: &String) -> Union2<(), FsError>;

    fn read_line<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream) -> Option<String>;

    fn read_all<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream) -> Union2<String, FsError>;

    fn read_bytes<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, max: i32) -> Union2<Vec<u8>, FsError>;

    fn read_to<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, FsError>;

    fn read_to__2<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, buf: &mut String) -> Union2<i64, FsError>;

    fn read_line_to<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, buf: &mut String) -> bool;

    fn position<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream) -> i64;

    fn close<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: InStream) -> Union2<(), FsError>;

    fn write<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream, text: &String) -> i64;

    fn write_line<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream, text: &String) -> i64;

    fn write_bytes<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream, data: &Vec<u8>) -> i64;

    fn position__2<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream) -> i64;

    fn flush<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream) -> Union2<(), FsError>;

    fn close__2<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: OutStream) -> Union2<(), FsError>;
}

impl __Impl_RestrictedFs for RestrictedFs {

    fn open_read<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<InStream, FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).open_read(&(real.as_ref().unwrap().clone()));
    }

    fn open_read_at<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String, offset: i64) -> Union2<InStream, FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).open_read_at(&(real.as_ref().unwrap().clone()), offset);
    }

    fn open_write<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<OutStream, FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).open_write(&(real.as_ref().unwrap().clone()));
    }

    fn open_append<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<OutStream, FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).open_append(&(real.as_ref().unwrap().clone()));
    }

    fn exists<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> bool {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return false;
        }
        return __Has_Fs::__get_Fs(&mut *__fx).exists(&(real.as_ref().unwrap().clone()));
    }

    fn metadata<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<FileInfo, FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<FileInfo, FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).metadata(&(real.as_ref().unwrap().clone()));
    }

    fn list_dir<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<Vec<String>, FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<Vec<String>, FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).list_dir(&(real.as_ref().unwrap().clone()));
    }

    fn create_dirs<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<(), FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).create_dirs(&(real.as_ref().unwrap().clone()));
    }

    fn delete<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, path: &String) -> Union2<(), FsError> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), FsError>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).delete(&(real.as_ref().unwrap().clone()));
    }

    fn rename_path<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, from: &String, to: &String) -> Union2<(), FsError> {
        let mut real_from = fs_resolve(&self.root, from);
        if real_from.is_none() {
            return Union2::<(), FsError>::U2(err(fs_escaped(from)));
        }
        let mut real_to = fs_resolve(&self.root, to);
        if real_to.is_none() {
            return Union2::<(), FsError>::U2(err(fs_escaped(to)));
        }
        return __Has_Fs::__get_Fs(&mut *__fx).rename_path(&(real_from.as_ref().unwrap().clone()), &(real_to.as_ref().unwrap().clone()));
    }

    fn read_line<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream) -> Option<String> {
        return __Has_Fs::__get_Fs(&mut *__fx).read_line(s);
    }

    fn read_all<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream) -> Union2<String, FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).read_all(s);
    }

    fn read_bytes<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, max: i32) -> Union2<Vec<u8>, FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).read_bytes(s, max);
    }

    fn read_to<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).read_to(s, buf, max);
    }

    fn read_to__2<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, buf: &mut String) -> Union2<i64, FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).read_to__2(s, buf);
    }

    fn read_line_to<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream, buf: &mut String) -> bool {
        return __Has_Fs::__get_Fs(&mut *__fx).read_line_to(s, buf);
    }

    fn position<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &InStream) -> i64 {
        return __Has_Fs::__get_Fs(&mut *__fx).position(s);
    }

    fn close<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: InStream) -> Union2<(), FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).close(s);
    }

    fn write<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream, text: &String) -> i64 {
        return __Has_Fs::__get_Fs(&mut *__fx).write(s, text);
    }

    fn write_line<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream, text: &String) -> i64 {
        return __Has_Fs::__get_Fs(&mut *__fx).write_line(s, text);
    }

    fn write_bytes<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream, data: &Vec<u8>) -> i64 {
        return __Has_Fs::__get_Fs(&mut *__fx).write_bytes(s, data);
    }

    fn position__2<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream) -> i64 {
        return __Has_Fs::__get_Fs(&mut *__fx).position__2(s);
    }

    fn flush<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: &OutStream) -> Union2<(), FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).flush(s);
    }

    fn close__2<__Fx: __Has_Fs>(&mut self, __fx: &mut __Fx, s: OutStream) -> Union2<(), FsError> {
        return __Has_Fs::__get_Fs(&mut *__fx).close__2(s);
    }
}
