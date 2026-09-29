use crate::core_iterator::*;
use crate::core_string::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileCredentials {
    pub profile: String,
    pub path: String,
}

impl crate::wire::__Wire for ProfileCredentials {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.profile, out);
        crate::wire::__Wire::__enc(&self.path, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            profile: crate::wire::__Wire::__dec(r)?,
            path: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn to_str__6(value: &ProfileCredentials) -> String {
    let mut out: String = [&"ProfileCredentials {".to_string()[..]].concat();
    out.push_str(&" ".to_string()[..]);
    out.push_str(&format!("profile: {}", value.profile.clone())[..]);
    out.push_str(&", ".to_string()[..]);
    out.push_str(&format!("path: {}", value.path.clone())[..]);
    out.push_str(&" }".to_string()[..]);
    return out;
}
