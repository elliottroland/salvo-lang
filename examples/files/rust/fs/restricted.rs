use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
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
    __dep_Fs: crate::fs::Fs,
}

impl RestrictedFs {
    pub fn new(root: String, __dep_Fs: crate::fs::Fs) -> Self {
        Self {
            root,
            __dep_Fs,
        }
    }
}

impl crate::fs::__Stateless_Fs for RestrictedFs {

    fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_read(real.as_ref().unwrap());
    }

    fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_read_at(real.as_ref().unwrap(), offset);
    }

    fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_write(real.as_ref().unwrap());
    }

    fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_append(real.as_ref().unwrap());
    }

    fn exists(&self, path: &String) -> bool {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return false;
        }
        return self.__dep_Fs.exists(real.as_ref().unwrap());
    }

    fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<FileInfo, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.metadata(real.as_ref().unwrap());
    }

    fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<Vec<String>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.list_dir(real.as_ref().unwrap());
    }

    fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.create_dirs(real.as_ref().unwrap());
    }

    fn delete(&self, path: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.delete(real.as_ref().unwrap());
    }

    fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        let mut real_from = fs_resolve(&self.root, from);
        if real_from.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(from)));
        }
        let mut real_to = fs_resolve(&self.root, to);
        if real_to.is_none() {
            return Union2::<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>>::U2(err(fs_escaped(to)));
        }
        return self.__dep_Fs.rename_path(real_from.as_ref().unwrap(), real_to.as_ref().unwrap());
    }

    fn read_line(&self, s: &InStream) -> Option<String> {
        return self.__dep_Fs.read_line(s);
    }

    fn read_all(&self, s: &InStream) -> Union2<String, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.read_all(s);
    }

    fn read_bytes(&self, s: &InStream, max: i32) -> Union2<Vec<u8>, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.read_bytes(s, max);
    }

    fn read_to(&self, s: &InStream, buf: &mut Vec<u8>, max: i32) -> Union2<i32, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.read_to(s, buf, max);
    }

    fn read_to__2(&self, s: &InStream, buf: &mut String) -> Union2<i64, Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.read_to__2(s, buf);
    }

    fn read_line_to(&self, s: &InStream, buf: &mut String) -> bool {
        return self.__dep_Fs.read_line_to(s, buf);
    }

    fn position(&self, s: &InStream) -> i64 {
        return self.__dep_Fs.position(s);
    }

    fn close(&self, s: InStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.close(s);
    }

    fn write(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_Fs.write(s, text);
    }

    fn write_line(&self, s: &OutStream, text: &String) -> i64 {
        return self.__dep_Fs.write_line(s, text);
    }

    fn write_bytes(&self, s: &OutStream, data: &Vec<u8>) -> i64 {
        return self.__dep_Fs.write_bytes(s, data);
    }

    fn position__2(&self, s: &OutStream) -> i64 {
        return self.__dep_Fs.position__2(s);
    }

    fn flush(&self, s: &OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.flush(s);
    }

    fn close__2(&self, s: OutStream) -> Union2<(), Checked<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>>> {
        return self.__dep_Fs.close__2(s);
    }
}
