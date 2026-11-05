// N1: namespaced names [name-dot] [mod-namespace]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- dot_names_in_unions_and_narrowing

namespace Environment {
    struct Id canbe Mut {
        value: Str
    }

    struct Name {
        value: Str
    }

    qualifier Tag for Str
}

struct Environment {
    id: Environment.Id
}

fn tag(v: Str) -> +Environment.Tag Str
=> !v {
    return v
}

// [mod-namespace] The leading-dot shorthand names an arm by its last
// segment where the expected type is in view.
fn pick(flag: Bool) -> Environment.Id | Environment.Name {
    if flag {
        return .Id {value: "id"}
    }
    return Environment.Name {value: "name"}
}

fn show(x: Environment.Id | Environment.Name) -> Str {
    when x {
        is .Id {
            return "id: ${x.value}"
        }
        is Environment.Name {
            return "name: ${x.value}"
        }
    }
}

fn maybe(t: Environment.Tag Str | None) -> Str {
    if t is Environment.Tag {
        return "tagged ${t}"
    }
    return "none"
}

fn main() [use] -> None {
    use StdConsole()
    println(show(pick(true)))
    println(show(pick(false)))
    let m = Mut Environment.Id {value: "before"}
    m.value = "after"
    println(m.value)
    println(maybe(tag("x")))
    println(maybe(None))
}
//= stdout
//= id: id
//= name: name
//= after
//= tagged x
//= none

//--- dot_names_flatten

struct Environment {
    id: Environment.Id,
    name: Environment.Name
}

namespace Environment {
    struct Id {
        value: Str
    }

    struct Name {
        value: Str
    }

    qualifier Tag for Str
}

fn tag(value: Str) -> +Environment.Tag Str
=> !value {
    return value
}

fn label(t: Str) -> Str {
    return "plain ${t}"
}

fn label(t: Environment.Tag Str) -> Str {
    return "tagged ${t}"
}

fn main() [use] -> None {
    use StdConsole()
    let env = Environment {
        id: Environment.Id {value: "prod"},
        name: Environment.Name {value: "Production"}
    }
    println("${env.id.value} / ${env.name.value}")
    println(label(tag("t1")))
    println(label("t2"))
}
//= stdout
//= prod / Production
//= tagged t1
//= plain t2

//--- namespaces_nest_flatten_and_resolve

namespace Attr {
    struct S { value: Str }
    struct N { value: Str }
    fn tag(v: Str) -> Str => !v {
        return "tag:${v}"
    }
}

namespace Outer {
    namespace Inner {
        struct Thing { value: Str }
    }
}

type AttrValue = Attr.S | Attr.N

fn make_s(v: Str) -> AttrValue => !v {
    return .S { value: v }
}

fn describe(v: AttrValue) -> Str {
    when v {
        is .S { return "S ${v.value}" }
        is .N { return "N ${v.value}" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(describe(make_s("x")))
    println(describe(Attr.N { value: "y" }))
    println(Attr.tag("z"))
    let t = Outer.Inner.Thing { value: "deep" }
    println(t.value)
}
//= stdout
//= S x
//= N y
//= tag:z
//= deep

//--- a_generic_struct_beside_a_namespace
// [name-dot] [kt-nested-dot-name] A generic struct may share its name with a
// namespace: Kotlin nests the members as plain nested classes, which never
// see the struct's type parameters; Rust concatenates.

struct Pair<S, T> {
    first: S,
    second: T
}

namespace Pair {
    struct Label {
        text: Str
    }
}

fn describe(p: Pair<Int, Str>, l: Pair.Label) -> Str {
    return "${l.text}: ${p.first} ${p.second}"
}

fn main() [use] -> None {
    use StdConsole()
    println(describe(Pair { first: 1, second: "one" }, Pair.Label { text: "pair" }))
}
//= stdout
//= pair: 1 one
