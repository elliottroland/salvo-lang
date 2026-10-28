// [op-safe-nav] `?.` optional chaining on the Rust backend.

//--- safe_navigation_field_chain_is_a_combinator_chain
// The same source and expected stdout as the Kotlin backend's
// `kotlinc_compiles_and_runs_safe_navigation`. A `?.` field chain feeds a
// `?:`, and a `?.` call link on a place receiver rides on dot-call lowering;
// the whole thing short-circuits to `None` at the first absent link. Run on
// both backends, since the lowering (a short-circuiting `Switch`) and both
// the native (Kotlin) and generic (Rust) renderings must agree.
//
// [op-safe-nav] [rs-safe-nav] A field-only `?.` chain is a combinator chain.
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
    // `o?.inner?.nanos` → an `and_then`/`map` chain, not nested `is_none()`
    // blocks.
    //~ emits: .and_then(|
    //~ emits: .map(|
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
