// [comptime-tuple] A comptime fn over a tuple type: `<T is Tuple>` stamped
// at written tuple types and at a `type` naming one, walking the elements
// with `[for index in T.indices]` and reading each with `t.[index]`; a
// struct template reaches it for a tuple field through `@[site]`.

//--- comptime_over_a_tuple
struct Person {
    name: Str,
    pair: (Str, Int)

    fn to_json(t: Person) -> Str by to_json
}

comptime fn to_json<T is Struct>(t: T) -> Str {
    let json = Mut Str("{")
    [for field in T.fields] {
        json.append@string("\"${field.name}\": ")
        json.append@string(to_json@[site](t.[field]))
        [if !field.last] {
            json.append@string(", ")
        }
    }
    json.append@string("}")
    return json
}

comptime fn to_json<T is Tuple>(t: T) -> Str {
    let json = Mut Str("[")
    [for index in T.indices] {
        let x: [index.type] = t.[index]
        json.append@string(to_json@[site](x))
        [if !index.last] {
            json.append@string(", ")
        }
    }
    json.append@string("]")
    return json
}

fn to_json(str: Str) -> Str {
    return "\"${str}\""
}

fn to_json(num: Int) -> Str {
    return num.to_str()
}

fn to_json(t: (Str, Int)) -> Str by to_json

type Triple = (Int, Str, Int)

fn to_json(t: Triple) -> Str by to_json

fn to_json(t: (Str, (Str, Int))) -> Str by to_json

fn main() [use] -> None {
    use StdConsole()
    let person = Person { name: "Roland", pair: ("age", 36) }
    println(person.to_json())
    let t: Triple = (1, "two", 3)
    println(to_json(t))
    println(to_json(("outer", ("inner", 2))))
}
//= stdout
//= {"name": "Roland", "pair": ["age", 36]}
//= [1, "two", 3]
//= ["outer", ["inner", 2]]
