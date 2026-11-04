// The tags for the result of a function that can fail: `Ok` marks the
// success arm of a union and `Err` the failure arm. A function that can fail
// returns `Ok T | Err E`, and its caller tells the arms apart with `is` or
// `when`, as on any union.
//
// There is no `Result` type: the union *is* the result. Each arm keeps its
// own payload type, so a result composes like any other union
// (`Ok Int | Err Str | None` is a result that may also be absent), and a
// caller may handle one arm and pass the rest on.
//
// ```
// fn parse_port(s: Str) [] -> Ok Int | Err Str => s {
//     let n = parse_int(s)
//     if n is None {
//         return err("not a number: ${s}")
//     }
//     return ok(n)
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     let r = parse_port("8080")
//     when r {
//         is Ok { println("port ${r}") }                // port 8080
//         is Err { println("bad port: ${r}") }
//     }
// }
// ```
//
// **A tag is earned, not assumed.** `Ok` and `Err` are qualifiers: an
// `Ok Int` *is* an `Int`, tagged, and reads as one once narrowed. A plain
// `Int` never becomes an `Ok Int` by itself; it gains the tag only through
// [ok] or [Ok] (and [err] or [Err] for failures). That is what keeps the two
// arms of `Ok Str | Err Str` apart even though both hold a string. At run
// time the tags are erased: the union's own representation records which arm
// a value is in.
//
// **Passing a failure on.** `?:` keeps a union's first arm and runs its right
// side otherwise, with `_` standing for the arms it left behind, so a
// function can unwrap the success and hand the failure to its own caller in
// one line:
//
// ```
// fn parse_port(s: Str) [] -> Ok Int | Err Str => s {
//     let n = parse_int(s)
//     if n is None {
//         return err("not a number: ${s}")
//     }
//     return ok(n)
// }
//
// fn next_port(s: Str) [] -> Ok Int | Err Str => s {
//     let p: Ok Int = parse_port(s) ?: return _
//     return ok(p + 1)
// }
// ```
//
// **Every error, not the first.** A result stops at its first failure, and so
// does `throw`. When a user should see every problem at once (each bad flag,
// each invalid row), use the accumulating outcome
// `Ok T | Errs NonEmpty List<E>`: [errs] makes a failure from one error, and
// [both] and [map2] combine two outcomes, collecting the errors of both
// sides when both failed.
//
// ```
// fn port(s: Str) [] -> Ok Int | Errs NonEmpty List<Str> => s {
//     let n = parse_int(s)
//     if n is None {
//         return errs("not a port: ${s}")
//     }
//     return Ok(n)
// }
//
// fn name(s: Str) [] -> Ok Str | Errs NonEmpty List<Str> => !s {
//     if size(s) == 0 {
//         return errs("empty name")
//     }
//     return Ok(s)
// }
//
// fn address(p: Str, n: Str) [] -> Ok Str | Errs NonEmpty List<Str> => p, !n {
//     return map2(port(p), name(n), (pt: Int, nm: Str) -> "${nm}:${pt}")
//     // for ("x", ""): Errs ["not a port: x", "empty name"]
// }
// ```
//
// For a failure that should leave several frames at once instead of being
// returned from each, see [std.throw].
// @compilerrules [qual-result-tags, mod-used-only]

// The success arm of a result union: `Ok T` is a `T` that a function
// answered as its successful result. Made by [ok] or [Ok].
// @compilerrules [qual-constructive, qual-ctor-same-file, qual-erasure]
// @compilerknown
export provenance qualifier Ok<T> for T
// The failure arm of a result union: `Err E` is an `E` that a function
// answered as its failure. Made by [err] or [Err].
export provenance qualifier Err<T> for T

// Tags a value as the success arm of a result union.
//
// The value is moved into the result. It may be linear (a value that must be
// used exactly once): its obligation travels with it.
//
// ```
// fn half(n: Int) [] -> Ok Int | Err Str => n {
//     if n % 2 != 0 {
//         return err("odd: ${n}")
//     }
//     return ok(n / 2)
// }
// ```
// @compilerrules [qual-ctor-fn, linear-generics]
export fn ok<T canbe linear>(value: T) [] -> +Ok T => !value {
    return value
}

