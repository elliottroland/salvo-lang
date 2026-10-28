// [mod-namespace] `namespace X { … }` blocks: flattening to dotted names,
// nesting, sharing a name with a type, the leading-dot shorthand, and
// visibility.

// [mod-namespace] A `namespace X { … }` flattens to dotted names, and a
// member referenced as `X.Thing` resolves — the namespace need not also be a
// struct.
namespace Plain {
    struct S { value: Str }
    qualifier Tag of Str
}

fn pick_plain() [] -> Plain.S {
    return Plain.S { value: "x" }
}

// [mod-namespace] Namespaces nest to any depth; a member of a nested
// namespace resolves by its full dotted path.
namespace Outer {
    namespace Inner {
        struct Thing { value: Str }
    }
}

fn make() [] -> Outer.Inner.Thing {
    return Outer.Inner.Thing { value: "x" }
}

// [mod-namespace] A namespace may share its name with a type at the same
// level (the `AttributeValue` case): the union alias and the namespace
// coexist.
namespace Attr {
    struct S { value: Str }
    struct N { value: Str }
}

type Attr = Attr.S | Attr.N

fn pick() [] -> Attr {
    return Attr.S { value: "x" }
}

// [mod-namespace] The leading-dot shorthand also resolves inside a `type`
// alias's own union-arm list, against the alias's own enclosing namespace
// (user decision 2026-10-19) — the one context with no outer expected type
// to derive a namespace from, unlike every other dot-shorthand position (an
// `is`/`when` arm, a typed value, a call argument, a return).
// `namespace Arg { type Type = .Int | .Str }` means `.Int` -> `Arg.Int`,
// `.Str` -> `Arg.Str`: the alias's own name (`Arg.Type`) names the namespace
// the bare arms are siblings of.
namespace Arg {
    struct Int {}
    struct Str {}
    struct Bool {}
    type Type = .Int | .Str | .Bool
}

fn describe(t: Arg.Type) -> Str {
    when t {
        is .Int { return "int" }
        is .Str { return "str" }
        is .Bool { return "bool" }
    }
}

fn make_int() -> Arg.Type {
    return Arg.Int {}
}

//--- an exported namespace's members import
// [mod-namespace] `export namespace` exports every member; a plain
// `namespace` keeps them private, so importing one across modules is
// refused.

// FILE: lib.sv
export namespace Attr {
    struct S { value: Str }
}

// FILE: main.sv
import lib.Attr

fn pick() [] -> Attr.S {
    return Attr.S { value: "x" }
}

//--- a private namespace's members do not
// FILE: lib.sv
namespace Attr {
    struct S { value: Str }
}

// FILE: main.sv
//~ warning[mod-namespace:exports-nothing]
import lib.Attr

//~ error[name-resolve:unknown-type]
fn pick() [] -> Attr.S {
    //~ error[name-resolve:unknown-type]
    return Attr.S { value: "x" }
}

//--- an import inside a namespace
// [mod-namespace] A declaration that is not a named declaration — an import —
// cannot appear inside a namespace (reported when the namespace is
// flattened, at parse time).
namespace X {
    //~ error[mod-namespace:not-a-declaration]
    import std.core.Str
}
