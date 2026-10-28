// ===== [comptime-inline] a binder's type spliced into a type position =====

// [comptime-inline] [comptime-fields] `[field.type]` stands for the field's
// type in every type position — a `let` annotation, a type argument of a
// call or of another type, under `?` and after a qualifier — substituted
// per copy (the CLI design record §13 (B), user decision 2026-10-20).
struct P { n: Int, s: Str }

fn probe<T>(x: T) [] -> Int => x {
    return 0
}

fn none<T>() [] -> T? {
    return None
}

comptime fn count(p: P) [] -> Int => p {
    let total = 0
    [for f in P.fields] {
        let v: [f.type] = p.[f]
        let _w = probe<[f.type]>(v)
        let _xs: List<[f.type]>? = None
        let _o: [f.type]? = none<[f.type]>()
        total = total + 1
    }
    return total
}

// Only the `Str` copy fails, so the type really is the field's (the error
// names the copy: "when `f` is `s`").
comptime fn count_one(p: P) [] -> Int => p {
    let total = 0
    [for f in P.fields] {
        //~ error[var-let-infer:annotation-mismatch]
        let v: [f.type] = 1
        let _w = probe<[f.type]>(v)
        let _xs: List<[f.type]>? = None
        let _o: [f.type]? = none<[f.type]>()
        total = total + 1
    }
    return total
}

// [comptime-inline] `[arm.type] { … }` is a struct literal whose head is the
// arm's type, per copy: over a union of structs each copy builds its own arm
// (the literal checked against that struct's fields). Building a qualified or
// literal arm is the CLI design record step 1.6. `B` also needs `m`, so only
// its copy fails.
struct A { n: Int }
struct B { n: Int, m: Int }
type U = A | B

fn keep(u: U) [] -> Int => u {
    return 0
}

comptime fn build() [] -> Int {
    let total = 0
    [for arm in U.arms] {
        //~ error[struct-defaults:missing-field]
        let v: [arm.type] = [arm.type] { n: arm.index }
        total = total + keep(v)
    }
    return total
}

// ===== [comptime-default] a field's declared default =====

// [comptime-default] `field.default` is the field's declared default where it
// has one and `None` where it has none, and `[if field.default is None]`
// tells the two apart: guarded, every copy checks; unguarded, only the copy
// for the field without a default fails, as `None` is not an `Int`.
struct D { a: Int = 3, b: Int, c: Int? = None }

comptime fn sum_guarded(p: D) [] -> Int => p {
    let total = 0
    [for f in D.fields] {
        [if f.default !is None && f.name !is "c"] {
            let d: [f.type] = f.default
            total = total + d
        }
    }
    return total
}

// A default passes to an optional either way.
comptime fn sum_optional(p: D) [] -> Int => p {
    let total = 0
    [for f in D.fields] {
        let d: Int? = f.default
        total = total + (d ?: 0)
    }
    return total
}

// Unguarded, the `b` and `c` copies fail.
comptime fn sum_unguarded(p: D) [] -> Int => p {
    let total = 0
    [for f in D.fields] {
        //~ error[var-let-infer:annotation-mismatch]
        //~ error[var-let-infer:annotation-mismatch]
        let d: Int = f.default
        total = total + d
    }
    return total
}

// [comptime-default] Only a field has a default: `.default` on an arm binder,
// as a value or in a head, is an error naming why.
struct A2 { n: Int }
struct B2 { n: Int }
type U2 = A2 | B2

comptime fn arm_default_value() [] -> Int {
    let total = 0
    [for arm in U2.arms] {
        //~ error[comptime-default:arm-default]
        let _d = arm.default
    }
    return total
}

comptime fn arm_default_head() [] -> Int {
    let total = 0
    [for arm in U2.arms] {
        //~ error[comptime-default:arm-default]
        [if arm.default is None] { total = total + 1 }
    }
    return total
}

// ===== [comptime-docs] doc comments as values =====

// [comptime-docs] A copy's `docs` is a `Str` literal or `None`, so it is
// tested in a head: `?:` on it is refused in each copy as on any value with
// one arm, which the error names ("when `f` is `a`").
struct Doc {
    // Doc.
    a: Int
}

comptime fn help() [] -> Str {
    let out: Mut Str = Mut Str("")
    [for f in Doc.fields] {
        //~ error[op-elvis:single-arm]
        append(out, f.docs ?: "-")
    }
    return out
}

// [node-origin] [comptime-inline] An unresolved name in a copy lands on the
// written code, saying which copy, like every other error in one: the
// unresolved-name funnel used to keep the copy's synthetic span, which
// printed past the end of the file with no prefix (found by the A2
// differential, 2026-10-27). (The name was `append`, which std declares.)
comptime fn help_unresolved() [] -> Str {
    let out: Mut Str = Mut Str("")
    [for f in Doc.fields] {
        //~ error[op-elvis:single-arm]
        //~ error[call-resolve:unknown-fn]
        append_text(out, f.docs ?: "-")
    }
    return out
}
