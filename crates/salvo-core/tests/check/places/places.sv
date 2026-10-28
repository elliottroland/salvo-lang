// Place-based flow narrowing [flow-place] [flow-place-invalidate].
//
// `is` narrows *places* — a variable or a field chain out of one — not just
// variables, so a checked field reads at its narrowed type. The observable
// used here is [interp-no-none]: interpolating a possibly-`None` value is an
// error, so an accepted interpolation of `p.surname` is proof the field
// narrowed, and a rejected one is proof the fact was invalidated.

struct Address canbe Mut {
    city: Str? = None
}

struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    nick: Str? = None,
    tags: Str?[],
    pair: (Str?, Str?),
    address: Mut Address
}

// Opaque observers. Deduction lists are *written*, which is what these cases
// turn on [decl-explicit]: the declared contract is authoritative, so the body
// only has to exist.
fn touch(p: Mut Person) [] -> None => p: Mut {
    p.name = p.name
}

fn read(p: Person) [] -> None => p {}

fn touch_address(a: Mut Address) [] -> None => a: Mut {
    a.city = a.city
}

// ===== narrowing =====

// [flow-place] A checked field reads at its narrowed type, so the
// interpolation [interp-no-none] used to reject is now legal.
fn field_narrows_in_branch(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        return "${p.surname}"
    }
    return p.name
}

// [is-narrow-guard] [flow-place] After a guard whose branch **exits**, the
// fall-through path carries the *else* fact: `p.surname` is not merely
// un-narrowed there, it is known to be `None` — so the read still fails, with
// the message for a value that has no text form rather than the one for a
// maybe-`None` optional.
fn a_guard_narrows_the_fall_through_path(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        return p.name
    }
    //~ error[interp-no-none:none-value]
    return "${p.surname}"
}

// The other half of the same rule: a guard that does **not** exit leaves the
// fall-through path un-narrowed, so the read is a maybe-`None` one.
fn a_branch_that_falls_through_narrows_nothing_after_it(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        touch(p)
    }
    //~ error[interp-no-none:optional-value]
    return "${p.surname}"
}

// [flow-place] Facts are per place: checking one field says nothing about its
// sibling.
fn sibling_fields_are_independent(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        //~ error[interp-no-none:optional-value]
        return "${p.nick}"
    }
    return p.name
}

// [flow-place] Field *chains* narrow, not just one level.
fn field_chain_narrows(p: Mut Person) -> Str => p: Mut {
    if p.address.city is Str {
        return "${p.address.city}"
    }
    return p.name
}

// [flow-place] `&&` accumulates place facts, as it does for variables.
fn conjunction_accumulates_place_facts(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str && p.nick is Str {
        return "${p.surname} ${p.nick}"
    }
    return p.name
}

// [flow-place] The `else` branch of a nullable check narrows to `None`, which
// has no text form — the negative fact reaches reads too.
fn else_branch_carries_the_negative_fact(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        return p.name
    } else {
        //~ error[interp-no-none:none-value]
        return "${p.surname}"
    }
}

// ===== invalidation [flow-place-invalidate] =====

// [flow-place-invalidate] Assigning to a *sibling* leaves the fact standing:
// the write cannot reach that storage.
fn assignment_to_a_sibling_preserves_the_fact(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        p.nick = None
        return "${p.surname}"
    }
    return p.name
}

// [flow-place-invalidate] Assigning to a *prefix* drops the facts below it:
// the whole subtree is replaced.
fn assignment_to_a_prefix_invalidates_below(p: Mut Person) -> Str => p: Mut {
    if p.address.city is Str {
        p.address = Mut Address {city: None}
        //~ error[interp-no-none:optional-value]
        return "${p.address.city}"
    }
    return p.name
}

// [flow-place-invalidate] A `Mut`-keeping call may mutate the value, so every
// fact about its parts falls.
fn mut_keeping_call_invalidates(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        touch(p)
        //~ error[interp-no-none:optional-value]
        return "${p.surname}"
    }
    return p.name
}

// [flow-place-invalidate] A `Mut` call through a *projection* invalidates the
// facts under that projection only: the sibling fact survives…
fn a_mut_call_through_a_projection_spares_a_sibling(p: Mut Person) -> Str => p: Mut {
    if p.address.city is Str && p.surname is Str {
        touch_address(p.address)
        return "${p.surname}"
    }
    return p.name
}

