
qualifier NonNegative for Int {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

struct Person {
    name: Str
    age: NonNegative Int
    address: Str?

    // TODO: Getting an error here about `t` no being a binder
    fn to_json(t: Person) -> Str by to_json
}

fn main() [use, spawn] {
    use StdConsole()
    // TODO: We should be able to check literals for qualifiers automatically, at least in easy cases, and make it a compile-time error if it doesn't qualify
    let age = 36
    assert!(age is NonNegative)
    let person = Person { name: "Roland", age, address: "Something" }
    println(person.to_json())
}

comptime fn to_json<T is Struct>(t: T) -> Str {
    let json = Mut Str("{")
    [for field in T.fields] {
        json.append@string("\"${field.name}\": ")
        [when field.kind] {
            // TODO: No hover for the arm types here
            is Struct { json.append@string(t.[field]) }
            // A union is just the to_json for that type
            is Union { json.append@string(to_json@[site](t.[field])) }
            is Tuple { refuse!("Tuples not supported") }
            is FnType { refuse!("Cannot encode function as JSON") }
            is Opaque { json.append@string(to_json@[site](t.[field])) }
        }
        [if !field.last] {
            json.append@string(", ")
        }
    }
    json.append@string("}")
    return json
    // TODO: This should complain when I'm not returning Str
}

fn to_json<T>(list: List<T>, ?to_json: (T) -> Str) -> Str {
    return "[${list.map(to_json).join(",")}]"
}

fn to_json(str: Str) -> Str {
    return "\"${str}\""
}

fn to_json(none: None) -> Str {
    return "null"
}

// TODO: We should support a way of doing a when over a union spread
// fn to_json<T, S>(value: T | ...S, ?to_json: (T) -> Str) -> Str {
//     return when value {
//         is T { value.to_json() }
//         is ...S { to_json(value) }
//     }
// }

fn to_json<T>(value: T?, ?to_json: (T) -> Str) -> Str {
    return when value {
        is None { "null" }
        is T { value.to_json() }
    }
}