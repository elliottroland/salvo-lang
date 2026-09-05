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
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type List<T> canbe Mut\nintrinsic fn empty_list<T>() [] -> [] Mut List<T>\nintrinsic fn of_list<T>(...elems: T[]) [] -> [] Mut List<T>\n";

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
    checked(src).1.errors
}

fn messages(src: &str) -> Vec<String> {
    errors(src).iter().map(|d| d.message.clone()).collect()
}

const PRELUDE: &str = r#"


fn size<T>(list: List<T>) [] -> [list] Int { return 0 }
fn add<T>(list: Mut List<T>, elem: T) [] -> [list: Mut] None {}
fn ignore<T>(value: T) [] -> [] None {}
"#;

fn src(body: &str) -> String {
    format!("{PRELUDE}\nfn probe() -> [] Int {{\n{body}\n    return 1\n}}\n")
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
        "{PRELUDE}\nfn fresh() -> [] Mut List<Int> {{\n    return empty_list()\n}}\n"
    );
    assert!(messages(&program).is_empty(), "{:?}", messages(&program));
}

/// A *concrete* parameter type flows into a nested call, so the argument
/// position determines the callee's type arguments too.
#[test]
fn a_concrete_parameter_determines_it() {
    let program = format!(
        "{PRELUDE}\nfn takes(xs: Mut List<Int>) -> [xs: Mut] None {{\n    \
         add(xs, 1)\n}}\n\nfn probe() -> [] Int {{\n    takes(empty_list())\n    \
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
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    let recorded: Vec<&Vec<Ty>> = out.call_type_args.values().collect();
    assert!(
        recorded
            .iter()
            .any(|args| args.len() == 1 && args[0] == Ty::named("Str")),
        "expected `T = Str` to be recorded, got {recorded:?}"
    );
}
