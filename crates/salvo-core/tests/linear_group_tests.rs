//! [linear-group] `Linear` as a designated obligation group. The rule's
//! diagnostics are test data over the real std, `tests/check/linear/`
//! (TESTS.md T1); what stays here cannot be: a declaration-site rule that
//! needs a std module of the test's own, and a parse-only refusal.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic fn discard<T canbe linear>(value: T) [] -> None => !value\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
);

// ===== [linear-group] The opaque form: `linear intrinsic type` =====

/// Two std files, so the same-file discharger rule has a boundary to be
/// about: the prelude, plus a `token` module whose contents the test writes.
/// `intrinsic` is std-only [platform-intrinsic-std-only], so an opaque linear type can
/// only be declared this way — and the real std has no linear intrinsic type
/// without its discharger, which is why this test keeps a std of its own.
fn errors_with_std_module(token_src: &str, main_src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "std/core/token.sv",
        SourceSet::classify(Path::new("std/core/token.sv")).unwrap(),
        token_src.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        main_src.to_string(),
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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    let mut msgs: Vec<String> = resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    msgs.extend(
        salvo_core::ir::analyze::analyze(&program, &symbols, &resolution, &checked)
            .into_iter()
            .map(|d| d.message),
    );
    msgs
}

/// [linear-group] `linear intrinsic type` carries the obligation on a type
/// whose representation belongs to the backend (user decision 2026-09-15 —
/// `Reply<T>` is a scheduler handle, so there is nothing to make a `linear
/// struct` of). The declaration-site rule is the struct's, unchanged: the
/// declaring file must contain a discharger. (That values of such a type owe
/// and its intrinsic discharges them is data: `check/linear/intrinsic.sv`.)
#[test]
fn a_linear_intrinsic_type_needs_a_discharger_in_its_own_file() {
    let errs = errors_with_std_module(
        "export linear intrinsic type Token<T>\n",
        "fn probe() -> Int {\n    return 1\n}\n",
    );
    assert!(
        errs.iter().any(|m| m.contains("linear intrinsic type `Token`")
            && m.contains("no discharger")),
        "expected the legal-death error at the declaration, got {errs:?}"
    );
}

/// The modifier belongs to a *declaration*, never to an alias: an alias is a
/// second name for a type that already decided whether it owes. Parse-only:
/// checked, the recovered declaration also reports
/// `linear-opaque:no-discharger` for the alias (ROADMAP §0ar).
#[test]
fn an_alias_cannot_be_declared_linear() {
    let source = "export linear intrinsic type Handle<T> = Int\n";
    let (_module, diagnostics) = salvo_syntax::parse_module(source);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("alias cannot be linear")),
        "expected the alias refusal: {diagnostics:?}"
    );
}
