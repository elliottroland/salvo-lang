// Unions lower to positional enums, one file per arity under `unions/`.

//--- unions_emit_enums
// [rs-union-enums] [type-union-arm-identity]
// One file per arity under `unions/mod.rs` [rs-union-enums].
//~ emits(unions/union2.rs): pub enum Union2<T1, T2>
//~ emits(unions/union3.rs): pub enum Union3<T1, T2, T3>
//~ emits(unions/union2.rs): impl<T1, T2> std::fmt::Display for Union2<T1, T2>
fn parse_age(input: Int) -> Ok Int | Err Str {
    if input >= 0 {
        // Wrap at return boundaries, positional arm identity.
        //~ emits: return crate::unions::Union2::U1(crate::std_core_result::ok::<i32>(input));
        return ok(input)
    }
    //~ emits: return crate::unions::Union2::U2(crate::std_core_result::err::<String>(String::from("negative age")));
    return err("negative age")
}

// `when` lowers to an arm test per branch, each with its narrowed read
// peeled out of the union of borrows the kept parameter is
// [rs-opt-borrow].
//~ emits: pub fn describe(mut result: crate::unions::Union2<i32, &String>) -> String
fn describe(result: Ok Int | Err Str) -> Str {
    //~ emits: if matches!(result, crate::unions::Union2::U1(_))
    //~ emits: match result { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() }
    let msg = when result {
        is Ok {
            "age ${result}"
        }
        is Err {
            "error: ${result}"
        }
    }
    return msg
}

fn main() [use] -> None {
    use StdConsole
    let good = parse_age(36)
    println(describe(good))
    let bad = parse_age(-1)
    println(describe(bad))

    let precise: Ok Str | Err Str | Err Bool = ok("yes")
    // Precise `is Err Str` tests a single arm.
    //~ emits: matches!(precise, crate::unions::Union3::U2(_))
    if precise is Err Str {
        println("err str: ${precise}")
    //~ emits: matches!(precise, crate::unions::Union3::U1(_))
    } elif precise is Ok {
        println("ok: ${precise}")
    } else {
        println("err bool: ${precise}")
    }

    let value = when good {
        is Ok {
            good
        }
        is Err {
            return
        }
    }
    println("value plus one is ${value + 1}")
}

//--- union_coercion_in_array_tuple_lambda
// [type-union] Constructor arguments, tuple elements and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.
// Also covers: a fn-type `let` annotation is dropped in Rust
// (`impl Trait` is invalid on bindings [fn-contract]).
qualifier Ok<T> of T
qualifier Err<T> of T

type Result = Ok Int | Err Str

fn tag_ok<T>(value: T) -> +Ok T
=> !value {
    return value
}

fn tag_err<T>(value: T) -> +Err T
=> !value {
    return value
}

fn describe(r: Result) -> Str {
    if r is Ok {
        return "ok ${r}"
    }
    return "err ${r}"
}

fn main() [use] -> None {
    use StdConsole
    // [col-no-literal] A variadic constructor does not yet coerce
    // differently-armed arguments into an explicit union `T`, so the second
    // arm is coerced at its own binding.
    //~ emits: crate::unions::Union2::U2(crate::tag_err::<String>(String::from("a")))
    let second: Result = tag_err("a")
    //~ emits: crate::unions::Union2::U1(crate::tag_ok::<i32>(1i32))
    let arr: List<Result> = List<Result>(tag_ok(1), second)
    for x in arr {
        println(describe(copy(x)))
    }
    //~ emits: crate::unions::Union2::U1(crate::tag_ok::<i32>(2i32))
    let tup: (Str, Result) = ("t", tag_ok(2))
    let (label, r) = tup
    println(describe(r))
    // The fn-type let annotation is dropped.
    //~ emits: let mut make = 
    let make: (flag: Bool) -> Result = (flag: Bool) -> {
        //~ emits: crate::unions::Union2::U1(crate::tag_ok::<i32>(3i32))
        return if flag { tag_ok(3) } else { tag_err("b") }
    }
    println(describe(make(true)))
    println(describe(make(false)))
}

//--- field_subject_is_lowers_to_union_test
// [is-narrowing] [is-binding] Field subjects get the same union-test
// lowering as identifier subjects, and narrow like them [flow-place];
// both backends must agree with the checker's is_tests lowering.
struct Holder {
    result: Ok Int | Err Str
}

fn main() [use] -> None {
    use StdConsole
    let h = Holder {result: ok(1)}
    //~ emits: matches!(h.result, crate::unions::Union2::U1(_))
    if h.result is Ok Int r {
        println("ok ${r}")
    }
    if h.result is Ok {
        println("plain ${h.result}")
    }
    let h2 = Holder {result: err("bad")}
    if h2.result is Err Str e {
        println("err ${e}")
    }
}
