// [effect-member-overload] [fn-param-default] [fn-variadic] A member call is
// selected and checked by the call's own rule (PIPELINE.md B3.5, census D17
// and D25): parameter defaults and named arguments fill a member's
// parameters as a fn's, a variadic member collects its tail, and a member
// with the wrong argument types is the call's no-overload error.

effect Log {
    fn log(msg: Str, level: Int = 1) -> Str => msg
    fn tag(a: Str, b: Str) -> Str => a, b
    fn total(first: Int, ...rest: Int[]) -> Int
}

// [fn-param-default] A defaulted member parameter may be left out or named
// (census D17: `log("hi")` was "takes 2 argument(s), found 1").
fn defaults() [Log] -> Str {
    return "${log("hi")}${log("hi", 2)}${log("hi", level = 3)}"
}

// [fn-param-default] Naming a parameter that has no default is refused, as at
// a fn ("`b` has no default").
fn named_without_default() [Log] -> Str {
    //~ error[fn-param-default:named-without-default]
    return tag("a", b = "c")
}

// [fn-variadic] A variadic member collects its tail; before, a variadic
// member could not win a ranking against a fn of its name.
fn variadic() [Log] -> Int {
    return total(1) + total(1, 2, 3)
}

// [call-resolve] A wrong argument type is the call's error.
fn wrong_type() [Log] -> Str {
    //~ error[call-resolve:no-overload]
    return log(1)
}
