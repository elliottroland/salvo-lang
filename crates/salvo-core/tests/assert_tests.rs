//! Assertions [assert-op] [assert-fn] [assert-narrow] (user decisions
//! 2026-09-23, ASSERTIONS.md's A-1…A-7).
//!
//! Three forms, one idea: a place where the program states a fact it cannot
//! prove, and fails if it is wrong. `expr!` asserts presence, `assert!(cond)`
//! asserts a condition — and **narrows** when the condition is an `is` test,
//! which is what makes it more than a check — and `unreachable!()` asserts that
//! a path is not taken.
//!
//! The observable for the narrowing tests is overload resolution: a
//! `NonEmpty`-demanding function matches only while the checker believes the
//! claim, so "clean" means the assertion informed the type.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\n\
     export intrinsic type Bool\nexport intrinsic type Never\n";

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
    resolution
        .errors
        .iter()
        .chain(checked.errors.iter())
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

// ===== [assert-op] `!` =====

/// [assert-op] The operator asserts *presence*, so it needs something that can
/// be absent. Before this rule the two backends disagreed about the leftover —
/// rustc refused `.unwrap()` on an `i32` while kotlinc warned and ran.
#[test]
fn a_bang_on_a_never_absent_value_is_refused() {
    let src = "fn f() -> Int {\n    let n = 3\n    return n!\n}\n";
    assert!(
        errors(src)
            .iter()
            .any(|e| e.contains("it is never absent, so the `!` says nothing")),
        "{:?}",
        errors(src)
    );
}

/// …and the diagnostic names the two things to do instead.
#[test]
fn the_refusal_names_the_alternatives() {
    let src = "fn f() -> Int {\n    let n = 3\n    return n!\n}\n";
    let shown = errors(src).join("\n");
    assert!(shown.contains("`?:`") && shown.contains("is None"), "{shown}");
}

/// [assert-op] A union with no `None` arm is the same mistake: `!` removes
/// `None` arms, so on `Int | Str` it says nothing.
#[test]
fn a_bang_on_a_none_less_union_is_refused() {
    let src = "fn f(v: Int | Str) -> Int | Str => v {\n    return v!\n}\n";
    assert!(
        errors(src)
            .iter()
            .any(|e| e.contains("it is never absent")),
        "{:?}",
        errors(src)
    );
}

/// The form it exists for still works: an optional operand.
#[test]
fn a_bang_on_an_optional_is_fine() {
    let src = "fn f(v: Int?) -> Int => v {\n    return v!\n}\n";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
}

// ===== [assert-fn] [assert-narrow] `assert!` =====

/// [assert-narrow] The point of the form: an `is` test asserted at run time
/// narrows for the rest of the scope, so the fact reaches the *type*.
#[test]
fn an_assert_narrows_a_union() {
    let src = "fn f(v: Int | Str) -> Int => !v {\n    \
               assert!(v is Int, \"expected an Int\")\n    \
               return v\n}\n";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
}

/// The control: without the assertion the same body is rejected, so the
/// acceptance above is the narrowing and not something else.
#[test]
fn without_the_assert_the_union_does_not_narrow() {
    let src = "fn f(v: Int | Str) -> Int => !v {\n    return v\n}\n";
    assert!(!errors(src).is_empty(), "expected a type error");
}

/// [assert-fn] The message is optional, and it is an ordinary expression — so
/// interpolation works, and (being compiler-owned) it is evaluated only on
/// failure.
#[test]
fn the_message_is_optional_and_interpolates() {
    let bare = "fn f(n: Int) -> None => n {\n    assert!(n > 0)\n}\n";
    assert!(errors(bare).is_empty(), "{:?}", errors(bare));
    let interp = "fn f(n: Int) -> None => n {\n    \
                  assert!(n > 0, \"n must be positive, was ${n}\")\n}\n";
    assert!(errors(interp).is_empty(), "{:?}", errors(interp));
}

/// [cond-bool] No truthiness: the condition is a `Bool`.
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

/// A message that is not a `Str` is refused, since it is what the trap prints.
#[test]
fn a_non_str_message_is_refused() {
    let src = "fn f(n: Int) -> None => n {\n    assert!(n > 0, n)\n}\n";
    assert!(
        errors(src)
            .iter()
            .any(|e| e.contains("message must be a `Str`")),
        "{:?}",
        errors(src)
    );
}

// ===== [assert-fn] `unreachable!` =====

/// [assert-fn] `unreachable!()` is typed `Never`, so it stands where a value is
/// expected — which is what a `when`'s impossible arm needs.
#[test]
fn unreachable_stands_in_for_a_value() {
    let src = "fn f(n: Int) -> Str => n {\n    \
               return when {\n        n > 0 { \"positive\" }\n        \
               n <= 0 { \"other\" }\n        \
               else { unreachable!(\"an Int compares one way or the other\") }\n    }\n}\n";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
}

/// …and it ends a path, so a function whose tail is one needs no return value
/// [fn-must-return].
#[test]
fn unreachable_ends_a_path() {
    let src = "fn f(n: Int) -> Str => n {\n    unreachable!()\n}\n";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
}

/// Both names stay ordinary identifiers: only `assert!(` and `unreachable!(`
/// are the forms.
#[test]
fn the_names_are_contextual() {
    let src = "fn f(assert: Int, unreachable: Int) -> Int => assert, unreachable {\n    \
               return assert + unreachable\n}\n";
    assert!(errors(src).is_empty(), "{:?}", errors(src));
}
