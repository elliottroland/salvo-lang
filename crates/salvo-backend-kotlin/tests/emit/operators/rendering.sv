// Operators: how binary expressions and `?.` chains render in Kotlin.

//--- safe_navigation_field_chain_is_native_kotlin
// [op-safe-nav] [kt-safe-nav]
// The same source and expected stdout as the Rust backend's
// `rustc_runs_safe_navigation`: a `?.` field chain feeding a `?:`, and a
// `?.` call link on a place receiver. On Kotlin the field chain renders as a
// native `o?.inner?.nanos` ([op-safe-nav]); both backends must agree on the
// short-circuiting result. (`SAFE_NAV_DEMO` in codegen_tests.rs.)
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
    // A field-only `?.` chain is a native Kotlin optional chain, not a raw
    // `when`/`if` over the receiver.
    //~ emits: o?.inner?.nanos
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

//--- binary_rendering_preserves_grouping
// Parser grouping must survive re-rendering: without precedence-aware
// parenthesization `(a - b) * c` would emit flat as `a - b * c` and
// silently re-associate. (`PRECEDENCE_DEMO` in codegen_tests.rs.)
fn main() [use] -> None {
    use StdConsole
    let a = 10
    let b = 3
    let c = 2
    // Every binary is parenthesized: the grouping is explicit rather than
    // left to precedence.
    //~ emits: ((a - b) * c)
    let grouped = (a - b) * c
    //~ emits: (a - (b - c))
    let nested = a - (b - c)
    //~ emits: (-(a + b))
    let neg = -(a + b)
    //~ emits: (((a < b) || (b > c)) && (a > c))
    let logical = (a < b || b > c) && a > c
    println("${grouped} ${nested} ${neg} ${logical}")
}

//--- operators
// [op-arith] [op-promote] [type-lit-adopt] [op-convert] The same source and
// expected output as the Rust backend's `rustc_compiles_and_runs_operators`
// [backend-parity]. Kotlin's own operator set covers the mixed widths
// (`Long.plus(Int)`, `Int.compareTo(Long)`), so only the adopted literals
// need rendering care (`1L`, `3.0`).
export fn main() [use] -> None {
    use StdConsole
    //~ emits: val x: Long = 1L
    let x: Long = 1
    let n = 5
    let y = x + n * 2
    let big = 4000000000L
    let scaled = big * 2 + n
    //~ emits: val d: Double = 3.0
    let d: Double = 3
    let g = d + to_double(n) + 0.25
    let f: Float = 0.5
    let h = f * 1.5f
    let cmp = n < x
    let back = to_int(big)
    let neg: Long = -7
    println("${y} ${scaled} ${g} ${h} ${cmp} ${back} ${neg}")
}
