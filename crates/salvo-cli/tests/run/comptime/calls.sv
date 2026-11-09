// [comptime-call] A comptime call `[name(…)]` in a template stamps another
// template at the argument's type, as a hidden fn, and calls it: no `fn … by`
// per tuple type.

//--- a_struct_with_tuple_fields_as_json
// The user's `to_json`: a struct template reaching the tuple template for a
// tuple field and itself for a struct field; the tuple template reaching
// itself for a nested tuple through a local that declares its type.
struct Person {
    name: Str,
    pair: (Str, Int),
    nested: (Int, (Str, Int)),
    home: Address

    fn to_json(t: Person) -> Str by to_json
}

struct Address {
    city: Str,
    zip: (Int, Int)
}

comptime fn to_json<T is Struct>(t: T) -> Str {
    let json = Mut Str("{")
    [for field in T.fields] {
        json.append@string("\"${field.name}\": ")
        [when field.kind] {
            is Struct { json.append@string([to_json(t.[field])]) }
            is Tuple { json.append@string([to_json(t.[field])]) }
            is Opaque { json.append@string(to_json@[site](t.[field])) }
            is Union { json.append@string("?") }
            is FnType { json.append@string("?") }
        }
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
        [when index.kind] {
            is Tuple {
                let x: [index.type] = t.[index]
                json.append@string([to_json(x)])
            }
            is Opaque { json.append@string(to_json@[site](t.[index])) }
            is Struct { json.append@string([to_json(t.[index])]) }
            is Union { json.append@string("?") }
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

fn main() [use] -> None {
    use StdConsole()
    let person = Person { name: "Roland", pair: ("age", 36), nested: (1, ("two", 3)), home: Address { city: "Bonn", zip: (53, 111) } }
    println(person.to_json())
}
//= stdout
//= {"name": "Roland", "pair": ["age", 36], "nested": [1, ["two", 3]], "home": {"city": "Bonn", "zip": [53, 111]}}

//--- a_union_reaching_itself
// A union reaching itself through a struct arm (`Branch`'s `kids`): the
// union template passes its parameter, which the `[when t]` arm narrows, to
// the struct template, whose list field is an ordinary call back to the
// declared `to_json(Tree)`.
type Tree = Leaf | Branch

struct Leaf {
    value: Int
}

struct Branch {
    label: Str,
    kids: List<Tree>
}

struct Doc {
    title: Str,
    root: Tree

    fn to_json(t: Doc) -> Str by to_json
}

fn to_json(t: Tree) -> Str by to_json

comptime fn to_json<T is Struct>(t: T) -> Str {
    let json = Mut Str("{")
    [for field in T.fields] {
        json.append@string("\"${field.name}\": ")
        [when field.kind] {
            is Union { json.append@string([to_json(t.[field])]) }
            is Opaque { json.append@string(to_json@[site](t.[field])) }
            is Struct { json.append@string([to_json(t.[field])]) }
            is Tuple { json.append@string([to_json(t.[field])]) }
            is FnType { json.append@string("?") }
        }
        [if !field.last] {
            json.append@string(", ")
        }
    }
    json.append@string("}")
    return json
}

comptime fn to_json<T is Union>(t: T) -> Str {
    [when t] {
        [arm] {
            return [to_json(t)]
        }
    }
}

fn to_json(num: Int) -> Str {
    return num.to_str()
}

fn to_json(str: Str) -> Str {
    return "\"${str}\""
}

fn to_json<T>(xs: List<T>, ?to_json: (x: T) -> Str) -> Str {
    let out = Mut Str("[")
    let i = 0
    for x in xs {
        if i > 0 {
            append(out, ", ")
        }
        append(out, to_json(x))
        i = i + 1
    }
    append(out, "]")
    return out
}

fn main() [use] -> None {
    use StdConsole()
    let one: Tree = Leaf { value: 1 }
    let two: Tree = Leaf { value: 2 }
    let inner: Tree = Branch { label: "inner", kids: List(two) }
    let tree: Tree = Branch { label: "top", kids: List(one, inner) }
    println(Doc { title: "t", root: tree }.to_json())
}
//= stdout
//= {"title": "t", "root": {"label": "top", "kids": [{"value": 1}, {"label": "inner", "kids": [{"value": 2}]}]}}

//--- site_means_the_outer_stamp_site
// The nested template lives in another module than the outer one, and the
// `to_json` overloads only where the `by` is written: `to_json@[site]` in the
// nested template is that site's, however deep the nesting.
import json

struct Point {
    name: Str,
    pos: (Int, Str)

    fn to_json(p: Point) -> Str by to_json@json
}

fn to_json(str: Str) -> Str {
    return "'${str}'"
}

fn to_json(num: Int) -> Str {
    return "#${num}"
}

fn main() [use] -> None {
    use StdConsole()
    println(Point { name: "p", pos: (3, "x") }.to_json())
}
// FILE: json.sv
import tuples

export comptime fn to_json<T is Struct>(p: T) -> Str {
    let json = Mut Str("{")
    [for field in T.fields] {
        json.append@string("\"${field.name}\": ")
        [when field.kind] {
            is Tuple { json.append@string([to_json@tuples(p.[field])]) }
            is Opaque { json.append@string(to_json@[site](p.[field])) }
            is Struct { json.append@string("?") }
            is Union { json.append@string("?") }
            is FnType { json.append@string("?") }
        }
        [if !field.last] {
            json.append@string(", ")
        }
    }
    json.append@string("}")
    return json
}
// FILE: tuples.sv
export comptime fn to_json<T is Tuple>(t: T) -> Str {
    let json = Mut Str("[")
    [for index in T.indices] {
        json.append@string(to_json@[site](t.[index]))
        [if !index.last] {
            json.append@string(", ")
        }
    }
    json.append@string("]")
    return json
}
//= stdout
//= {"name": 'p', "pos": [#3, 'x']}
