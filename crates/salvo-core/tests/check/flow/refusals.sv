// [type-query] [type-assert] What the two forms refuse.

// A mismatch is an error naming both types.
fn a_mismatch(s: Str, i: Int) [] -> None => s, i {
    //~ error[type-assert:mismatch]
    type!(i: Idx(s) Int)
    //~ error[type-assert:mismatch]
    type!(s: Int)
    //~ error[type-assert:mismatch]
    type!(s: proj Str)
    if i is Idx(s) {
        // Exact: the claim is part of the type.
        //~ error[type-assert:mismatch]
        type!(i: Int)
    }
}

// The argument is a place: a local, a field path, a destructured binding.
fn not_a_place(s: Str) [] -> None => s {
    //~ error[type-query:not-a-place]
    type?(size(s))
    //~ error[type-assert:not-a-place]
    type!(size(s): Int)
    //~ error[type-query:not-a-place]
    type?(1)
}

// An unknown name is reported as any read of it is; a fn named by itself is
// a value, not a place.
fn an_unknown_name() {
    //~ error[name-ident-resolve:unknown]
    type?(nowhere)
    //~ error[name-ident-resolve:unknown]
    type!(nowhere: Int)
    //~ error[type-query:not-a-place]
    type?(an_unknown_name)
    //~ error[type-assert:not-a-place]
    type!(an_unknown_name: () -> None)
}

// A `-` before a number is a negative literal type, not a drop.
fn a_negative_literal_type(i: Int) [] -> None => i {
    //~ error[type-assert:mismatch]
    type!(i: -1)
}

// `assert!(x is Q)` is unchanged: a run-time assertion, whose condition may
// be a re-typing (`(o: -Ok) is Int`).
fn the_runtime_assertion_is_unchanged(o: Ok Int | Str) [] -> None => o {
    if o is Ok {
        assert!((o: -Ok) is Int)
    }
}

// `assert!` is never a type check: `assert!(i: Int)` asserts the re-typing
// `i: Int` at run time, and an `Int` is not a condition.
fn assert_is_not_a_type_check(i: Int) [] -> None => i {
    //~ error[expr-cond-bool:not-bool]
    assert!(i: Int)
}

//--- type_bang_needs_its_type
// `type!` takes a place and its type: without the `:`, a parse error.
fn no_type(i: Int) [] -> None => i {
    //~ error[syntax-expected:token]
    type!(i)
}

//--- a_signed_list
// A drop or an addition is not a type: the form checks, it changes nothing.
fn signed(i: Int) [] -> None => i {
    //~ error[type-assert:signed]
    type!(i: -Idx)
}

//--- a_plus
fn plus(i: Int) [] -> None => i {
    //~ error[type-assert:signed]
    type!(i: +Idx)
}

//--- a_message
// Checked when compiling, so it never fails at run time with a message.
fn message(i: Int) [] -> None => i {
    //~ error[type-assert:message]
    type!(i: Int, "why")
}
