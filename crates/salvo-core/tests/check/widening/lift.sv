// The widening check `^` [qual-lift].
//
// `^` is the dual of `is`: where a successful `is` narrows the subject (a
// qualifier added, a union arm picked), a successful `^` **generalizes** it
// by removing the listed qualifiers. Boolean-valued, same places, same
// runtime test — only the type in the branch differs. Its reason to exist
// is the qualified union: `Ok (Ok Int | Err Str)` is a claim *about* a
// union, and `is ^Ok` is what lets a branch `when` the union inside it.
// `Ok`, `Err`, `ok` and `err` are std's [qual-result-tags].

struct Person canbe Mut {
    name: Str
}

qualifier Surname of Person

fn note(text: Str) [] -> None => text {}
fn read(p: Person) [] -> None => p {}
fn touch(p: Mut Person) [] -> None => p: Mut {}

// [qual-lift] The motivating case: a `^` branch head tests the arm *and*
// removes the claim, so the branch can `when` the union inside it — no
// intermediate binding and no repeated type.
fn nested() [] -> Ok (Ok Int | Err Str) | Err Str { return err("e") }

fn a_widening_branch_opens_a_nested_union() [] -> None {
    let o = nested()
    when o {
        is ^Ok {
            when o {
                is Ok { note("value") }
                is Err { note("inner error") }
            }
        }
        is Err { note("outer error") }
    }
}

// [qual-lift] In an `if`, the same check on a plain qualified value: the
// qualifier is statically present, so it cannot fail — and the subject
// reads without it inside the branch, which is what picks the *unqualified*
// overload.
fn widening_strips_a_qualifier_in_an_if(p: Mut Person) [] -> None => p: Mut {
    if p is ^Mut {
        read(p)
    }
}

// [qual-lift] And the point of it: after widening, the value no longer
// satisfies what the qualifier granted.
fn a_widened_value_loses_what_the_qualifier_granted(p: Mut Person) [] -> None => p: Mut {
    if p is ^Mut {
        //~ error[call-resolve:no-overload]
        touch(p)
    }
}

// ===== the exclusion list =====

// [qual-lift] The capability qualifiers that may never be dropped, read
// from the *same* list `Qual T <: T` uses (user decision 2026-09-05), so
// the two cannot drift. `proj` is in the same list; since 2026-09-11 it *is*
// writable [proj-anywhere], but it is stripped at lowering (a borrow is
// provenance, not part of the type), so a `^` never meets it.
fn once_cannot_be_widened_away(p: Person) [] -> None => p {
    //~ error[qual-lift:cannot-remove]
    if p is ^once {
        read(p)
    }
}

fn linear_cannot_be_widened_away(p: Person) [] -> None => p {
    //~ error[qual-lift:cannot-remove]
    if p is ^Linear {
        read(p)
    }
}

// ===== the error cases =====

// [qual-lift] Nothing to remove is an error, not a silently-false test
// (user decision 2026-09-05).
fn widening_a_qualifier_the_value_lacks(p: Person) [] -> None => p {
    //~ error[qual-lift:not-carried]
    if p is ^Surname {
        read(p)
    }
}

// [qual-lift] One widened view cannot stand for two arms: each would peel
// a different wrapper position.
fn pair() [] -> Ok Int | Ok Str { return ok(1) }

fn widening_more_than_one_arm() [] -> None {
    let o = pair()
    //~ error[qual-lift:several-arms]
    if o is ^Ok {
        note("ok")
    }
}

// [qual-lift] A lift **takes a binding** (user decision 2026-09-21,
// superseding the 2026-09-05 no-binding rule): `is ^Mut plain` names the
// lifted value. That is what makes a multi-arm lift possible — the lifted
// value is materialized once, into the binding — so: the single-arm binding
// checks clean, and a *two*-arm lift is accepted with a binding and refused
// without one, naming it.
fn a_lift_takes_a_binding(p: Mut Person) [] -> None => p: Mut {
    if p is ^Mut plain {
        read(plain)
    }
}

fn three() [] -> Ok Int | Ok Str | Err Str { return err("e") }

// [type-rewrap] A multi-arm lift is built end to end since 2026-09-21: the
// bound value spans fewer arms than the storage, so it is produced by mapping
// arm to arm — the same mapping an annotated `let` gets [var-let-infer].
fn a_multi_arm_lift_with_a_binding() [] -> None {
    let o = three()
    if o is ^Ok value {
        let _lifted = value
    }
}

fn a_multi_arm_lift_without_one() [] -> None {
    let o = three()
    //~ error[qual-lift:several-arms]
    if o is ^Ok {
        note("lifted")
    }
}

// [qual-lift] A `^` branch consumes the arms it matched, exactly as `is`
// does, so exhaustiveness needs no new rule.
fn two() [] -> Ok Int | Err Str { return err("e") }

fn a_widening_branch_consumes_its_arms() [] -> None {
    let o = two()
    //~ error[when-exhaustive:missing-arms]
    when o {
        is ^Ok { note("ok") }
    }
}

// ===== constructing a group [qual-group] =====
//
// The other half of the qualified union: `^` *reads* a group, and these build
// one. `Ty::Qualified` keeps a flat, sorted, deduplicated qualifier list whose
// base is never itself `Qualified`, so applying a qualifier to an
// already-qualified value appends rather than nests — `emitted(ok(1))` is
// `Qualified { quals: [Emitted, Ok], base: Int }`, shape-identical to "two
// qualifiers on an `Int`". Where the expected arm is `Q (A | B)`, the value is
// read the other way round: the group's qualifiers come off the list and the
// remainder tries the union's arms. Fixed 2026-09-10; before that the
// construction needed an annotated intermediate `let`. `Emitted` and
// `emitted` are std's (`std.core.iterator`).

// [qual-group] The defect this closed: a qualifier applied to an
// already-qualified value flattens, and the arm it must land in is a
// qualified union group.
fn a_qualifier_applied_to_a_qualified_value(flag: Bool) [] -> Emitted (Ok Int | Err Str) | Err Str {
    if flag { return emitted(ok(1)) }
    return err("done")
}

// [qual-group] Either inner arm, so the arm index really is derived from the
// remainder rather than assumed to be the first.
fn the_remainder_picks_the_inner_arm(flag: Bool) [] -> Emitted (Ok Int | Err Str) | Ok Bool {
    if flag { return emitted(err("bad")) }
    return emitted(ok(1))
}

// [qual-group] The remainder still has to fit: the group's qualifier being
// present is not on its own a licence to enter the arm. (The outer union's
// other arm is deliberately unrelated — `Emitted` is droppable, so an
// `Err Str` arm would legitimately accept `Emitted Err Str` and this would
// test nothing.)
fn a_remainder_that_fits_no_inner_arm() [] -> Emitted (Ok Int | Ok Str) | Ok Bool {
    //~ error[fn-syntax:return-mismatch]
    //~ error[type-union:no-arm]
    return emitted(err("bad"))
}
