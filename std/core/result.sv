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
export provenance qualifier Ok<T> of T
export provenance qualifier Err<T> of T

// [qual-ctor-fn] Tags a value as the success arm of a result union. The
// value is moved into the result, so nothing is kept ([] deductions);
// linear values may be tagged, since the obligation travels with them
// [linear-generics].
export fn ok<T canbe linear>(value: T) [] -> +Ok T => !value {
    return value
}

// [qual-ctor-fn] Tags a value as the failure arm of a result union.
export fn err<T canbe linear>(value: T) [] -> +Err T => !value {
    return value
}

// [fn-type-named] The same constructors, named after the provenance
// qualifier they establish: `Ok(value)` reads as "the `Ok` arm of this
// value". A type-named function whose name *is* a provenance qualifier
// (no base type) is the provenance form of [fn-type-named] — it carries no
// base, only the tag — and is a constructor like the lowercase `ok`/`err`
// above, subject to the same same-file rule [qual-ctor-same-file].
export fn Ok<T canbe linear>(value: T) [] -> +Ok T => !value {
    return value
}

export fn Err<T canbe linear>(value: T) [] -> +Err T => !value {
    return value
}
