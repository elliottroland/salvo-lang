// Names: casing and namespaced names [name-casing] [name-dot]. One program
// per error, since a parse error stops a program.

//--- a_declaration_name_has_no_dot
// [name-dot] A dotted name is spelled with a `namespace` block; a declaration
// names one segment, so the dot is a plain parse error.
//~ error[syntax-expected:token]
struct A.B {
    v: Str
}

//--- casing_struct
// [name-casing] Types are uppercase, values are not: one case per
// declaration form.
//~ error[name-casing:type-name-lowercase]
struct thing {
    v: Str
}

//--- casing_qualifier
//~ error[name-casing:type-name-lowercase]
qualifier tag for Str

//--- casing_type
//~ error[name-casing:type-name-lowercase]
type alias = Str

//--- casing_effect
//~ error[name-casing:type-name-lowercase]
effect logger {
    fn log(m: Str) -> None => m
}

//--- casing_parameter
//~ error[name-casing:value-name-uppercase]
fn f(Bad: Str) -> None {
}

//--- casing_field
struct S {
    //~ error[name-casing:value-name-uppercase]
    Value: Str
}

//--- casing_let
fn f() -> None {
    //~ warning[var-unused:never-used]
    //~ error[name-casing:value-name-uppercase]
    let Bad = 1
}

//--- casing_for
fn f(xs: List<Int>) -> None {
    //~ warning[var-unused:never-used]
    //~ error[name-casing:value-name-uppercase]
    for Item in xs {
    }
}

//--- casing_lambda_parameter
fn f() -> None {
    //~ error[name-casing:value-name-uppercase]
    let _g = (X) -> 1
}

//--- generic_parameters_must_be_uppercase
// [name-casing] Generic parameters are type names.
//~ error[name-casing:type-name-lowercase]
fn f<t>(v: t) -> t => !v {
    return v
}
