// Comptime forms the parser refuses. One program per error, since a parse
// error stops a program.

//--- a_terminal_comptime_if_refuses_else_dots
// [expr-terminal-scope] [comptime-inline] A comptime `[if …]` takes a
// terminal `...` body and an inline braced `else`, never `else ...`.
struct P { a: Int }

comptime fn f(p: P) [] -> Int => p {
    //~ error[expr-terminal-scope:terminal-comptime-if-else]
    [if P.kind is Struct] ... else ...
    return 0
}

//--- a_subject_comptime_when_takes_no_else_kind
// [when-exhaustive] A comptime `[when]` with a subject takes no `else`, as
// an ordinary `when` with one does not (user decision 2026-10-20): the arms
// are the cases, and the error names what covers the rest — `is Opaque`
// for a kind…
struct P { a: Int }

comptime fn f(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[when-exhaustive:kind-arms-missing]
        //~ error[when-exhaustive:comptime-when-else]
        [when f.kind] { is Struct {} else {} }
    }
    return 0
}

//--- a_subject_comptime_when_takes_no_else_name
// …an arm for the rest of the names…
struct P { a: Int }

comptime fn f(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[when-exhaustive:comptime-when-else]
        [when f.name] { is "a" {} else {} }
    }
    return 0
}

//--- a_subject_comptime_when_takes_no_else_type
// …and `is * { … }` for a type.
struct P { a: Int }

comptime fn f(p: P) [] -> Int => p {
    [for f in P.fields] {
        //~ error[when-exhaustive:comptime-when-else]
        [when f.type] { is Int {} else {} }
    }
    return 0
}

//--- a_bare_binder_type_in_a_let
// [comptime-inline] The bare projection in a type position is not the
// language (no compatibility form, AGENTS.md): a parse error naming the
// bracketed spelling, in a `let`…
comptime fn f<T is Struct>(v: T) [] -> Int => v {
    [for field in T.fields] {
        //~ error[comptime-inline:unbracketed-binder-type]
        let a: field.type = v.[field]
    }
    return 0
}

//--- a_bare_binder_type_in_type_arguments
// …and inside a generic call's `<…>` (where a speculative parse rolls back).
comptime fn f<T is Struct>(v: T) [] -> Int => v {
    [for field in T.fields] {
        //~ error[comptime-inline:unbracketed-binder-type]
        take<field.type>(v.[field])
    }
    return 0
}

//--- a_splice_outside_a_comptime_fn
// [comptime-inline] A splice outside a comptime fn names the scope.
fn f(p: Int) -> Int => p {
    //~ error[var-let-infer:annotation-mismatch]
    //~ error[name-resolve:unknown-type]
    //~ error[comptime-inline:outside-compfn]
    let a: [x.type] = p
    //~ error[fn-syntax:return-mismatch]
    return a
}

//--- a_default_test_compares_with_none_only
// [comptime-default] [comptime-docs] `field.default` is tested only for
// being there: any type after `is` but `None` is an error naming the two
// forms.
comptime fn f<T is Struct>(v: T) [] -> Int => v {
    [for field in T.fields] {
        //~ error[comptime-default:unset-test]
        [if field.default is Int] { }
    }
    return 0
}
