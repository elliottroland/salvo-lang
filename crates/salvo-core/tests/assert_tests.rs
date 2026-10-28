//! Assertions [assert-op] [assert-fn] [assert-narrow] (user decisions
//! 2026-09-23, ASSERTIONS.md's A-1…A-7).
//!
//! Two forms, one idea: a place where the program states a fact it cannot
//! prove, and fails if it is wrong (the postfix `!` presence assertion was
//! removed 2026-10-14; presence is `?: unreachable!(…)`). `assert!(cond)`
//! asserts a condition — and **narrows** when the condition is an `is` test,
//! which is what makes it more than a check — and `unreachable!()` asserts that
//! a path is not taken.
//!
//! The observable for the narrowing tests is overload resolution: a
//! `NonEmpty`-demanding function matches only while the checker believes the
//! claim, so "clean" means the assertion informed the type.
//!
//! The cases are test data now, `tests/check/assertions/` (TESTS.md T1).
//! Two stay here until a defect is fixed (ROADMAP §0ar): the postfix-`!`
//! test is parse-only, because past the parse error the recovered `return
//! v` also reports `fn-syntax:return-mismatch`; and `assert!(n)` reports
//! both `expr-cond-bool:assert-condition` and `:not-bool` for its one
//! mistake.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\n\
     export intrinsic type Bool\nexport intrinsic type Never\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
);

fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
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
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

/// [assert-op] There is no postfix `!`: a presence assertion is spelled out,
/// so it is not the easy thing to write (user decision 2026-10-14). The parse
/// error names both replacements.
#[test]
fn a_postfix_bang_is_a_parse_error_naming_the_alternatives() {
    let src = "fn f(v: Int?) -> Int => v {\n    return v!\n}\n";
    let (_, diagnostics) = salvo_syntax::parse_module(src);
    let shown: Vec<String> = diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect();
    assert!(
        shown.iter().any(|m| m.contains("postfix `!`") && m.contains("unreachable!") && m.contains("assert!(x !is None)")),
        "{shown:?}"
    );
}

/// [expr-cond-bool] No truthiness: the condition is a `Bool`.
#[test]
fn a_non_bool_condition_is_refused() {
    let src = "fn f(n: Int) -> None => n {\n    assert!(n)\n}\n";
    assert!(
        errors(src)
            .iter()
            .any(|e| e.contains("must be a `Bool`")),
        "{:?}",
        errors(src)
    );
}
