// [comptime-bound] [comptime-inline] Comptime fns beyond `by` stamping: the
// expanded-in-place form called from ordinary code, what its conditions can
// say, and how it reports (the 2026-10-20 comptime expansion; COMPLETED.md's
// "Design record: comptime expansion"). Which copies a condition keeps is
// read from the `refuse!` texts, so those cases stay in comptime_tests.rs.

// ===== [comptime-bound] an expanded-in-place comptime fn is an ordinary fn =====

// [comptime-bound] A comptime fn with no kind bound is expanded once where it
// is declared, and is then an ordinary fn: ordinary code calls it (user
// decision 2026-10-20 — this had worked by construction, undecided and
// untested).
struct P1 { a: Int, b: Int, c: Int }

comptime fn total(p: P1) [] -> Int => p {
    let n = 0
    [for field in P1.fields] {
        n = n + p.[field]
    }
    return n
}

fn use_it(p: P1) [] -> Int => p {
    return total(p) + total(p)
}

// [comptime-bound] [comptime-inline] The expanded body is checked like a
// hand-written one: a copy that does not type-check is an error **where the
// failing code is written**, saying in the code's own terms which copy failed
// — "when `field` is `b` (`Str`): …" (2026-10-20, two TODOs in
// lib/std/args/parse.sv: it landed on the field's declaration, away from the code
// to fix, and its stamp-style prefix read as jargon).
struct P2 {
    a: Int,
    b: Str
}

comptime fn total2(p: P2) [] -> Int => p {
    let n = 0
    [for field in P2.fields] {
        //~ error[call-resolve:no-overload]
        n = n + p.[field]
    }
    return n
}

// ===== [comptime-inline] diagnostics land where they are written =====

// [comptime-inline] A mistake in a condition is reported once, at the
// condition — not once per unrolled copy, at the `comptime` keyword.
comptime fn once_at_the_condition(p: P1) [] -> Int => p {
    let n = 0
    [for field in P1.fields] {
        //~ error[comptime-name:unknown-name]
        [if field.name == "zz"] {
            n = n + 1
        }
    }
    return n
}

// [comptime-refuse] A `refuse!` in an expanded-in-place fn names the fn (there
// is no stamped type) and lands at the `refuse!`.
struct P3 { a: Int, b: Int }

comptime fn f(p: P3) [] -> Int => p {
    [for field in P3.fields] {
        [if field.name == "b"] {
            //~ error[comptime-refuse:refused]
            refuse!("`b` is not handled yet")
        }
    }
    return 0
}

// [comptime-logic] Every leaf is validated, including one a short circuit
// would skip: a misspelled field name is a mistake wherever it stands.
struct P4 { a: Int, b: Int, c: Int, d: Str }

comptime fn short_circuit(p: P4) [] -> Int => p {
    [for f in P4.fields] {
        //~ error[comptime-name:unknown-name]
        [if f.first || f.name == "zz"] {
            //~ error[comptime-refuse:refused]
            refuse!("${f.name}")
        }
    }
    return 0
}

// [when-exhaustive] [comptime-kind] An incomplete `[when f.kind]` in an
// expanded-in-place fn is reported once, at the line it is written on — it
// was pushed with the copy's virtual span, past the end of the file, once per
// field (found 2026-10-20 with the arm completions).
struct P5 { a: Int, b: Str }

comptime fn kinds(p: P5) [] -> Int => p {
    [for f in P5.fields] {
        //~ error[when-exhaustive:kind-arms-missing]
        [when f.kind] {
            is Struct {}
        }
    }
    return 0
}
