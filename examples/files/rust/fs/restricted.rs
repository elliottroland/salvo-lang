use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_nonempty::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::fs::*;
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
        let mut seg = segs.get((i) as i64 as usize).unwrap();
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
        parts.push(kept.get((j) as i64 as usize).expect("salvo: value is absent at fs.restricted:57:24").clone());
        j = j - 1;
    }
    let mut rel = parts.join(&"/".to_string()[..]);
    if ((rel.chars().count() as i32) == 0) {
        return Some(root.clone());
    }
    return Some(format!("{}/{}", root.clone(), rel));
}

pub fn fs_escaped(path: &String) -> Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> {
    return checked(Union8::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>::U5(PathEscapes { path: path.clone() }));
}

#[derive(Clone)]
pub struct RestrictedFs {
    root: String,
    __dep_Fs: crate::fs::__Mon_Fs,
}

impl RestrictedFs {
    pub fn new(root: String, __dep_Fs: crate::fs::__Mon_Fs) -> Self {
        Self {
            root,
            __dep_Fs,
        }
    }
}

impl Fs for RestrictedFs {

    fn open_read(&mut self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).open_read(real.as_ref().unwrap());
    }

    fn open_read_at(&mut self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).open_read_at(real.as_ref().unwrap(), offset);
    }

    fn open_write(&mut self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).open_write(real.as_ref().unwrap());
    }

    fn open_append(&mut self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).open_append(real.as_ref().unwrap());
    }

    fn exists(&mut self, path: &String) -> bool {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return false;
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).exists(real.as_ref().unwrap());
    }

    fn metadata(&mut self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).metadata(real.as_ref().unwrap());
    }

    fn list_dir(&mut self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).list_dir(real.as_ref().unwrap());
    }

    fn create_dirs(&mut self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).create_dirs(real.as_ref().unwrap());
    }

    fn delete(&mut self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).delete(real.as_ref().unwrap());
    }

    fn rename_path(&mut self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real_from = fs_resolve(&self.root, from);
        if real_from.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(from)));
        }
        let mut real_to = fs_resolve(&self.root, to);
        if real_to.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(to)));
        }
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).rename_path(real_from.as_ref().unwrap(), real_to.as_ref().unwrap());
    }

    fn read_line(&mut self, s: &InStream) -> Option<String> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).read_line(s);
    }

    fn read_all(&mut self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).read_all(s);
    }

    fn read_bytes(&mut self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).read_bytes(s, max);
    }

    fn read_to(&mut self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).read_to(s, buf, max);
    }

    fn read_to__2(&mut self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).read_to__2(s, buf);
    }

    fn read_line_to(&mut self, s: &InStream, buf: &mut String) -> bool {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).read_line_to(s, buf);
    }

    fn position(&mut self, s: &InStream) -> i64 {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).position(s);
    }

    fn close(&mut self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).close(s);
    }

    fn write(&mut self, s: &OutStream, text: &String) -> i64 {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).write(s, text);
    }

    fn write_line(&mut self, s: &OutStream, text: &String) -> i64 {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).write_line(s, text);
    }

    fn write_bytes(&mut self, s: &OutStream, data: &Vec<u8>) -> i64 {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).write_bytes(s, data);
    }

    fn position__2(&mut self, s: &OutStream) -> i64 {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).position__2(s);
    }

    fn flush(&mut self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).flush(s);
    }

    fn close__2(&mut self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return __Has_Fs::__get_Fs(&mut self.__dep_Fs).close__2(s);
    }
}
