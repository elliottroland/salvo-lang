// ===== [comptime-call] a comptime call: `[name(…)]` expands another template =====
// What the stamped copies print is the run data's
// (crates/salvo-cli/tests/run/comptime/calls.sv).

//--- the_accepted_forms
// A struct template reaching the tuple template at a field's type, the
// tuple one reaching it back for a struct element and itself for a nested
// tuple through a local that declares its type, and a union template passing
// its own parameter, which an arm narrows: no `fn … by` per tuple type.
struct Inner {
    n: Int
}

struct Outer {
    pair: (Str, Inner),
    deep: (Int, (Str, Int)),
    either: Shape
}

struct Circle {
    r: Int
}

struct Square {
    side: Int
}

type Shape = Circle | Square

comptime fn to_json<T is Struct>(t: T) [] -> Str {
    let out = Mut Str("{")
    [for field in T.fields] {
        [when field.type.kind] {
            is Struct { append@string(out, [to_json(t.[field])]) }
            is Tuple { append@string(out, [to_json(t.[field])]) }
            is Union { append@string(out, [to_json(t.[field])]) }
            is Opaque { append@string(out, to_json@[site](t.[field])) }
            is FnType { append@string(out, "?") }
        }
    }
    append@string(out, "}")
    return out
}

comptime fn to_json<T is Tuple>(t: T) [] -> Str {
    let out = Mut Str("[")
    [for index in T.indices] {
        [when index.type.kind] {
            is Tuple {
                let x: [index.type] = t.[index]
                append@string(out, [to_json(x)])
            }
            is Struct { append@string(out, [to_json(t.[index])]) }
            is Union { append@string(out, "?") }
            is Opaque { append@string(out, to_json@[site](t.[index])) }
            is FnType { append@string(out, "?") }
        }
    }
    append@string(out, "]")
    return out
}

comptime fn to_json<T is Union>(t: T) [] -> Str {
    [when t] {
        [arm] {
            return [to_json(t)]
        }
    }
}

fn to_json(s: Str) [] -> Str {
    return "'${s}'"
}

fn to_json(n: Int) [] -> Str {
    return "${n}"
}

fn to_json(t: Outer) [] -> Str by to_json

fn uses(o: Outer) [] -> Str {
    return to_json(o)
}

// The stamped fns are hidden: a tuple type has no `to_json` of its own.
fn plain() [] -> Str {
    //~ error[call-resolve:no-overload]
    return to_json(("a", Inner { n: 1 }))
}

//--- a_recursive_type_ends
// A union reaching itself through a struct arm: the stamp at `Tree` is named
// before its body is expanded, so the copy at `Branch` calls it and the
// expansion ends — and what is left to say is the checker's.
type Tree = Leaf | Branch

struct Leaf {
    value: Int
}

struct Branch {
    //~ error[type-no-cycle:contains-itself]
    left: Tree,
    right: Tree
}

comptime fn show<T is Struct>(t: T) [] -> Str {
    let out = Mut Str("")
    [for field in T.fields] {
        [when field.type.kind] {
            is Union { append@string(out, [show(t.[field])]) }
            is Opaque { append@string(out, show@[site](t.[field])) }
            is Struct { append@string(out, [show(t.[field])]) }
            is Tuple { append@string(out, "?") }
            is FnType { append@string(out, "?") }
        }
    }
    return out
}

comptime fn show<T is Union>(t: T) [] -> Str {
    [when t] {
        [arm] {
            return [show(t)]
        }
    }
}

fn show(n: Int) [] -> Str {
    return "${n}"
}

fn show(t: Tree) [] -> Str by show

//--- what_a_call_refuses
// Each refusal lands at the `by` site of the declaration being stamped, with
// the note naming the template's line.
struct Point {
    x: Int,
    pair: (Int, Int),
    shape: Shape
}

struct Circle {
    r: Int
}

type Shape = Circle | None

fn make() [] -> Point {
    return Point { x: 1, pair: (1, 2), shape: None }
}

