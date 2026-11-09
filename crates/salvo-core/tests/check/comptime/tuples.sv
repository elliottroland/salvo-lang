// ===== [comptime-tuple] a comptime fn over a tuple type =====
// What the stamped copies print is the run data's
// (crates/salvo-cli/tests/run/comptime/tuples.sv).

fn show(s: Str) [] -> Str {
    return "'${s}'"
}

fn show(n: Int) [] -> Str {
    return "${n}"
}

// [comptime-tuple] `<T is Tuple>` is a kind bound; `[for index in
// T.indices]` unrolls once per element, `t.[index]` reads it, and
// `[index.type]` is its type. A written tuple type is the target.
comptime fn show<T is Tuple>(t: T) [] -> Str => t {
    let out: Mut Str = Mut Str("(")
    [for index in T.indices] {
        [if !index.first] {
            append@string(out, ", ")
        }
        let x: [index.type] = t.[index]
        [when index.kind] {
            is Struct { append@string(out, "?") }
            is Union { append@string(out, "?") }
            is Tuple { append@string(out, show@[site](x)) }
            is FnType { append@string(out, "?") }
            is Opaque { append@string(out, show@[site](x)) }
        }
        [if index.index == 1 && index.type is Int] {
            append@string(out, "!")
        }
    }
    append@string(out, ")")
    return out
}

fn show(t: (Str, Int)) [] -> Str by show

// …a `type` naming a tuple is one too, and a nested tuple reaches the
// site's `show` for its element.
type Pair = (Int, Int)

fn show(t: Pair) [] -> Str by show

fn show(t: (Str, (Str, Int))) [] -> Str by show

fn uses() [] -> Str {
    return "${show(("a", 1))} ${show((1, 2))} ${show(("b", ("c", 3)))}"
}

// [comptime-tuple] An `Index` has no name, docs or default, carries no
// comptime qualifier, and names no field in a struct literal. The errors of
// a stamp land at its `by` site.
comptime fn index_name<T is Tuple>(t: T) [] -> Str => t {
    [for index in T.indices] {
        return index.name
    }
    return ""
}

//~ error[comptime-tuple:unknown-property]
fn index_name(t: (Str, Int)) [] -> Str by index_name

comptime fn index_head_name<T is Tuple>(t: T) [] -> Str => t {
    [for index in T.indices] {
        [if index.name == "0"] {
            return "zero"
        }
    }
    return ""
}

//~ error[comptime-tuple:unknown-property]
fn index_head_name(t: (Str, Int)) [] -> Str by index_head_name

comptime qualifier Key for Field

comptime fn index_key<T is Tuple>(t: T) [] -> Int => t {
    [for index in T.indices] {
        [if index is Key] {
            return 1
        }
    }
    return 0
}

//~ error[comptime-tuple:qualifier-on-index]
fn index_key(t: (Str, Int)) [] -> Int by index_key

struct Holder { a: Str }

comptime fn index_entry<T is Tuple>(t: T) [] -> Holder => t {
    [for index in T.indices] {
        [if index.first] {
            return Holder { [index]: "x" }
        }
    }
    return Holder { a: "" }
}

//~ error[comptime-access:not-a-field-binder]
fn index_entry(t: (Str, Int)) [] -> Holder by index_entry

// [comptime-tuple] A tuple has no name; `.indices` is a tuple's only, and
// `.fields` a struct's.
comptime fn tuple_name<T is Tuple>(t: T) [] -> Str => t {
    return T.name
}

//~ error[comptime-eval:no-property]
fn tuple_name(t: (Str, Int)) [] -> Str by tuple_name

comptime fn tuple_fields<T is Tuple>(t: T) [] -> Int => t {
    let n = 0
    [for f in T.fields] {
        n = n + 1
    }
    return n
}

//~ error[comptime-fields:fields-of-non-struct]
fn tuple_fields(t: (Str, Int)) [] -> Int by tuple_fields

struct Point { x: Int, y: Int }

comptime fn struct_indices<T is Struct>(t: T) [] -> Int => t {
    let n = 0
    [for i in T.indices] {
        n = n + 1
    }
    return n
}

//~ error[comptime-tuple:indices-of-non-tuple]
fn struct_indices(t: Point) [] -> Int by struct_indices

// [comptime-tuple] A tuple picks the `<T is Tuple>` template of the name:
// with none, nothing of that kind is there to stamp.
comptime fn only_struct<T is Struct>(t: T) [] -> Int => t {
    return 0
}

//~ error[obligation-by:names-nothing]
fn only_struct(t: (Str, Int)) [] -> Int by only_struct

// [comptime-tuple] A stamped fn's element types are known only where it is
// stamped, so a `[when index.type]` over them is refused at the template, as
// for a field.
comptime fn index_types<T is Tuple>(t: T) [] -> Int => t {
    [for index in T.indices] {
        //~ error[comptime-pattern:when-over-derived-type]
        [when index.type] { is Int {} is * {} }
    }
    return 0
}

//--- a_misspelled_kind
// [comptime-bound] The bound names an arm of `Kind`: `Struct`, `Union` or
// `Tuple`.
//~ error[comptime-bound:unbounded-param]
comptime fn f<T is Tupel>(t: T) [] -> Int => t {
    return 0
}
