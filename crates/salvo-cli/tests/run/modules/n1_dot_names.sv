// N1: dot-names [name-dot]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- dot_names_in_unions_and_narrowing

struct Environment.Id canbe Mut {
    value: Str
}

struct Environment.Name {
    value: Str
}

struct Environment {
    id: Environment.Id
}

qualifier Environment.Tag of Str

fn tag(v: Str) -> +Environment.Tag Str
=> !v {
    return v
}

fn pick(flag: Bool) -> Environment.Id | Environment.Name {
    if flag {
        return Environment.Id {value: "id"}
    }
    return Environment.Name {value: "name"}
}

fn show(x: Environment.Id | Environment.Name) -> Str {
    when x {
        is Environment.Id {
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

struct Environment.Id {
    value: Str
}

struct Environment.Name {
    value: Str
}

qualifier Environment.Tag of Str

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
