//! The subject-less `when` [when-condition] and the boolean-condition rule
//! [cond-bool].
//!
//! `when` is always exhaustive, in one of two ways. With a subject it is
//! exhaustive over the union's arms and takes no `else`
//! [when-union-subject]; without one it is a condition chain whose `else` is
//! mandatory — which is exactly what separates it from `if`/`elif`/`else`,
//! since an `if` without an `else` folds `None` into its value
//! [if-else-none]. Conditions on both forms (and on `if`/`elif`/`while`) are
//! `Bool` and nothing else: Salvo has no truthiness.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// Parses + resolves + checks one file (no std) and returns every error
/// message — parse errors included, since half of this feature's rules are
/// enforced by the grammar.
fn errors(src: &str) -> Vec<String> {
    let mut sources = SourceSet::default();
    let (module, kind) = SourceSet::classify(Path::new("main.sv"), "kotlin").unwrap();
    sources.add("main.sv", module, kind, src.to_string(), false);
    let (ast, diagnostics) = salvo_syntax::parse_module(&sources.files[0].content);
    let mut out: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect();
    let program = Program {
        files: sources.files,
        modules: vec![ast],
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    out.extend(
        resolution
            .errors
            .iter()
            .chain(checked.errors.iter())
            .map(|d| d.message.clone()),
    );
    out
}

fn assert_clean(errs: &[String]) {
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

fn assert_has(errs: &[String], needle: &str) {
    assert!(
        errs.iter().any(|e| e.contains(needle)),
        "expected an error containing {needle:?}, got {errs:?}"
    );
}

const PRELUDE: &str = r#"
intrinsic type Int
intrinsic type Str
intrinsic type Bool

qualifier Ok<T> of T
qualifier Err<T> of T

external fn note(text: Str) [] -> [text] None
"#;

// ===== [when-condition] =====

/// The plain form: bare boolean branch heads, a mandatory `else`.
#[test]
fn a_subjectless_when_is_a_condition_chain() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         return when {{\n\
         n < 0 {{ \"negative\" }}\n\
         n == 0 {{ \"zero\" }}\n\
         else {{ \"positive\" }}\n\
         }}\n\
         }}\n"
    ));
    assert_clean(&errs);
}

/// [when-value] The mandatory `else` is the point: every path produces a
/// value, so the type is the join of the branches with no `None` folded in —
/// unlike an `if` chain without an `else` [if-else-none]. A `Str` return
/// type accepting the chain is the observable proof.
#[test]
fn the_mandatory_else_keeps_none_out_of_the_value() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         let label = when {{\n\
         n < 0 {{ \"negative\" }}\n\
         else {{ \"other\" }}\n\
         }}\n\
         return label\n\
         }}\n"
    ));
    assert_clean(&errs);
    // The same chain written as an `if` is `Str?`, and returning it fails.
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         let label = if n < 0 {{\n\
         \"negative\"\n\
         }}\n\
         return label\n\
         }}\n"
    ));
    assert_has(&errs, "Str?");
}

/// Without an `else` there is nothing to make the chain exhaustive, and
/// `when` is always exhaustive.
#[test]
fn a_subjectless_when_requires_an_else() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         return when {{\n\
         n < 0 {{ \"negative\" }}\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "must end with an `else`");
}

/// An `else` with no condition above it decides nothing.
#[test]
fn a_when_with_only_an_else_is_an_error() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         return when {{\n\
         else {{ \"whatever\" }}\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "nothing to decide");
}

/// The `else` closes the chain: branches after it would be dead.
#[test]
fn else_must_be_the_last_branch() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         return when {{\n\
         n < 0 {{ \"negative\" }}\n\
         else {{ \"other\" }}\n\
         n > 0 {{ \"positive\" }}\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "last branch");
}

/// [when-union-subject] The subject form is exhaustive over the arms, so an
/// `else` there is a category error — and the diagnostic names the other
/// form rather than reporting a missing `is`.
#[test]
fn the_subject_form_rejects_an_else_and_names_the_other_form() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         external fn outcome() [] -> [] Ok Int | Err Str\n\
         fn probe() [] -> [] Str {{\n\
         let o = outcome()\n\
         return when o {{\n\
         is Ok {{ \"ok\" }}\n\
         else {{ \"err\" }}\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "it takes no `else`");
    assert_has(&errs, "when { cond {");
}

/// Branch heads are ordinary boolean expressions, so `is` works in them and
/// narrows its branch — and the negative narrowing accumulates into the
/// later branches exactly as in an `if`/`elif` chain [is-narrowing].
#[test]
fn is_heads_narrow_their_branch_and_the_else() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn describe(value: Str | Int) [] -> [] None {{\n\
         when {{\n\
         value is Str {{ note(value) }}\n\
         else {{ note(\"an int\") }}\n\
         }}\n\
         }}\n"
    ));
    assert_clean(&errs);
    // The `else` has the arm removed: `note` takes a `Str`, and there the
    // value is an `Int`.
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn describe(value: Str | Int) [] -> [] None {{\n\
         when {{\n\
         value is Str {{ note(\"a str\") }}\n\
         else {{ note(value) }}\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "note");
}

/// [fn-must-return] A total chain returns on every path, so no fall-through
/// return is needed after it.
#[test]
fn a_subjectless_when_can_be_the_only_return() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn classify(n: Int) [] -> [] Str {{\n\
         when {{\n\
         n < 0 {{ return \"negative\" }}\n\
         else {{ return \"other\" }}\n\
         }}\n\
         }}\n"
    ));
    assert_clean(&errs);
}

// ===== [cond-bool] =====

/// Conditions are `Bool`. There is no truthiness rule that could turn an
/// `Int` into a decision, so the non-boolean condition is an error on every
/// construct that takes one.
#[test]
fn conditions_must_be_bool() {
    for (what, body) in [
        ("if", "if n {\nnote(\"x\")\n}\n"),
        (
            "elif",
            "if n > 0 {\nnote(\"x\")\n} elif n {\nnote(\"y\")\n}\n",
        ),
        ("while", "while n {\nnote(\"x\")\n}\n"),
        ("when", "when {\nn { note(\"x\") }\nelse { note(\"y\") }\n}\n"),
    ] {
        let errs = errors(&format!(
            "{PRELUDE}\nfn f(n: Int) [] -> [] None {{\n{body}}}\n"
        ));
        assert!(
            errs.iter()
                .any(|e| e.contains("a condition must be a `Bool` (found `Int`)")),
            "expected a Bool-condition error for {what}, got {errs:?}"
        );
    }
}

/// The check lands on the *leaf* that has the wrong type, so a compound
/// condition reports the operand rather than the whole expression.
#[test]
fn each_leaf_of_a_compound_condition_is_checked() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(n: Int) [] -> [] None {{\n\
         if n > 0 && n {{\n\
         note(\"x\")\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "a condition must be a `Bool` (found `Int`)");
    assert_eq!(errs.len(), 1, "only the bad leaf reports: {errs:?}");
}

/// A possibly-absent boolean is not a boolean: which way `None` should
/// decide is exactly what the rule refuses to guess. `!` (or an `is` test)
/// is the way through.
#[test]
fn an_optional_bool_is_not_a_condition() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(b: Bool?) [] -> [] None {{\n\
         if b {{\n\
         note(\"x\")\n\
         }}\n\
         }}\n"
    ));
    assert_has(&errs, "a condition must be a `Bool` (found `Bool?`)");
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f(b: Bool?) [] -> [] None {{\n\
         if b! {{\n\
         note(\"x\")\n\
         }}\n\
         }}\n"
    ));
    assert_clean(&errs);
}

/// [type-unknown-lenient] A type the checker could not infer produces one
/// diagnostic, not two: the condition rule stays quiet on `Unknown`.
#[test]
fn an_uninferred_condition_stays_lenient() {
    let errs = errors(&format!(
        "{PRELUDE}\n\
         fn f() [] -> [] None {{\n\
         if unknown_thing() {{\n\
         note(\"x\")\n\
         }}\n\
         }}\n"
    ));
    assert!(
        !errs
            .iter()
            .any(|e| e.contains("a condition must be a `Bool`")),
        "the unresolved call is the only error: {errs:?}"
    );
}
