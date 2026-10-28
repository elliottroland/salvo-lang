// Generic binding widening in unify: a later argument may reveal the more
// general type. `pick(1, x)` with `x: Int?` binds `T = Int?` regardless of
// argument order — first-binding-wins would let the checker believe the
// result is plain `Int`.

fn pick<T>(a: T, b: T) -> T => !a {
    return a
}

fn later_argument_widens() -> None {
    let x: Int? = None
    //~ error[var-let-infer:annotation-mismatch]
    let _z: Int = pick(1, x)
}

fn earlier_argument_widens() -> None {
    let x: Int? = None
    //~ error[var-let-infer:annotation-mismatch]
    let _z: Int = pick(x, 1)
}

// Unrelated bindings still fail the overload (no accidental widening across
// incompatible types).
fn incompatible_bindings_reject() -> None {
    //~ error[call-resolve:no-overload]
    let _z = pick(1, "s")
}
