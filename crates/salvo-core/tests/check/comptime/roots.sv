// ===== [comptime-root-visible] a root is a type name written here =====

// [comptime-root-visible] `Pt.fields` names `Pt`, so `Pt` must be visible
// where it is written, like any type name (user decision 2026-10-20). The
// expansion's lookup is program-wide; before this a struct from a module the
// file never imported was walked silently.
// FILE: geo.sv
export struct Pt {
    x: Int,
    y: Int
}
// FILE: main.sv
comptime fn count() [] -> Int {
    let n = 0
    //~ error[name-resolve:unknown-type]
    [for field in Pt.fields] {
        n = n + 1
    }
    return n
}

//--- imported, the root is visible
// FILE: geo.sv
export struct Pt {
    x: Int,
    y: Int
}
// FILE: main.sv
import geo

comptime fn count() [] -> Int {
    let n = 0
    [for field in Pt.fields] {
        n = n + 1
    }
    return n
}

//--- the root means the imported declaration
// [comptime-root-visible] Of two same-named structs, the root means the one
// the file imports: `z` exists on `b`'s `Pt`.
// FILE: a.sv
export struct Pt {
    x: Int
}
// FILE: b.sv
export struct Pt {
    x: Int,
    y: Int,
    z: Int
}
// FILE: main.sv
import b

comptime fn count() [] -> Int {
    let n = 0
    [for field in Pt.fields] {
        [if field.name == "z"] {
            n = n + 100
        }
    }
    return n
}
