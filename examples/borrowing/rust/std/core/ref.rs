
pub fn NotSame_qualifies<T: Clone>(b: &T, a: &T) -> bool {
    return !(std::ptr::eq(a, b));
}
