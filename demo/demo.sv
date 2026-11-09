
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

// TODO: No hover for `Struct` here
comptime fn to_json<T is Struct>(t: T) -> Str {
    // TODO: Go-to definition should work for contructor-type functions
    let json = Mut Str("{")
    // TODO: Should get auto-complete when typing `[`
    // TODO: Should get auto-complete on comptime fields, like `t.`
    [for field in t.fields] {
        // TODO: The auto-complete doesn't include the `.append` here? 
        json.append@string("\"${field.name}\": ")
        // TODO: No quick-fix for adding the when-arms here
        // TODO: I originally wrote `field.type` and then switched it to `field.kind`, but I still get a hover for `field.type` here
        // TODO: The doc for the arms here is outdated. It should fetch this info from the comptime.sv module
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