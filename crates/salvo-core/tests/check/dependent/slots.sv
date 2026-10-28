// [qual-depend] Dependent qualifiers — value slots, the filled `is` test,
// fate-root binding, and conservative cross-value stripping (the
// refinement-types sequence, step 2). A claim that was stripped shows as the
// widen check `^` below it finding nothing to remove [qual-lift].

struct Box canbe Mut { n: Int }
struct Key { s: Str }

qualifier Inside(box: Box) of Key {
    fn qualifies(key: Key, box: Box) -> Bool {
        return box.n > 0
    }
}

fn bump(box: Mut Box) [] -> None => box: Mut {
    box.n = box.n + 1
}

fn read(box: Box) [] -> Int => box {
    return box.n
}

// The filled test narrows: after `assert!(key is Inside(box))` the claim is
// in the type, which the widen check observes [qual-lift].
fn a_filled_is_test_narrows(key: Key, box: Box) [] -> Bool => key, box {
    assert!(key is Inside(box))
    return key is ^Inside
}

// [qual-depend] Any `Mut` use of the depended-on value strips the claim —
// the widen check below it no longer finds `Inside`.
fn mutating_the_linked_value_strips(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {
    assert!(key is Inside(box))
    bump(box)
    //~ error[qual-lift:not-carried]
    return key is ^Inside
}

// A *read* of the depended-on value keeps the claim: only mutation can
// invalidate a fact about contents [deduce-syntax].
fn reading_the_linked_value_keeps(key: Key, box: Box) [] -> Bool => key, box {
    assert!(key is Inside(box))
    let _n = read(box)
    return key is ^Inside
}

// Mutating a *different* value strips nothing: the claim is bound to the
// fate roots of the place that filled the slot.
fn mutating_an_unrelated_value_keeps(key: Key, box: Box, other: Mut Box) [] -> Bool => key, box, other: Mut {
    assert!(key is Inside(box))
    bump(other)
    return key is ^Inside
}

// An unfilled test of a dependent qualifier names the filled form.
fn an_unfilled_dependent_check(key: Key) [] -> Bool => key {
    //~ error[qual-depend:unfilled]
    return key is Inside
}

// The place must fit the slot's type.
fn a_slot_mismatch(key: Key, wrong: Key) [] -> Bool => key, wrong {
    //~ error[qual-depend:slot-mismatch]
    return key is Inside(wrong)
}

// [qual-depend] **Reassignment strips dependent claims exactly as
// mutation does**: the old value is gone, so a claim bound to it
// describes nothing. Before the 2026-09-24 fix an `Idx(xs)` claim held
// across `xs = [9]` kept resolving the total `get` — a checked
// out-of-bounds read at runtime.
fn reassigning_the_linked_value_strips(key: Key) [] -> Bool => key {
    let box = Mut Box { n: 1 }
    assert!(key is Inside(box))
    box = Mut Box { n: 2 }
    //~ error[qual-lift:not-carried]
    return key is ^Inside
}

// [qual-depend] A dependent predicate's `qualifies` takes the subject and
// then one parameter per value slot; fewer is an error naming the count.
qualifier Short(box: Box) of Key {
    //~ error[qual-depend:param-count]
    fn qualifies(key: Key) -> Bool {
        return true
    }
}

// --- Overloads on a dependent claim ---

fn open(box: Box, key: Inside(box) Key) [] -> Int => box, key {
    return box.n
}

fn open(box: Box, key: Key) [] -> Int? => box, key {
    return None
}

// [qual-depend] A parameter's dependent claim names a sibling parameter:
// the argument's claim must be about *this call's* value, so a claim about
// a different box falls back to the optional overload — visible in the
// result type, `Int?`, which does not fit `-> Int`.
fn a_claim_about_another_value(key: Key, right: Box, wrong: Box) [] -> Int => key, right, wrong {
    assert!(key is Inside(wrong))
    //~ error[fn-syntax:return-mismatch]
    return open(right, key)
}

// The matching claim picks the total overload, and the call answers the
// element type.
fn a_matching_claim_picks_the_total_overload(key: Key, box: Box) [] -> Int => key, box {
    assert!(key is Inside(box))
    return open(box, key)
}

// An alias of the depended-on value is the same value: the claim binds
// fate roots, not names.
fn an_alias_of_the_value_still_matches(key: Key, box: Box) [] -> Int => key, box {
    let same = box
    assert!(key is Inside(same))
    return open(box, key)
}

// --- [qual-preserve] A refinement that preserves a dependent claim ---

qualifier Held(box: Box) of Key {
    fn qualifies(key: Key, box: Box) -> Bool {
        return box.n > 0
    }
    refn grow(box: Mut Box) => box: preserve Held
}

fn grow(box: Mut Box) [] -> None => box: Mut {
    box.n = box.n + 1
}

fn shrink(box: Mut Box) [] -> None => box: Mut {
    box.n = 0
}

// [qual-preserve] A refined call keeps the dependent claim alive: the
// widen check still finds it below the mutation.
fn a_preserving_call_keeps(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {
    assert!(key is Held(box))
    grow(box)
    return key is ^Held
}

// An unrefined mutator still strips: preservation is per call, opt-in.
fn an_unrefined_mutator_still_strips(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {
    assert!(key is Held(box))
    shrink(box)
    //~ error[qual-lift:not-carried]
    return key is ^Held
}

// [qual-preserve] A fn's own `preserve` entry is checked: a body that
// hands the parameter to a non-preserving mutator is refused at that call.
fn wrap_badly(box: Mut Box) [] -> None
=> box: Mut, box: preserve Held {
    //~ error[qual-preserve:call-invalidates]
    shrink(box)
    return None
}

// A body whose mutating calls all preserve satisfies the promise, and a
// caller of the wrapper keeps its claim through it.
fn wrap(box: Mut Box) [] -> None
=> box: Mut, box: preserve Held {
    grow(box)
    return None
}

fn an_own_preserve_promise_carries_to_callers(key: Key, box: Mut Box) [] -> Bool => key, box: Mut {
    assert!(key is Held(box))
    wrap(box)
    return key is ^Held
}

// [op-elvis-guard] A leaving right side takes its flow effects with it: a
// `return elem` there must not consume `elem` on the fall-through path —
// found on `std.heap`'s sift (2026-09-24). Here a positional pick's
// diverging right side consumes `prize`, which the final `return prize`
// still owns.
fn classify(k: Key) [] -> Ok Key | Err Key
=> !k {
    return ok(k)
}

fn a_leaving_right_side_keeps_its_consumptions(k: Key, prize: Key) [] -> Key => !prize, !k {
    let r = classify(k)
    let _hit: Ok Key = r ?: return prize
    return prize
}
