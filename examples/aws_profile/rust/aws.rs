use crate::core_iterator::*;
use crate::core_string::*;
use crate::unions::*;

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

#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentCredentials {
}

impl crate::wire::__Wire for EnvironmentCredentials {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefaultChain {
}

impl crate::wire::__Wire for DefaultChain {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub code: String,
}

impl crate::wire::__Wire for Region {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.code, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            code: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AwsConfig {
    pub credentials: Union3<ProfileCredentials, EnvironmentCredentials, DefaultChain>,
    pub region: Region,
    pub endpoint: Option<String>,
}

impl crate::wire::__Wire for AwsConfig {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.credentials, out);
        crate::wire::__Wire::__enc(&self.region, out);
        crate::wire::__Wire::__enc(&self.endpoint, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            credentials: crate::wire::__Wire::__dec(r)?,
            region: crate::wire::__Wire::__dec(r)?,
            endpoint: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AwsError {
    pub code: String,
    pub message: String,
}

impl crate::wire::__Wire for AwsError {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.code, out);
        crate::wire::__Wire::__enc(&self.message, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            code: crate::wire::__Wire::__dec(r)?,
            message: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn to_str__7(value: &ProfileCredentials) -> String {
    let mut out: String = [&"ProfileCredentials {".to_string()[..]].concat();
    out.push_str(&" ".to_string()[..]);
    out.push_str(&format!("profile: {}", value.profile.clone())[..]);
    out.push_str(&", ".to_string()[..]);
    out.push_str(&format!("path: {}", value.path.clone())[..]);
    out.push_str(&" }".to_string()[..]);
    return out;
}
