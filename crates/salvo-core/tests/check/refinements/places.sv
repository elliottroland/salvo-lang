// [qual-refn-scope] A qualifier's refinements travel with it: a file that
// does not import the qualifier does not get its refinements, which is what
// "the user opts into the refinements when they opt into the qualifier"
// means. Here, the file that imports it; the file that does not is a Rust
// test, `refine_tests.rs`. (std has its own `NonEmpty`, of `List`; the store's is `Stocked`.)
// FILE: quals.sv
export struct Store<T> canbe Mut {
    value: T
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

export fn needs_stocked<T>(s: Stocked Store<T>) -> Int => s {
    return 1
}

export qualifier Stocked<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(s: Mut Store<T>, value: T) => s: +Stocked
}

fn qualified<T>(s: Store<T>) -> +Stocked Store<T>
=> !s {
    return s
}

// FILE: user.sv
// Importing the qualifier: the refinement applies.
import quals.Stocked
import quals.Store
import quals.push
import quals.needs_stocked

export fn g(s: Mut Store<Int>) -> None {
    push(s, 1)
    let _n = needs_stocked(s)
}

//--- refinements from two places
// [qual-refn] Both places' refinements apply (user decision 2026-10-12):
// one refn refines many overloads, all matching refns apply, and there is no
// call-site `@place` picking or ambiguity. `lib` owns a qualifier whose
// refinement establishes `LibClaim` on `push`, `main` owns one that
// establishes `MyClaim`: a `push(s, 1)` in `main`, with both refinements of
// `push` in scope, establishes *both* — `needs_lib(s)` and `needs_mine(s)`
// both resolve.
import lib.Store
import lib.push
import lib.LibClaim
import lib.needs_lib

qualifier MyClaim<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(s: Mut Store<T>, value: T) => s: +MyClaim
}

fn needs_mine<T>(s: MyClaim Store<T>) [] -> Int => s {
    return 2
}

fn f(s: Mut Store<Int>) -> None {
    push(s, 1)
    let _a = needs_lib(s)
    let _b = needs_mine(s)
}

// FILE: lib.sv
export struct Store<T> canbe Mut {
    value: T
}

export qualifier LibClaim<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(s: Mut Store<T>, value: T) => s: +LibClaim
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
}

export fn needs_lib<T>(s: LibClaim Store<T>) [] -> Int => s {
    return 1
}

//--- a top-level refinement
// [qual-refn-scope] A refinement belongs to the qualifier whose claim it is
// about, so the **top-level** form is a parse error naming the move (user
// decision 2026-09-26). It existed to *reconcile* two qualifiers that
// disagreed; disagreement is now refused at the call and settled by naming a
// place [qual-refn], so it has no job left.
export struct Store<T> canbe Mut {
    value: T
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

export qualifier Stocked<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }
}

//~ error[qual-refn-scope:top-level-refn]
refn push<T>(s: Mut Store<T>, value: T) => s: +Stocked
