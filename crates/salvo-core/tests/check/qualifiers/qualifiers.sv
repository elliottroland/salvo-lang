// Qualifier declarations and uses the checker refuses: a qualifier applied
// twice or to a type it is not of, constructor functions out of place or of
// the wrong shape, and constructive qualifiers met where only a constructor
// may supply them.

//--- duplicate_qualifier_is_rejected
// [qual-no-dup]
qualifier Ok<T> of T

//~ error[qual-no-dup:applied-twice]
fn f(x: Ok Ok Int) -> None {
}

//--- qualifier_of_type_is_enforced
// [qual-of]
export qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

//~ error[qual-of:does-not-apply]
export fn f(x: Positive Str) -> None {
}

//--- constructor_must_live_with_its_qualifier
// [qual-ctor-same-file]
// FILE: quals.sv
export qualifier Fancy of Int
// FILE: other.sv
import quals.Fancy

//~ error[qual-ctor-same-file:other-file]
fn make() -> +Fancy Int {
    return 1
}

//--- constructor_return_type_must_be_simple
// [qual-ctor-simple]
export qualifier Fancy of Int

//~ error[qual-ctor-simple:union]
export fn make() -> +Fancy (Int | Str) {
    return 1
}

//--- constructive_qualifier_cannot_be_is_tested
// [qual-constructive]
export qualifier Fancy of Int

export fn f(x: Int) -> None {
    //~ error[qual-constructive:is-test]
    if x is Fancy {
    }
}

//--- qualifies_signature_is_validated
// [qual-predicate]
export qualifier Weird of Int {
    //~ error[qual-predicate:return-type]
    fn qualifies(int: Int) -> Str {
        return "nope"
    }
}

//--- constructive_values_only_come_from_constructors
// [qual-constructive] [qual-ctor-fn]
// Plain values never satisfy a constructive qualifier: the assignment is a
// type error, so the only way in is the constructor fn.
export qualifier Fancy of Int

export fn f() -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _x: Fancy Int = 1
}
