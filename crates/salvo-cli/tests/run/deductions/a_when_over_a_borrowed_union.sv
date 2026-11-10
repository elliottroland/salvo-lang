// [when-proj-subject] a `when` over a borrowed union narrows to borrowed arms
//
// `proj Json` is matched arm by arm, each arm a `proj Json.Str` whose fields
// read as borrows: on Rust the subject is a union of borrows (a kept
// parameter) or an `&UnionN` (a `for` element, an entry's value), and a
// borrowed union handed on to a `proj` parameter lends with `as_ref`. The
// elements are built with an explicit type argument, which each one widens
// to [call-type-args].

//--- the_json_printer
namespace Json {
    struct Object { value: Map<Str, Json> }
    struct Array { value: List<Json> }
    struct Str { value: Str }
    struct Number { value: Int }
    struct Bool { value: Bool }
    struct Null {}
}

type Json = Json.Object | Json.Array | Json.Str | Json.Number | Json.Bool | Json.Null

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

fn main() [use] {
    use StdConsole()
    let m = Mut Map<Str, Json>()
    m.put("a", Json.Str { value: "x" })
    m.put("b", Json.Number { value: 3 })
    m.put("c", Json.Array { value: List<Json>(Json.Bool { value: true }, Json.Null {}) })
    let j: Json = Json.Object { value: m }
    println(to_str(j))
}
//= stdout
//= {a: "x", b: 3, c: [true, null]}

//--- heads_bindings_and_guards
struct A { x: Int, name: Str }
struct B { s: Str }
struct C {}
type ABC = A | B | C

fn len(s: Str) [] -> Int => s { return s.size() }

fn label(v: proj ABC) [] -> Str {
    return when v {
        is A a { "A${a.x}" }
        is B | C { "other" }
    }
}

fn guard(v: proj ABC) [] -> Str {
    when v {
        is C { return "c" }
        is A | B { }
    }
    return when v {
        is A { "a${len(v.name)}" }
        is B { "b${len(v.s)}" }
    }
}

fn main() [use] {
    use StdConsole()
    let l = List<ABC>(A { x: 1, name: "one" }, B { s: "bee" }, C {})
    for v in l {
        let t = when v {
            is A { "A${v.x}" }
            is B {
                let s = v.s
                "B${s}${len(s)}"
            }
            is C { "C" }
        }
        println("${t} ${label(v)} ${guard(v)}")
    }
    let second = when e = get(l, 1) ?: unreachable!() {
        is B { e.s }
        is A | C { "?" }
    }
    println(second)
}
//= stdout
//= A1 A1 a3
//= Bbee3 other b3
//= C other c
//= bee

//--- a_borrowed_optional
struct A { x: Int, name: Str }

fn describe(a: proj A) [] -> Str { return "d${a.x}" }

fn main() [use] {
    use StdConsole()
    let opts = List<A?>(A { x: 7, name: "seven" }, None)
    for o in opts {
        let w = when o {
            is A { o.name }
            is None { "absent" }
        }
        println("${o?.x ?: -1} ${o?.name ?: "none"} ${o?.describe() ?: "nd"} ${w}")
    }
}
//= stdout
//= 7 seven d7 seven
//= -1 none nd absent

//--- a_borrow_of_a_union_is_a_union_of_borrows
// `proj (A | B | C) <: (proj A) | (proj B) | (proj C)`, a borrowed union
// narrowed by a guard passes on to a narrower `proj` union, Copy-scalar arms
// read as values, and a kept `proj (A?)` parameter takes `?.`.
struct A { x: Int }
struct B { s: Str }
struct C {}
type ABC = A | B | C

fn takes_int(n: Int) [] -> Int { return n + 1 }

fn spread(v: (proj A) | (proj B) | (proj C)) [] -> Str {
    return when v {
        is A { "A${v.x}" }
        is B { v.s }
        is C { "c" }
    }
}

fn opt_param(o: proj (A?)) [] -> Int {
    return o?.x ?: -1
}

fn pair(v: proj (A | B)) [] -> Str {
    return when v {
        is A { "pa" }
        is B { "pb" }
    }
}

fn narrowed_on(v: proj ABC) [] -> Str {
    when v {
        is C { return "nc" }
        is A | B { }
    }
    return pair(v)
}

fn main() [use] {
    use StdConsole()
    let l = List<ABC>(A { x: 1 }, B { s: "bee" }, C {})
    for v in l {
        println(spread(v))
        let w: (proj A) | (proj B) | (proj C) = v
        println(spread(w))
        println(narrowed_on(v))
    }
    let ns = List<Int | Str>(1, "two")
    for v in ns {
        let t = when v {
            is Int { "${takes_int(v)}" }
            is Str { "${v}" }
        }
        println(t)
    }
    let os = List<Int?>(4, None)
    let n = 0
    for o in os {
        n += o ?: 0
    }
    println("${n}")
    let ao = List<A?>(A { x: 3 }, None)
    for o in ao {
        println("${opt_param(o)}")
    }
}
//= stdout
//= A1
//= A1
//= pa
//= bee
//= bee
//= pb
//= c
//= c
//= nc
//= 2
//= two
//= 4
//= 3
//= -1
