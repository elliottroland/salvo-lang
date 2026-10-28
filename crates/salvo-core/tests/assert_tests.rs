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

/// Like [`errors`], but keeps only the *warnings* — for `todo!()` and the
/// dead-code-after-divergence warning, which do not reject the program.
fn warnings(src: &str) -> Vec<String> {
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
        let (ast, _) = salvo_syntax::parse_module(&file.content);
        modules.push(ast);
    }
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let checked = check_program(&program, &resolution, &symbols);
    checked
        .errors
        .iter()
        .filter(|d| !d.is_error())
        .map(|d| d.message.clone())
        .collect()
}

// ===== [assert-fn] `todo!()` and dead code =====

/// `todo!()` warns at every site, with or without a message, and does not
/// reject the program.
#[test]
fn todo_warns_at_every_site() {
    let src = "\
fn stub() -> Int {\n    todo!()\n}\n\
fn stub_msg() -> Int {\n    todo!(\"later\")\n}\n";
    assert!(errors(src).is_empty(), "todo! is not an error: {:?}", errors(src));
    let warnings = warnings(src);
    let todos: Vec<&String> = warnings.iter().filter(|w| w.contains("not implemented")).collect();
    assert_eq!(todos.len(), 2, "one warning per todo!(): {warnings:?}");
}

/// A statement after a diverging one (`return`, `unreachable!()`, `todo!()`)
/// is unreachable: a warning on the first such statement, not an error.
#[test]
fn code_after_a_diverging_statement_warns() {
    let after_return = "fn f() -> Int {\n    return 1\n    let x = 2\n}\n";
    assert!(
        warnings(after_return).iter().any(|w| w.contains("unreachable")),
        "dead code after return warns: {:?}",
        warnings(after_return)
    );
    assert!(errors(after_return).is_empty(), "it is a warning, not an error");
    // After `unreachable!()` too.
    let after_unreachable = "fn g() -> Int {\n    unreachable!()\n    let y = 2\n}\n";
    assert!(
        warnings(after_unreachable).iter().any(|w| w.contains("unreachable")),
        "dead code after unreachable! warns: {:?}",
        warnings(after_unreachable)
    );
    // Reachable code does not warn.
    let live = "fn h() -> Int {\n    let z = 1\n    return z\n}\n";
    assert!(
        !warnings(live).iter().any(|w| w.contains("unreachable")),
        "no false positive on live code: {:?}",
        warnings(live)
    );
}

// ===== [loop-never] `while true` with no break =====

/// A `while true {}` with no `break` targeting it never terminates, so it is
/// `Never`: a fn whose body ends in one needs no return, and code after it is
/// unreachable.
#[test]
fn infinite_loop_with_no_break_is_never() {
    // `-> Int` with no return, body ends in `while true {}`: accepted.
    let spins = "fn f() -> Int {\n    while true {\n    }\n}\n";
    assert!(errors(spins).is_empty(), "a non-escaping `while true` ends the fn: {:?}", errors(spins));
    // Code after it is unreachable: a dead-code warning (not an error).
    let after = "fn g() -> Int {\n    while true {\n    }\n    let x = 3\n}\n";
    assert!(errors(after).is_empty(), "{:?}", errors(after));
    assert!(
        warnings(after).iter().any(|w| w.contains("unreachable")),
        "code after an infinite loop warns: {:?}",
        warnings(after)
    );
}

/// A `break` targeting the `while true` lets control out, so it is **not**
/// `Never`: a `-> Int` fn whose only tail is that loop is then missing a
/// return.
#[test]
fn an_escapable_infinite_loop_is_not_never() {
    let breaks = "fn f(n: Int) -> Int {\n    while true {\n        if n > 0 { break }\n    }\n}\n";
    assert!(
        errors(breaks).iter().any(|m| m.contains("return")),
        "a `while true` with a break does not end the fn: {:?}",
        errors(breaks)
    );
}