// Tags a value as the failure arm of a result union. The value is moved
// into the result, as for [ok].
// @compilerrules [qual-ctor-fn]
export fn err<T canbe linear>(value: T) [] -> +Err T => !value {
    return value
}

// [ok] under the tag's own name: `Ok(value)` reads as "the `Ok` arm of this
// value". The two are the same function; use whichever reads better.
// @compilerrules [fn-type-named, qual-ctor-same-file]
export fn Ok<T canbe linear>(value: T) [] -> +Ok T => !value {
    // A type-named function whose name is a provenance qualifier (no base
    // type) carries only the tag, and is a constructor like the lowercase
    // `ok`, subject to the same same-file rule.
    return value
}

// [err] under the tag's own name: `Err(value)`.
export fn Err<T canbe linear>(value: T) [] -> +Err T => !value {
    return value
}

// The failure arm of an accumulating outcome: `Errs NonEmpty List<E>` holds
// every error found, at least one, in the order they were found. Made by
// [errs] or [Errs].
//
// Where a result's `Err E` stops at the first failure, the outcome
// `Ok T | Errs NonEmpty List<E>` lets a validation report everything wrong
// at once: [both] and [map2] keep collecting after one side has failed.
// @compilerrules [type-result-errs]
export provenance qualifier Errs<T> for T

// The failure arm of an accumulating outcome, holding one error.
//
// ```
// fn positive(n: Int) [] -> Ok Int | Errs NonEmpty List<Str> => n {
//     if n <= 0 {
//         return errs("not positive: ${n}")
//     }
//     return Ok(n)
// }
// ```
//
// @param first  the error, moved into a new one-element list
// @compilerrules [type-result-errs]
export fn errs<E>(first: E) [] -> +Errs NonEmpty List<E> => !first {
    return List(first)
}

// Tags a value as the failure arm of an accumulating outcome, for a
// non-empty list of errors already gathered: `Errs(List(a, b))`.
// @compilerrules [type-result-errs]
export fn Errs<T canbe linear>(value: T) [] -> +Errs T => !value {
    return value
}

// Both outcomes' values as a pair, or every error either one holds.
//
// When both sides failed, the result holds `a`'s errors and then `b`'s,
// copied into one new list.
//
// ```
// fn checked_pair(a: Ok Int | Errs NonEmpty List<Str>, b: Ok Int | Errs NonEmpty List<Str>) [] -> Ok (Int, Int) | Errs NonEmpty List<Str>
// => !a, !b {
//     return both(a, b)
// }
// ```
//
// @param copy  found at the call site: copies one error
// @returns     `Ok((a, b))` when both succeeded; otherwise the errors
// @compilerrules [type-result-errs]
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

// Combines two outcomes' values with a function, or answers every error
// either one holds.
//
// `f` runs only when both sides succeeded. When both failed, the result
// holds `a`'s errors and then `b`'s. The module overview has an example.
//
// @param f     combines the two values; its effects are the call's
// @param copy  found at the call site: copies one error
// @returns     `Ok(f(a, b))` when both succeeded; otherwise the errors
// @compilerrules [type-result-errs]
export fn map2<A, B, C, E, [Fx]>(a: Ok A | Errs NonEmpty List<E>, b: Ok B | Errs NonEmpty List<E>, f: (x: A, y: B) [Fx] -> C, ?copy: (v: E) -> E) -> Ok C | Errs NonEmpty List<E>
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

// Two error lists as one, `a`'s first.
fn joined<X>(a: NonEmpty List<X>, b: NonEmpty List<X>, ?copy: (v: X) -> X) [] -> NonEmpty List<X> => !a, !b {
    // Its type parameter is not `E`: a generic caller passing its own
    // `?copy` down to a callee whose parameter has the caller's name
    // misinfers it (ROADMAP §2d item 7).
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
