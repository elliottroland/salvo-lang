
pub fn NonEmpty_qualifies<T>(value: &T, size: &mut dyn FnMut(&T) -> i32) -> bool {
    return (size(value) > 0i32);
}
