// [obligation-by] [fn-by] [comptime-instantiate] **A stamped member has the
// signature it is declared with, never the template's** (user decision
// 2026-10-14, ROADMAP §2c): the written one for `fn … by X`, the group
// member's with `self` substituted for an obligation clause — which is one
// `fn … by X` per member — and the template supplies only the body, checked
// against that signature. Its effect list and deductions stop counting.
// (What the stamps' IR says is `stamp_signature_tests.rs`.)
//
// [fn-by] The parameter **names** are the template's, though (user decision
// 2026-10-30): the declaration has to spell them as the template does, so the
// body is the template's as written. A default is the declaration's: the
// group's member says `sep = " "`, the template `sep = ","`, and a call
// leaving `sep` out gets the group's [fn-param-default].
// FILE: derive.sv
export fns Labelled<T> {
    fn label(value: T, sep: Str = " ") -> Str
}

export comptime fn label<T is Struct>(value: T, sep: Str = ",") [] -> Str => value {
    let out: Mut Str = Mut Str("${T.name}")
    [for field in T.fields] {
        append@string(out, sep)
        append@string(out, "${value.[field]}")
    }
    return out
}

// FILE: main.sv
import derive

struct P : Labelled<self> by derive { x: Int, y: Str }

fn probe() [] -> Str {
    return label(P { x: 1, y: "a" }, sep = "-")
}

fn plain() [] -> Str {
    return label(P { x: 1, y: "a" })
}

//--- a written declaration named otherwise
// [fn-by] A written declaration naming a parameter otherwise than the
// template is refused at the parameter, and nothing is stamped.
struct P : Ordered<self> {
    x: Int
    //~ error[fn-by:param-name]
    fn cmp(first: P, b: P) -> Int by auto
}

//--- a clause whose template names its parameters otherwise
// [obligation-by] [fn-by] Through a clause the declaration is the group's
// member, which the clause's author does not write: a template written for a
// group names its parameters as the group's member does, and one that does
// not is refused once, at the clause, naming both.
// FILE: derive.sv
export comptime fn cmp<T is Struct>(x: T, y: T) [] -> Int => x, y {
    return 0
}
// FILE: main.sv
import derive

//~ error[fn-by:param-name]
struct W : Ordered<self> by derive { n: Int }
