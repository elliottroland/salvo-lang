// [fn-param-default] Parameter defaults: what a default may read, what it
// may do, and which parameters a call may name.

// [fn-param-default] A defaulted parameter may be omitted — positionally
// (trailing) or by naming a later one — with no error.
fn greet(name: Str, greeting: Str = "Hi", punct: Str = "!") -> Str {
    return "${greeting}, ${name}${punct}"
}

fn omits_defaults() -> None {
    let _a = greet("Ada")
    let _b = greet("Ada", "Hey")
    let _c = greet("Ada", punct = "?")
}

// [fn-param-default] A default may read an earlier Copy-scalar parameter.
fn span(lo: Int, hi: Int = lo + 10) -> Int {
    return hi - lo
}

fn reads_an_earlier_scalar() -> None {
    let _a = span(5)
}

// [fn-param-default] A default MAY read a non-scalar earlier parameter (a
// `List`, a struct, anything with identity): the IR builder binds the
// parameter once (`Lower::args_for`/`block_expr`) and reads it from both its
// own argument slot and the default, so there is nothing to forbid based on
// the parameter's type.
fn sized(xs: List<Int>, y: Int = size(xs)) -> Int {
    return y
}

fn reads_an_earlier_list() -> None {
    let _a = sized(List(1, 2, 3))
}

// [fn-param-default] A default cannot read a *later* parameter: only the
// earlier ones are in scope, matching how a struct field default cannot read
// a sibling field.
//~ error[name-ident-resolve:unknown]
fn later(a: Int = b, b: Int) -> Int {
    return a + b
}

// [fn-param-default] A default is checked with no effects available, so an
// effectful default is refused rather than silently colouring every caller
// that omits the argument.
fn noisy() [Console] -> Int {
    println("hi")
    return 1
}

//~ error[effect-available:callee-no-handler]
fn effectful(x: Int = noisy()) [Console] -> Int {
    return x
}

// [fn-param-default] A named argument may only target a parameter that has a
// default; naming an ordinary required parameter is refused.
fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn names_required_parameters() -> None {
    //~ error[fn-param-default:named-without-default]
    //~ error[fn-param-default:named-without-default]
    let _c = add(a = 1, b = 2)
}

//--- a variadic default
// [fn-param-default] A variadic or an implicit parameter cannot carry a
// default — each already fills itself its own way. Parser-level, so each is
// a program of its own.
//~ error[fn-param-default:variadic-default]
fn f(...xs: Int[] = 5) -> Int {
    return 0
}

//--- an implicit default
//~ error[fn-param-default:implicit-default]
fn f(?cmp: (Int, Int) -> Int = 5) -> Int {
    return 0
}
