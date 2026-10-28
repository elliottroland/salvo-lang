// The result tags std supplies, and a qualifier nobody declared.

//--- std_supplies_the_result_tags
// [qual-result-tags] [qual-ctor-fn] `std.core.result` supplies the result tags: a program can
// write `Ok Int | Err Str` and call `ok`/`err` without declaring anything,
// and `is`/`when` narrow the arms as they do for local qualifiers.
fn parse_age(input: Int) -> Ok Int | Err Str | None {
    if input >= 0 {
        return ok(input)
    }
    return err("negative age")
}

fn main() [use] {
    use StdConsole
    let result = parse_age(36)
    when result {
        is Ok {
            println("age ${result}")
        }
        is Err {
            println("error: ${result}")
        }
        is None {
            println("none")
        }
    }
}

//--- union_arm_arguments_resolve_against_union_params
// [type-union] A union-arm value passes where the union is expected, in
// argument position too: `describe(ok("x"))` against
// `describe(v: Ok Str | Err Str)` resolves and wraps (regression: the
// qualifier-stripping unify arm used to precede the union arm, so a
// qualified argument could never match a union's qualified arm).
fn describe(v: Ok Str | Err Str) -> Str {
    when v {
        is Ok {
            return "ok"
        }
        is Err {
            return "err"
        }
    }
}

fn main() [use] {
    use StdConsole
    println(describe(ok("x")))
}

//--- undeclared_qualifier_reports_the_name
// [name-resolve] An undeclared qualifier is reported as the unresolved
// name it is. Regression: `is Ok` with no `Ok` in scope used to report
// "this check can never succeed" (the name was read as a base type that
// no union arm matched) and then cascade onto the subject.
//~ error[name-resolve:unknown-type]
//~ error[name-resolve:unknown-type]
fn f() -> Yes Int | No Str {
    // A value fitting no arm is reported twice, the arm and the return
    // mismatch (ROADMAP §0ar, annotated as it is).
    //~ error[fn-syntax:return-mismatch]
    //~ error[type-union:no-arm]
    return 1
}

fn main() [use] {
    use StdConsole
    let v = f()
    //~ error[name-resolve:unknown-type-or-qualifier]
    if v is Yes {
    }
}
