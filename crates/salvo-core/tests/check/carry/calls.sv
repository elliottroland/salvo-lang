// [implicit-resolve-body] An implicit joins resolution by argument types
// (user decision 2026-10-05, option A): `cmp(a, a)` at `T` goes through it,
// `cmp(1, 2)` at `Int` reaches the visible `cmp(Int, Int)`.
fn joins<T>(a: T, ?cmp: (T, T) -> Int) [] -> Int => a {
    return cmp(a, a) + cmp(1, 2)
}

// [call-resolve] A call through a fn-typed local checks its arguments: an
// ordinary local has no other declarations to fall through to ("`f` takes
// `Str` here, found `Int`"; "`f` takes 1 argument(s), found 2").
fn through_a_local(f: (Str) -> Int) [] -> Int => f {
    //~ error[call-resolve:fn-value-arg-mismatch]
    return f(1)
}

fn through_a_local_count(f: (Str) -> Int) [] -> Int => f {
    //~ error[call-resolve:fn-value-arg-count]
    return f("a", "b")
}

// [call-resolve] [fn-value] A **computed** callee (`adder()(1)`, an element
// of a list of fns) is checked like a call through a local (B1 census D19,
// 2026-10-27): before, each argument was only typed against its parameter,
// so a wrong argument or a wrong count passed the checker and rustc and
// kotlinc refused the generated code ("this function value takes `Int` here,
// found `Str`"; "… takes 1 argument(s), found 2").
fn adder() [] -> (n: Int) -> Int {
    return (n: Int) -> n + 1
}

fn computed() [] -> Int {
    return adder()(1)
}

fn computed_wrong_type() [] -> Int {
    //~ error[call-resolve:fn-value-arg-mismatch]
    return adder()("x")
}

fn computed_wrong_count() [] -> Int {
    //~ error[call-resolve:fn-value-arg-count]
    return adder()(1, 2)
}

// [call-resolve] A call through a fn-typed local checks the
// **claims** its parameter demands, as a declared call does (B1 census D20,
// 2026-10-27): `f(ys)` with `f: (xs: NonEmpty List<Int>) -> Int` and `ys:
// List<Int>` was accepted, and an empty list reached a `NonEmpty`
// parameter. The diagnostic: "`f` takes `NonEmpty List<Int>` here, found
// `List<Int>`", "does not carry what the parameter claims".
fn through(f: (xs: NonEmpty List<Int>) -> Int, ys: List<Int>) [] -> Int => f, ys {
    //~ error[call-resolve:fn-value-claim-mismatch]
    return f(ys)
}

// A claimed argument still fits, and so does one whose claim the parameter
// does not ask for.
fn through_narrowed(f: (xs: NonEmpty List<Int>) -> Int, ys: List<Int>) [] -> Int => f, ys {
    if ys is NonEmpty {
        return f(ys)
    }
    return 0
}

fn loose(f: (xs: List<Int>) -> Int, ys: NonEmpty List<Int>) [] -> Int => f, ys {
    return f(ys)
}

// [qual-depend] A claim about another parameter is about this call's
// argument there, as at a declared call ("found `Idx(ys) Int`").
fn other(f: (xs: List<Int>, i: Idx(xs) Int) -> Int, ys: List<Int>, zs: List<Int>, j: Idx(ys) Int) [] -> Int => f, ys, zs, j {
    //~ error[call-resolve:fn-value-claim-mismatch]
    return f(zs, j)
}

fn same(f: (xs: List<Int>, i: Idx(xs) Int) -> Int, ys: List<Int>, j: Idx(ys) Int) [] -> Int => f, ys, j {
    return f(ys, j)
}
