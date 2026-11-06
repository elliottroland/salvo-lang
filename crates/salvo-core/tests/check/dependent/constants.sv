// [qual-const] Constant slots: `Percent(0, 100) Int` in a parameter demands
// a claim with exactly those constants — carried, matched, or the plain
// overload takes the call.

qualifier Percent(lo: Int, hi: Int) for Int {
    fn qualifies(n: Int, lo: Int, hi: Int) -> Bool {
        return n >= lo && n <= hi
    }
}

fn shade(n: Percent(0, 100) Int) [] -> Int => n {
    return n + 0
}

fn constant_slots_match_exactly(n: Int) [] -> Int => n {
    assert!(n is Percent(0, 100))
    return shade(n)
}

// A claim with different constants is a different fact: the call is
// refused rather than silently accepted.
fn different_constants_refuse(n: Int) [] -> Int => n {
    assert!(n is Percent(0, 255))
    //~ error[call-resolve:no-overload]
    return shade(n)
}

// [qual-const] `Char` and `Str` constants (the CLI design record Q5c): `Tag('p')`
// and `Prefix("arn:")` fill slots of their own type, establish by `is`, and
// agree exactly — a `Prefix("arm:")` claim does not fill a
// `Prefix("arn:")` position.
qualifier Prefix(p: Str) for Str {
    fn qualifies(s: Str, p: Str) -> Bool {
        return true
    }
}

qualifier Tag(c: Char) for Int {
    fn qualifies(n: Int, c: Char) -> Bool {
        return true
    }
}

fn arn(s: Prefix("arn:") Str) [] -> Int => s {
    return 0
}

fn tagged(n: Tag('p') Int) [] -> Int => n {
    return n + 0
}

fn char_and_str_constants_match(s: Str, n: Int) [] -> Int => s, n {
    assert!(s is Prefix("arn:"))
    assert!(n is Tag('p'))
    return arn(s) + tagged(n)
}

fn another_prefix_does_not_fill(s: Str) [] -> Int => s {
    assert!(s is Prefix("arm:"))
    //~ error[call-resolve:no-overload]
    return arn(s)
}

//~ error[qual-const:slot-mismatch]
fn a_str_in_a_char_slot(n: Tag("p") Int) [] -> Int => n {
    return 0
}

fn an_int_in_a_char_slot(n: Int) [] -> Bool => n {
    //~ error[qual-const:slot-mismatch]
    return n is Tag(1)
}

// [qual-depend] `i++` rebinds the variable, so dependent claims bound to
// its old value strip too — the step is a reassignment in every flow
// sense [op-inc-dec].
qualifier Above(floor: Int) for Int {
    fn qualifies(n: Int, floor: Int) -> Bool {
        return n > floor
    }
}

fn stepping_a_linked_int_strips(n: Int, base: Int) [] -> Bool => n, base {
    let floor = base + 0
    assert!(n is Above(floor))
    floor++
    //~ error[qual-retype:not-carried]
    let _n = n: -Above
    return true
}

// [qual-const] A value slot a signature fills with a parameter takes this
// call's literal there, as a place argument brings its roots [qual-depend]:
// `clamp(n, 1, 65535)` is an `InRange(1, 65535) Int`.
fn clamp(n: Int, lo: Int, hi: Int) [] -> InRange(lo, hi) Int {
    let m = if n < lo { lo } elif n > hi { hi } else { n }
    if m is InRange(lo, hi) {
        return m
    }
    return unreachable!()
}

fn port(p: InRange(1, 65535) Int) [] -> Int => !p {
    return p
}

fn literal_arguments_fill_constant_slots(n: Int) [] -> Int {
    return port(clamp(n, 1, 65535))
}

fn other_literals_are_another_claim(n: Int) [] -> Int {
    //~ error[call-resolve:no-overload]
    return port(clamp(n, 0, 10))
}

// [qual-const] A negative constant, written and tested.
fn signed(n: InRange(-5, 5) Int) [] -> Int => !n {
    return n
}

fn a_negative_constant(n: Int) [] -> Int {
    assert!(n is InRange(-5, 5))
    return signed(n) + signed(clamp(n, -5, 5))
}
