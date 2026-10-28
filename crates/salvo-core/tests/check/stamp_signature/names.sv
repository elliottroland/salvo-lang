// [obligation-by] [fn-by] [comptime-instantiate] **A stamped member has the
// signature it is declared with, never the template's** (user decision
// 2026-10-14, ROADMAP §2c): the written one for `fn … by X`, the group
// member's with `self` substituted for an obligation clause — which is one
// `fn … by X` per member — and the template supplies only the body, checked
// against that signature. Its parameter names, effect list and deductions
// stop counting. (What the stamps' IR says is `stamp_signature_tests.rs`.)
//
// [obligation-by] The clause's member takes the **group's** parameter names:
// a named argument says `sep`, the group's, and the template's `between` is
// no name of the stamped fn — while the body, which reads `v` and
// `between`, reads the declared parameters.
//
// A derive module whose template names its parameters `v` and `between`,
// where the group's member says `value` and `sep` — with a default of its
// own, so that a call may name it [fn-param-default].
// FILE: derive.sv
export fns Labelled<T> {
    fn label(value: T, sep: Str = " ") -> Str
}

export comptime fn label<T is Struct>(v: T, between: Str = ",") [] -> Str => v {
    let out: Mut Str = Mut Str("${T.name}")
    [for field in T.fields] {
        append(out, between)
        append(out, "${v.[field]}")
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

fn by_template_name() [] -> Str {
    //~ error[implicit-override:no-implicit]
    return label(P { x: 1, y: "a" }, between = "-")
}
