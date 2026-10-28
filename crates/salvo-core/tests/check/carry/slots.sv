// ===== the slot declaration =====

// A fn slot must have a fn type — the same requirement an implicit parameter
// has, and for the same reason [implicit-param]. (`NoSlot` was `Bad`.)
//~ error[cmp-carry:slot-not-fn-type]
qualifier NoSlot<T>(?cmp: Int) of List<T>

// [struct-slot] [cmp-carry] A **struct** declares fn slots as a type
// declaration does: the identity is in the type, so two differently ordered
// values refuse to mix, and a fn over the struct captures the binder from its
// parameter [cmp-binder].
struct Ranked<T>(?cmp: (T, T) -> Int) canbe Mut {
    items: List<T> canbe Mut
}

fn ranked<T>(?cmp: (T, T) -> Int) [] -> Mut Ranked<T>(?cmp) {
    return Mut Ranked<T>(?cmp) { items: Mut List<T>() }
}

fn better<T>(r: Ranked<T>(?cmp), a: T, b: T) [] -> Bool => r, a, b {
    return cmp(a, b) < 0
}

fn backwards(a: Int, b: Int) [] -> Int {
    return cmp(b, a)
}

fn use_it() [] -> Bool {
    let up = ranked<Int>()
    let down = ranked<Int>(cmp = backwards)
    return better(up, 1, 2) && better(down, 1, 2)
}

// The mixed one: "expected `Ranked<Int>(cmp…)`, found `Mut
// Ranked<Int>(backwards)`".
fn mix() [] -> None {
    let down = ranked<Int>(cmp = backwards)
    //~ error[var-let-infer:annotation-mismatch]
    let _up: Ranked<Int>(cmp) = down
}

// Only a qualifier, a struct or an `intrinsic type` may declare one: on a fn
// the binder binds bare in the signature instead (the ordering round's
// decision 11). That case stays a Rust test, `carry_tests.rs`: over the real
// std a slot in a fn's `<…>` reports two errors.
