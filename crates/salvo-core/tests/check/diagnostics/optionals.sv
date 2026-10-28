// [op-no-none] [interp-no-none] Optionals at operators and in interpolation
// are errors (user decisions 2026-09-02).

// [op-no-none] Arithmetic, ordering, and equality all reject a
// possibly-`None` operand; `None` itself is rejected outright.
fn optional_operands() -> None {
    let maybe: Int? = None
    //~ error[op-no-none:optional-operand]
    let _a = maybe + 1
    //~ error[op-no-none:optional-operand]
    let _b = maybe < 2
    //~ error[op-no-none:optional-operand]
    let _c = maybe == "x"
    //~ error[op-no-none:optional-operand]
    let _d = 1 - maybe
    //~ error[op-no-none:none-operand]
    let _e = None * 2
}

// [op-no-none] Narrowing or asserting makes the operand legal.
fn narrowed_operands() -> None {
    let maybe: Int? = None
    if maybe is Int m {
        let _a = m + 1
        let _b = m < 2
    }
    let _c = (maybe ?: unreachable!()) + 1
}

// [op-arm-eq] Equality against a value matching one arm is legal without
// narrowing: the checker reduces it to an arm test plus a comparison.
fn equality_against_a_matching_arm() -> None {
    let maybe: Int? = None
    let _a = maybe == 3
    let _b = maybe != 3
}

struct Person {
    name: Str,
    surname: Str? = None
}

// [interp-no-none] Interpolating a possibly-`None` value is an error —
// including a struct field, which does not flow-narrow [is-narrowing].
fn interpolating_an_optional() -> None {
    let maybe: Int? = None
    let p = Person {name: "Ada"}
    //~ error[interp-no-none:optional-value]
    let _a = "${maybe}"
    //~ error[interp-no-none:optional-value]
    let _b = "${p.surname}"
    //~ error[interp-no-none:none-value]
    let _c = "${None}"
}

// [interp-no-none] The `is`-binding and `!` forms interpolate fine — and this
// is exactly the docs/language/ nullability idiom.
fn interpolating_a_narrowed_optional() -> None {
    let p = Person {name: "Ada", surname: "L"}
    if p.surname is Str surname {
        let _a = "${p.name} ${surname}"
    }
    let _b = "${p.surname ?: unreachable!()}"
}
