
qualifier NonNegative for Int {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

namespace Json {
    struct Object { value: Map<Str, Json> }
    struct Array { value: List<Json> }
    struct Str { value: Str }
    struct Number { value: Number }
    struct Bool { value: Bool }
    struct Null {}

    type ParseError = .EmptyString | .InvalidChar | .UnexpectedEnd
    struct EmptyString {}
    struct InvalidChar { pos: Int, message: Str }
    struct UnexpectedEnd {}
}

type Json = Json.Object | Json.Array | Json.Str | Json.Number | Json.Bool | Json.Null {
    fn to_str(json: proj Json) -> Str {
        return when json {
            is .Object { json.to_str() }
            is .Array { json.to_str() }
            is .Str { "\"${json.value}\"" }
            is .Number { "${json.value}" }
            is .Bool { "${json.value}" }
            is .Null { "null" }
        }
    }
}

fn to_str(json: Json.Object) -> Str {
    let str = Mut Str("{")
    let remaining_entries = json.value.size()
    for entry in json.value.entries() {
        str.append("${entry.key}: ")
        str.append(entry.value.to_str())
        if remaining_entries-- > 1 {
            str.append(", ")
        }
    }
    str.append("}")
    return str
}

fn to_str(json: Json.Array) -> Str {
    let str = Mut Str("[")
    let remaining = json.value.size()
    for elem in json.value {
        str.append(elem.to_str())
        if remaining-- > 1 {
            str.append(", ")
        }
    }
    str.append("]")
    return str
}

comptime fn to_json<T is Struct>(t: T) -> Json {
    let value = Mut Map<Str, Json>()
    [for field in T.fields] {
        // TODO: Should support assigning the value like this
        let json_value: Json? = None
        [when field.type.kind] {
            is Struct { json_value = to_json@[site](t.[field]) }
            is Union { json_value = to_json@[site](t.[field]) }
            is Tuple { json_value = [to_json@demo(t.[field])] }
            is Opaque { json_value = to_json@[site](t.[field]) }
            is FnType { refuse!("Cannot encode function as JSON") }
        }
        value.put@map(field.name, json_value)
    }
    return Json.Object { value }
}

comptime fn to_json<T is Tuple>(t: T) -> Json {
    let value = Mut List<Json>()
    [for index in T.indices] {
        value.add@list(to_json@[site](t.[index]))
    }
    return Json.Array { value }
}

fn to_json(str: Str) -> Json {
    return Json.Str { value: str.copy() }
}

fn to_json(num: Number) -> Json {
    return Json.Number { value: num.copy() }
}

fn to_json(bool: Bool) -> Json {
    return Json.Bool { value: bool.copy() }
}

fn to_json<T>(value: T?, ?to_json: (T) -> Json) -> Json {
    return value?.to_json() ?: Json.Null {}
}

fn parse_json(str: Str) -> Ok Json | Err Json.ParseError {
    if str.size() == 0 {
        return Json.EmptyString {}
    }
    let parsing = Mut ParsingJson { str, pos: 0 as! Idx(str) }
    todo!()
}

fn next_number(parsing: Mut ParsingJson) -> Ok Json.Number | Err Json.ParseError {
    let number_str = Mut Str()
    let used_dot = false
    let { str, pos } = parsing
    // TODO: Why did char_at() not auto-complete in the while condition?
    while pos < str.size() - 1 { // We 
        // TODO: char_at() is not picking up the overload here. Is it prioritizing the platform function?
        let char = parsing.str.char_at(pos)
        if char >= '0' && char <= '9' {
            number_str.append(char)
        } elif char == '.' && !used_dot {
            number_str.append(char)
            used_dot = true
        } else {
            return Json.InvalidChar { pos: parsing.pos, message: "Number cannot have two decimal points" }
        }
        // TODO: Hover here shows `pos: proj Idx(str) Idx(str) Int` -- why the double Idx(str)?
        pos++
        assert!(pos is Idx(str)) // Guaranteed by the condition in the loop
    }

}

fn next_str(parsing: Mut ParsingJson) -> Ok Json.Str | Err Json.ParseError {
    // We require every string to start with `"`
    let { str, pos } = parsing
    if str.char_at(pos) != '"' {
        return Json.InvalidChar { pos, message: "String must start with '\"'" }
    }
    let value = Mut Str()
    let escape_next = false
    let value_closed = false
    while ++pos is Idx(str) {
        let char = str.char_at(pos)
        if char == '"' && !escape_next {
            value_closed = true
            break
        } elif char == '\\' {
            escape_next = true
            continue
        }
        value.append(char)
    }
    // If we didn't encounter a close-quote then the string we're trying to parse ended before it should have
    if !value_closed {
        return Json.UnexpectedEnd {}
    }
    // We bump the position up by one, so that the next bit starts from the next character
    pos++
    return Json.Str { value }
}

// The state used by an ongoing [parse_json] call, keeps track of the [str] from which [Json] types are
// going to be extracted, from the [pos] onwards. We only need a projection of the [str] since we're only
// ever reading chars out of it.
struct ParsingJson canbe Mut {
    str: proj Str
    pos: Idx(str) Int
}

struct Person {
    name: Str
    age: NonNegative Int
    address: Str?
    pair: (Str, Int)

    fn to_json(t: Person) -> Json by to_json
    // fn to_json(t: Person) -> Str by to_json
}

fn main() [use, spawn] {
    use StdConsole()
    let person = Person {
        name: "Roland",
        age: 36 as! NonNegative,
        pair: ("Some", -1),
        address: None
    }
    println(person.to_json())
}

// comptime fn to_json<T is Struct>(t: T) -> Str {
//     let json = Mut Str("{")
//     [for field in T.fields] {
//         json.append@string("\"${field.name}\": ")
//         [when field.type.kind] {
//             is Struct { json.append@string(t.[field]) }
//             is Union { json.append@string(to_json@[site](t.[field])) }
//             is Tuple { json.append@string([to_json@demo(t.[field])]) }
//             is Opaque { json.append@string(to_json@[site](t.[field])) }
//             is FnType { refuse!("Cannot encode function as JSON") }
//         }
//         [if !field.last] {
//             json.append@string(", ")
//         }
//     }
//     json.append@string("}")
//     return json
// }

// comptime fn to_json<T is Tuple>(t: T) -> Str {
//     let json = Mut Str("[")
//     [for index in T.indices] {
//         json.append@string(to_json@[site](t.[index]))
//         [if !index.last] {
//             json.append@string(", ")
//         }
//     }
//     json.append@string("]")
//     return json
// }

// fn to_json<T>(list: List<T>, ?to_json: (T) -> Str) -> Str {
//     return "[${list.map(to_json).join(",")}]"
// }

// fn to_json(str: Str) -> Str {
//     return "\"${str}\""
// }

// fn to_json(num: Int) -> Str {
//     return num.to_str()
// }

// fn to_json<T>(value: T?, ?to_json: (T) -> Str) -> Str {
//     return value?.to_json() ?: "null"
// }