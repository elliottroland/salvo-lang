// ===== [fate-field-disjoint] L5: field-disjoint precision =====
//
// A fate link records *which projection* of the root the derived value came
// from, and an event poisons a link only when the two places overlap
// ([flow-place]'s `Place::overlaps`, the substrate P1 built). So reading one
// field while another is mutated is legal, and everything that could name the
// same storage still poisons: the same field, a prefix, the whole variable,
// and a dynamic index (which may alias any element).
//
// Before this, all four accept cases below were rejected and the remedy was
// `copy`, which costs a real clone on the Rust backend.

struct Person canbe Mut {
    name: Str,
    tags: Mut List<Str>
}

struct Outer {
    inner: Person,
    other: Str
}

fn touch_all(p: Mut Person) [] -> None => p: Mut {}
fn read_str(s: Str) [] -> Int => s { return 0 }
fn tally(list: List<Str>) [] -> Int => list { return 0 }
fn add_tag(list: Mut List<Str>, s: Str) [] -> None => list: Mut, s {}
fn eat(list: Mut List<Str>) [] -> None => !list {}
fn take_person(p: Person) [] -> None => !p {}

// [fate-field-disjoint] Reading `p.name` and mutating `p.tags` are disjoint,
// so the derived value survives — the shape that motivated L5.
fn a_disjoint_field_survives_a_mutation() -> None {
    let p = Person { name: "a", tags: Mut List() }
    let n = p.name
    add_tag(p.tags, "y")
    let _k = read_str(n)
}

// [fate-field-disjoint] Two disjoint fields of one struct, each reached
// independently.
fn two_disjoint_fields_are_independent() -> None {
    let o = Outer { inner: Person { name: "a", tags: Mut List() }, other: "o" }
    let n = o.inner.name
    add_tag(o.inner.tags, "y")
    let _k = read_str(n)
}

// [fate-field-disjoint] An assignment is an event on the field it writes:
// writing `p.tags` leaves a value derived from `p.name` alone.
fn an_assignment_to_a_disjoint_field_does_not_poison() -> None {
    let p = Mut Person { name: "a", tags: Mut List() }
    let n = p.name
    p.tags = Mut List()
    let _k = read_str(n)
}

// [fate-field-disjoint] [deduce-field] A **contents** mutation of the same
// field leaves a *handle to that field* standing (rung ⑤ v2, 2026-09-25):
// the list object is still the one `t` names, and both backends observe the
// addition through it — Rust because the binding re-materializes the place,
// Kotlin because it holds that object. What the rule refuses is a derivation
// reaching *into* the contents, and a **replacement** of the field; those
// are the cases below.
fn a_contents_mutation_spares_a_handle_to_the_same_field() -> None {
    let p = Person { name: "a", tags: Mut List() }
    let t = p.tags
    add_tag(p.tags, "y")
    let _k = tally(t)
}

// [deduce-field] …and **replacing** the field does poison the handle: the
// storage identity is gone, which is exactly the divergence the rule keeps
// unobservable (a re-read would see the new value where Kotlin's binding
// holds the old object).
fn replacing_the_same_field_still_poisons() -> None {
    let p = Mut Person { name: "a", tags: Mut List() }
    let t = p.tags
    p.tags = Mut List()
    //~ error[fate-poison:shared-root]
    let _k = tally(t)
}

// [fate-field-disjoint] A mutation of the **whole variable** is a prefix of
// every projection, so it poisons everything derived from it — the pre-L5
// behavior, unchanged.
fn a_whole_variable_mutation_still_poisons_every_field() -> None {
    let p = Mut Person { name: "a", tags: Mut List() }
    let n = p.name
    touch_all(p)
    //~ error[fate-poison:shared-root]
    let _k = read_str(n)
}

// [fate-field-disjoint] A **prefix** overlaps: a value derived from
// `o.inner` is poisoned by a mutation of `o.inner.tags`, since the mutation
// changes part of what it names.
fn a_prefix_projection_is_poisoned_by_a_deeper_mutation() -> None {
    let o = Outer { inner: Person { name: "a", tags: Mut List() }, other: "o" }
    let i = o.inner
    add_tag(o.inner.tags, "y")
    //~ error[fate-poison:shared-root]
    let _k = read_str(i.name)
}

