// [op-safe-nav] `?.` optional chaining
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- safe_navigation

struct Inner {
    nanos: Long
}

struct Outer {
    inner: Inner?
}

fn label(i: Inner) -> Str {
    return "nanos=${i.nanos}"
}

fn nanos_or(o: Outer?) -> Long {
    return o?.inner?.nanos ?: -1L
}

fn describe(i: Inner?) -> Str {
    return i?.label() ?: "absent"
}

fn main() [use] {
    use StdConsole()
    let a = Outer { inner: Inner { nanos: 42L } }
    let b = Outer { inner: None }
    let c: Outer? = None
    println(to_str(nanos_or(a)))
    println(to_str(nanos_or(b)))
    println(to_str(nanos_or(c)))
    println(describe(Inner { nanos: 42L }))
    let no_inner: Inner? = None
    println(describe(no_inner))
}
//= stdout
//= 42
//= -1
//= -1
//= nanos=42
//= absent

//--- a_call_link_on_any_receiver
// [when-lowered] A `?.` is a `when` binding its receiver once, so a call link
// no longer needs the receiver to be a place: `found(n)?.label()` and a
// chain whose second link is a call both run (they were
// `op-safe-nav:receiver-not-place` before the lowering). A general union
// passes its other arms through.

struct Inner {
    nanos: Long
}

struct Outer {
    inner: Inner?
}

fn label(i: Inner) -> Str {
    return "nanos=${i.nanos}"
}

fn found(n: Long) -> Inner? {
    if n > 0L {
        return Inner { nanos: n }
    }
    return None
}

fn wrap(n: Long) -> Outer? {
    return Outer { inner: found(n) }
}

fn main() [use] {
    use StdConsole()
    println(found(7L)?.label() ?: "absent")
    println(found(0L)?.label() ?: "absent")
    println(wrap(3L)?.inner?.label() ?: "absent")
    println(wrap(0L)?.inner?.label() ?: "absent")
    let r: Ok Inner | Err Str = ok(Inner { nanos: 9L })
    let e: Ok Inner | Err Str = err("no")
    println("${r?.nanos} ${e?.nanos}")
}
//= stdout
//= nanos=7
//= absent
//= nanos=3
//= absent
//= 9 no
