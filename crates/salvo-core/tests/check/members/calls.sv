// Member-resolution rules [call-resolve] [struct-field-resolve]
// [col-index-resolve] [iter-resolve]: Salvo assumes it can see everything, so a
// call, field read, subscript, or `for` over something the compiler cannot
// justify is an error. Reaching a target-language feature means *declaring* it
// — user decision 2026-09-03, which replaced the interop pass-through that used
// to make these silent.
//
// The narrow leniency that remains is [type-unknown-lenient]: a type the
// checker could not *infer* (`Ty::Unknown`) still passes through, so one
// mistake never cascades.

struct Person {
    name: Str
}

fn shout(s: Str) [] -> Str => s { return "" }

// ===== calls [call-resolve] =====

// [call-resolve] A name nothing declares is not a call into the target
// language any more — it is a mistake.
fn unresolved_call(p: Person) -> Int => p {
    //~ error[call-resolve:unknown-fn]
    nowhere()
    return 1
}

// [call-resolve] Calling a value whose type is known and is not a function:
// the old leniency emitted `n()` and let kotlinc/rustc reject it. (A
// parameter: a local `n` called only so would also warn that it is never
// used.)
fn calling_a_non_fn(n: Int) -> Int => n {
    //~ error[call-resolve:not-callable-local]
    n()
    return 1
}

// [call-resolve] A generic parameter is opaque: Salvo has no bounds, so
// nothing makes a `T` callable. The diagnostic says *that*, and names the
// remedy (a fn type), rather than pointing at a declaration that could not
// help.
fn calling_a_generic<T>(f: T) -> Int => f {
    //~ error[call-resolve:type-param-call]
    f()
    return 1
}

// [call-resolve] [fn-dot] Dot-notation is sugar for a function call, so an
// undeclared method name is unresolved — the diagnostic says how to reach a
// target-language method (a `platform handler`).
fn unresolved_dot_call(p: Person) -> Int => p {
    //~ error[fn-dot:unknown-fn]
    p.missing_method()
    return 1
}

// [call-resolve] A *declared* function is reachable by dot-notation, which is
// how a member-looking call is spelled.
fn declared_dot_call(p: Person) -> Int => p {
    let _s = "x".shout()
    return 1
}

// ===== fields [struct-field-resolve] =====

// [struct-field-resolve] Only structs have fields.
fn field_on_a_non_struct(p: Person) -> Int => p {
    let n = 5
    //~ error[struct-field-resolve:not-a-struct]
    let _x = n.whatever
    return 1
}

// [struct-field-resolve] An opaque (interop) type exposes nothing by itself:
// its members are reached through declared accessors. (`Bytes` is std's
// platform type.)
fn field_on_an_opaque_type(o: Bytes) -> Int => o {
    //~ error[struct-field-resolve:not-a-struct]
    let _x = o.member
    return 1
}

// [struct-field-resolve] A generic value is opaque too — and its diagnostic
// must not suggest a declared accessor, which cannot help for a `T`: it names
// a concrete type as the remedy.
fn field_on_a_generic<T>(v: T) -> Int => v {
    //~ error[struct-field-resolve:type-param]
    let _x = v.name
    return 1
}

// ===== subscripts [col-index-resolve] =====

// [col-index-resolve] `[]` is for arrays; other collections expose element
// access as functions.
fn indexing_a_non_array(p: Person) -> Int => p {
    let n = 5
    //~ error[col-index-resolve:not-indexable]
    let _x = n[0]
    return 1
}

// [col-index-resolve] Arrays still work, of course.
fn indexing_an_array(p: Person) -> Int => p {
    let xs: Int[] = array_of(1, 2)
    let _x = xs[0]
    return 1
}

// ===== iteration [iter-resolve] =====

// [iter-resolve] `for` needs an array, a pass with a `next`, or a value some
// `iter` function accepts.
fn iterating_a_non_iterable(p: Person) -> Int => p {
    let n = 5
    //~ error[iter-resolve:not-iterable]
    for x in n {
        let _y = x
    }
    return 1
}

// [iter-resolve] Arrays iterate.
fn iterating_an_array(p: Person) -> Int => p {
    let xs: Int[] = array_of(1, 2)
    for x in xs {
        let _y = x
    }
    return 1
}

// ===== the leniency that remains =====

// [type-unknown-lenient] A type the checker could not *infer* still passes
// through: one mistake must not cascade. Here the unresolved call is reported
// once, and reading a member of its `Unknown` result adds nothing.
fn uninferred_types_pass_through(p: Person) -> Int => p {
    //~ error[call-resolve:unknown-fn]
    let v = nowhere()
    let _x = v.member
    return 1
}

// ===== identifier references [name-ident-resolve] =====

// [name-ident-resolve] [backend-never-wrong] A bare name nothing declares used
// to pass the checker as `Ty::Unknown` and reach the emitters, which spelled it
// straight into the output — so the *target* compiler reported it. It is a
// Salvo error now: the same rule as a call, a field or a subscript.
fn an_undeclared_identifier(p: Person) -> Int => p {
    //~ error[name-ident-resolve:unknown]
    let _n = undeclared_thing
    return 1
}

// [name-ident-resolve] Reading a variable *before* its declaration is the same
// mistake: scopes are not hoisted.
fn a_variable_before_its_declaration(p: Person) -> Int => p {
    //~ error[name-ident-resolve:unknown]
    let _a = later
    let _later = 1
    return 1
}

// [name-ident-resolve] A name that *is* declared but is not a value says what
// it is, rather than claiming nothing declares it — the courtesy the effect and
// handler rules already extend [effect-not-a-type].
fn a_declared_non_value_name(p: Person) -> Int => p {
    //~ error[name-ident-resolve:not-a-value]
    let _s = Person
    return 1
}

// [name-ident-resolve] [type-unknown-lenient] One mistake, one diagnostic: the
// unresolved name's `Unknown` type does not cascade into the read below it.
fn an_undeclared_identifier_does_not_cascade(p: Person) -> Int => p {
    //~ error[name-ident-resolve:unknown]
    let v = undeclared_thing
    let _x = v.member
    return 1
}
