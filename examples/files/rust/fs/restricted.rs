use crate::unions::*;
use crate::core_checked::Checked;
use crate::core_checked::checked;
use crate::core_result::err;
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
use crate::stream::InStream;
use crate::stream::OutStream;
use crate::stream::__Stateful_Streams as _;
use crate::stream::__Stateless_Streams as _;

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
        crate::core_list::add_platform(&mut parts, crate::core_list::get_platform(&kept, j).expect("salvo: value is absent at fs.restricted:59:24").clone());
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
    root: Path,
    __dep_Fs: crate::fs::Fs,
    __dep_Streams: crate::stream::Streams,
}

impl RestrictedFs {
    pub fn new(root: Path, __dep_Fs: crate::fs::Fs, __dep_Streams: crate::stream::Streams) -> Self {
        Self {
            root,
            __dep_Fs,
            __dep_Streams,
        }
    }
}

impl crate::fs::__Stateless_Fs for RestrictedFs {

    fn open_read(&self, path: &Path) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.open_read(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn open_read_at(&self, path: &Path, offset: i64) -> Union2<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<InStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.open_read_at(&(Path { text: real.as_ref().unwrap().clone() }), offset);
    }

    fn open_write(&self, path: &Path) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.open_write(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn open_append(&self, path: &Path) -> Union2<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<OutStream, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.open_append(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn exists(&self, path: &Path) -> bool {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return false;
        }
        return self.__dep_Fs.exists(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn metadata(&self, path: &Path) -> Union2<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<FileInfo, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.metadata(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn list_dir(&self, path: &Path) -> Union2<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<Vec<String>, Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.list_dir(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn create_dirs(&self, path: &Path) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.create_dirs(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn delete(&self, path: &Path) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut path_text = to_str(path);
        let mut real = fs_resolve(&(to_str(&self.root)), &path_text);
        if real.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&path_text)));
        }
        return self.__dep_Fs.delete(&(Path { text: real.as_ref().unwrap().clone() }));
    }

    fn rename_path(&self, from: &Path, to: &Path) -> Union2<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>> {
        let mut from_text = to_str(from);
        let mut to_text = to_str(to);
        let mut real_from = fs_resolve(&(to_str(&self.root)), &from_text);
        if real_from.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&from_text)));
        }
        let mut real_to = fs_resolve(&(to_str(&self.root)), &to_text);
        if real_to.is_none() {
            return Union2::<(), Checked<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>>>::U2(err(fs_escaped(&to_text)));
        }
        return self.__dep_Fs.rename_path(&(Path { text: real_from.as_ref().unwrap().clone() }), &(Path { text: real_to.as_ref().unwrap().clone() }));
    }
}
