// [qual-result-tags] The success/failure tags for result unions: a function that can fail
// returns `Ok T | Err E` and callers discriminate the arms with `when`.
// There is no `Result` type: the union *is* the result, so the arms carry
// their own payload types and `Ok Int | Err Str | None` composes like any
// other union.
//
// Own module rather than part of `core.basic` so a program that never
// mentions these names emits no code for them [mod-used-only].

// [qual-constructive] Both tags are constructive: a plain value never
// subtypes `Ok T`, it gains the tag by going through the constructor
// below — which must live in this file [qual-ctor-same-file]. Erased in
// generated code [qual-erasure]: the union wrapper carries the arm, the
// qualifier only decides which one.
qualifier Ok<T> of T
qualifier Err<T> of T

// [qual-ctor-fn] Tags a value as the success arm of a result union. The
// value is moved into the result, so nothing is kept ([] deductions);
// linear values may be tagged, since the obligation travels with them
// [linear-generics].
fn ok<T canbe Linear>(value: T) [] -> T as Ok {
    return value
}

// [qual-ctor-fn] Tags a value as the failure arm of a result union.
fn err<T canbe Linear>(value: T) [] -> T as Err {
    return value
}
