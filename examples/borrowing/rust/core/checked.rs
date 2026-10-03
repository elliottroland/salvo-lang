use crate::core_iterator::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Checked<T: Clone> {
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

pub fn checked<T: Clone>(value: T) -> Checked<T> {
    return Checked { value: value };
}

pub fn ignore<T: Clone>(checked: Checked<T>) {
    drop(checked);
}

pub fn detach<T: Clone>(checked: Checked<T>) -> T {
    return checked.value;
}
