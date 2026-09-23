use crate::core_iterator::*;

pub fn ok<T: Clone>(value: T) -> T {
    return value;
}

pub fn err<T: Clone>(value: T) -> T {
    return value;
}
