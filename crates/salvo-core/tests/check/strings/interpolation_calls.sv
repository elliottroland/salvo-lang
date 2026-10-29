// [interp-to-str] An interpolated part is the call `to_str(part)`, resolved as
// that call written in the body is (PIPELINE.md §II.10 P4, option (a), user
// decision 2026-10-28; built in B3.2): the enclosing fn's own implicit first,
// then the fns and the available effects' members of the name as one set.

struct Point { x: Int }

effect Fmt {
    fn to_str(p: Point) -> Str => p
}

// [effect-member-overload] An available effect's member takes the part (R10
// D3): before, `${p}` saw fns only and refused a `Point` with no fn of its own.
fn by_member(p: Point) [Fmt] -> Str => p {
    return "${p}"
}

// [interp-to-str] With no handler in scope there is nothing to take it.
fn no_handler(p: Point) -> Str => p {
    //~ error[interp-to-str:no-text]
    return "${p}"
}

// [implicit-resolve-body] An enclosing `?to_str` takes a scalar part too
// (census D1: before, `${n}` was the native rendering beside it).
fn by_implicit(n: Int, ?to_str: (x: Int) -> Str) -> Str => n, to_str {
    return "${n}"
}

//--- a member and a fn of one signature
// [fn-overload-ambiguous] A member and a fn taking the part alike are the
// call's ambiguity, said of the interpolation ("in this interpolation:
// ambiguous call to `to_str(Point)`", `to_str@Fmt(...)`, `to_str@main(...)`).
struct Point { x: Int }

effect Fmt {
    fn to_str(p: Point) -> Str => p
}

fn to_str(p: Point) -> Str => p { return "fn" }

fn both(p: Point) [Fmt] -> Str => p {
    //~ error[fn-overload-ambiguous:call]
    return "${p}"
}