// No template of that name at all; a basic type; no union template.
comptime fn names<T is Struct>(t: T) [] -> Str {
    [for field in T.fields] {
        return [nothing(t.[field])]
    }
    return ""
}

//~ error[comptime-call:names-nothing]
fn names(t: Point) [] -> Str by names

comptime fn plain_kind<T is Struct>(t: T) [] -> Str {
    [for field in T.fields] {
        [if field.name == "x"] {
            return [plain_kind(t.[field])]
        }
    }
    return ""
}

//~ error[comptime-call:not-stampable]
fn plain_kind(t: Point) [] -> Str by plain_kind

comptime fn kinds<T is Struct>(t: T) [] -> Str {
    [for field in T.fields] {
        [if field.name == "shape"] {
            return [kinds(t.[field])]
        }
    }
    return ""
}

//~ error[comptime-call:no-template]
fn kinds(t: Point) [] -> Str by kinds

// An argument whose type the expansion cannot tell; no argument.
comptime fn unknown<T is Struct>(t: T) [] -> Str {
    return [unknown(make@main())]
}

//~ error[comptime-call:unknown-type]
fn unknown(t: Point) [] -> Str by unknown

comptime fn empty<T is Struct>(t: T) [] -> Str {
    return [empty()]
}

//~ error[comptime-call:no-argument]
fn empty(t: Point) [] -> Str by empty

// An error in the template a call stamps is reported at the outer site.
comptime fn nested<T is Struct>(t: T) [] -> Str {
    [for field in T.fields] {
        [if field.name == "pair"] {
            return [nested(t.[field])]
        }
    }
    return ""
}

comptime fn nested<T is Tuple>(t: T) [] -> Str {
    [for index in T.indices] {
        return index.name
    }
    return ""
}

//~ error[comptime-tuple:unknown-property]
fn nested(t: Point) [] -> Str by nested

//--- the_bracketed_name_is_no_call
// A bracketed name and then arguments is a splice call, of a binder's local
// or a test's generated fn: `[old](…)` names no binder, and `[old@main](…)`
// could not, so neither stamps a template (the error says to write the
// whole call in the brackets).
struct Point {
    x: Int
}

comptime fn old<T is Struct>(t: T) [] -> Str {
    //~ error[comptime-access:not-a-binder]
    return [old@main](t)
}

comptime fn old_bare<T is Struct>(t: T) [] -> Str {
    return [old_bare](t)
}

//~ error[comptime-access:not-a-binder]
fn old_bare(t: Point) [] -> Str by old_bare

//--- a_generic_element
// A stamp at a tuple of the declaration's own type parameters cannot stamp
// at an element of type `A`: that is the implicit's to call.
comptime fn to_json<T is Tuple>(t: T) [] -> Str {
    let out = Mut Str("")
    [for index in T.indices] {
        append@string(out, [to_json(t.[index])])
    }
    return out
}

//~ error[comptime-call:generic-type]
fn to_json<A>(t: (A, A), ?to_json: (x: A) -> Str) [] -> Str by to_json

//--- two_modules_declare_it
// Two visible templates of the kind: the call says which with `@module`.
import one
import two

struct Holder {
    pair: (Int, Int)
}

comptime fn pick<T is Struct>(t: T) [] -> Str {
    let out = Mut Str("")
    [for field in T.fields] {
        append@string(out, [show(t.[field])])
    }
    return out
}

//~ error[comptime-call:ambiguous]
fn pick(t: Holder) [] -> Str by pick

comptime fn chosen<T is Struct>(t: T) [] -> Str {
    let out = Mut Str("")
    [for field in T.fields] {
        append@string(out, [show@two(t.[field])])
    }
    return out
}

fn chosen(t: Holder) [] -> Str by chosen

fn uses(h: Holder) [] -> Str {
    return chosen(h)
}
// FILE: one.sv
export comptime fn show<T is Tuple>(t: T) [] -> Str {
    return "one"
}
// FILE: two.sv
export comptime fn show<T is Tuple>(t: T) [] -> Str {
    return "two"
}
