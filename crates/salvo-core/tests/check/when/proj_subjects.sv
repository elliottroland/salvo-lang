// [when-proj-subject] A `when` over a borrowed union, `proj (A | B)`, matches
// the union's arms, and each arm stays borrowed: inside `is A` the subject is
// a `proj A`, whose fields read as borrows. It is exhaustive over the arms
// like any `when` [when-exhaustive], and what a borrow may not do — be
// mutated, be moved — it may not do narrowed either [proj-readonly].

//--- borrowed_arms_read_their_fields
struct A { x: Int, name: Str }
struct B { s: Str }
struct C {}
type ABC = A | B | C

fn len(s: Str) [] -> Int => s { return s.size() }

fn plain(v: proj ABC) [] -> Int {
    return when v {
        is A { v.x + len(v.name) }
        is B { len(v.s) }
        is C { 0 }
    }
}

// A union head, a binding, and the binding form over a borrowed subject.
fn heads(v: proj ABC) [] -> Str {
    return when v {
        is A a { a.name }
        is B | C { "other" }
    }
}

fn bound(xs: List<ABC>) [] -> Str => xs {
    return when e = get(xs, 0) ?: unreachable!() {
        is B { e.s }
        is A | C { "?" }
    }
}

// A branch that leaves takes its arms with it [is-narrow-guard].
fn guard(v: proj ABC) [] -> Int {
    when v {
        is C { return 0 }
        is A | B { }
    }
    return when v {
        is A { v.x }
        is B { len(v.s) }
    }
}

// A `for` element of a list of unions is a borrowed union.
fn total(xs: List<ABC>) [] -> Int => xs {
    let n = 0
    for v in xs {
        n += when v {
            is A { v.x }
            is B | C { 1 }
        }
    }
    return n
}

// The lowered `?.` and `?:` over a borrowed optional [when-lowered].
fn names(xs: List<A?>) [] -> Int => xs {
    let n = 0
    for o in xs {
        n += o?.x ?: 0
        n += len(o?.name ?: "")
    }
    return n
}

//--- borrowed_arms_are_exhaustive
struct A { x: Int }
struct B {}
type AB = A | B

fn missing(v: proj AB) [] -> Int {
    //~ error[when-exhaustive:missing-arms]
    return when v {
        is A { v.x }
    }
}

fn dead(v: proj AB) [] -> Int {
    return when v {
        is A | B { 0 }
        //~ error[when-exhaustive:dead-branch]
        is A { 1 }
    }
}

fn several(v: proj AB) [] -> Int {
    return when v {
        //~ error[is-union:binding]
        is A | B both { 0 }
    }
}

//--- borrowed_arms_stay_borrows
struct A canbe Mut { x: Int, name: Str }
struct B {}
type AB = A | B

fn keep(a: A) [] -> None => !a {}

fn bump(a: Mut A) [] -> None => a: Mut { a.x = a.x + 1 }

fn moves(v: proj AB) [] -> None {
    when v {
        is A {
            //~ error[call-resolve:no-overload]
            bump(v)
            //~ error[struct-mut:immutable-field-assign]
            v.x = 2
            //~ error[proj-type:projection-consumed]
            keep(v)
        }
        is B { }
    }
}

//--- a_claim_about_a_union_is_still_refused
// [when-union-subject] Only a borrow reads through to the arms: any other
// qualifier on a union is a claim *about* it.
export qualifier Ok<T> for T
struct A {}
struct B {}

fn claim(v: Ok (A | B)) [] -> Int {
    //~ error[when-union-subject:qualified-union]
    return when v {
        is A { 0 }
        is B { 1 }
    }
}

//--- a_handle_to_a_union_is_not_matched_yet
// A handle to a union (`at` over a `List<Mut A | Mut B>`) is refused with its
// own diagnostic: it is not a claim, so dropping it is no remedy, and
// narrowing a handle to its arms is ROADMAP §8's open question.
struct A canbe Mut { x: Int }
struct B canbe Mut {}

fn walk() [] -> Int {
    let xs = Mut List<Mut A | Mut B>()
    xs.add(Mut A { x: 1 })
    let h = at(xs, 0) ?: unreachable!()
    //~ error[when-proj-subject:mutable]
    return when h {
        is A { 1 }
        is B { 0 }
    }
}

//--- a_borrow_of_a_union_is_a_union_of_borrows
// `proj (A | B) <: (proj A) | (proj B)`: a borrow of a union is a borrow of
// whichever arm it holds.
struct A {}
struct B {}

fn whole(v: proj (A | B)) [] -> Int {
    return when v {
        is A { 0 }
        is B { 1 }
    }
}

fn arms(v: (proj A) | (proj B)) [] -> Int {
    return when v {
        is A { 0 }
        is B { 1 }
    }
}

fn forward(v: proj (A | B)) [] -> Int {
    return arms(v)
}
