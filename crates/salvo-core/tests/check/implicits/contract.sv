// ===== the contract is part of fitting, and does not print =====
//
// A fn whose *types* match but whose contract does not cannot fill the
// position — and `Ty`'s Display shows no contract, so the bare mismatch
// would read "expects `(Cell, Cell) -> Cell`, found `(Cell, Cell) -> Cell`".
// The diagnostic has to name the argument, the direction, and the fix:
// "*consumes* `a` while this position keeps it", "the types match, the
// contracts do not", and a `=>[` clause. A program of its own: its `add`
// consumes `a`, where the other cases' `add` keeps it.
//
// Over a struct rather than the old prelude's `Int`: over the real std,
// moving an `Int` is a copy, and `=> !a` on one is warned about
// (`copy-scalar-free:moved-scalar`).

fns Additive<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

struct Cell { n: Int }

fn add(a: Cell, b: Cell) -> Cell
=> !a {
    return a
}

fn zero() -> Cell { return Cell { n: 0 } }

fn total<T>(a: T, ?Additive<T>) -> T => !a {
    return add(a, zero())
}

fn probe() -> Cell {
    //~ error[implicit-resolve:no-fit]
    return total(Cell { n: 1 })
}

// The same explanation for a value written at the call site: there the
// mismatch is between the *given* fn and the parameter — "`add` does not fit
// here", "`eats` *consumes* `a`".
fn eats(a: Cell, b: Cell) -> Cell
=> !a {
    return a
}

fn override_with_the_wrong_contract() -> Cell {
    //~ error[implicit-override:does-not-fit]
    return total(Cell { n: 1 }, add = eats)
}
