//! [call-type-args] A generic call's type arguments must be *determined*:
//! by the arguments, by an explicit type-argument list, or by the expected
//! type at the call. A type argument that reaches the result type and that
//! nothing determines is an error, because the checker would otherwise hand
//! the backends a `T` it never resolved — which one target language may
//! infer for itself and another may not (`mutableListOf()` is not valid
//! Kotlin, while rustc infers `vec![]` from a later use).
//!
//! The recorded bindings are also what a backend's intrinsic lowering
//! renders when it has to spell an element type out [backend-intrinsic].

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols, Ty};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn empty_list<T>() [] -> Mut List<T>\nexport intrinsic fn of_list<T>(...elems: T[]) [] -> Mut List<T>\n";

fn checked(src: &str) -> (Program, salvo_core::Checked) {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let out = check_program(&program, &resolution, &symbols);
    (program, out)
}

fn errors(src: &str) -> Vec<FileDiagnostic> {
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and these tests are about legality.
    let mut diags = checked(src).1.errors;
    diags.retain(|d| d.is_error());
    diags
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

const PRELUDE: &str = r#"


fn size<T>(list: List<T>) [] -> Int => list { return 0 }
fn add<T>(list: Mut List<T>, elem: T) [] -> None => list: Mut, !elem {}
fn ignore<T>(value: T) [] -> None => !value {}
fn apply<T, U>(value: T, f: (T) -> U) [] -> U => !value, !f { return f(value) }
fn apply_late<T, U>(f: (T) -> U, value: T) [] -> U => !f, !value { return f(value) }
"#;

fn src(body: &str) -> String {
    format!("{PRELUDE}\nfn probe() -> Int {{\n{body}\n    return 1\n}}\n")
}

#[test]
fn an_undetermined_type_argument_is_an_error() {
    let msgs = messages(&src("    let xs = empty_list()"));
    assert_eq!(msgs.len(), 1, "expected exactly one diagnostic: {msgs:?}");
    assert!(
        msgs[0].contains("cannot infer type argument `T` of `empty_list`")
            && msgs[0].contains("write the type argument")
            && msgs[0].contains("annotate"),
        "the diagnostic should name the parameter and both remedies: {}",
        msgs[0]
    );
}

#[test]
fn arguments_determine_type_arguments() {
    assert!(messages(&src("    let xs = of_list(1, 2)")).is_empty());
}

#[test]
fn an_explicit_type_argument_determines_it() {
    assert!(messages(&src("    let xs = empty_list<Int>()")).is_empty());
}

#[test]
fn a_let_annotation_determines_it() {
    assert!(messages(&src("    let xs: Mut List<Int> = empty_list()")).is_empty());
}

/// The enclosing fn's return type is an expected type like any other.
#[test]
fn a_return_type_determines_it() {
    let program = format!(
        "{PRELUDE}\nfn fresh() -> Mut List<Int> {{\n    return empty_list()\n}}\n"
    );
    assert!(messages(&program).is_empty(), "{:?}", messages(&program));
}

/// A *concrete* parameter type flows into a nested call, so the argument
/// position determines the callee's type arguments too.
#[test]
fn a_concrete_parameter_determines_it() {
    let program = format!(
        "{PRELUDE}\nfn takes(xs: Mut List<Int>) -> None => xs: Mut {{\n    \
         add(xs, 1)\n}}\n\nfn probe() -> Int {{\n    takes(empty_list())\n    \
         return 1\n}}\n"
    );
    assert!(messages(&program).is_empty(), "{:?}", messages(&program));
}

/// A *generic* parameter cannot: `ignore<T>(value: T)` tells the nested call
/// nothing, so it is still undetermined — and reporting it is right, since no
/// target language could infer it either.
#[test]
fn a_generic_parameter_does_not_determine_it() {
    let msgs = messages(&src("    ignore(empty_list())"));
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(msgs[0].contains("cannot infer type argument"), "{}", msgs[0]);
}

/// A type argument that never reaches the *result* type needs no context:
/// nothing downstream could observe it.
#[test]
fn a_type_argument_only_in_parameters_needs_no_context() {
    assert!(messages(&src("    ignore(1)")).is_empty());
}

/// [type-unknown-lenient] One mistake, one diagnostic: an argument whose
/// type the checker could not determine does not also produce an
/// "undetermined type argument" report.
#[test]
fn an_unknown_argument_stays_lenient() {
    let msgs = messages(&src("    let xs = of_list(nope())"));
    assert_eq!(msgs.len(), 1, "expected only the unresolved-call error: {msgs:?}");
    assert!(msgs[0].contains("no function named `nope`"), "{}", msgs[0]);
}

/// The resolved bindings are recorded per call, in the callee's declaration
/// order — this is what a `define fn` template interpolates as `${T}`.
#[test]
fn resolved_type_arguments_are_recorded_per_call() {
    let (_, out) = checked(&src("    let xs: Mut List<Str> = empty_list()"));
    assert!(out.errors.iter().all(|d| !d.is_error()), "{:?}", out.errors);
    let recorded: Vec<&Vec<Ty>> = out.call_type_args.values().collect();
    assert!(
        recorded
            .iter()
            .any(|args| args.len() == 1 && args[0] == Ty::named("Str")),
        "expected `T = Str` to be recorded, got {recorded:?}"
    );
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

/// [call-generic-progressive] The lambda's parameter type comes from the
/// binding an *earlier* argument made (`T = Int`), and its body then
/// determines the callee's remaining variable (`U = Int`).
#[test]
fn an_earlier_argument_types_a_later_lambda() {
    let msgs = messages(&src("    let n = apply(1, x -> x)"));
    assert!(msgs.is_empty(), "{msgs:?}");
}

/// The lambda's *return* is what determines `U`, so a body of a different
/// type than the parameter binds it to that type — the call still matches.
#[test]
fn a_lambda_body_determines_the_result_type_argument() {
    let msgs = messages(&src("    let s = apply(1, x -> size(of_list(x)))"));
    assert!(msgs.is_empty(), "{msgs:?}");
}

/// An explicit type-argument list seeds the bindings before the first
/// argument is checked, so it types the lambda too [call-type-args].
#[test]
fn an_explicit_type_argument_types_a_lambda() {
    let msgs = messages(&src("    let n = apply<Int, Int>(1, x -> x)"));
    assert!(msgs.is_empty(), "{msgs:?}");
}

/// An annotated lambda parameter needs no binding at all, in any position.
#[test]
fn an_annotated_lambda_parameter_needs_no_earlier_binding() {
    let msgs = messages(&src("    let n = apply_late((x: Int) -> x, 1)"));
    assert!(msgs.is_empty(), "{msgs:?}");
}

/// Left to right, deliberately: a lambda *before* the argument that would
/// bind its parameter type has nothing to go on, and says so rather than
/// inventing one. (Annotating the parameter is the remedy — the test
/// above.)
#[test]
fn a_lambda_before_its_binding_argument_is_not_inferred() {
    let msgs = messages(&src("    let n = apply_late(x -> x, 1)"));
    assert!(
        !msgs.is_empty(),
        "expected the un-inferrable lambda to be reported"
    );
}

/// [struct-literal-arg] A **bare generic struct literal** determines the
/// struct's own type arguments from its **field values**, which is the only
/// evidence a literal written as a call argument has: there is no annotation,
/// and the enclosing call's substitution is not solved yet. Before this,
/// `unwrap(Box { value: 7 })` was "no matching overload for `unwrap(Box)`" —
/// the literal's type had no arguments at all, so it could not match `Box<T>`
/// (defect closed 2026-09-26; the collection-literal half was
/// [col-literal-arg]).
#[test]
fn a_bare_generic_struct_literal_determines_its_type_argument() {
    let msgs = messages(
        "struct Box<T> { value: T }\n\n\
         fn unwrap<T>(b: Box<T>) -> T => !b {\n    return b.value\n}\n\n\
         fn go() [] -> Int {\n    return unwrap(Box { value: 7 })\n}\n",
    );
    assert!(msgs.is_empty(), "the field value determines `T`: {msgs:?}");
}

/// [struct-literal-arg] Two parameters, and a field each: both are determined,
/// and the *declared* field order does not have to match the generic order.
#[test]
fn every_parameter_a_field_mentions_is_determined() {
    let msgs = messages(
        "struct Pair<A, B> { second: B, first: A }\n\n\
         fn left<A, B>(p: Pair<A, B>) -> A => !p {\n    return p.first\n}\n\n\
         fn go() [] -> Str {\n    return left(Pair { second: 1, first: \"x\" })\n}\n",
    );
    assert!(msgs.is_empty(), "both parameters are determined: {msgs:?}");
}

/// [struct-literal-arg] The fields are evidence of **last resort**: a written
/// type argument still decides, so a literal whose field disagrees with it is
/// reported rather than silently re-typed.
#[test]
fn a_written_type_argument_still_decides() {
    let msgs = messages(
        "struct Box<T> { value: T }\n\n\
         fn go() [] -> None {\n    let b: Box<Str> = Box<Str> { value: 7 }\n}\n",
    );
    assert!(
        !msgs.is_empty(),
        "the annotation decides and the field must fit it"
    );
}
