// [op-arith] [op-promote] [op-bool] [type-lit-adopt] Operator typing over the
// real std: arithmetic on numbers only, widening within the integer and float
// classes, an explicit conversion across them, no `Str +`, `Bool`-only
// logicals, and unsuffixed literals adopting the expected numeric type. An
// operator is the call it names, so a missing function is the call's
// diagnostic, said of the operator.

fn numeric_arithmetic_is_legal() -> None {
    let a = 1 + 2 * 3 % 4
    let _b = 1L + 2L
    let _c = 1.5 * 2.0
    let d = -a
    let _e = a + d
}

fn mixing_classes_needs_a_conversion() -> None {
    //~ error[op-promote:mixed-classes]
    let _a = 1 + 2.5
    //~ error[op-promote:mixed-classes]
    let _b = 1L < 2.5
    let _c = to_double(1) + 2.5
    let _d = to_long(1) + 2L
}

fn non_numbers_do_not_add() -> None {
    //~ error[call-resolve:no-overload]
    let _a = "x" + "y"
    //~ error[call-resolve:no-overload]
    let _b = true * false
    //~ error[call-resolve:no-overload]
    let _c = 1 - "y"
}

// [type-byte] A `Byte` is no number to the operators; the conversion pair is
// the way through, and equality needs none.
fn bytes_compute_through_int() -> None {
    let b = to_byte(1)
    //~ error[call-resolve:no-overload]
    let _a = b + b
    let n = to_int(to_byte(200)) + 1
    let _c = to_byte(n)
    let _same = to_byte(10) == to_byte(10)
}

// [type-unknown-lenient] One mistake, one diagnostic: nothing cascades
// through the operators.
fn unknown_operands_stay_lenient() -> None {
    //~ error[call-resolve:unknown-fn]
    let a = missing()
    let b = a + 1
    let _c = b < 2
}

fn unary_operands_are_checked() -> None {
    let _a = -1
    let _b = -2.5
    let c = !true
    let _d = !c
    //~ error[op-arith:unary-minus]
    let _e = -true
    //~ error[op-bool:not-bool]
    let _f = !5
}

fn logical_operands_are_bool() -> None {
    let a = true && false
    let _b = a || true
    //~ error[op-bool:not-bool]
    let _c = 1 && true
}

fn ordering_on_numbers() -> None {
    let a = 1 < 2
    let b = 1 <= 2L
    let c = 2.5 > 1.0f
    let _d = a == b == c
}

// [backend-never-wrong] A type parameter is no number either, for the
// operators that name no function.
fn rem_of_t<T>(a: T) -> T {
    //~ error[op-arith:non-numeric]
    return a % a
}

fn neg_of_t<T>(a: T) -> T {
    //~ error[op-arith:unary-minus]
    return -a
}

fn and_of_t<T>(a: T) -> Bool {
    //~ error[op-bool:not-bool]
    return a && true
}

// [implicit-forward] At a `T`, `+` is the enclosing `?plus` or nothing.
fn twice<T>(a: T, ?plus: (x: T, y: T) -> T) -> T {
    return a + a
}

fn twice_without<T>(a: T) -> T {
    //~ error[call-resolve:no-overload]
    return a + a
}

fn add_through(a: Int, b: Int, ?plus: (x: Int, y: Int) -> Int) -> Int {
    return a + b
}

fn uses_them() -> Int {
    return add_through(1, 2) + twice(3)
}

fn literals_adopt_the_expected_type() -> None {
    let a: Long = 1
    let b: Double = 3
    let c: Float = 0.5
    let d: Long = -5
    let e: Long? = 1
    let _f = a + 1 == a || b > 0.1 || c > 0.1f || d < (e ?: unreachable!())
    // A suffixed literal stays what it says, and variables never widen.
    //~ error[var-let-infer:annotation-mismatch]
    let _g: Long = 1.5
    let n = 1
    //~ error[var-let-infer:annotation-mismatch]
    let _h: Long = n
}

fn powers_on_the_numeric_scalars() -> None {
    let _a: Int = 2 ** 10
    let _b: Long = 3L ** 2
    let _c: Double = 2.0 ** 3
    let _d: Double = 2.0 ** 0.5
    let _e: Int = -2 ** 2 ** 3
    let _f: Double = 2.0 ** -1
    // Nothing widens: what std does not declare is refused.
    //~ error[call-resolve:no-overload]
    let _g = 2L ** 2L
    //~ error[call-resolve:no-overload]
    let _h = 1.5f ** 2
}

// [op-compound] `x += e` is `x = x + e`: the same rules.
fn compound_assignment() -> None {
    let n = 1
    n += 2
    n -= 1
    n *= 3
    n /= 2
    let s = "a"
    //~ error[call-resolve:no-overload]
    s += "b"
    let m = 1
    //~ error[op-promote:mixed-classes]
    m += to_double(2)
}

struct E {
    hp: Int
}

fn make() -> E {
    return E { hp: 1 }
}

// [op-assign] A target that is not a place is one mistake, reported once.
fn not_a_place() -> None {
    //~ error[op-assign:not-a-place]
    make().hp = 2
}
