// Narrowing: `is` on field subjects and narrowed field reads.

//--- field_subject_is_lowers_to_union_test
// [is-narrowing] [is-binding] Field subjects get the same union-test
// lowering as identifier subjects, and narrow like them [flow-place];
// `when` still requires a variable subject [when-union-subject].
// (`FIELD_IS_DEMO` in codegen_tests.rs.)
struct Holder {
    result: Ok Int | Err Str
}

fn main() [use] -> None {
    use StdConsole
    let h = Holder {result: ok(1)}
    //~ emits: h.result is Union2.U1<*, *>
    if h.result is Ok Int r {
        println("ok ${r}")
    }
    if h.result is Ok {
        println("plain ${h.result}")
    }
    let h2 = Holder {result: err("bad")}
    if h2.result is Err Str e {
        println("err ${e}")
    }
}

//--- narrowed_field_reads_unwrap
// [flow-place] [kt-narrow-field-assert] A narrowed nullable *field* read
// asserts instead of relying on a smart cast (Kotlin refuses to smart-cast
// a property, and a `canbe Mut` struct's fields are `var`); a narrowed
// wrapper-union field reads its arm payload.
// [flow-place] `is` narrows *places*: after `p.surname is Str` the field
// itself reads as `Str` (no binding needed), field chains included.
// Invalidation is [flow-place-invalidate]; `when` stays variable-only
// [when-union-subject]. (`PLACE_NARROW_DEMO` in codegen_tests.rs.)
struct Address canbe Mut {
    city: Str? = None
}

struct Person canbe Mut {
    name: Str,
    surname: Str? = None,
    address: Mut Address
}

struct Holder {
    result: Ok Int | Err Str
}

fn describe(p: Person) -> Str => p {
    if p.surname is Str {
        // [ir-narrow] A narrowed projection is read through a narrowing
        // binding made where the test held.
        //~ emits: : String = p.surname!!
        return "${p.name} ${p.surname}"
    }
    return p.name
}

fn main() [use] -> None {
    use StdConsole
    println(describe(Person {name: "Ann", surname: "Lee", address: Mut Address {city: "Rome"}}))
    println(describe(Person {name: "Bo", address: Mut Address {city: None}}))
    let p = Person {name: "Cy", surname: "Ray", address: Mut Address {city: "Oslo"}}
    if p.address.city is Str {
        //~ emits: : String = p.address.city!!
        println("city ${p.address.city}")
    }
    let h = Holder {result: ok(3)}
    if h.result is Ok {
        //~ emits: : Int = ((h.result as Union2.U1<*, *>).value as Int)
        println("ok ${h.result}")
    }
}

//--- narrowed_val_field_relies_on_the_smart_cast
// [kt-narrow-field-assert] The assert is only needed for `var`
// properties: a field of a struct without `canbe Mut` emits as `val`,
// which kotlinc smart-casts — asserting there would produce an
// "unnecessary non-null assertion" warning.
// [flow-place] [op-no-none] A narrowed field is usable as an *operand*,
// which the optional-strictness rules used to reject outright.
// (`PLACE_OPERAND_DEMO` in codegen_tests.rs.)
struct Reading {
    label: Str,
    value: Int? = None
}

fn main() [use] -> None {
    use StdConsole
    let r = Reading {label: "temp", value: 21}
    if r.value is Int {
        // [ir-narrow] One rule for `val` and `var` properties: the narrowing
        // binding, which a `val` needs no more than a `var` does but costs
        // nothing either.
        //~ emits: : Int = r.value!!
        println("${r.label}: ${r.value + 1}")
    }
    let empty = Reading {label: "none"}
    if empty.value is Int {
        println("unreachable")
    } else {
        println("${empty.label}: no value")
    }
}

//--- an_is_binding_over_a_call_hoists_its_subject_kotlin
// [is-bind-once] …and the shape: one `val`, read by the test and the binding,
// with the `while` becoming `while (true)` so the single evaluation happens
// once *per iteration*.
// [is-bind-once] The defect this rule closed, as a program
// (`IS_BIND_ONCE` in codegen_tests.rs): an `is` binding whose subject is a
// call must evaluate it exactly once, in all four emitter paths.
fn drain_ints(xs: Mut List<Int>) [Console] -> None => xs: Mut {
    // statement `while`: one call per turn, so every element is seen
    //~ emits: while (true) {
    //~ emits: if (!((__subject_1 != null))) {
    //~ emits: val n: Int = __subject_1!!
    while remove_first(xs) is Int n {
        println("  took ${n}")
    }
}

fn first_or(xs: Mut List<Int>, fallback: Int) [] -> Int => xs: Mut, fallback {
    // value-position `if`: one call, and the binding is that call's value
    return if remove_first(xs) is Int n { n } else { fallback }
}

fn sum_down(xs: Mut List<Int>) [] -> Int => xs: Mut {
    let total = 0
    // value-position `while`: the loop's value is its last body value
    let last = while remove_first(xs) is Int n {
        total = total + n
        n
    } else { 0 }
    discard(last)
    return total
}

fn main() [use] {
    use StdConsole()
    let a: Mut List<Int> = Mut List(1, 2, 3)
    drain_ints(a)
    println("left ${size(a)}")

    let b: Mut List<Int> = Mut List(7, 8)
    println("first ${first_or(b, 0)} then ${size(b)}")

    let c: Mut List<Int> = Mut List(1, 2, 3, 4)
    println("sum ${sum_down(c)} left ${size(c)}")

    // statement `if`: the call happens once, so the list loses exactly one
    let d: Mut List<Int> = Mut List(5, 6)
    if remove_first(d) is Int n {
        println("one ${n} left ${size(d)}")
    }
}
