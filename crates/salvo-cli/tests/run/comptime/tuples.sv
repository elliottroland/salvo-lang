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

//--- comptime_at_a_generic_tuple
// [comptime-generic] [comptime-tuple] [fn-by] A declaration at a tuple of its
// own type parameters is stamped generically, with the implicits its copies
// call written in the declaration: an element of type `A` is printed by the
// `?to_json: (A) -> Str` the call fills. One declaration per arity. A
// concrete stamp beside them is the more specific overload, and a generic
// struct written at its own parameters is stamped the same way.
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
        // `Opaque` covers an element of a type parameter (kind `Generic`);
        // only a concrete stamp sees a `Str` element.
        [when index.type.kind] {
            is Opaque {
                [if index.type is Str] {
                    json.append@string("'${t.[index]}'")
                } else {
                    json.append@string(to_json@[site](t.[index]))
                }
            }
            is Struct { json.append@string(to_json@[site](t.[index])) }
            is Union { json.append@string("?") }
            is Tuple { json.append@string(to_json@[site](t.[index])) }
            is FnType { json.append@string("?") }
        }
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

fn to_json<T>(list: List<T>, ?to_json: (T) -> Str) -> Str {
    let out = Mut Str("<")
    for x in list {
        out.append(to_json(x))
    }
    out.append(">")
    return out
}

fn to_json<A, B>(t: (A, B), ?to_json: (A) -> Str, ?to_json: (B) -> Str) -> Str by to_json

fn to_json<A, B, C>(t: (A, B, C), ?to_json: (A) -> Str, ?to_json: (B) -> Str, ?to_json: (C) -> Str) -> Str by to_json

// A concrete stamp: chosen over the generic one for `(Str, Str)`.
fn to_json(t: (Str, Str)) -> Str by to_json

struct Wrapper<T> {
    value: T,
    n: Int
}

fn to_json<T>(t: Wrapper<T>, ?to_json: (T) -> Str) -> Str by to_json

// …and at a concrete instance, an ordinary stamp.
fn to_json(t: Wrapper<Str>) -> Str by to_json

// A generic fn forwarding its own implicit to the generic stamp.
fn tagged<X>(x: X, ?to_json: (X) -> Str) -> Str
=> !x {
    return to_json(("tag", x))
}

fn main() [use] -> None {
    use StdConsole()
    println(to_json(("a", 1)))
    println(to_json(("a", 1, "c")))
    println(to_json(("a", (1, "b"))))
    println(to_json((1, List(2, 3))))
    println(tagged(7))
    println(tagged((1, 2)))
    println(to_json(("x", "y")))
    println(to_json(Wrapper { value: ("w", 5), n: 2 }))
    println(to_json(Wrapper { value: "s", n: 3 }))
}
//= stdout
//= ["a", 1]
//= ["a", 1, "c"]
//= ["a", [1, "b"]]
//= [1, <23>]
//= ["tag", 7]
//= ["tag", [1, 2]]
//= ['x', 'y']
//= {"value": ["w", 5], "n": 2}
//= {"value": "s", "n": 3}

//--- the_kind_of_an_element_is_its_types
// [comptime-kind] [comptime-generic] The kind is the element type's,
// `index.type.kind`: at a stamp over the declaration's own type parameters
// an element of type `A` is `Generic`, an `Int` element `Basic`, and the
// halves of `Opaque` are told apart.
comptime fn kinds<T is Tuple>(t: T) -> Str {
    let out = Mut Str("")
    [for index in T.indices] {
        [if index.type.kind is Generic] {
            out.append@string("g")
        }
        [if index.type.kind is Basic] {
            out.append@string("b")
        }
        [if index.type.kind is Tuple] {
            out.append@string("t")
        }
    }
    return out
}

fn kinds<A>(t: (A, Int, (A, A))) -> Str by kinds

fn kinds(t: (Str, Int)) -> Str by kinds

fn main() [use] -> None {
    use StdConsole()
    println(kinds((true, 1, (false, true))))
    println(kinds(("a", 2)))
}
//= stdout
//= gbt
//= bb
