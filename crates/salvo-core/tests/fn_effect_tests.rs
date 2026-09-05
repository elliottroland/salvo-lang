//! Effects on fn types [fn-effects].
//!
//! An effect list on a fn type is a *requirement the caller of the value
//! supplies*, not a capability the value carries: `(s: Str) [Logger] -> Str`
//! means "call me with Logger available". So a lambda body performs only the
//! effects its type declares, a higher-order fn *inherits* its fn-typed
//! parameters' effects (user decision 2026-09-04 — the only reason to take
//! `f` is to call it), and a fn value can be stored or passed freely because
//! it holds no handler: only *calling* it needs one.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add("main.sv", module, kind, src.to_string(), false);
    let (ast, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let program = Program {
        files: sources.files,
        modules: vec![ast],
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"
internal type Int
internal type Str
internal type Bool

effect Logger {
    fn log(message: Str) -> [message] None
}

effect Counter {
    fn bump() -> [] None
}

external handler QuietLogger of Logger
external handler ZeroCounter of Counter

external fn note(text: Str) [] -> [text] None
"#;

// ===== a lambda body performs what its type declares =====

/// [fn-effects] A lambda passed where a fn type declares the effect may
/// perform it — the call site supplies it.
#[test]
fn a_declared_effect_is_available_in_the_body() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn probe() [Logger] -> [] None {{\n\
         run(s -> {{\n\
         log(\"in lambda ${{s}}\")\n\
         return \"seen ${{s}}\"\n\
         }})\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-effects] The dual: a lambda checked against a fn type that declares
/// *nothing* may not perform an effect, even though the enclosing scope has
/// one. Lexical leakage is what made a fn value's capabilities invisible in
/// its type.
#[test]
fn an_undeclared_effect_in_a_lambda_body_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn probe() [Logger] -> [] None {{\n\
         run(s -> {{\n\
         log(\"in lambda ${{s}}\")\n\
         return \"seen ${{s}}\"\n\
         }})\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Logger`")),
        "expected the lambda body to be rejected, got: {errs:?}"
    );
}

// ===== inheritance (user decision 2026-09-04) =====

/// [fn-effects] A fn taking an effectful fn value *inherits* the effect: it
/// needs no list of its own to call `f`, and its callers supply it.
#[test]
fn a_fn_inherits_its_fn_typed_parameters_effects() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-effects] The other half of inheritance: a caller that *cannot*
/// supply the inherited effect is rejected, since that is where the value
/// comes from at run time.
#[test]
fn a_caller_must_supply_an_inherited_effect() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [] -> [] None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no handler for effect `Logger`") && e.contains("run")),
        "expected the caller to be rejected, got: {errs:?}"
    );
}

/// [fn-effects] Inheritance reaches through a qualifier: `Once` fn types
/// carry effects the same way.
#[test]
fn inheritance_reaches_through_a_qualifier() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run_once(f: Once () [Logger] -> None) -> [] None {{\n\
         f()\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run_once(() -> {{ log(\"once\") }})\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

// ===== variance =====

/// [fn-effects] A value performing *fewer* effects fits where more are
/// expected: the caller supplies what it declared and the value ignores it.
#[test]
fn fewer_effects_fit_where_more_are_expected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn pure_fn(s: Str) [] -> [s] Str {{\n\
         return \"plain ${{s}}\"\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(pure_fn)\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

/// [fn-effects] Never the reverse: a value that performs an effect its
/// position does not declare would reach a call site that cannot supply it.
#[test]
fn more_effects_do_not_fit_where_fewer_are_expected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn shout(s: Str) [Logger] -> [s] Str {{\n\
         log(s)\n\
         return \"loud ${{s}}\"\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(shout)\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no matching overload")
            || e.contains("expected")
            || e.contains("Logger")),
        "expected the effectful fn to be rejected, got: {errs:?}"
    );
}

// ===== inference =====

/// [fn-effects] An un-annotated lambda's effect set is *inferred* from its
/// body (inference from a visible body is what [decl-explicit] permits), so
/// it does not silently fit a pure position.
#[test]
fn an_unannotated_lambdas_effects_are_inferred() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         let f = (s: Str) -> {{\n\
         log(s)\n\
         return \"logged ${{s}}\"\n\
         }}\n\
         run(f)\n\
         }}\n"
    ));
    assert!(
        !errs.is_empty(),
        "expected the inferred `[Logger]` to be rejected by the pure position"
    );
}

// ===== a fn value carries no capability =====

/// [fn-effects] Storing an effectful fn value is fine — it holds no handler.
/// What needs the effect is *calling* it, which is why "forbid escape" was
/// dropped from the design (user decision 2026-09-04).
#[test]
fn calling_is_what_needs_the_effect_not_holding() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn probe() [Logger] -> [] None {{\n\
         let f = (s: Str) -> {{\n\
         log(s)\n\
         return \"logged ${{s}}\"\n\
         }}\n\
         let g: (s: Str) -> [s] Str = (s: Str) -> {{\n\
         return f(s)\n\
         }}\n\
         note(g(\"x\"))\n\
         }}\n"
    ));
    // `g` declares no effects, so the `f(s)` inside it has no Logger.
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Logger`")
            && e.contains("function value")),
        "expected the inner call to be rejected, got: {errs:?}"
    );
}

/// [fn-effects] Two effects on one fn type both have to be supplied.
#[test]
fn every_declared_effect_must_be_available_at_the_call() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger, Counter] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> [] None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| e.contains("no handler for effect `Counter`")),
        "expected the missing second effect to be reported, got: {errs:?}"
    );
}

// ===== `use` is not part of a fn type =====

/// [fn-effects] Registering a handler is local to a body, so `use` in a fn
/// type's effect list is meaningless — a lambda may `use` exactly when the
/// function containing it may.
#[test]
fn use_in_a_fn_type_is_rejected() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [use] -> [s] Str) -> [] None {{\n\
         note(f(\"x\"))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("a fn type cannot declare `use`")),
        "expected the `use` rejection, got: {errs:?}"
    );
}
