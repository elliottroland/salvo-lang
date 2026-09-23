// Wraps a value in a linear struct, ensuring that the user checks it on pain of a compile-time error.
// Useful for cases where the user should not accidentally ignore the response of a function.
// Two mechanisms exist for discharging the obligation: either explicitly [ignore] it, or [take] the value to check.
linear struct Checked<T> {
    value: T
}

fn checked<T canbe linear>(value: T) -> Checked<T> {
    return Checked<T> { value: value }
}

// In this case, we can't allow a linear [T], because this would mistakenly discharge its obligation as well.
fn ignore<T>(checked: Checked<T>) => !checked {
    discard(checked)
}

// Discharges the linear obligation on [checked], and returns the value within it.
fn take<T canbe linear>(checked: Checked<T>) -> T => !checked {
    return checked.value
}