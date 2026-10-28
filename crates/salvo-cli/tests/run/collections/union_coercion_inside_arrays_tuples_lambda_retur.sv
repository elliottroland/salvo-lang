// union coercion inside arrays/tuples/lambda returns
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- nested_coercion

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
    let second: Result = tag_err("a")
    let arr: List<Result> = List<Result>(tag_ok(1), second)
    for x in arr {
        println(describe(copy(x)))
    }
    let tup: (Str, Result) = ("t", tag_ok(2))
    let (label, r) = tup
    println(describe(r))
    let make: (flag: Bool) -> Result = (flag: Bool) -> {
        return if flag { tag_ok(3) } else { tag_err("b") }
    }
    println(describe(make(true)))
    println(describe(make(false)))
}
//= stdout
//= ok 1
//= err a
//= ok 2
//= ok 3
//= err b
