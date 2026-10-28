// [op-unique-shape] D-4: a `fns` group's member signatures and comptime
// templates are exempt; a stamp is a declaration like a written one, refused
// outside the type's file. Effect members are covered (`Fmt.to_str`, on
// `Int`, module `std.core.basic`); a handler's members implement theirs.
// FILE: pt.sv
export struct Pt { x: Int }

export effect Show {
    fn to_str(p: Pt) -> Str => p
}

// FILE: main.sv
import pt
fns Shown<T> {
    fn to_str(x: T) -> Str
}
comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {
    return true
}
//~ error[op-unique-shape:outside-home]
fn eq(a: Pt, b: Pt) [] -> Bool => a, b by auto
effect Fmt {
    //~ error[op-unique-shape:outside-home]
    fn to_str(x: Int) -> Str => x
}
handler Plain of Show {
    fn to_str(p: Pt) -> Str { return "pt" }
}
