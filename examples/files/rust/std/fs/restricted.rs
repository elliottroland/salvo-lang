use crate::std_fs::AlreadyExists;
use crate::std_core_checked::Checked;
use crate::std_fs::FileInfo;
use crate::std_fs::Fs;
use crate::std_stream::InStream;
use crate::std_fs::IoError;
use crate::std_fs::NotADirectory;
use crate::std_fs::NotFound;
use crate::std_stream::OutStream;
use crate::std_fs_path::Path;
use crate::std_fs::PathEscapes;
use crate::std_fs::PermissionDenied;
use crate::std_fs::Streaming;
use crate::std_stream::Streams;
use crate::std_core_list::add_platform;
use crate::std_core_checked::checked;
use crate::std_core_result::err;
use crate::std_core_list::get_platform;
use crate::std_core_string::join_platform;
use crate::std_core_string::split_platform;
use crate::std_core_string::starts_with_platform;
use crate::std_fs_path::to_str;


pub fn fs_resolve(root: &String, path: &String) -> Option<String> {
    if crate::std_core_string::starts_with_platform(path, &String::from("/")) {
        return None;
    };
    let mut segs: Vec<String> = crate::std_core_string::split_platform(path, &String::from("/"));
    let mut kept: Vec<String> = vec![];
    let mut skip: i32 = 0i32;
    let mut i: i32 = i32::wrapping_sub(crate::std_core_list::size_platform::<String>(&segs), 1i32);
    loop {
        if !((i >= 0i32)) {
            break;
        };
        let mut seg: &String = {
            let mut __pick_1: Option<&String> = crate::std_core_list::get_platform::<String>(&segs, i);
            if __pick_1.is_some() {
                let mut __picked_2 = __pick_1.unwrap();
                __picked_2
            } else {
                panic!("salvo: unreachable at std.fs.restricted:36:35")
            }
        };
        if (&seg[..] == &String::from("..")[..]) {
            skip = i32::wrapping_add(skip, 1i32);
        } else {
            if ((crate::std_core_string::size_platform(seg) == 0i32) || (&seg[..] == &String::from(".")[..])) {
            } else {
                if (skip > 0i32) {
                    skip = i32::wrapping_sub(skip, 1i32);
                } else {
                    crate::std_core_list::add_platform::<String>(&mut kept, (seg).clone());
                };
            };
        };
        i = i32::wrapping_sub(i, 1i32);
    }
    if (skip > 0i32) {
        return None;
    };
    let mut parts: Vec<String> = vec![];
    let mut j: i32 = i32::wrapping_sub(crate::std_core_list::size_platform::<String>(&kept), 1i32);
    loop {
        if !((j >= 0i32)) {
            break;
        };
        crate::std_core_list::add_platform::<String>(&mut parts, {
            let mut __pick_4: Option<&String> = crate::std_core_list::get_platform::<String>(&kept, j);
            if __pick_4.is_some() {
                let mut __picked_5 = __pick_4.unwrap();
                (__picked_5).clone()
            } else {
                panic!("salvo: unreachable at std.fs.restricted:59:40")
            }
        });
        j = i32::wrapping_sub(j, 1i32);
    }
    let mut rel: String = crate::std_core_string::join_platform(&parts, &String::from("/"));
    if (crate::std_core_string::size_platform(&rel) == 0i32) {
        return Some((root).clone());
    };
    return Some(format!("{}/{}", root, rel));
}

pub fn fs_escaped(path: &String) -> crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> {
    return crate::std_core_checked::checked::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(crate::unions::Union7::U5(crate::std_fs::PathEscapes { path: (path).clone() }));
}

#[derive(Clone)]
pub struct RestrictedFs {
    root: crate::std_fs_path::Path,
    __dep0: crate::std_fs::Fs,
    __dep1: crate::std_stream::Streams,
}

impl RestrictedFs {
    pub fn new(root: crate::std_fs_path::Path, __dep0: crate::std_fs::Fs, __dep1: crate::std_stream::Streams) -> Self {
        Self {
            root,
            __dep0,
            __dep1
        }
    }
}

impl crate::std_fs::__Stateless_Fs for RestrictedFs {
    fn open_read(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.open_read(&crate::std_fs_path::Path { value: real_1 });
    }
    fn open_read_at(&self, path: &crate::std_fs_path::Path, offset: i64) -> crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.open_read_at(&crate::std_fs_path::Path { value: real_1 }, offset);
    }
    fn open_write(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_stream::OutStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.open_write(&crate::std_fs_path::Path { value: real_1 });
    }
    fn open_append(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_stream::OutStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.open_append(&crate::std_fs_path::Path { value: real_1 });
    }
    fn exists(&self, path: &crate::std_fs_path::Path) -> bool {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return false;
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.exists(&crate::std_fs_path::Path { value: real_1 });
    }
    fn metadata(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<crate::std_fs::FileInfo, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.metadata(&crate::std_fs_path::Path { value: real_1 });
    }
    fn list_dir(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<Vec<String>, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.list_dir(&crate::std_fs_path::Path { value: real_1 });
    }
    fn create_dirs(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.create_dirs(&crate::std_fs_path::Path { value: real_1 });
    }
    fn delete(&self, path: &crate::std_fs_path::Path) -> crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut path_text: String = crate::std_fs_path::to_str(path);
        let mut real: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &path_text);
        if real.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&path_text)));
        };
        let mut real_1 = real.unwrap();
        return self.__dep0.delete(&crate::std_fs_path::Path { value: real_1 });
    }
    fn rename_path(&self, from: &crate::std_fs_path::Path, to: &crate::std_fs_path::Path) -> crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> {
        let mut from_text: String = crate::std_fs_path::to_str(from);
        let mut to_text: String = crate::std_fs_path::to_str(to);
        let mut real_from: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &from_text);
        if real_from.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&from_text)));
        };
        let mut real_to: Option<String> = crate::std_fs_restricted::fs_resolve(&crate::std_fs_path::to_str(&self.root), &to_text);
        if real_to.is_none() {
            return crate::unions::Union2::U2(crate::std_core_result::err::<crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>>(crate::std_fs_restricted::fs_escaped(&to_text)));
        };
        let mut real_from_1 = real_from.unwrap();
        let mut real_to_2 = real_to.unwrap();
        return self.__dep0.rename_path(&crate::std_fs_path::Path { value: real_from_1 }, &crate::std_fs_path::Path { value: real_to_2 });
    }
}
