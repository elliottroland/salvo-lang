// ===== [comptime-scope] a comptime body is a scope =====

// Expanded-in-place fns over one struct, each declaring a local `total`
// before its comptime body.
struct P { a: Int, b: Str }

// [comptime-scope] What a kept `[if]` branch, its `else`, or a chosen
// `[when]` arm declares is not visible after it — the body was spliced flat
// before, and `return k` compiled (user request 2026-10-20). (`_total`: it
// is only written, here.)
comptime fn leak_if(p: P) [] -> Int => p {
    let _total: Int = 0
    [if P.kind is Struct] {
        let k = 1
        _total = k
    }
    //~ error[name-ident-resolve:unknown]
    return k
}

comptime fn leak_else(p: P) [] -> Int => p {
    let _total: Int = 0
    [if P.kind is Union] {} else {
        let k = 1
        _total = k
    }
    //~ error[name-ident-resolve:unknown]
    return k
}

comptime fn leak_when(p: P) [] -> Int => p {
    let _total: Int = 0
    [when P.kind] {
        is Struct {
            let k = 1
            _total = k
        }
        is Union {} is Tuple {} is FnType {} is Opaque {}
    }
    //~ error[name-ident-resolve:unknown]
    return k
}

comptime fn leak_for(p: P) [] -> Int => p {
    let _total: Int = 0
    [for f in P.fields] {
        [if f.first] {
            let k = 1
            _total = k
        }
    }
    //~ error[name-ident-resolve:unknown]
    return k
}

// Assigning to a local declared outside is unaffected.
comptime fn kept_if(p: P) [] -> Int => p {
    let total: Int = 0
    [if P.kind is Struct] {
        let k = 1
        total = k
    }
    return total
}

comptime fn kept_else(p: P) [] -> Int => p {
    let total: Int = 0
    [if P.kind is Union] {} else {
        let k = 1
        total = k
    }
    return total
}

comptime fn kept_when(p: P) [] -> Int => p {
    let total: Int = 0
    [when P.kind] {
        is Struct {
            let k = 1
            total = k
        }
        is Union {} is Tuple {} is FnType {} is Opaque {}
    }
    return total
}

comptime fn kept_for(p: P) [] -> Int => p {
    let total: Int = 0
    [for f in P.fields] {
        [if f.first] {
            let k = 1
            total = k
        }
    }
    return total
}

// [comptime-scope] [var-no-shadow] The rename that keeps a body's locals in
// does not let them hide a visible one: refused by the name as written, also
// in a `[for]` copy (which renamed, and so allowed it, before), and through
// both renamings when an `[if]` sits in a copy.
comptime fn shadow_if(p: P) [] -> Int => p {
    let total: Int = 0
    //~ warning[var-unused:never-used]
    //~ error[var-no-shadow:already-declared]
    [if P.kind is Struct] { let total = 1 }
    return total
}

comptime fn shadow_for(p: P) [] -> Int => p {
    let total: Int = 0
    //~ warning[var-unused:never-used]
    //~ error[var-no-shadow:already-declared]
    //~ error[var-no-shadow:already-declared]
    [for f in P.fields] { let total = 1 }
    return total
}

comptime fn shadow_for_if(p: P) [] -> Int => p {
    let total: Int = 0
    [for f in P.fields] {
        //~ warning[var-unused:never-used]
        let k = 1
        //~ warning[var-unused:never-used]
        //~ error[var-no-shadow:already-declared]
        [if f.first] { let k = 2 }
    }
    return total
}

// Sibling copies and sibling branches each declare their own.
comptime fn siblings(p: P) [] -> Int => p {
    let total: Int = 0
    [for f in P.fields] {
        [if f.first] {
            let k = 1
            total = k
        } else {
            let k = 2
            total = k
        }
    }
    return total
}

// [comptime-scope] [expr-terminal-scope] A terminal `[if …] ...` renames the rest
// of the block, which holds every use, so it reads as before…
comptime fn terminal(p: P) [] -> Int => p {
    let total: Int = 0
    [if P.kind is Struct] ...
    let k = total
    return k
}

// …and a renamed local is named as written in a warning.
comptime fn renamed_unused(p: P) [] -> Int => p {
    let total: Int = 0
    //~ warning[var-unused:never-used]
    [if P.kind is Struct] { let unused = 1 }
    return total
}

// ===== [comptime-copy-warn] warnings raised inside copies =====

// [comptime-copy-warn] A warning inside a copy of an expanded-in-place fn
// lands on the written code (it kept the copy's synthetic span, past the end
// of the file, before 2026-10-20) and is reported once: an unused local only
// when no copy used it (`z`, not `y`), and any other warning once for every
// copy that raises it alike.
struct S { a: Int, b: Str }
comptime fn copies(s: S) [] -> Int => s {
    let total: Int = 0
    [for f in S.fields] {
        //~ warning[expr-dead-code:unreachable]
        let y = 1
        [if f.type is Int] { total = y }
        //~ warning[var-unused:never-used]
        let z = 2
        //~ warning[assert-fn:todo]
        [if f.first] { todo!() }
    }
    return total
}
