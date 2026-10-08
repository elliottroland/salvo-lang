use crate::core_string::append_platform;
use crate::core_string::mut_str;


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

pub type Credentials = crate::unions::Union3<crate::aws::ProfileCredentials, crate::aws::EnvironmentCredentials, crate::aws::DefaultChain>;

/// Factories for the host: one per arm of the union [platform-factory].
impl Credentials {
    pub fn profile_credentials(value: crate::aws::ProfileCredentials) -> Self {
        crate::unions::Union3::U1(value)
    }
    pub fn environment_credentials(value: crate::aws::EnvironmentCredentials) -> Self {
        crate::unions::Union3::U2(value)
    }
    pub fn default_chain(value: crate::aws::DefaultChain) -> Self {
        crate::unions::Union3::U3(value)
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
    pub credentials: crate::unions::Union3<crate::aws::ProfileCredentials, crate::aws::EnvironmentCredentials, crate::aws::DefaultChain>,
    pub region: crate::aws::Region,
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

pub fn to_str(value: &crate::aws::ProfileCredentials) -> String {
    let mut out: String = crate::core_string::mut_str(vec![String::from("ProfileCredentials {")]);
    crate::core_string::append_platform(&mut out, &String::from(" "));
    crate::core_string::append_platform(&mut out, &format!("profile: {}", value.profile));
    crate::core_string::append_platform(&mut out, &String::from(", "));
    crate::core_string::append_platform(&mut out, &format!("path: {}", value.path));
    crate::core_string::append_platform(&mut out, &String::from(" }"));
    return out;
}
