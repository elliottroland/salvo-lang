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

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n\
     export qualifier Emitted<T> of T\n\
     export struct Finished {}\n\
     export fn emitted<T>(value: T) [] -> T as Emitted => !value {\n    return value\n}\n\
     export fn finished() [] -> Finished {\n    return Finished {}\n}\n\
     export params Yield<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n";

fn errors(src: &str) -> Vec<String> {
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
    let checked = check_program(&program, &resolution, &symbols);
    // Errors only: an unused-variable *warning* [unused-var] is a different
    // severity, and these tests are about which programs are rejected.
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

const PRELUDE: &str = r#"

effect Logger {
    fn log(message: Str) -> None => message
}

effect Counter {
    fn bump() -> None
}

handler QuietLogger of Logger {
    fn log(message: Str) -> None => message {}
}
handler ZeroCounter of Counter {
    fn bump() -> None {}
}

fn note(text: Str) [] -> None => text {}
"#;

// ===== a lambda body performs what its type declares =====

/// [fn-effects] A lambda passed where a fn type declares the effect may
/// perform it — the call site supplies it.
#[test]
fn a_declared_effect_is_available_in_the_body() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run(f: (s: Str) [Logger] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn probe() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn probe() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) [Logger] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) [Logger] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [] -> None {{\n\
         run(s -> \"${{s}}\")\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("no handler for effect `Logger`") && e.contains("run")),
        "expected the caller to be rejected, got: {errs:?}"
    );
}

/// [fn-effects] Inheritance reaches through a qualifier: `once` fn types
/// carry effects the same way.
#[test]
fn inheritance_reaches_through_a_qualifier() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn run_once(f: once () [Logger] -> None) -> None => !f {{\n\
         f()\n\
         }}\n\
         fn caller() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) [Logger] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn pure_fn(s: Str) [] -> Str => s {{\n\
         return \"plain ${{s}}\"\n\
         }}\n\
         fn caller() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn shout(s: Str) [Logger] -> Str => s {{\n\
         log(s)\n\
         return \"loud ${{s}}\"\n\
         }}\n\
         fn caller() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> None {{\n\
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
         fn probe() [Logger] -> None {{\n\
         let f = (s: Str) -> {{\n\
         log(s)\n\
         return \"logged ${{s}}\"\n\
         }}\n\
         let g: (s: Str) -> Str = (s: Str) -> {{\n\
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
         fn run(f: (s: Str) [Logger, Counter] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n\
         fn caller() [Logger] -> None {{\n\
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
         fn run(f: (s: Str) [use] -> Str) -> None =>[f] s {{\n\
         note(f(\"x\"))\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("a fn type cannot declare `use`")),
        "expected the `use` rejection, got: {errs:?}"
    );
}

// ===== [fn-effects] a pass performs its effects in its `next` =====
//
// A producer used to be a *type* (`Logger Iter<Int>`) whose driving performed
// the claim, so its effects were written in qualifier position on the return
// type (D8, 2026-09-07). With the reduction to `next` a producer is a struct
// and its `next` is an ordinary function, so there is nothing special left:
// `[fn-effects]` says it all, and the old spellings are refused.

/// An effect name in qualifier position is refused, naming the replacement:
/// the effect list of the `next` that performs it.
#[test]
fn an_effect_in_qualifier_position_is_refused() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(n: Logger Int) -> None => !n {{\n\
         return None\n\
         }}\n"
    ));
    assert!(
        errs.iter().any(|e| {
            e.contains("is an effect, not a qualifier") && e.contains("yield fn next")
        }),
        "expected the effect-position refusal, got: {errs:?}"
    );
}

/// A fn type is redirected to its own bracket list, as it always was: two
/// spellings of one thing is how they drift.
#[test]
fn an_effect_on_a_fn_type_names_the_bracket_form() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(g: Logger (Int) -> Int) -> None => !g {{\n\
         return None\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("declares its effects in its own list")),
        "expected the fn-type redirection, got: {errs:?}"
    );
}

/// The restriction is on *producing*, not consuming: a `for` loop sits in an
/// ordinary fn and may perform whatever that fn declares.
#[test]
fn consuming_a_pass_may_perform_effects() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn report(xs: Int[]) [Logger] -> None => xs {{\n\
         for v in xs {{\n\
         log(\"one\")\n\
         }}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "{errs:?}");
}
