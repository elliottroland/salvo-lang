// Qualifier declarations and uses the checker refuses: a qualifier applied
// twice or to a type it is not of, constructor functions out of place or of
// the wrong shape, and constructive qualifiers met where only a constructor
// may supply them.

//--- duplicate_qualifier_is_rejected
// [qual-no-dup]
qualifier Ok<T> for T

//~ error[qual-no-dup:applied-twice]
fn f(x: Ok Ok Int) -> None {
}

//--- qualifier_of_type_is_enforced
// [qual-of]
export qualifier Positive for Int {
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
export qualifier Fancy for Int
// FILE: other.sv
import quals.Fancy

//~ error[qual-ctor-same-file:other-file]
fn make() -> +Fancy Int {
    return 1
}

//--- constructor_return_type_must_be_simple
// [qual-ctor-simple]
export qualifier Fancy for Int

//~ error[qual-ctor-simple:union]
export fn make() -> +Fancy (Int | Str) {
    return 1
}

//--- constructive_qualifier_cannot_be_is_tested
// [qual-constructive]
export qualifier Fancy for Int

export fn f(x: Int) -> None {
    //~ error[qual-constructive:is-test]
    if x is Fancy {
    }
}

//--- qualifies_signature_is_validated
// [qual-predicate]
export qualifier Weird for Int {
    //~ error[qual-predicate:return-type]
    fn qualifies(int: Int) -> Str {
        return "nope"
    }
}

//--- constructive_values_only_come_from_constructors
// [qual-constructive] [qual-ctor-fn]
// Plain values never satisfy a constructive qualifier: the assignment is a
// type error, so the only way in is the constructor fn.
export qualifier Fancy for Int

export fn f() -> None {
    //~ error[var-let-infer:annotation-mismatch]
    let _x: Fancy Int = 1
}

//--- two_modules_qualifiers_of_one_name_are_two_claims
// [qual-identity] Each module's `Pos` is its own claim: `a`'s positive
// number is no proof of `main`'s negative one, though both are spelled `Pos`
// and `main` reaches `a`'s through `pos`'s result.
// FILE: a.sv
export qualifier Pos for Int {
    fn qualifies(n: Int) -> Bool {
        return n > 0
    }
}

export fn pos() [] -> Pos Int {
    let n = 5
    if n is Pos {
        return n
    }
    return pos()
}
// FILE: main.sv
import a.pos

qualifier Pos for Int {
    fn qualifies(n: Int) -> Bool {
        return n < 0
    }
}

fn need(x: Pos Int) [] -> Int => !x {
    return x
}

fn mine() [] -> Int {
    let n = -5
    if n is Pos {
        return need(n)
    }
    //~ error[call-resolve:no-overload]
    return need(pos())
}

//--- a_programs_claim_of_a_std_name_is_its_own
// [qual-identity] A program's `NonEmpty` over `List` (a stricter meaning)
// sits beside std's generic one, and std's refinements establish std's
// claim only: `List`'s `refn add => list: +NonEmpty` leaves a one-element
// list short of the program's.
qualifier NonEmpty<T> for List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 1
    }
}

fn need(xs: NonEmpty List<Int>) [] -> Int => xs {
    return 1
}

fn f() -> None {
    let xs: Mut List<Int> = Mut List()
    add(xs, 1)
    //~ error[call-resolve:no-overload]
    let _n = need(xs)
}
