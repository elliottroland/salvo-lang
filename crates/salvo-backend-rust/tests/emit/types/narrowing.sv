// Narrowed places and tuple elements: what a narrowed read lowers to.

//--- narrowed_field_reads_unwrap
// [flow-place] `is` narrows *places*: after `p.surname is Str` the field
// itself reads as `Str` (no binding needed), field chains included.
// Invalidation is [flow-place-invalidate]; `when` stays variable-only
// [when-union-subject].
// [flow-place] A narrowed nullable field read unwraps the `Option`
// physically [rs-option]; a narrowed wrapper-union field read takes the
// arm accessor [rs-union-enums].
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
        // The narrowed field read unwraps.
        //~ emits: let mut __narrowed_1 = p.surname.as_ref().unwrap();
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
        // So does a narrowed field *chain* read.
        //~ emits: let mut __narrowed_3 = p.address.city.as_ref().unwrap();
        println("city ${p.address.city}")
    }
    let h = Holder {result: ok(3)}
    if h.result is Ok {
        // The narrowed wrapper-union field read uses the arm accessor.
        //~ emits: let mut __narrowed_4 = match &h.result { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        println("ok ${h.result}")
    }
}

//--- narrowed_field_operand_unwraps
// [flow-place] [op-no-none] A narrowed field is usable as an *operand*,
// which the optional-strictness rules used to reject outright.
// [flow-place] [rs-option] The operand read unwraps the `Option`; Rust has
// no smart cast to lean on, so this is the same lowering everywhere.
struct Reading {
    label: Str,
    value: Int? = None
}

fn main() [use] -> None {
    use StdConsole
    let r = Reading {label: "temp", value: 21}
    if r.value is Int {
        //~ emits: let mut __narrowed_3 = r.value.unwrap();
        println("${r.label}: ${r.value + 1}")
    }
    let empty = Reading {label: "none"}
    if empty.value is Int {
        println("unreachable")
    } else {
        println("${empty.label}: no value")
    }
}

//--- tuple_elements_emit_native_indexes
// `t.0` reads a tuple element; a constant index narrows like a field
// [flow-place], and nesting (`t.1.0`) is two projections.
// [rs-tuple-index] Rust tuples index natively; a narrowed element unwraps
// its `Option` [rs-option].
fn main() [use] -> None {
    use StdConsole
    let t: (Int, Str, Bool) = (1, "two", true)
    //~ emits: t.0
    println("${t.0} ${t.1} ${t.2}")
    let nested: (Int, (Str, Int)) = (7, ("in", 9))
    //~ emits: nested.1.0, nested.1.1
    println("${nested.1.0} ${nested.1.1}")
    let maybe: (Str?, Int) = ("here", 5)
    if maybe.0 is Str {
        //~ emits: let mut __narrowed_3 = maybe.0.as_ref().unwrap();
        println("some ${maybe.0} ${maybe.1 + 1}")
    } else {
        println("none")
    }
}
