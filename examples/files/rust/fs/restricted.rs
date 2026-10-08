use crate::fs::AlreadyExists;
use crate::core_checked::Checked;
use crate::fs::FileInfo;
use crate::fs::Fs;
use crate::stream::InStream;
use crate::fs::IoError;
use crate::fs::NotADirectory;
use crate::fs::NotFound;
use crate::stream::OutStream;
use crate::fs_path::Path;
use crate::fs::PathEscapes;
use crate::fs::PermissionDenied;
use crate::fs::Streaming;
use crate::stream::Streams;
use crate::core_list::add_platform;
use crate::core_checked::checked;
use crate::core_result::err;
use crate::core_list::get_platform;
use crate::core_string::join_platform;
use crate::core_string::split_platform;
use crate::core_string::starts_with_platform;
use crate::fs_path::to_str;


pub fn fs_resolve(root: &String, path: &String) -> Option<String> {
    if crate::core_string::starts_with_platform(path, &String::from("/")) {
        return None;
    };
    let mut segs: Vec<String> = crate::core_string::split_platform(path, &String::from("/"));
    let mut kept: Vec<String> = vec![];
    let mut skip: i32 = 0i32;
    let mut i: i32 = i32::wrapping_sub(crate::core_list::size_platform::<String>(&segs), 1i32);
    loop {
        if !((i >= 0i32)) {
            break;
        };
        let mut seg: &String = {
            let mut __nn_1: Option<&String> = crate::core_list::get_platform::<String>(&segs, i);
            if __nn_1.is_none() {
                panic!("salvo: value is absent at fs.restricted:36:19");
            } else {
                let mut __some_2 = __nn_1.unwrap();
                __some_2
            }
        };
        if (&seg[..] == &String::from("..")[..]) {
            skip = i32::wrapping_add(skip, 1i32);
        } else {
            if (((crate::core_string::size_platform(seg)) == (0i32)) || (&seg[..] == &String::from(".")[..])) {
            } else {
                if (skip > 0i32) {
                    skip = i32::wrapping_sub(skip, 1i32);
                } else {
                    crate::core_list::add_platform::<String>(&mut kept, (seg).clone());
                };
            };
        };
        i = i32::wrapping_sub(i, 1i32);
    }
    if (skip > 0i32) {
        return None;
    };
    let mut parts: Vec<String> = vec![];
    let mut j: i32 = i32::wrapping_sub(crate::core_list::size_platform::<String>(&kept), 1i32);
    loop {
        if !((j >= 0i32)) {
            break;
        };
        crate::core_list::add_platform::<String>(&mut parts, ({
            let mut __nn_3: Option<&String> = crate::core_list::get_platform::<String>(&kept, j);
            if __nn_3.is_none() {
                panic!("salvo: value is absent at fs.restricted:59:24");
            } else {
                let mut __some_4 = __nn_3.unwrap();
                __some_4
            }
        }).clone());
        j = i32::wrapping_sub(j, 1i32);
    }
    let mut rel: String = crate::core_string::join_platform(&parts, &String::from("/"));
    if ((crate::core_string::size_platform(&rel)) == (0i32)) {
        return Some((root).clone());
    };
    return Some(format!("{}/{}", root, rel));
}

pub fn fs_escaped(path: &String) -> crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> {
    return crate::core_checked::checked::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(crate::unions::Union7::U5(crate::fs::PathEscapes { path: (path).clone() }));
}

#[derive(Clone)]
pub struct RestrictedFs {
    root: crate::fs_path::Path,
    __dep0: crate::fs::Fs,
    __dep1: crate::stream::Streams,
}

impl RestrictedFs {
    pub fn new(root: crate::fs_path::Path, __dep0: crate::fs::Fs, __dep1: crate::stream::Streams) -> Self {
        Self {
            root,
            __dep0,
            __dep1
        }
    }
}

impl crate::fs::__Stateless_Fs for RestrictedFs {
    fn open_read(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.open_read(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn open_read_at(&self, path: &crate::fs_path::Path, offset: i64) -> crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.open_read_at(&crate::fs_path::Path { text: real_1.clone() }, offset);
    }
    fn open_write(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.open_write(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn open_append(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.open_append(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn exists(&self, path: &crate::fs_path::Path) -> bool {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return false;
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.exists(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn metadata(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.metadata(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn list_dir(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.list_dir(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn create_dirs(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.create_dirs(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn delete(&self, path: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut path_text: String = crate::fs_path::to_str(path);
        let mut real: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.as_ref().unwrap();
        return self.__dep0.delete(&crate::fs_path::Path { text: real_1.clone() });
    }
    fn rename_path(&self, from: &crate::fs_path::Path, to: &crate::fs_path::Path) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> {
        let mut from_text: String = crate::fs_path::to_str(from);
        let mut to_text: String = crate::fs_path::to_str(to);
        let mut real_from: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &from_text);
        if real_from.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&from_text)));
        };
        let mut real_to: Option<String> = crate::fs_restricted::fs_resolve(&crate::fs_path::to_str(&self.root), &to_text);
        if real_to.is_none() {
            return crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>>(crate::fs_restricted::fs_escaped(&to_text)));
        };
        let mut real_from_1 = real_from.as_ref().unwrap();
        let mut real_to_2 = real_to.as_ref().unwrap();
        return self.__dep0.rename_path(&crate::fs_path::Path { text: real_from_1.clone() }, &crate::fs_path::Path { text: real_to_2.clone() });
    }
}
