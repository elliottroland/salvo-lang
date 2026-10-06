use crate::core_actor::__Stateful_Faults as _;
use crate::core_actor::__Stateless_Faults as _;
use crate::core_compare::mix_hash;
use crate::core_list::List;
use crate::core_string::Str;
use crate::core_string::ends_with_platform;
use crate::core_string::is_empty;
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

pub fn path(text: &String) -> Path {
    return Path { text: text.clone() };
}

pub fn to_str(p: &Path) -> String {
    return p.text.clone();
}

pub fn is_absolute(p: &Path) -> bool {
    return crate::core_string::starts_with_platform(&p.text, &("/".to_string()));
}

pub fn join__Path_Str(p: &Path, child: &String) -> Path {
    if crate::core_string::starts_with_platform(child, &("/".to_string())) || is_empty(&p.text) {
        return path(child);
    }
    if crate::core_string::ends_with_platform(&p.text, &("/".to_string())) {
        return path(&(format!("{}{}", p.text.clone(), child.clone())));
    }
    return path(&(format!("{}/{}", p.text.clone(), child.clone())));
}

pub fn join__Path_Path(p: &Path, child: &Path) -> Path {
    return join__Path_Str(p, &child.text);
}

pub fn parent(p: &Path) -> Option<Path> {
    let mut text = trim_trailing_slashes(&p.text);
    let mut cut = split_last(&text, &("/".to_string()));
    if cut.is_none() {
        return None;
    }
    let (mut dir, mut _name) = cut.as_ref().unwrap().clone();
    if is_empty(&dir) {
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
        if is_empty(&text) {
            return None;
        }
        return Some(text);
    }
    let (mut _dir, mut name) = cut.as_ref().unwrap().clone();
    if is_empty(&name) {
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
        if is_empty(&stem) {
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
        if is_empty(&stem) {
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
        let mut name = if is_empty(ext) {
            base
        } else {
            format!("{}.{}", base, ext.clone())
        };
        let mut up = parent(p);
        if up.is_some() {
            let mut dir = up.as_ref().unwrap().clone();
            return join__Path_Str(&dir, &name);
        }
        return path(&name);
    }
    return path(&(p.text.clone()));
}

pub fn segments(p: &Path) -> Vec<String> {
    let mut out = vec![];
    for mut part in crate::platform_core_list::each(&(crate::core_string::split_platform(&p.text, &("/".to_string())))).map(|__x| __x.clone()) {
        if !is_empty(&part) {
            crate::core_list::add_platform(&mut out, part.clone());
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

pub fn hash(value: &Path) -> i64 {
    let mut h = 17i64;
    h = mix_hash(h, { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&value.text[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) });
    return h;
}

pub fn eq(a: &Path, b: &Path) -> bool {
    if !(&a.text[..] == &b.text[..]) {
        return false;
    }
    return true;
}
