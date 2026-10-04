use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_deque::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

pub fn path(text: &String) -> Path {
    return Path { text: text.clone() };
}

pub fn to_str__6(p: &Path) -> String {
    return p.text.clone();
}

pub fn is_absolute(p: &Path) -> bool {
    return crate::core_string::starts_with_platform(&p.text, &("/".to_string()));
}

pub fn join__3(p: &Path, child: &String) -> Path {
    if crate::core_string::starts_with_platform(child, &("/".to_string())) || is_empty__2(&p.text) {
        return path(child);
    }
    if crate::core_string::ends_with_platform(&p.text, &("/".to_string())) {
        return path(&(format!("{}{}", p.text.clone(), child.clone())));
    }
    return path(&(format!("{}/{}", p.text.clone(), child.clone())));
}

pub fn join__4(p: &Path, child: &Path) -> Path {
    return join__3(p, &child.text);
}

pub fn parent(p: &Path) -> Option<Path> {
    let mut text = trim_trailing_slashes(&p.text);
    let mut cut = split_last(&text, &("/".to_string()));
    if cut.is_none() {
        return None;
    }
    let (mut dir, mut _name) = cut.as_ref().unwrap().clone();
    if is_empty__2(&dir) {
        if crate::core_string::starts_with_platform(&text, &("/".to_string())) && crate::core_string::size_platform(&text) > 1 {
            return Some(path(&("/".to_string())));
        }
        return None;
    }
    return Some(path(&dir));
}

pub fn file_name(p: &Path) -> Option<String> {
    let mut text = trim_trailing_slashes(&p.text);
    let mut cut = split_last(&text, &("/".to_string()));
    if cut.is_none() {
        if is_empty__2(&text) {
            return None;
        }
        return Some(text);
    }
    let (mut _dir, mut name) = cut.as_ref().unwrap().clone();
    if is_empty__2(&name) {
        return None;
    }
    return Some(name);
}

pub fn extension(p: &Path) -> Option<String> {
    let mut name = file_name(p);
    if name.is_some() {
        let mut n = name.as_ref().unwrap().clone();
        let mut cut = split_last(&n, &(".".to_string()));
        if cut.is_none() {
            return None;
        }
        let (mut stem, mut ext) = cut.as_ref().unwrap().clone();
        if is_empty__2(&stem) {
            return None;
        }
        return Some(ext);
    }
    return None;
}

pub fn stem(p: &Path) -> Option<String> {
    let mut name = file_name(p);
    if name.is_some() {
        let mut n = name.as_ref().unwrap().clone();
        let mut cut = split_last(&n, &(".".to_string()));
        if cut.is_none() {
            return Some(n);
        }
        let (mut stem, mut _ext) = cut.as_ref().unwrap().clone();
        if is_empty__2(&stem) {
            return Some(n);
        }
        return Some(stem);
    }
    return None;
}

pub fn with_extension(p: &Path, ext: &String) -> Path {
    let mut s = stem(p);
    if s.is_some() {
        let mut base = s.as_ref().unwrap().clone();
        let mut name = if is_empty__2(ext) {
            base
        } else {
            format!("{}.{}", base, ext.clone())
        };
        let mut up = parent(p);
        if up.is_some() {
            let mut dir = up.as_ref().unwrap().clone();
            return join__3(&dir, &name);
        }
        return path(&name);
    }
    return path(&(p.text.clone()));
}

pub fn segments(p: &Path) -> Vec<String> {
    let mut out = vec![];
    for mut part in crate::core_string::split_platform(&p.text, &("/".to_string())) {
        if !is_empty__2(&part) {
            out.push(part.clone());
        }
    }
    return out;
}

pub fn trim_trailing_slashes(text: &String) -> String {
    let mut t = text.clone();
    while crate::core_string::size_platform(&t) > 1 && crate::core_string::ends_with_platform(&t, &("/".to_string())) {
        t = crate::core_string::trim_suffix_platform(&t, &("/".to_string()));
    }
    return t;
}

pub fn hash__4(value: &Path) -> i64 {
    let mut h = 17i64;
    h = ((h).wrapping_mul(31).wrapping_add({ let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.text[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }));
    return h;
}

pub fn eq__5(a: &Path, b: &Path) -> bool {
    if !(&a.text[..] == &b.text[..]) {
        return false;
    }
    return true;
}
