// [when-exhaustive] A `[when]` with a subject follows an ordinary `when`'s
// rule: it covers its subject's range, so the range must be one the
// declaration can see. In a **stamped** comptime fn the fields of `T` are
// known only where it is stamped, so a `[when f.type]` or `[when f.name]`
// over them is refused at the template — once, with no echo at the `by` site
// — while `[when f.kind]` (over `Kind`, the same everywhere) stays, and is
// checked for every kind there too (user decisions 2026-10-20).
// FILE: tags.sv
export comptime fn tag1<T is Struct>(value: T) [] -> Int => value {
    [for f in T.fields] {
        //~ error[comptime-pattern:when-over-derived-type]
        [when f.type] { is Int {} is Str {} }
    }
    return 0
}

export comptime fn tag2<T is Struct>(value: T) [] -> Int => value {
    [for f in T.fields] {
        //~ error[comptime-pattern:when-over-derived-type]
        [when f.type] { is Int {} is * {} }
    }
    return 0
}

export comptime fn tag3<T is Struct>(value: T) [] -> Int => value {
    [for f in T.fields] {
        //~ error[comptime-pattern:when-over-derived-type]
        [if f.type is <E> List<E>] { [when E] { is Int {} is * {} } }
    }
    return 0
}

export comptime fn tag4<T is Struct>(value: T) [] -> Int => value {
    [for f in T.fields] {
        //~ error[comptime-name:when-over-derived-name]
        [when f.name] { is "x" {} is "s" {} }
    }
    return 0
}

export comptime fn tag5<T is Struct>(value: T) [] -> Int => value {
    [for f in T.fields] {
        //~ error[when-exhaustive:kind-arms-missing]
        [when f.kind] { is Struct {} }
    }
    return 0
}

// A kind dispatch is checkable anywhere.
export comptime fn tag6<T is Struct>(value: T) [] -> Int => value {
    [for f in T.fields] {
        [when f.kind] { is Struct {} is Union {} is Tuple {} is FnType {} is Opaque {} }
    }
    return 0
}
// FILE: main.sv
import tags

struct P { x: Int, s: Str }

fn tag1(p: P) [] -> Int => p by tags
fn tag2(p: P) [] -> Int => p by tags
fn tag3(p: P) [] -> Int => p by tags
fn tag4(p: P) [] -> Int => p by tags
fn tag5(p: P) [] -> Int => p by tags
fn tag6(p: P) [] -> Int => p by tags
