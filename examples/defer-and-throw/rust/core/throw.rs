use crate::core_iterator::*;

pub fn thrown<M: Clone + 'static>(message: M) -> M {
    return message;
}
