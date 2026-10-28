// --- [qual-refn-narrow] A refinement that *keeps* a claim ---
// (std has its own `NonEmpty`, of `List`; the store's is `Stocked`.)

export struct Store<T> canbe Mut {
    value: T
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

export fn needs_stocked<T>(s: Stocked Store<T>) -> Int => s {
    return 1
}

// A mutator that cannot establish the claim but cannot destroy it either:
// `touch` rearranges a store's contents. Refining it unconditionally would
// be a lie (touching an empty store does not fill it), so the refinement
// writes a **narrower** parameter and states what happens when the claim is
// already there (user correction 2026-09-23).
export fn touch<T>(s: Mut Store<T>) [] -> None => s: Mut {
    s.value = s.value
}

export qualifier Stocked<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    // Pushing a value leaves the store non-empty.
    refn push(s: Mut Store<T>, value: T) => s: +Stocked

    // Touching an *already* non-empty store leaves it non-empty.
    refn touch(s: Stocked Mut Store<T>) => s: +Stocked
}

// [qual-refn-narrow] The claim the caller already has survives the call.
fn a_narrowed_refinement_keeps_a_claim_the_argument_has(s: Stocked Mut Store<Int>) -> None {
    touch(s)
    let _n = needs_stocked(s)
}

// …and **only** that claim: the same call on a store nothing is claimed
// about does not invent one. This is the case that makes the narrowed form
// necessary — an unconditional `refn touch(s: Mut Store<T>) => s: +Stocked`
// would accept this program, wrongly.
fn a_narrowed_refinement_does_not_establish_the_claim(s: Mut Store<Int>) -> None {
    touch(s)
    //~ error[call-resolve:no-overload]
    let _n = needs_stocked(s)
}

// [qual-refn-narrow] A *kept* claim survives a conditional call, because it
// was there whether or not the call happened. An establishing refinement
// must not (and does not — the next case).
fn a_kept_claim_survives_a_branch(s: Stocked Mut Store<Int>, cond: Bool) -> None => s: Stocked Mut, cond {
    if cond {
        touch(s)
    }
}

// The asymmetry that justifies the distinction: a call that may not have
// happened cannot have *established* anything, so a promise resting on one
// inside a branch is refused.
//~ error[deduce-gained:qualifier-removed]
fn an_established_claim_does_not_survive_a_branch(s: Stocked Mut Store<Int>, cond: Bool) -> None => s: Stocked Mut, cond {
    if cond {
        push(s, 1)
    }
}

// [qual-refn-match] Qualifiers (including `Mut`) are *ignored* by the gate
// (user decision 2026-10-12): a refinement that drops the `Mut` still
// matches by base type. Extra qualifiers on the refn become a precondition
// [qual-refn-narrow], not a mismatch.
export qualifier Loose<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn touch(s: Loose Store<T>) => s: +Loose
}

// --- [deduce-gained] Reporting a claim the body established ---

// [deduce-gained] A function whose parameter does **not** declare a claim may
// still report it, when the body establishes it through a refinement the
// claim's owner wrote: the promise is *checked* against the body, unlike `+Q`,
// which is trusted and gated to the qualifier's own file.
fn fill(s: Mut Store<Int>) -> None => s: Stocked Mut {
    push(s, 1)
}

// …and the caller learns it, which is the point: this is how `std.heap`'s
// `push` hands back a heap that `pop` can read without an optional.
fn a_caller_sees_a_reported_claim(s: Mut Store<Int>) -> None {
    fill(s)
    let _n = needs_stocked(s)
}

// The control: a body that establishes nothing cannot report anything, and the
// diagnostic distinguishes "the parameter never had it" from "the body may
// drop it".
//~ error[deduce-gained:qualifier-not-established]
fn a_plain_entry_cannot_report_a_claim_nothing_establishes(s: Mut Store<Int>) -> None => s: Stocked Mut {
    touch(s)
}

// [deduce-gained] A *bodiless* declaration has nothing to check against, so
// there the old refusal stands — an `intrinsic fn` cannot report a gain. (An
// `intrinsic` is std's alone [platform-intrinsic-std-only], which is refused
// too; the case predates that rule.)
//~ error[deduce-gained:bodiless]
//~ error[platform-intrinsic-std-only:outside-std]
intrinsic fn poke(s: Mut Store<Int>) [] -> None => s: Stocked Mut
