// [type-query] [assert-type] What the two forms refuse.

// A mismatch is an error naming both types.
fn a_mismatch(s: Str, i: Int) [] -> None => s, i {
    //~ error[assert-type:mismatch]
    assert!(i: Idx(s) Int)
    //~ error[assert-type:mismatch]
    assert!(s: Int)
    //~ error[assert-type:mismatch]
    assert!(s: proj Str)
    if i is Idx(s) {
        // Exact: the claim is part of the type.
        //~ error[assert-type:mismatch]
        assert!(i: Int)
    }
}

// The argument is a place: a local, a field path, a destructured binding.
fn not_a_place(s: Str) [] -> None => s {
    //~ error[type-query:not-a-place]
    type?(size(s))
    //~ error[assert-type:not-a-place]
    assert!(size(s): Int)
    //~ error[type-query:not-a-place]
    type?(1)
}

// An unknown name is reported as any read of it is; a fn named by itself is
// a value, not a place.
fn an_unknown_name() {
    //~ error[name-ident-resolve:unknown]
    type?(nowhere)
    //~ error[name-ident-resolve:unknown]
    assert!(nowhere: Int)
    //~ error[type-query:not-a-place]
    type?(an_unknown_name)
    //~ error[assert-type:not-a-place]
    assert!(an_unknown_name: () -> None)
}

// A `-` before a number is a negative literal type, not a drop.
fn a_negative_literal_type(i: Int) [] -> None => i {
    //~ error[assert-type:mismatch]
    assert!(i: -1)
}

// `assert!(x is Q)` is unchanged, and a re-typing inside it keeps its
// parentheses.
fn the_runtime_assertion_is_unchanged(o: Ok Int | Str) [] -> None => o {
    if o is Ok {
        assert!((o: -Ok) is Int)
    }
}

//--- a_signed_list
// A drop or an addition is not a type: the form checks, it changes nothing.
fn signed(i: Int) [] -> None => i {
    //~ error[assert-type:signed]
    assert!(i: -Idx)
}

//--- a_plus
fn plus(i: Int) [] -> None => i {
    //~ error[assert-type:signed]
    assert!(i: +Idx)
}

//--- a_message
// Checked when compiling, so it never fails at run time with a message.
fn message(i: Int) [] -> None => i {
    //~ error[assert-type:message]
    assert!(i: Int, "why")
}
