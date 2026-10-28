// A `when` over a union subject: it must be exhaustive over the arms, and
// its subject must be a union.

//--- when_must_be_exhaustive
// [when-exhaustive]
export qualifier Ok<T> of T
export qualifier Err<T> of T

export fn f(x: Ok Int | Err Str) -> Int {
    //~ error[when-exhaustive:missing-arms]
    let v = when x {
        is Ok {
            1
        }
    }
    return v
}

//--- when_requires_union_subject
// [when-union-subject]
export fn f(x: Int) -> None {
    //~ error[when-union-subject:not-a-union]
    when x {
        is Int {
            x
        }
    }
}

//--- when_field_subject_is_rejected
// [when-union-subject] `when` still requires a plain variable subject.
export qualifier Ok<T> of T

//~ error[deduce-default:kept-by-default-moved]
export fn ok<T>(value: T) -> +Ok T {
    return value
}

export struct Holder {
    result: Ok Int | Str
}

export fn main() [use] -> None {
    use StdConsole
    //~ error[fn-overload-ambiguous:call]
    let h = Holder {result: ok(1)}
    //~ error[when-union-subject:not-a-variable]
    when h.result {
        is Ok {
            println("ok")
        }
        is Str {
            println("str")
        }
    }
}
