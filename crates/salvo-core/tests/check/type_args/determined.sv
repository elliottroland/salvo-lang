// [call-type-args] A generic call's type arguments must be *determined*:
// by the arguments, by an explicit type-argument list, or by the expected
// type at the call. A type argument that reaches the result type and that
// nothing determines is an error, because the checker would otherwise hand
// the backends a `T` it never resolved — which one target language may
// infer for itself and another may not (`mutableListOf()` is not valid
// Kotlin, while rustc infers `vec![]` from a later use).
//
// std's empty constructor `Mut List<T>()` is the call that determines
// nothing by its arguments (the old prelude's `empty_list`).

fn swallow<T>(value: T) [] -> None => !value {}
fn apply<T, U>(value: T, f: (T) -> U) [] -> U => !value, !f { return f(value) }
fn apply_late<T, U>(f: (T) -> U, value: T) [] -> U => !f, !value { return f(value) }

// The diagnostic names the parameter and both remedies (write the type
// argument, annotate), once.
fn undetermined() -> None {
    //~ error[call-type-args:cannot-infer]
    let _xs = Mut List()
}

fn by_the_arguments() -> None {
    let _xs = Mut List(1, 2)
}

fn by_an_explicit_type_argument() -> None {
    let _xs = Mut List<Int>()
}

fn by_a_let_annotation() -> None {
    let _xs: Mut List<Int> = Mut List()
}

// The enclosing fn's return type is an expected type like any other.
fn fresh() -> Mut List<Int> {
    return Mut List()
}

// A *concrete* parameter type flows into a nested call, so the argument
// position determines the callee's type arguments too.
fn takes(xs: Mut List<Int>) -> None => xs: Mut {
    add(xs, 1)
}

fn by_a_concrete_parameter() -> None {
    takes(Mut List())
}

// A *generic* parameter cannot: `swallow<T>(value: T)` tells the nested call
// nothing, so it is still undetermined — and reporting it is right, since no
// target language could infer it either.
fn not_by_a_generic_parameter() -> None {
    //~ error[call-type-args:cannot-infer]
    swallow(Mut List())
}

// A type argument that never reaches the *result* type needs no context:
// nothing downstream could observe it.
fn only_in_parameters() -> None {
    swallow(1)
}

// [type-unknown-lenient] One mistake, one diagnostic: an argument whose
// type the checker could not determine does not also produce an
// "undetermined type argument" report.
fn unknown_argument() -> None {
    //~ error[call-resolve:unknown-fn]
    let _xs = Mut List(nope())
}

// ===== [call-generic-progressive] lambda arguments =====
//
// A callee's type variables bind progressively, left to right, so an
// argument's expected type is the parameter pattern with everything the
// earlier arguments already determined substituted in. Without it an
// un-annotated lambda is checked against a pattern still mentioning `T`,
// so its inferred type *contains the callee's own variable* — the one
// thing `unify`'s deliberate lack of an occurs check assumes cannot
// happen [fn-overload] — and the call fails to match itself.

// [call-generic-progressive] The lambda's parameter type comes from the
// binding an *earlier* argument made (`T = Int`), and its body then
// determines the callee's remaining variable (`U = Int`).
fn earlier_argument_types_a_lambda() -> None {
    let _n = apply(1, x -> x)
}

// The lambda's *return* is what determines `U`, so a body of a different
// type than the parameter binds it to that type — the call still matches.
fn lambda_body_determines_the_result() -> None {
    let _s = apply(1, x -> size(Mut List(x)))
}

// An explicit type-argument list seeds the bindings before the first
// argument is checked, so it types the lambda too [call-type-args].
fn explicit_type_arguments_type_a_lambda() -> None {
    let _n = apply<Int, Int>(1, x -> x)
}

// An annotated lambda parameter needs no binding at all, in any position.
fn annotated_lambda_parameter() -> None {
    let _n = apply_late((x: Int) -> x, 1)
}

// Left to right, deliberately: a lambda *before* the argument that would
// bind its parameter type has nothing to go on, and says so rather than
// inventing one. (Annotating the parameter is the remedy — the case above.)
fn lambda_before_its_binding_argument() -> None {
    //~ error[call-type-args:cannot-infer]
    let _n = apply_late(x -> x, 1)
}

// [call-type-args] A `None` argument meets `Q?`'s `None` arm and leaves `Q`
// to what follows: binding `Q` to `None` first refused a lambda after it,
// whose return then had to be a `None?` (found building `args`' derive,
// 2026-10-20). The order of the arguments no longer matters.
fn m<Q>(d: Q?, r: (v: Q) -> Q?) [] -> Int =>[r] v => d, r { return 1 }

fn late<Q>(r: (v: Q) -> Q?, d: Q?) [] -> Int =>[r] v => d, r { return 1 }

fn none_written() [] -> Int {
    return m<Str>(None, (x: Str) -> x)
}

fn none_inferred() [] -> Int {
    return m(None, (x: Str) -> x)
}

fn none_last() [] -> Int {
    return late((x: Str) -> x, None)
}
