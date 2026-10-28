// [linear-group] `Linear` as a designated obligation group (roadmap R4, user
// decisions 2026-09-08): declaring a `linear struct` *is* declaring how the
// obligation is discharged, because a same-file discharger must be supplied.
//
// Two things this settles that the `canbe linear` spelling could not: a
// `close` is **required** of anything claiming to be linear, and `discard` no
// longer discharges — dropping a handle is the leak the obligation exists to
// prevent. The presence of a `close` never implies linearity either; only the
// marker does.

linear struct Lines {
    name: Str
}

fn close(l: Lines) -> None => !l { discard(l) }

fn open_lines(n: Str) -> Lines => !n {
    return Lines { name: n }
}

// The `close` implementation is where a linear value legitimately dies — the
// one place linearity would otherwise make impossible to write.
fn declaring_the_obligation_with_its_close_is_clean() -> None {
    let l = open_lines("a")
    close(l)
}

// [group-obligation] Declared without one, the error lands at the *struct*
// and names the signature it wants.
//~ error[linear-obligation:no-discharger]
linear struct Leaky {
    fd: Int
}

// [linear-discard] The rule that changed: `discard` is no longer the escape
// hatch for a linear value, and the diagnostic names what is.
fn discard_cannot_drop_a_linear_value() -> None {
    let l = open_lines("a")
    //~ error[linear-discard:linear]
    discard(l)
}

// It still drops a non-linear one: that is what it is for now.
fn discard_still_drops_a_plain_value() -> None {
    discard("note")
}

// The leak diagnostic names `close`, not `discard`.
fn a_leak_names_close_as_the_discharge() -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let _l = open_lines("a")
}

// [linear-group] A `close` **never** implies linearity (user decision
// 2026-09-08): only the marker does. Without it the type is ordinary, so
// letting a value go out of scope is unremarkable — which is what keeps a
// generated pass with a `defer` composable.
struct Plain {
    fd: Int
}

fn close(p: Plain) -> None => !p { discard(p) }

fn make_plain() -> Plain {
    return Plain { fd: 1 }
}

fn a_close_alone_does_not_make_a_type_linear() -> None {
    let _p = make_plain()
}

// [qual-canbe-optin] The old spelling is gone from *declarations*: `canbe`
// grants a qualifier, `linear` declares an obligation. The error names the
// new form.
//~ error[linear-group:canbe-linear]
struct Old canbe linear {
    fd: Int
}

// [linear-generics] But `canbe linear` on a **type parameter** keeps its
// spelling: permission on a parameter is not obligation on a declaration.
fn hold<T canbe linear>(value: T) -> T
=> !value {
    return value
}

fn canbe_linear_on_a_type_parameter_still_works() -> None {
    let l = hold(open_lines("a"))
    close(l)
}
