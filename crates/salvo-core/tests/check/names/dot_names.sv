// [name-casing] [name-dot] Dot-named structs and qualifiers, the namespace
// constraints, and the collision ban that keeps Rust's flattened spelling
// unambiguous.

// [name-dot] A dot-name resolves as one dotted name, and the namespace
// struct in the same file satisfies the rule.
struct Environment {
    id: Environment.Id
}

struct Environment.Id {
    value: Str
}

qualifier Environment.Tag of Str

//--- the namespace must be a struct in the same file
// [name-dot] The namespace must be a struct declared in the same file.

// FILE: other.sv
export struct Environment {
    v: Str
}

// FILE: main.sv
//~ error[name-dot:not-a-namespace]
export struct Environment.Id {
    value: Str
}

//--- a type namespaces its members
// [name-dot] A `type` declaration namespaces its members too — an enum's
// arms live under the enum (`StorageClass.Standard`, user decision
// 2026-09-29) — and the members are reached through an import of the type,
// as a struct's are [name-dot-import].

// FILE: sc.sv
export type StorageClass = StorageClass.Standard | StorageClass.Unknown
export struct StorageClass.Standard {}
export struct StorageClass.Unknown { value: Str }

// FILE: main.sv
import sc.StorageClass

fn pick(u: Bool) [] -> StorageClass {
    if u {
        return StorageClass.Unknown { value: "x" }
    }
    return StorageClass.Standard {}
}

//--- a generic type or struct cannot namespace
// [name-dot] A generic `type` is refused as a generic struct is.
type Gen<T> = Gen.A | T
//~ error[name-dot:generic-namespace]
struct Gen.A {}

// [name-dot] A generic namespace cannot nest a member: Kotlin's nested class
// cannot reference the outer type parameters.
struct Pair<S, T> {
    first: S,
    second: T
}

//~ error[name-dot:generic-namespace]
struct Pair.Entry {
    value: Str
}

//--- the concatenated spelling collides
// [name-dot] Nothing visible may carry the concatenated spelling — Rust
// flattens dot-names and overload mangling uses the same form.
struct Environment {
    v: Str
}

//~ error[name-dot:flattened-collision]
struct Environment.Id {
    value: Str
}

struct EnvironmentId {
    value: Str
}

//--- the concatenated spelling collides with an import
// [name-dot] The ban reaches *imported* names too, which a file-scoped check
// would miss.

// FILE: other.sv
export struct EnvironmentId {
    value: Str
}

// FILE: main.sv
import other.EnvironmentId

struct Environment {
    v: Str
}

//~ error[name-dot:flattened-collision]
struct Environment.Id {
    value: Str
}

//--- importing a namespace brings its members
// [name-dot-import] Importing a namespace struct brings its dot-named
// members; importing a member directly works too.

// FILE: types.sv
export struct Environment {
    id: Environment.Id
}

export struct Environment.Id {
    value: Str
}

// FILE: main.sv
import types.Environment

fn read(id: Environment.Id) -> Str {
    return id.value
}

// FILE: member.sv
import types.Environment.Id

fn read(id: Environment.Id) -> Str {
    return id.value
}
