// [op-elvis] [expr-placeholder] The optional-or-else operator `?:` and the
// placeholder `_`. `?:` is the **first-arm pick** (user decision 2026-10-14,
// simplifying the earlier positional-pick form that also had `??:`/`:?`): it
// keeps the subject union's first arm as the value; the right side runs
// otherwise, with `_` bound to the arms left behind. The right side is an
// ordinary expression, which since [expr-escape] includes
// `return`/`break`/`continue` — so `maybe_t() ?: return None` is the form the
// whole sequence was for.

// A reader of a plain `Str`: the probe the narrowing cases below pass `t` to.
fn takes_str(s: Str) [] -> None => s {}

fn parse(t: Str) [] -> Ok Int | Err Str
=> !t {
    return err(t)
}

// [op-elvis] The two shapes that matter: a right side that **escapes** (the
// result is just the picked type) and one that gives a **value** (the result
// is the join). `?:` is `Forward(1)`, so on a `Str?` it picks the `Str` arm
// and `_` is the `None` left behind.
fn nick(s: Str?) [] -> Str? => !s {
    let n: Str = s ?: return None
    return n
}

fn greet(s: Str?) [] -> Str {
    return s ?: "friend"
}

// [op-elvis] A pick on a one-arm subject leaves nothing for the right side —
// the same "dead scaffolding" refusal a throw-free `try` gets.
fn one_arm(s: Str) [] -> Str {
    //~ error[op-elvis:single-arm]
    return s ?: "other"
}

// [expr-placeholder] `_` is the arms a pick left behind, so it reads only in
// the right-hand side of a pick. Outside one it is an error, not a silent
// `Unknown`.
fn stray_placeholder() [] -> Str {
    //~ error[expr-placeholder:outside-pick]
    return _
}

// [op-elvis] [type-rewrap] Widening is now allowed (user decision
// 2026-10-11): a right side outside the subject's arms joins into the result.
// `Int? ?: "none"` is `Int | Str`.
fn widening(n: Int?) [] -> Str | Int {
    return n ?: "none"
}

// [op-elvis] `_` carries the arms left behind, tags and all, so the right
// side can route on them. A multi-arm `_` is a sub-union of the storage
// [type-rewrap].
fn routed(t: Str) [] -> Ok Int | Err Str
=> !t {
    let n: Ok Int = parse(t) ?: return _
    return n
}

// [op-elvis] A pick on a multi-arm subject keeps the **first** arm and leaves
// the rest to `_`. On `Ok Int | Ok Str | Err Str` it picks `Ok Int` and `_`
// is `Ok Str | Err Str`.
fn multi_arm(r: Ok Int | Ok Str | Err Str) [] -> Ok Int | Ok Str | Err Str {
    let kept: Ok Int = r ?: return _
    return kept
}

// [op-elvis-guard] A `?:` whose right side **leaves** has proved its subject
// was one of the picked arms, so the place narrows on the path below — the
// same facts an `is` guard leaves [is-narrow-guard], reached from an
// expression. Passing `t` where a `Str` is wanted is the probe: it refuses a
// possibly-`None` value, so it passes only if the narrowing landed.
fn guarding(t: Str?) [] -> None {
    let _v: Str = t ?: return
    takes_str(t)
}

// [op-elvis-guard] A right side that **yields a value** proves nothing: that
// path is the one where the subject was not picked, so the place keeps its
// type.
fn valued(t: Str?) [] -> None {
    let _v: Str = t ?: "other"
    //~ error[call-resolve:no-overload]
    takes_str(t)
}

// [op-elvis-guard] After a guarding pick the place reads as the picked arm,
// tag included.
fn guarding_pick(text: Str) [] -> None
=> !text {
    let r = parse(text)
    let _n: Ok Int = r ?: return
    let _again: Ok Int = r
}
