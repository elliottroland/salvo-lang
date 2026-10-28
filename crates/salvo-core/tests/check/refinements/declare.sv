// What a refinement declaration may say [qual-refn] [qual-refn-match]: which
// function it is about, which parameter, and which claim. (std has its own
// `NonEmpty` and `Sorted`, of `List`; the store's claims are named apart.)

export struct Store<T> canbe Mut {
    value: T
}

export fn push<T>(s: Mut Store<T>, value: T) [] -> None => s: Mut, !value {
    s.value = value
}

// [qual-refn-match] Matching is positional by base type (user decision
// 2026-10-12), a loose gate — not an exact-signature match. A different
// parameter *name* still matches, and a compatible base still matches; only
// a genuinely non-matching shape, a missing fn, or a deduction naming a
// non-parameter is an error.
qualifier RenamedParam<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(store: Mut Store<T>, value: T) => store: +RenamedParam
}

qualifier NoMut<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(s: Store<T>, value: T) => s: +NoMut
}

qualifier Wildcard<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    refn push(s: Mut Store<T>, *) => s: +Wildcard
}

qualifier NoSuchFn<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn-match:no-function]
    refn shove(s: Mut Store<T>, value: T) => s: +NoSuchFn
}

qualifier NotAParam<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn:unknown-param]
    refn push(s: Mut Store<T>, value: T) => other: +NotAParam
}

// [qual-refn-match] A type parameter has to be bound, and the diagnostic
// names the remedy rather than reporting a shape mismatch: `U` is neither
// the qualifier's parameter nor the refinement's own.
qualifier Unbound<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn-match:unknown-type]
    refn push(s: Mut Store<U>, value: U) => s: +Unbound
}

// [qual-refn] A qualifier may only speak about its *own* claim. That is
// what makes "opting into a qualifier opts into its refinements" honest,
// and it is why conflicts reduce to two qualifiers that cannot co-apply.
qualifier Ordered<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }
}

qualifier Stocked<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn:foreign-qualifier]
    refn push(s: Mut Store<T>, value: T) => s: +Ordered
}

// [qual-refn] Only *state* qualifiers can be refined [qual-subject]:
// provenance cannot be invalidated in the first place, and the compiler's
// own qualifiers carry representation choices — `Mut` is not even erased,
// so handing one out is not a refinement's business. Inside a qualifier body
// the own-claim rule fires first, which makes the same point: a qualifier
// speaks only for itself.
provenance qualifier Trusted<T> of Store<T>

qualifier Vouched<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn:foreign-qualifier]
    refn push(s: Mut Store<T>, value: T) => s: +Trusted
}

qualifier Mutable<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn:foreign-qualifier]
    refn push(s: Mut Store<T>, value: T) => s: +Mut
}

// [qual-refn] There is nothing to refine about a parameter the call takes
// away: a caller knows nothing about a moved value afterwards.
qualifier Taken<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-refn:moved-param]
    refn push(s: Mut Store<T>, value: T) => value: +Taken
}

// [qual-refn] The claim has to be about the parameter's type [qual-of].
qualifier Counted<T> of Store<T> {
    fn qualifies(s: Store<T>) -> Bool {
        return true
    }

    //~ error[qual-of:wrong-subject]
    refn tally(text: Str) => text: +Counted
}

fn tally(text: Str) [] -> Int => text {
    return 0
}
