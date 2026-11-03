// The re-typing `e: -Q` and `place := -Q` [qual-retype] (user decisions
// 2026-10-30, replacing the `^` lift).
//
// A re-typing only drops qualifiers, and tests nothing: the runtime work is
// `is`'s. Its list is the deduction clause's [deduce-syntax] — `-A -B` drops
// those, `A B` keeps only those, optionally ending in the value's own base
// type — less `+Q`, `None` and `Never`. `Ok`, `Err`, `ok` and `err` are std's
// [qual-result-tags].

//--- what_a_re_typing_drops
struct Person canbe Mut {
    name: Str
}

qualifier Surname of Person

fn note(text: Str) [] -> None => text {}
fn read(p: Person) [] -> None => p {}
fn touch(p: Mut Person) [] -> None => p: Mut {}

// [qual-retype] The motivating case: after the arm test, dropping the claim
// opens the qualified union inside the arm.
fn nested() [] -> Ok (Ok Int | Err Str) | Err Str { return err("e") }

fn a_dropped_claim_opens_a_nested_union() [] -> None {
    let o = nested()
    when o {
        is Ok {
            o := -Ok
            when o {
                is Ok { note("value") }
                is Err { note("inner error") }
            }
        }
        is Err { note("outer error") }
    }
}

// [qual-retype] Inline, for one use: the unqualified overload.
fn an_inline_drop_picks_the_plain_overload(p: Mut Person) [] -> None => p: Mut {
    read((p: -Mut))
    touch(p)
}

// [qual-retype] As a statement it lasts: the value no longer grants what the
// qualifier did.
fn a_dropped_mut_stays_dropped(p: Mut Person) [] -> None => p: Mut {
    p := -Mut
    read(p)
    //~ error[call-resolve:no-overload]
    touch(p)
}

// [qual-retype] `p = p: …` is the same statement: no write.
fn the_assignment_spelling_is_a_re_typing(p: Mut Person) [] -> None => p: Mut {
    p = p: Person
    //~ error[call-resolve:no-overload]
    touch(p)
}

// [qual-retype] A kept list may end in the value's own base type.
fn a_kept_list_names_the_base(p: Mut Person) [] -> None => p: Mut {
    let _a = p: Person
    let _b = p: Mut
    let _c = p: Mut Person
}

// [qual-retype] Several arms at once, after the test that leaves them: each
// arm drops the claim.
fn three() [] -> Ok Int | Ok Str | Err Str { return err("e") }

fn several_arms_drop_together() [] -> None {
    let o = three()
    if o is Ok {
        let _lifted = o: -Ok
    }
}

// [qual-retype] A dropped claim is lost at a join where any path lost it.
fn lost_on_one_path(p: Mut Person, flag: Bool) [] -> None => p: Mut {
    if flag {
        p := -Mut
    }
    //~ error[call-resolve:no-overload]
    touch(p)
}

//--- what_a_re_typing_refuses
struct Person canbe Mut {
    name: Str
}

qualifier Surname of Person

fn two() [] -> Ok Int | Err Str { return err("e") }

fn refused(p: Mut Person, f: once () -> None) [] -> None => p: Mut, !f {
    // [qual-retype] Never adds a qualifier.
    //~ error[qual-retype:not-carried]
    let _a = p: -Surname
    //~ error[qual-retype:not-carried]
    let _b = p: Surname
    //~ error[qual-retype:base-mismatch]
    let _c = p: Int
    // [qual-retype] On a union every value arm must carry what is dropped…
    let o = two()
    //~ error[qual-retype:arm-not-carried]
    let _d = o: -Ok
    // …and a base type names one arm, which a union is not.
    //~ error[qual-retype:union-base]
    let _e = o: Ok Int
    // [qual-retype] The single exclusion list, shared with `Qual T <: T`.
    //~ error[qual-retype:cannot-remove]
    let _g = f: -once
    // [qual-retype] `:=` re-types a place.
    //~ error[qual-retype:not-a-place]
    two() := -Ok
}

// [qual-retype] A re-typing that would merge two arms changes the value's
// runtime shape: refused.
fn merged(o: Ok Surname Person | Ok Person) [] -> None => o {
    //~ error[qual-retype:arms-merge]
    let _m = o: Ok
}

//--- a_plus_is_a_parse_error
struct Person canbe Mut {
    name: Str
}

fn f(p: Mut Person) [] -> None => p: Mut {
    //~ error[qual-retype:reapply]
    let _a = p: +Mut
}

//--- none_is_a_parse_error
struct Person canbe Mut {
    name: Str
}

fn f(p: Mut Person) [] -> None => p: Mut {
    //~ error[qual-retype:none-or-never]
    let _a = p: None
}

//--- keep_and_drop_is_a_parse_error
struct Person canbe Mut {
    name: Str
}

fn f(p: Mut Person) [] -> None => p: Mut {
    //~ error[qual-retype:keep-and-drop]
    let _a = p: Mut -Mut
}

//--- a_top_level_call_colon_is_a_pair
// [qual-retype] [col-pairs] At the top of a call argument `k: v` is a pair;
// a re-typing argument is parenthesized.
struct Person canbe Mut {
    name: Str
}

fn read(p: Person) [] -> None => p {}

//~ error[deduce-default:promised-back-moved]
fn f(p: Mut Person) [] -> None => p: Mut {
    read((p: -Mut))
    //~ error[name-ident-resolve:not-a-value]
    //~ error[call-resolve:no-overload]
    read(p: Person)
}

//--- a_drop_assignment_lowers_the_claim
// [var-drop-assign] A value with fewer qualifiers is assigned, and the
// variable reads without them until a value gives them back.
fn head2(xs: NonEmpty List<Int>) [] -> Int => xs { return first(xs) }

fn f(flag: Bool) [] -> None {
    let xs = List(1, 2)
    let _a = head2(xs)
    if flag {
        xs = List<Int>()
    }
    //~ error[call-resolve:no-overload]
    let _b = head2(xs)
    xs = List(3)
    let _c = head2(xs)
}
