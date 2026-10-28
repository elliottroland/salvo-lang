// ===== [comptime-pattern] type patterns: `*` and declared binders =====
// Which copies a pattern keeps is comptime_tests.rs's; these are the
// patterns' refusals and warnings.

// A struct with fields of several shapes.
struct Arg<T> { default: T? }
struct Pair<A, B> { a: A, b: B }
struct S canbe Mut {
    ints: List<Int> canbe Mut,
    args: List<Arg<Bool>>,
    same: Pair<Int, Int>,
    mixed: Pair<Int, Str>,
    opt: Int?,
    either: Int | Str,
    name: Str
}

// [comptime-pattern] What a test proves holds only where the test did
// [flow-or-not]: under `||` or `!` a binder binds nothing, and declaring one
// there is an error.
comptime fn under_or(s: S) [] -> Int => s {
    [for f in S.fields] {
        //~ error[comptime-pattern:binder-under-or-not]
        [if f.type is <E> List<E> || f.first] {}
    }
    return 0
}

comptime fn under_not(s: S) [] -> Int => s {
    [for f in S.fields] {
        //~ error[comptime-pattern:binder-under-or-not]
        [if !(f.type is <E> List<E>)] {}
    }
    return 0
}

comptime fn under_not_is(s: S) [] -> Int => s {
    [for f in S.fields] {
        //~ error[comptime-pattern:binder-under-or-not]
        [if f.type !is <E> List<E>] {}
    }
    return 0
}

// [comptime-pattern] A binder may not hide a type, a type parameter or an
// enclosing binder…
comptime fn hides_a_type(s: S) [] -> Int => s {
    [for f in S.fields] {
        //~ error[comptime-pattern:binder-hides-name]
        [if f.type is <Int> List<Int>] {}
    }
    return 0
}

// …one no copy uses warns once, naming `*` as the remedy…
comptime fn unused_binder(s: S) [] -> Int => s {
    [for f in S.fields] {
        //~ warning[comptime-pattern:unused-binder]
        [if f.type is <E> List<E>] {}
    }
    return 0
}

// …and a used binder does not warn.
comptime fn used_binder(s: S) [] -> Int => s {
    [for f in S.fields] {
        [if f.type is <E> List<E>] { let _e: E? = None }
    }
    return 0
}

// [comptime-pattern] [when-exhaustive] A `[when field.type]` must cover the
// types its subject ranges over, with no `else` (`is *` is the catch-all) — known, since
// the walk is over one struct's fields or one union's arms (user question
// 2026-10-20). Each uncovered type is reported, naming it: here `Int?` and
// `Int | Str`.
comptime fn uncovered(s: S) [] -> Int => s {
    [for f in S.fields] {
        //~ error[comptime-pattern:no-arm-matches]
        //~ error[comptime-pattern:no-arm-matches]
        [when f.type] { is List<*> {} is Pair<*, *> {} is Str {} }
    }
    return 0
}

// A union's arms are a known range too: covered…
type U = Int | Str | List<Int>

comptime fn arms_covered(u: U) [] -> Int => u {
    [for a in U.arms] {
        [when a.type] { is Int {} is Str {} is List<*> {} }
    }
    return 0
}

// …and short of `List<Int>`.
comptime fn arms_short(u: U) [] -> Int => u {
    [for a in U.arms] {
        //~ error[comptime-pattern:no-arm-matches]
        [when a.type] { is Int {} is Str {} }
    }
    return 0
}
