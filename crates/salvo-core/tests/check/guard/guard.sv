// [is-narrow-guard] Narrowing survives an **early-returning guard** (user
// decision 2026-09-09, closing the defect found by the R5 probe).
//
// `if e is None { return … }` leaves the rest of the block on the *else*
// path, so every condition's else-narrows hold there — the same facts
// [is-narrowing] already gives an `else` branch, carried past a statement
// whose branches cannot fall through. That is what makes the ordinary way of
// writing a `next` over a container — guard on the end, then use the
// element — expressible without an `else` block or a two-armed `when`.
//
// The rule keys on *every* branch exiting: `return`, `break`, `continue`,
// and a diverging call ([type-any-never]) all count, and a branch that can
// fall through narrows nothing after the `if`.

// The shape the defect was found in: guard on the empty case, then use the
// value at the narrowed type. Assigning it to a `Str` local is the
// assertion — it only type-checks if the fact survived the `if`.
fn describe(s: Str?) -> Str => s {
    if s is None {
        return "none"
    }
    let _text: Str = s
    return "got"
}

// A `break` leaves the block just as a `return` does.
fn count_break(s: Str?) -> Int => s {
    let n = 0
    while n < 3 {
        if s is None {
            break
        }
        let _text: Str = s
        n = n + 1
    }
    return n
}

// So does a `continue`.
fn count_continue(s: Str?) -> Int => s {
    let n = 0
    while n < 3 {
        n = n + 1
        if s is None {
            continue
        }
        let _text: Str = s
    }
    return n
}

// [type-any-never] A diverging *call* is an exit too: nothing after it
// runs, so the branch cannot fall through.
fn give_up(reason: Str) -> Never => !reason {
    return give_up(reason)
}

fn describe_diverging(s: Str?) -> Str => s {
    if s is None {
        give_up("none")
    }
    let _text: Str = s
    return "got"
}

// An `elif` chain accumulates: with both arms exiting, what is left is the
// third.
fn pick(v: Int | Str | Bool) -> Bool => v {
    if v is Int {
        return false
    } elif v is Str {
        return true
    }
    let _flag: Bool = v
    return true
}

// The negative half: a branch that *can* fall through leaves the value at
// its declared type, since either path may have been taken.
fn describe_falls_through(s: Str?) -> Str => s {
    let _seen = false
    if s is None {
        _seen = true
    }
    //~ error[var-let-infer:annotation-mismatch]
    let _text: Str = s
    return "got"
}

// One exiting branch is not enough when another falls through.
fn pick_mixed(v: Int | Str | Bool) -> Bool => v {
    let _n = 0
    if v is Int {
        return false
    } elif v is Str {
        _n = 1
    }
    //~ error[var-let-infer:annotation-mismatch]
    let _flag: Bool = v
    return true
}

// An explicit `else` that falls through is the path taken, and it carries
// the same facts — so the narrowing holds there too, and after the `if`.
fn describe_else(s: Str?) -> Str => s {
    let _seen = false
    if s is None {
        return "none"
    } else {
        _seen = true
    }
    let _text: Str = s
    return "got"
}

// [is-narrow-assign-reset] An assignment *on the surviving path* still resets:
// the fact was about the old value. (A local, as below: a parameter is a
// constant binding [fn-param-const].)
fn describe_reset(input: Str?, other: Str?) -> Str => input, other {
    let s = input
    if s is None {
        return "none"
    } else {
        s = other
    }
    //~ error[var-let-infer:annotation-mismatch]
    let _text: Str = s
    return "got"
}

// But an assignment inside the *exiting* branch does not: that path never
// reaches the code below (the same reason a fall-through merge ignores it).
//
// The assigned name is a **local** rather than the parameter: a parameter is
// a constant binding [fn-param-const], and what this case is about is the
// narrowing reset, not who may be assigned.
fn describe_exiting_assign(input: Str?, other: Str?) -> Str => input, other {
    let s = input
    if s is None {
        s = other
        return "none"
    }
    let _text: Str = s
    return "got"
}

// [is-qualifies] [fn-overload-rank] The same rule through a **negated** guard
// over a *predicate qualifier*, which is the shape `demo/heap.sv` reaches for
// (the heap plan's item 5, COMPLETED.md's log): `if !(xs is Filled) { return … }` puts the
// claim in the condition's else-narrows — `Not` swaps them — so the rest of the
// block holds it, and the call below routes to the overload that *demands* it.
//
// Verified rather than assumed (2026-09-22): [is-qualifies]' "no else
// information" note reads as though nobody had tested the negated path. Both
// halves already worked; what the demo still waits for is only the `!is`
// spelling. (The qualifier was `NonEmpty`, which std declares over its
// collections.)
qualifier Filled of Str {
    fn qualifies(s: Str) -> Bool {
        return true
    }
}

fn taste(s: Str) -> Int => s {
    return 0
}

fn taste(s: Filled Str) -> Str => s {
    return "nonempty"
}

fn probe(s: Str) -> Str => s {
    if !(s is Filled) {
        return "empty"
    }
    let text: Str = taste(s)
    return text
}

// The negative half, so the case above is about the *guard* and not about
// overload luck: without it, `taste` answers the plain overload's `Int`.
fn probe_unguarded(s: Str) -> Str => s {
    //~ error[var-let-infer:annotation-mismatch]
    let text: Str = taste(s)
    return text
}

// [is-not] The same guard in the spelling it is usually reached for
// (`!is`, user decision 2026-09-22): sugar for `!(x is Q)`, so it narrows the
// fall-through exactly as the parenthesized form does — one `Expr::Is` under a
// `Not`, which is all the checker ever sees.
fn describe_not_is(s: Str?) -> Str
=> !s {
    if s !is Str {
        return "none"
    }
    let text: Str = s
    return text
}
