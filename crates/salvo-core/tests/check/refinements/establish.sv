// Qualifier refinements [qual-refn]: a qualifier states what functions it
// does not own do to *its* claim, which is what recovers the precision
// [deduce-syntax] gives up when it forbids a mutating function from
// promising a qualifier it has never heard of.
//
// The observable in most of these cases is *overload resolution*: a
// `needs_…` overload matches only while the checker still believes the value
// carries the claim, so "clean" means the refinement applied and "no
// matching overload" means it did not. (std has its own `NonEmpty` and
// `Sorted`, of `List`; the store's claims are named apart from them.)

// A list-like type with a mutator whose exhaustive deduction list
// necessarily drops every claim it does not name [deduce-syntax] — the
// situation refinements exist for.
export struct Store<T> canbe Mut {
    value: T
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

// `Stocked` (non-empty) with the refinement that makes `push` establish it.
export qualifier Stocked<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    // Pushing a value leaves the store non-empty.
    refn push(s: Mut Store<T>, value: T) => s: +Stocked
}

export fn needs_stocked<T>(s: Stocked Store<T>) -> Int => s {
    return 1
}

// --- The motivating case ---

// [qual-refn] The claim's owner states what a mutating call it does not own
// does to its claim, and the call site believes it.
fn a_refinement_establishes_a_qualifier_at_a_call_site(s: Mut Store<Int>) -> None {
    push(s, 1)
    let _n = needs_stocked(s)
}

// The control: without the refinement the same program is rejected, so the
// acceptance above is the refinement's doing and not something else.
qualifier Bare<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }
}

fn needs_bare<T>(s: Bare Store<T>) -> Int => s {
    return 1
}

fn without_the_refinement_the_qualifier_is_gone(s: Mut Store<Int>) -> None {
    push(s, 1)
    //~ error[call-resolve:no-overload]
    let _n = needs_bare(s)
}

// [qual-refn] `-Q` is the other direction: the qualifier declares that a
// call *invalidates* its claim, even one that keeps everything else.
qualifier Fragile<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn read(s: Store<T>) => s: -Fragile
}

fn read<T>(s: Store<T>) [] -> Int => s {
    return 0
}

fn fragile<T>(s: Store<T>) -> +Fragile Store<T> => !s {
    return s
}

fn needs_fragile<T>(s: Fragile Store<T>) -> Int => s {
    return 1
}

// The invalidation reaches the contract too: the default deduction keeps `s`
// with its `Fragile`, which the body has removed.
//~ error[deduce-gained:qualifier-removed]
fn a_refinement_can_invalidate_a_qualifier(s: Fragile Store<Int>) -> None {
    let _a = read(s)
    //~ error[call-resolve:no-overload]
    let _b = needs_fragile(s)
}

// --- Several refinements of one call [qual-refn] ---

// [qual-refn] Two refinements establishing *different* qualifiers both apply
// (user decision 2026-10-12): there is no `with` allowlist and no conflict —
// only a `+Q`/`-Q` contradiction for one qualifier is an error. And a
// conflict is precisely "these could not have been written together", not
// "there are two of them".
qualifier Ordered<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(s: Mut Store<T>, value: T) => s: +Ordered
}

fn needs_both<T>(s: Stocked Ordered Store<T>) -> Int => s {
    return 3
}

fn needs_ordered<T>(s: Ordered Store<T>) -> Int => s {
    return 2
}

fn two_refinements_of_different_qualifiers_both_apply(s: Mut Store<Int>) -> None {
    push(s, 1)
    let _n = needs_both(s)
}

fn compatible_refinements_both_apply(s: Mut Store<Int>) -> None {
    push(s, 1)
    let _a = needs_stocked(s)
    let _b = needs_ordered(s)
}

// [qual-refn-scope] A **constructive** qualifier — one with no `qualifies` —
// may still carry refinements, which is what the move into the qualifier
// needs: `Minted` is mint-only and has an insert to speak about. A body makes
// a qualifier predicate only when it holds a `qualifies`.
qualifier Minted<T> of Store<T> {
    refn push(s: Mut Store<T>, value: T) => s: +Minted
}

fn needs_minted<T>(s: Minted Store<T>) -> None {
}

fn a_constructive_qualifier_may_hold_refinements(s: Mut Store<Int>) -> None {
    push(s, 1)
    let _n = needs_minted(s)
}

// --- Inference [qual-refn-infer] ---

// [qual-refn-infer] A refinement reaches the contract, so the fact
// propagates one frame outward: a fn whose parameter declares the qualifier
// and whose body pushes may promise it back. (Deductions are no longer
// inferred from the body, so the promise is written, `s: Mut Stocked, !v`;
// the point — a refinement can be promised back one frame outward — holds.)
fn refill<T>(s: Mut Stocked Store<T>, v: T) -> None => s: Mut Stocked, !v {
    push(s, v)
}

fn a_refinement_reaches_a_deduction(s: Mut Store<Int>) -> None {
    push(s, 1)
    refill(s, 2)
    let _n = needs_stocked(s)
}

// [qual-refn-infer] …and a *written* list promising it back validates
// against the same body facts, which is the same mechanism seen from the
// other side ([deduce-infer] rejects a promise the body may break): without
// the refinement, the promise is refused.
//~ error[deduce-gained:qualifier-removed]
export fn refill_bare<T>(s: Mut Bare Store<T>, v: T) -> None => s: Mut Bare, !v {
    push(s, v)
}

// [qual-refn-infer] An addition inside a block that may not run does *not*
// reach the contract: the deduction pass is a meet over all uses rather than
// a flow analysis, so a conditional call cannot establish a fact the
// signature then promises. The call site, which *is* flow-sensitive, still
// narrows inside the branch.
//~ error[deduce-gained:qualifier-removed]
fn maybe<T>(s: Mut Stocked Store<T>, v: T, c: Bool) -> None => s: Mut Stocked, !v {
    if c {
        push(s, v)
    }
}
