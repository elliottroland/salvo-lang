// [qual-result-tags] The success/failure tags for result unions: a function that can fail
// returns `Ok T | Err E` and callers discriminate the arms with `when`.
// There is no `Result` type: the union *is* the result, so the arms carry
// their own payload types and `Ok Int | Err Str | None` composes like any
// other union.
//
// Own module rather than part of `std.core.basic` so a program that never
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

// [result-errs] The accumulating outcome: `Ok T | Errs NonEmpty List<E>` —
// every error, not the first (the CLI design record X12, Q11). Where `throw` and
// `Err E` stop at the first failure, a validation that wants the user to fix
// all problems at once (bad flags, a config file, a batch of rows) collects
// them; `both`, `map2` and `all` keep collecting after a failure. A
// provenance tag like `Err`, minted by its constructor below.
export provenance qualifier Errs<T> of T

// [result-errs] The failure arm of an accumulating outcome, from one error.
export fn errs<E>(first: E) [] -> +Errs NonEmpty List<E> => !first {
    return List(first)
}

// [result-errs] The same, named after the tag, from a non-empty list of
// errors: `Errs(List(a, b))`.
export fn Errs<T canbe linear>(value: T) [] -> +Errs T => !value {
    return value
}

// [result-errs] Both outcomes' values, or every error either holds — `a`'s
// first, then `b`'s (copied into one list, as `partition` copies).
export fn both<A, B, E>(a: Ok A | Errs NonEmpty List<E>, b: Ok B | Errs NonEmpty List<E>, ?copy: (v: E) -> E) [] -> Ok (A, B) | Errs NonEmpty List<E>
=> !a, !b {
    when a {
        is Ok {
            when b {
                is Ok { return Ok((a, b)) }
                is Errs { return Errs(b) }
            }
        }
        is Errs {
            when b {
                is Ok { return Errs(a) }
                is Errs { return Errs(joined(a, b)) }
            }
        }
    }
}

// [result-errs] `f` of both values, or every error either holds — `a`'s
// first, then `b`'s.
export fn map2<A, B, C, E>(a: Ok A | Errs NonEmpty List<E>, b: Ok B | Errs NonEmpty List<E>, f: (x: A, y: B) -> C, ?copy: (v: E) -> E) [] -> Ok C | Errs NonEmpty List<E>
=>[f] !x, !y => !a, !b, f {
    when a {
        is Ok {
            when b {
                is Ok { return Ok(f(a, b)) }
                is Errs { return Errs(b) }
            }
        }
        is Errs {
            when b {
                is Ok { return Errs(a) }
                is Errs { return Errs(joined(a, b)) }
            }
        }
    }
}

// Two error lists as one, `a`'s first. Its type parameter is not `E`: a
// generic caller passing its own `?copy` down to a callee whose parameter
// has the caller's name misinfers it (ROADMAP §2d item 7).
fn joined<X>(a: NonEmpty List<X>, b: NonEmpty List<X>, ?copy: (v: X) -> X) [] -> NonEmpty List<X> => !a, !b {
    let errors: Mut List<X> = Mut List<X>()
    for e in a {
        add(errors, copy(e))
    }
    for e in b {
        add(errors, copy(e))
    }
    if errors is NonEmpty {
        return errors
    }
    unreachable!("two non-empty lists joined are not empty")
}
