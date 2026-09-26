// `Checked<T>` wraps a value the caller must *look at*: it is a `linear struct`,
// so letting one go out of scope is a compile-time error rather than a silently
// dropped answer. The shape exists for the answers a program should not be able
// to ignore by accident — "did the write land?", "what went wrong?" — where a
// plain `Bool` or a plain error value is one forgotten line away from a bug
// [linear-group].
//
// Two ways to discharge the obligation: [detach] the value and deal with it, or
// [ignore] it, which says in the source that not looking was the intent.
//
// In `core` because std's own surfaces answer with it (`swap` on a list), and a
// type in a returned position has to be visible wherever the function is.
export linear struct Checked<T> {
    value: T
}

// Wraps [value] in the obligation.
export fn checked<T canbe linear>(value: T) -> Checked<T> {
    return Checked<T> { value: value }
}

// Discharges the obligation without reading the value: the explicit "I know,
// and I do not care" that keeps the rule honest — silence would make the
// obligation a nuisance rather than a check.
//
// A linear [T] is refused here, because discarding the wrapper would discharge
// the *inner* obligation too, which nothing has looked at.
export fn ignore<T>(checked: Checked<T>) => !checked {
    discard(checked)
}

// Discharges the obligation and answers the value inside, which is what a
// caller that means to check it writes.
export fn detach<T canbe linear>(checked: Checked<T>) -> T => !checked {
    return checked.value
}
