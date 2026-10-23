
#[derive(Clone, Debug, PartialEq)]
pub struct Checked<T> {
    pub value: T,
}

impl<T: Clone + 'static + crate::wire::__Wire> crate::wire::__Wire for Checked<T> {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn checked<T>(mut value: T) -> crate::core_checked::Checked<T> {
    return crate::core_checked::Checked { value: value };
}

pub fn ignore<T: Clone>(mut checked: crate::core_checked::Checked<T>) {
    std::mem::drop(checked);
}

pub fn detach<T>(mut checked: crate::core_checked::Checked<T>) -> T {
    return checked.value;
}

pub fn to_str<T: Clone>(checked: &crate::core_checked::Checked<T>, to_str: &mut dyn FnMut(&T) -> String) -> String {
    return to_str(&checked.value);
}
