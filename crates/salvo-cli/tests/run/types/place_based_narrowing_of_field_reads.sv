// place-based narrowing of field reads [flow-place]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- place_narrowing

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
        println("city ${p.address.city}")
    }
    let h = Holder {result: ok(3)}
    if h.result is Ok {
        println("ok ${h.result}")
    }
}
//= stdout
//= Ann Lee
//= Bo
//= city Oslo
//= ok 3

//--- place_operand

struct Reading {
    label: Str,
    value: Int? = None
}

fn main() [use] -> None {
    use StdConsole
    let r = Reading {label: "temp", value: 21}
    if r.value is Int {
        println("${r.label}: ${r.value + 1}")
    }
    let empty = Reading {label: "none"}
    if empty.value is Int {
        println("unreachable")
    } else {
        println("${empty.label}: no value")
    }
}
//= stdout
//= temp: 22
//= none: no value
