// [implicit-with] Implicits that are **filled together**: `=> eq with hash` on
// a function or on a `fns` group, and the one-source rule a call site obeys.
//
// The user's design (2026-09-26), from a defect: filling one slot of
// `?Hashed<T>` and leaving the other to resolution builds a pair nobody
// checked, and because a hash container buckets by `hash` first, the written
// slot quietly does nothing rather than failing. `with` says the two are one
// decision — symmetric, because it states that they must *agree* and agreement
// has no direction, and transitive by the check, so a chain is one class.
//
// Over std's own `Hashed<T> => eq with hash` and `Set`.

export fn all_same(a: Int, b: Int) -> Bool => a, b {
    return true
}

// A **written** member beside a **resolved** one: the defect this exists for.
// The diagnostic names both sources and the remedy, writing them out.
fn writes_one() -> None {
    //~ error[implicit-with:different-places]
    let s: Mut Set<Int> = Mut Set(eq = all_same)
    add(s, 1)
}

// Writing **both** is the legal spelling, and the one the diagnostic asks for.
fn writes_both() -> None {
    let s: Mut Set<Int> = Mut Set(hash = hash, eq = all_same)
    add(s, 1)
}

// Leaving the **whole** pair to resolution is untouched by the rule: the author
// wrote nothing, so there is nothing to disagree.
fn writes_none() -> None {
    let s: Mut Set<Int> = Mut Set()
    add(s, 1)
}

// **Forwarded** beside **resolved**, which arrives with nothing written at the
// call at all — the case that decided the rule reads sources rather than
// writes (user decision 2026-09-26). Here writing them out is *impossible* — a
// container's identity must be a name a type can carry [cmp-carry] — so the
// diagnostic offers the other remedy first ("declare the rest of them too").
fn collect_half(a: Int, ?hash: (Int) -> Long) -> Set<Int> {
    //~ error[implicit-with:different-places]
    let s: Mut Set<Int> = Mut Set()
    add(s, a)
    return s
}

fn holds_half() -> None {
    let _n = size(collect_half(1, hash = hash))
}

// Holding the **whole** pair is the fix, and then every member is forwarded.
fn collect_whole(a: Int, ?Hashed<Int>) -> Set<Int> {
    let s: Mut Set<Int> = Mut Set()
    add(s, a)
    return s
}

fn holds_whole() -> None {
    let _n = size(collect_whole(1))
}

// Declaring the members **individually** takes them unbound: the binding is a
// property of the *spread*, which is what makes "a function may add pairs and
// never remove them" cost nothing (user decision 2026-09-26).
fn pick_unbound(a: Int, ?hash: (Int) -> Long, ?eq: (Int, Int) -> Bool) -> Long => a {
    return hash(a)
}

fn individually() -> None {
    let _n = pick_unbound(1, hash = hash)
}

// A fn may **add** a pair of its own, over implicits nothing else binds; the
// diagnostic names `hash` and `eq`.
fn pick_bound(a: Int, ?hash: (Int) -> Long, ?eq: (Int, Int) -> Bool) -> Long
=> a, hash with eq {
    return hash(a)
}

fn own_pair() -> None {
    //~ error[implicit-with:different-places]
    let _n = pick_bound(1, hash = hash)
}

// A **chain** is one component: `a with b with c` binds all three, so filling
// one of them names the other two (user decision 2026-09-26).
fn pick_chained(
    a: Int,
    ?hash: (Int) -> Long,
    ?eq: (Int, Int) -> Bool,
    ?cmp: (Int, Int) -> Int
) -> Long => a, hash with eq with cmp {
    return hash(a)
}

fn chain() -> None {
    //~ error[implicit-with:different-places]
    let _n = pick_chained(1, hash = hash)
}

// `with` relates the `?name` positions a call fills, so a name that is not an
// implicit parameter is a mistake — reported where it is written.
//~ error[implicit-with:not-implicit]
fn pick_ordinary(a: Int, ?hash: (Int) -> Long) -> Long => a, hash with a {
    return hash(a)
}