/// A `break` in a *nested* loop targets the inner loop, so the outer
/// `while true` still never terminates — `Never`, no return needed.
#[test]
fn a_nested_break_does_not_escape_the_outer_loop() {
    let nested = "fn f(flag: Bool) -> Int {\n    while true {\n        while flag {\n            break\n        }\n    }\n}\n";
    assert!(
        errors(nested).is_empty(),
        "a nested break leaves the outer `while true` diverging: {:?}",
        errors(nested)
    );
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

/// [assert-op] The replacement for a value, `?: unreachable!(…)`, still needs
/// something that can be absent: on a never-absent value the pick keeps its
/// only arm, refused like any pointless `?:`.
#[test]
fn an_unreachable_pick_on_a_never_absent_value_is_refused() {
    let src = "fn f() -> Int {\n    let n = 3\n    return n ?: unreachable!()\n}\n";
    assert!(
        errors(src).iter().any(|e| e.contains("keeps its only arm")),
        "{:?}",
        errors(src)
    );
}

/// The form it exists for works: an optional operand.
#[test]
fn an_unreachable_pick_on_an_optional_is_fine() {
    let src = "fn f(v: Int?) -> Int => v {\n    return v ?: unreachable!()\n}\n";
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

/// [qual-whynot] `whynot?(x is Q)` is a `Str?` — usable where one is
/// expected and refused where a `Str` is — and explains a predicate test
/// only: a binding and a union arm test are refused.
#[test]
fn whynot_answers_an_optional_message_for_a_predicate_test() {
    let pre = "qualifier Even of Int {\n    // @whynot [n] is odd\n    fn qualifies(n: Int) -> Bool {\n        return true\n    }\n}\n\n";
    let ok = format!("{pre}fn f(n: Int) [] -> Str? => n {{\n    return whynot?(n is Even Int)\n}}\n");
    assert!(errors(&ok).is_empty(), "got {:?}", errors(&ok));
    let not_str = format!("{pre}fn f(n: Int) [] -> Str => n {{\n    return whynot?(n is Even Int)\n}}\n");
    assert!(!errors(&not_str).is_empty(), "a `Str?` is not a `Str`");
    let bad = format!(
        "{pre}fn f(n: Int, u: Int | Str) [] -> None => n, u {{\n    let a = whynot?(n is Even Int e)\n    let b = whynot?(u is Int)\n}}\n"
    );
    let errs = errors(&bad);
    for want in ["`whynot?` answers a message, and binds nothing", "this one tests which arm a union holds"] {
        assert!(errs.iter().any(|e| e.contains(want)), "missing {want:?} in {errs:?}");
    }
}

/// [doc-tags] The doc tags are checked where they are written: a tag this
/// version does not know, an `@param` naming no parameter (or nothing, or on
/// a declaration that is not a fn), and an `@whynot` that is not on a
/// qualifier's `qualifies` warn, at the documented name.
#[test]
fn doc_tags_warn_when_they_cannot_mean_anything() {
    let src = "// A point.\n//\n// @param x  nothing\n// @frob something\nstruct P {\n    x: Int\n}\n\n\
               // Adds.\n//\n// @param a  the first\n// @param c  no such\n// @param\n// @whynot nope\n// @returns  the sum,\n//           of both\n\
               fn add(a: Int, b: Int) [] -> Int => a, b {\n    return a + b\n}\n";
    assert!(errors(src).is_empty(), "tags never reject a program: {:?}", errors(src));
    let warns = warnings(src);
    for want in [
        "`@param` describes a fn's parameter, and `P` is not a fn",
        "`@frob` is not a doc tag",
        "`@param c`: `add` has no parameter `c`",
        "`@param` names the parameter it describes",
        "`@whynot` says why a value fails a predicate qualifier",
    ] {
        assert!(warns.iter().any(|w| w.contains(want)), "missing {want:?} in {warns:?}");
    }
    assert_eq!(warns.len(), 5, "{warns:?}");
}
