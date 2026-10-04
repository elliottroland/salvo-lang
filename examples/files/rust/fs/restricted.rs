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

pub fn fs_resolve(root: &String, path: &String) -> Option<String> {
    if crate::core_string::starts_with_platform(path, &("/".to_string())) {
        return None;
    }
    let mut segs = crate::core_string::split_platform(path, &("/".to_string()));
    let mut kept: Vec<String> = vec![];
    let mut skip = 0;
    let mut i = crate::core_list::size_platform(&segs) - 1;
    while i >= 0 {
        let mut seg = crate::core_list::get_platform(&segs, i).unwrap();
        if seg.clone() == "..".to_string() {
            skip = skip + 1;
        } else {
            if crate::core_string::size_platform(seg) == 0 || seg.clone() == ".".to_string() {
            } else {
                if skip > 0 {
                    skip = skip - 1;
                } else {
                    crate::core_list::add_platform(&mut kept, seg.clone());
                }
            }
        }
        i = i - 1;
    }
    if skip > 0 {
        return None;
    }
    let mut parts: Vec<String> = vec![];
    let mut j = crate::core_list::size_platform(&kept) - 1;
    while j >= 0 {
        crate::core_list::add_platform(&mut parts, crate::core_list::get_platform(&kept, j).expect("salvo: value is absent at fs.restricted:58:24").clone());
        j = j - 1;
    }
    let mut rel = crate::core_string::join_platform(&parts, &("/".to_string()));
    if crate::core_string::size_platform(&rel) == 0 {
        return Some(root.clone());
    }
    return Some(format!("{}/{}", root.clone(), rel));
}

pub fn fs_escaped(path: &String) -> Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> {
    return checked(Union7::<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>::U5(PathEscapes { path: path.clone() }));
}

#[derive(Clone)]
pub struct RestrictedFs {
    root: String,
    __dep_Fs: crate::fs::Fs,
    __dep_Streams: crate::stream::Streams,
}

impl RestrictedFs {
    pub fn new(root: String, __dep_Fs: crate::fs::Fs, __dep_Streams: crate::stream::Streams) -> Self {
        Self {
            root,
            __dep_Fs,
            __dep_Streams,
        }
    }
}

impl crate::fs::__Stateless_Fs for RestrictedFs {

    fn open_read(&self, path: &String) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_read(real.as_ref().unwrap());
    }

    fn open_read_at(&self, path: &String, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_read_at(real.as_ref().unwrap(), offset);
    }

    fn open_write(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.open_write(real.as_ref().unwrap());
    }

    fn open_append(&self, path: &String) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
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

    fn metadata(&self, path: &String) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.metadata(real.as_ref().unwrap());
    }

    fn list_dir(&self, path: &String) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.list_dir(real.as_ref().unwrap());
    }

    fn create_dirs(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.create_dirs(real.as_ref().unwrap());
    }

    fn delete(&self, path: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real = fs_resolve(&self.root, path);
        if real.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(path)));
        }
        return self.__dep_Fs.delete(real.as_ref().unwrap());
    }

    fn rename_path(&self, from: &String, to: &String) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut real_from = fs_resolve(&self.root, from);
        if real_from.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(from)));
        }
        let mut real_to = fs_resolve(&self.root, to);
        if real_to.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(to)));
        }
        return self.__dep_Fs.rename_path(real_from.as_ref().unwrap(), real_to.as_ref().unwrap());
    }
}
