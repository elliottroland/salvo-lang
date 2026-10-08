use crate::core_list::add_platform;
use crate::core_string::ends_with_platform;
use crate::core_string::is_empty;
use crate::core_compare::mix_hash;
use crate::core_string::size_platform;
use crate::core_string::split_last;
use crate::core_string::split_platform;
use crate::core_string::starts_with_platform;
use crate::core_string::trim_suffix_platform;


#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub text: String,
}

impl crate::wire::__Wire for Path {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.text, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            text: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn path(text: &String) -> crate::fs_path::Path {
    return crate::fs_path::Path { text: (text).clone() };
}

pub fn to_str(p: &crate::fs_path::Path) -> String {
    return (p.text).clone();
}

pub fn is_absolute(p: &crate::fs_path::Path) -> bool {
    return crate::core_string::starts_with_platform(&p.text, &String::from("/"));
}

pub fn join__Path_Str(p: &crate::fs_path::Path, child: &String) -> crate::fs_path::Path {
    if (crate::core_string::starts_with_platform(child, &String::from("/")) || crate::core_string::is_empty(&p.text)) {
        return crate::fs_path::path(child);
    };
    if crate::core_string::ends_with_platform(&p.text, &String::from("/")) {
        return crate::fs_path::path(&format!("{}{}", p.text, child));
    };
    return crate::fs_path::path(&format!("{}/{}", p.text, child));
}

pub fn join__Path_Path(p: &crate::fs_path::Path, child: &crate::fs_path::Path) -> crate::fs_path::Path {
    return crate::fs_path::join__Path_Str(p, &child.text);
}

pub fn parent(p: &crate::fs_path::Path) -> Option<crate::fs_path::Path> {
    let mut text: String = crate::fs_path::trim_trailing_slashes(&p.text);
    let mut cut: Option<(String, String)> = crate::core_string::split_last(&text, &String::from("/"));
    if cut.is_none() {
        return None;
    };
    let mut cut_1 = cut.as_ref().unwrap();
    let mut __destructured_2: (String, String) = cut_1.clone();
    let mut dir: String = __destructured_2.0;
    let mut _name: String = __destructured_2.1;
    if crate::core_string::is_empty(&dir) {
        if (crate::core_string::starts_with_platform(&text, &String::from("/")) && (crate::core_string::size_platform(&text) > 1i32)) {
            return Some(crate::fs_path::path(&String::from("/")));
        };
        return None;
    };
    return Some(crate::fs_path::path(&dir));
}

pub fn file_name(p: &crate::fs_path::Path) -> Option<String> {
    let mut text: String = crate::fs_path::trim_trailing_slashes(&p.text);
    let mut cut: Option<(String, String)> = crate::core_string::split_last(&text, &String::from("/"));
    if cut.is_none() {
        if crate::core_string::is_empty(&text) {
            return None;
        };
        return Some(text.clone());
    };
    let mut cut_1 = cut.unwrap();
    let mut __destructured_2: (String, String) = cut_1;
    let mut _dir: String = __destructured_2.0;
    let mut name: String = __destructured_2.1;
    if crate::core_string::is_empty(&name) {
        return None;
    };
    return Some(name.clone());
}

pub fn extension(p: &crate::fs_path::Path) -> Option<String> {
    let mut name: Option<String> = crate::fs_path::file_name(p);
    if name.is_some() {
        let mut n = name.as_ref().unwrap();
        let mut cut: Option<(String, String)> = crate::core_string::split_last(n, &String::from("."));
        if cut.is_none() {
            return None;
        };
        let mut cut_1 = cut.unwrap();
        let mut __destructured_2: (String, String) = cut_1;
        let mut stem: String = __destructured_2.0;
        let mut ext: String = __destructured_2.1;
        if crate::core_string::is_empty(&stem) {
            return None;
        };
        return Some(ext.clone());
    };
    return None;
}

pub fn stem(p: &crate::fs_path::Path) -> Option<String> {
    let mut name: Option<String> = crate::fs_path::file_name(p);
    if name.is_some() {
        let mut n = name.as_ref().unwrap();
        let mut cut: Option<(String, String)> = crate::core_string::split_last(n, &String::from("."));
        if cut.is_none() {
            return Some(n.clone());
        };
        let mut cut_1 = cut.unwrap();
        let mut __destructured_2: (String, String) = cut_1;
        let mut stem: String = __destructured_2.0;
        let mut _ext: String = __destructured_2.1;
        if crate::core_string::is_empty(&stem) {
            return Some(n.clone());
        };
        return Some(stem.clone());
    };
    return None;
}

pub fn with_extension(p: &crate::fs_path::Path, ext: &String) -> crate::fs_path::Path {
    let mut s: Option<String> = crate::fs_path::stem(p);
    if s.is_some() {
        let mut base = s.as_ref().unwrap();
        let mut name: String = if crate::core_string::is_empty(ext) {
            base.clone()
        } else {
            format!("{}.{}", base, ext)
        };
        let mut up: Option<crate::fs_path::Path> = crate::fs_path::parent(p);
        if up.is_some() {
            let mut dir = up.as_ref().unwrap();
            return crate::fs_path::join__Path_Str(dir, &name);
        };
        return crate::fs_path::path(&name);
    };
    return crate::fs_path::path(&(p.text).clone());
}

pub fn segments(p: &crate::fs_path::Path) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for mut part in crate::core_string::split_platform(&p.text, &String::from("/")).into_iter() {
        if !(crate::core_string::is_empty(&part)) {
            crate::core_list::add_platform::<String>(&mut out, (part).clone());
        };
    }
    return out;
}

pub fn trim_trailing_slashes(text: &String) -> String {
    let mut t: String = (text).clone();
    loop {
        if !(((crate::core_string::size_platform(&t) > 1i32) && crate::core_string::ends_with_platform(&t, &String::from("/")))) {
            break;
        };
        t = crate::core_string::trim_suffix_platform(&t, &String::from("/"));
    }
    return t;
}

pub fn hash(value: &crate::fs_path::Path) -> i64 {
    let mut h: i64 = 17i64;
    h = crate::core_compare::mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.text[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq(a: &crate::fs_path::Path, b: &crate::fs_path::Path) -> bool {
    if !((&a.text[..] == &b.text[..])) {
        return false;
    };
    return true;
}
