// ===== [comptime-name] `field.name` is a literal union =====
// Which copies a name test keeps is comptime_tests.rs's (it reads the
// `refuse!` texts); these are the tests' refusals and the clean programs.

struct P { a: Int, b: Int, c: Int, d: Str }

// [comptime-name] `field.name` is a literal of the union of the struct's
// field names, so `is` / `!is` test it against one literal or a union of
// them (user decision 2026-10-20); a literal outside the union is refused.
comptime fn outside_the_union(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[comptime-name:unknown-name]
        [if f.name is "a" | "e"] {
            refuse!("${f.name}")
        }
    }
    return 0
}

// [comptime-name] `[when field.name]` dispatches on the name and must cover
// every field — it takes no `else`, as no `when` with a subject does: a field
// added later is an error in every `[when field.name]` that did not consider
// it. A missing field is named…
comptime fn missing_a_field(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[comptime-name:missing-names]
        [when f.name] {
            is "a" | "b" {} is "c" {}
        }
    }
    return 0
}

// …and a name covered twice, or one the struct does not have, is an error.
comptime fn covered_twice(p: P) [] -> Int => p {
    [for f in P.fields] {
        [when f.name] {
            //~ error[comptime-name:covered-twice]
            is "a" | "b" {} is "b" {} is "c" | "d" {}
        }
    }
    return 0
}

comptime fn not_a_field(p: P) [] -> Int => p {
    [for f in P.fields] {
        [when f.name] {
            //~ error[comptime-name:unknown-name]
            is "a" | "z" {} is "b" | "c" | "d" {}
        }
    }
    return 0
}

// [comptime-name] An arm binder's name is the literal union of the union's
// arm names, with the same tests and the same exhaustiveness.
struct A { n: Int }
struct B { s: Str }
type AB = A | B

comptime fn arm_names() [] -> Int {
    [for arm in AB.arms] {
        [when arm.name] {
            is "A" {} is "B" {}
        }
    }
    return 0
}

comptime fn arm_names_missing() [] -> Int {
    [for arm in AB.arms] {
        //~ error[comptime-name:missing-names]
        [when arm.name] {
            is "A" {}
        }
    }
    return 0
}

// [comptime-name] `[if a.name == b.name]` relates binders of two walks
// (the CLI design record X16); a side that is not a binder is an error.
struct Ports { port: Int }

comptime fn not_a_binder() [] -> Int {
    [for a in Ports.fields] {
        //~ error[comptime-name:non-binder-compare]
        [if a.name == z.name] { }
    }
    return 0
}
