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
