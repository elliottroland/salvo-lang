// An answer the caller must look at. A function that returns `Checked<T>`
// instead of a plain `T` makes ignoring its answer a compile-time error, so
// "did the write land?" or "what went wrong?" cannot be dropped by
// forgetting a line.
//
// `Checked<T>` is a **linear** type: a value of it must be used exactly once,
// and letting one go out of scope unused is reported where it happens (see
// the Linear types topic). There are two ways to use it, and both are
// explicit in the source:
//
// - [detach] answers the value inside, for a caller that means to act on it;
// - [ignore] discards it, for a caller that has decided the answer does not
//   matter.
//
// ```
// let xs = Mut List(1, 2, 3)
// if detach(swap(xs, 0, 2)) {
//     println("swapped")                               // printed
// }
// ignore(swap(xs, 0, 9))                               // out of range; we do not care
// ```
//
// A list's `swap` answers a `Checked<Bool>`, and the filesystem and stream
// functions report failures as `Err Checked<FsError>` or
// `Err Checked<StreamError>`. In a result union like those, narrowing the
// value to its `Ok` arm also settles the obligation, because the `Ok` arm
// holds no `Checked`.
//
// The obligation is checked entirely at compile time: at run time a
// `Checked<T>` is a plain struct holding its value, with no destructor and
// no runtime test on either backend.

// A value wrapped so that the caller has to look at it: through [detach],
// which answers it, or [ignore], which discards it.
// @compilerrules [linear-group]
export linear struct Checked<T> {
    // The answer being reported.
    value: T
}

// Wraps a value in the obligation to look at it. This is what a function
// returning a [Checked] calls on its way out.
//
// ```
// fn saved(ok: Bool) [] -> Checked<Bool> => !ok {
//     return checked(ok)
// }
// ```
//
// @param value  the answer, moved into the wrapper
export fn checked<T canbe linear>(value: T) -> Checked<T> => !value {
    return Checked<T> { value: value }
}

// Discards a checked value without reading it: the explicit "I know, and I
// do not care".
//
// The value inside must not owe anything itself. A [Checked] around a
// linear value is refused here, because discarding the wrapper would also
// discard the inner obligation, which nothing has looked at: [detach] it and
// deal with the value instead.
//
// ```
// let xs = Mut List(1, 2)
// ignore(swap(xs, 0, 1))
// ```
export fn ignore<T>(checked: Checked<T>) => !checked {
    // Silence would make the obligation a nuisance rather than a check, so
    // not looking has to be written.
}

// Settles the obligation and answers the value inside, for a caller that
// means to act on it.
//
// This is also how a failure is kept for later: detach it, and store the
// `FsError` it answers rather than the checked value.
//
// ```
// let xs = Mut List(1, 2)
// let moved: Bool = detach(swap(xs, 0, 1))             // true
// ```
export fn detach<T canbe linear>(checked: Checked<T>) -> T => !checked {
    return checked.value
}

// The text of a checked value, which is its value's text. An error can be
// interpolated without detaching it first; reading it does not settle the
// obligation, which stays with the caller.
//
// @param to_str  found at the call site: the text form of the value inside
// @compilerrules [interp-to-str]
export fn to_str<T>(checked: Checked<T>, ?to_str: (x: T) -> Str) [] -> Str => checked {
    return to_str(checked.value)
}
