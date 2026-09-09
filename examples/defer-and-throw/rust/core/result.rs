use crate::core_iterator::*;

pub fn ok<T: Clone + 'static>(value: T) -> T {
    return value;
}

pub fn err<T: Clone + 'static>(value: T) -> T {
    return value;
}