// [fate-field-disjoint] A **dynamic index** may land on any element, so
// `arr[i]` and `arr[j]` are treated as possibly the same storage
// ([`proj::Element`]'s may-alias rule) — precision stops where the analysis
// cannot tell two places apart.
fn a_dynamic_index_stays_conservative() -> None {
    let arr: Mut List<Str>[] = array_of(Mut List<Str>(), Mut List<Str>())
    let one = arr[0]
    add_tag(arr[1], "z")
    //~ error[fate-poison:shared-root]
    let _k = tally(one)
}

// ===== [fate-partial-move] L5's move half: partial moves =====
//
// Moving a projection out leaves the *rest* of the root readable, instead of
// consuming the whole variable. The moved place is recorded per root, a read
// overlapping it is an error naming the field that left, and a whole-value
// use is always refused (a value missing a part cannot be handed on).
// Reassigning the place puts it back.
//
// This matches Rust, where partial moves are function-local: a borrowed
// parameter refuses a move out of it (kept deductions still error), and an
// owned one may be partially moved because the caller already gave it up.

// [fate-partial-move] The shape the move half exists for: hand one field to
// a consuming fn, keep reading the other.
fn a_disjoint_field_survives_a_projection_move() -> None {
    let p = Person { name: "a", tags: Mut List() }
    eat(p.tags)
    let _k = read_str(p.name)
}

// [fate-partial-move] The same through a move-mode *binding* rather than a
// consuming call: `let t = p.tags` takes ownership of that field only.
fn a_move_mode_binding_of_a_field_leaves_its_siblings() -> None {
    let p = Person { name: "a", tags: Mut List() }
    let t = p.tags
    add_tag(t, "z")
    let _k = read_str(p.name)
}

// [fate-partial-move] Reading the moved field back is the error, and it
// names the field rather than the variable, and the move's line (the IR flow
// pass's diagnostic since R8 piece 3b [flow-use-after-move]).
fn reading_a_moved_field_back_is_an_error() -> None {
    let p = Person { name: "a", tags: Mut List() }
    eat(p.tags)
    //~ error[fate-partial-move:overlapping-use]
    let _k = tally(p.tags)
}

// [fate-partial-move] The whole value can never be used once a part has left
// — the rule that keeps an incomplete struct from being handed on.
fn the_whole_value_cannot_be_used_after_a_partial_move() -> None {
    let p = Person { name: "a", tags: Mut List() }
    eat(p.tags)
    //~ error[fate-partial-move:whole-use]
    take_person(p)
}

// [fate-partial-move] Assigning the place puts data back, so the variable is
// whole again — including reads of the field that had left.
fn reassigning_a_moved_field_revives_it() -> None {
    let p = Mut Person { name: "a", tags: Mut List() }
    eat(p.tags)
    p.tags = Mut List()
    let _k = tally(p.tags)
    let _j = read_str(p.name)
}

// [fate-partial-move] Flow-sensitive like consumption: moved on *some* path
// is moved after the join.
fn a_move_on_one_branch_is_moved_after_the_join() -> None {
    let p = Person { name: "a", tags: Mut List() }
    if read_str(p.name) > 0 {
        eat(p.tags)
    }
    //~ error[fate-partial-move:overlapping-use]
    let _k = tally(p.tags)
}

// …but a disjoint field is still readable on every path.
fn a_disjoint_field_is_readable_after_the_join() -> None {
    let p = Person { name: "a", tags: Mut List() }
    if 1 > 0 {
        eat(p.tags)
    }
    let _k = read_str(p.name)
}

// [fate-partial-move] A **kept** parameter still refuses the move outright:
// the caller keeps the value, so nothing may be taken from it. This is
// Rust's rule for a borrowed parameter (E0507), and it is why partial moves
// never need to appear in a signature.
fn kept(p: Person) -> None => p {
    //~ error[fate-move-mode:kept-mutable]
    eat(p.tags)
    return None
}
