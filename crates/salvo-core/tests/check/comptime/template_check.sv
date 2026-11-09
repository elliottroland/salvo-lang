// ===== [comptime-template-check] a template is held to its rules where it
// is declared, whether or not anything stamps it. =====

struct Point { x: Int, y: Int }

// A head reads the *type*: `T.fields`, not `t.fields` of a value of it.
comptime fn value_fields<T is Struct>(t: T) [] -> Int => t {
    let n = 0
    //~ error[comptime-inline:not-a-binder]
    [for field in t.fields] {
        n = n + 1
    }
    return n
}

// A name no head declared.
comptime fn unknown_root<T is Struct>(t: T) [] -> Int => t {
    let n = 0
    //~ error[comptime-inline:not-a-binder]
    [for field in missing.fields] {
        n = n + 1
    }
    return n
}

// A template with a result returns it on every path.
//~ error[fn-must-return:missing-return]
comptime fn no_return<T is Struct>(t: T) [] -> Int => t {
    let n = 0
    [for field in T.fields] {
        n = n + 1
    }
}

// What a `[for]` unrolls to is a stamp's to say; a template that returns in
// its copies, in every arm of a `[when]`, or in both halves of an `[if]` does
// return, and so does one that refuses.
comptime fn returns_in_a_for<T is Struct>(t: T) [] -> Int => t {
    [for field in T.fields] ...
    return field.index
}

comptime fn returns_in_both_halves<T is Struct>(t: T) [] -> Int => t {
    [if T.mutable] {
        return 1
    } else {
        return 2
    }
}

comptime fn refuses<T is Struct>(t: T) [] -> Int => t {
    refuse!("no")
}

// …and a binder, a `[let]` and a lambda's parameter are names.
comptime fn declared_names<T is Struct>(t: T) [] -> Int => t {
    [let keys = filter(T.fields, (f) -> f.name == "x")]
    [for key in keys] {
        return key.index
    }
    return 0
}
