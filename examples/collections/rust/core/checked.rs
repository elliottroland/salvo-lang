use crate::core_iterator::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Checked<T: Clone + 'static> {
    pub value: T,
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
