// Union wrappers: the sealed interfaces, one file per arity, and the wraps,
// arm tests and `when`s a program's unions lower to.

//--- unions_emit_sealed_wrappers
// [kt-union-wrappers] [type-union-arm-identity]
// The M3 demo (`UNIONS_DEMO` in codegen_tests.rs): Result-style unions with
// qualifiers, `when` exhaustiveness, precise `is Err Str` checks, and flow
// narrowing.
// One file per arity [kt-union-wrappers].
//~ emits(unions/Union2.kt): sealed interface Union2
//~ emits(unions/Union3.kt): sealed interface Union3
// [kt-union-wrappers] The arms nest in the interface: `Union2.U1`, as
// Rust's `Union2::U1` (user decision 2026-10-01, ABI D5).
//~ emits(unions/Union2.kt): sealed interface Union2<out T1, out T2> {
//~|     val value: Any?
//~|     data class U1<out T1, out T2>(override val value: T1) : Union2<T1, T2>
//~|
//~ !emits(unions/Union2.kt): U2_1
//~ !emits(unions/Union3.kt): U2_1
//~ !emits(unions/Union2Codec.kt): U2_1
//~ !emits(unions/Union3Codec.kt): U2_1
fn parse_age(input: Int) -> Ok Int | Err Str {
    if input >= 0 {
        // Wrap at return boundaries, positional arm identity (the
        // constructor call is wrapped into the union arm).
        //~ emits: return Union2.U1<Int, String>(salvo.std.core.result.ok(input))
        return ok(input)
    }
    //~ emits: return Union2.U2<Int, String>(salvo.std.core.result.err("negative age"))
    return err("negative age")
}

fn describe(result: Ok Int | Err Str) -> Str {
    // `when` lowers to a `when` over the arm tests, each arm reading the
    // payload through its narrowing binding [ir-narrow].
    //~ emits: (result is Union2.U1<*, *>) ->
    //~ emits: ((result as Union2.U1<*, *>).value as Int)
    // [when-exhaustive] An exhaustive `when` with no written `else` carries
    // an explicit unreachable fallthrough from the IR, so the emitter prints
    // it rather than synthesizing one: `else -> { throw AssertionError(...) }`
    // naming the unhandled-arm trap.
    //~ emits: else -> {
    //~ emits: unhandled union arm
    //~ !emits: IllegalStateException("salvo: unreachable arm")
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

    // Qualifier-tagged wrap picks the right arm of the 3-union.
    //~ emits: Union3.U1<String, String, Boolean>(salvo.std.core.result.ok("yes"))
    let precise: Ok Str | Err Str | Err Bool = ok("yes")
    // Precise `is Err Str` tests a single arm; `is Ok` another.
    //~ emits: precise is Union3.U2<*, *, *>
    //~ emits: precise is Union3.U1<*, *, *>
    if precise is Err Str {
        println("err str: ${precise}")
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

//--- union_arm_arguments_wrap_at_call_sites
// [type-union] A union-arm argument (`ok("x")` where `Ok Str | Err Str`
// is expected) resolves and wraps into the declared union's arm at the
// call site — checker overload resolution and emitter wrapping agree.
export fn describe(v: Ok Str | Err Str) -> Str {
    when v {
        is Ok {
            return "ok"
        }
        is Err {
            return "err"
        }
    }
}

export fn main() [use] -> None {
    use StdConsole
    // Positional arm identity: Ok -> first arm, Err -> second arm.
    //~ emits: describe(Union2.U1<String, String>(salvo.std.core.result.ok("x")))
    println(describe(ok("x")))
    //~ emits: describe(Union2.U2<String, String>(salvo.std.core.result.err("y")))
    println(describe(err("y")))
}

//--- union_coercion_in_array_tuple_lambda
// [type-union] Constructor arguments, tuple elements and lambda tail returns
// receive expected types, so union wrapping is recorded and emitted.
// (`NESTED_COERCION_DEMO` in codegen_tests.rs.)
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
    //~ emits: Union2.U2<Int, String>(salvo.main.tagErr("a"))
    let second: Result = tag_err("a")
    // [col-no-literal] `List<Result>(…)` lowers to `listOf`, with the
    // coercion wrapper on the argument; the second arm is wrapped at its
    // own binding.
    //~ emits: listOf<Union2<Int, String>>(Union2.U1<Int, String>(salvo.main.tagOk(1)), second)
    let arr: List<Result> = List<Result>(tag_ok(1), second)
    for x in arr {
        println(describe(copy(x)))
    }
    //~ emits: kotlin.Pair("t", Union2.U1<Int, String>(salvo.main.tagOk(2)))
    let tup: (Str, Result) = ("t", tag_ok(2))
    let (label, r) = tup
    println(describe(r))
    let make: (flag: Bool) -> Result = (flag: Bool) -> {
        //~ emits: Union2.U1<Int, String>(salvo.main.tagOk(3))
        return if flag { tag_ok(3) } else { tag_err("b") }
    }
    println(describe(make(true)))
    println(describe(make(false)))
}