// …and the mutated subtree's fact falls.
fn a_mut_call_through_a_projection_invalidates_that_subtree(p: Mut Person) -> Str => p: Mut {
    if p.address.city is Str {
        touch_address(p.address)
        //~ error[interp-no-none:optional-value]
        return "${p.address.city}"
    }
    return p.name
}

// [flow-place-invalidate] A call that *keeps* the value immutably cannot
// mutate it, so narrowing survives it (user decision P1b).
fn kept_immutable_call_preserves_the_fact(p: Mut Person) -> Str => p: Mut {
    if p.surname is Str {
        read(p)
        return "${p.surname}"
    }
    return p.name
}

// [flow-place] Element places do not narrow (user decision P1a): `arr[i]`
// cannot be told from `arr[j]`, so the read stays at the declared type.
fn array_elements_do_not_narrow(p: Mut Person) -> Str => p: Mut {
    if p.tags[0] is Str {
        //~ error[interp-no-none:optional-value]
        return "${p.tags[0]}"
    }
    return p.name
}

// ===== tuple elements [expr-tuple-index] =====

// [expr-tuple-index] [flow-place] A constant index names one location, so a
// checked tuple element narrows exactly like a field.
fn tuple_element_narrows(p: Mut Person) -> Str => p: Mut {
    if p.pair.0 is Str {
        return "${p.pair.0}"
    }
    return p.name
}

// [expr-tuple-index] [flow-place] Sibling elements are independent.
fn tuple_elements_are_independent(p: Mut Person) -> Str => p: Mut {
    if p.pair.0 is Str {
        //~ error[interp-no-none:optional-value]
        return "${p.pair.1}"
    }
    return p.name
}

// [expr-tuple-index] The index must exist.
fn tuple_index_out_of_range_is_an_error(p: Mut Person) -> Str => p: Mut {
    //~ error[expr-tuple-index:out-of-range]
    return "${p.pair.7}"
}

// [expr-tuple-index] Only tuples have elements.
fn tuple_index_on_a_non_tuple_is_an_error(p: Mut Person) -> Str => p: Mut {
    //~ error[expr-tuple-index:not-a-tuple]
    return "${p.name.0}"
}

// [expr-tuple-index] Tuple elements are read-only: `Mut` cannot apply to a
// tuple, so no tuple value can grant mutation permission.
fn tuple_elements_cannot_be_assigned(p: Mut Person) -> Str => p: Mut {
    //~ error[expr-tuple-index:assign]
    p.pair.1 = "x"
    return p.name
}

// ===== merging =====

// [flow-place] A fact only some fall-through paths agree on does not survive
// the join.
fn facts_must_hold_on_every_path_to_survive(p: Mut Person) -> Str => p: Mut {
    if p.name is Str {
        if p.surname is Str {
            read(p)
        }
    }
    //~ error[interp-no-none:optional-value]
    return "${p.surname}"
}

// ===== destructuring =====

// [var-let-destructure] A tuple pattern needs a tuple *of that arity* behind
// it. Until 2026-09-18 a mismatch bound `Unknown`s and said nothing, so
// `let (a, b) = 7` compiled and the emitters wrote target code that could not
// build — the mistake was reported, at best, by rustc or kotlinc.
fn a_tuple_pattern_needs_a_tuple(p: Mut Person) -> Str => p: Mut {
    //~ error[var-let-destructure:arity]
    let (_a, _b) = 7
    return p.name
}

// …and the arity is part of it: two names cannot take a triple. The matching
// pattern is legal and only the mismatch reports, naming both arities.
fn a_tuple_pattern_must_match_the_arity(p: Mut Person) -> Str => p: Mut {
    let (_a, _b) = p.pair
    //~ error[var-let-destructure:arity]
    let (_c, _d, _e) = p.pair
    return p.name
}

// [var-let-destructure] A **loop** element may be destructured, and the same
// rule applies to it: the element has to be a tuple of that arity. `p.tags` is
// an array of `Str?`, so a pair pattern reports rather than binding `Unknown`s
// (which is what it did until 2026-09-18).
fn a_loop_pattern_is_checked_against_the_element(p: Mut Person) -> Str => p: Mut {
    //~ error[var-let-destructure:arity]
    for (_a, _b) in p.tags {
        read(p)
    }
    return p.name
}
