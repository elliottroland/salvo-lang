// [name-casing] [name-dot] Namespaced structs and qualifiers, and the
// collision ban that keeps Rust's flattened spelling unambiguous.

// [name-dot] [mod-namespace] A namespaced name resolves as one dotted name,
// and the namespace may share its name with a struct beside it.
struct Environment {
    id: Environment.Id
}

namespace Environment {
    struct Id {
        value: Str
    }

    qualifier Tag for Str
}

//--- a type and its arms share a name
// [name-dot] [mod-namespace] A `type` and a namespace of the same name: an
// enum's arms live under the enum (`StorageClass.Standard`), and the
// members are reached through an import of the name [name-dot-import].

// FILE: sc.sv
export type StorageClass = StorageClass.Standard | StorageClass.Unknown

export namespace StorageClass {
    struct Standard {}
    struct Unknown { value: Str }
}

// FILE: main.sv
import sc.StorageClass

fn pick(u: Bool) [] -> StorageClass {
    if u {
        return StorageClass.Unknown { value: "x" }
    }
    return StorageClass.Standard {}
}

//--- a generic struct may share its name with a namespace
// [name-dot] [mod-namespace] The namespace is a separate facet, so the struct
// beside it may be generic: the members cannot see its type parameters.
struct Pair<S, T> {
    first: S,
    second: T
}

namespace Pair {
    struct Entry {
        value: Str
    }
}

fn entry() [] -> Pair.Entry {
    return Pair.Entry { value: "x" }
}

//--- the concatenated spelling collides
// [name-dot] Nothing visible may carry the concatenated spelling — Rust
// flattens namespaced names and overload mangling uses the same form.
namespace Environment {
    //~ error[name-dot:flattened-collision]
    struct Id {
        value: Str
    }
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

namespace Environment {
    //~ error[name-dot:flattened-collision]
    struct Id {
        value: Str
    }
}

//--- importing a namespace brings its members
// [name-dot-import] Importing a namespace brings its members, and the struct
// of the same name with them; importing a member directly works too.

// FILE: types.sv
export struct Environment {
    id: Environment.Id
}

export namespace Environment {
    struct Id {
        value: Str
    }
}

// FILE: main.sv
import types.Environment

fn read(e: Environment) -> Str {
    return e.id.value
}

// FILE: member.sv
import types.Environment.Id

fn read(id: Environment.Id) -> Str {
    return id.value
}
