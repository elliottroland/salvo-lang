//! The optional-or-else operator `?:` and the placeholder `_` [elvis]
//! [placeholder].
//!
//! `?…?:` / `:?…?` is the **positional pick**: a run of `n` `?`s names how many
//! arms of the subject's union — counted from the declared front (`Forward`) or
//! back (`Backward`) — the operator keeps. Those become the value; the right
//! side runs otherwise, with `_` bound to the arms left behind. `?:` is
//! `Forward(1)`. The right side is an ordinary expression, which since
//! [expr-escape] includes `return`/`break`/`continue` — so `maybe_t() ?: return
//! None` is the form the whole sequence was for.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn first<T>(list: List<T>) [] -> proj(list) T? => list\nexport intrinsic fn to_upper(s: Str) [] -> Str => s\nexport intrinsic fn note(s: Str) [] -> None => s\nexport qualifier Ok<T> of T\nexport qualifier Err<T> of T\nexport fn ok<T>(value: T) [] -> +Ok T {\n    return value\n}\nexport fn err<T>(value: T) [] -> +Err T {\n    return value\n}\n";

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
    // [comptime-instantiate] The `by` sites stamp before resolution.
    let _expansion = salvo_core::expand(&sources.files, &mut modules);
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

/// [elvis] The two shapes that matter: a right side that **escapes** (the
/// result is just the picked type) and one that gives a **value** (the result
/// is the join). `?:` is `Forward(1)`, so on a `Str?` it picks the `Str` arm
/// and `_` is the `None` left behind.
#[test]
fn an_elvis_picks_the_first_arm() {
    let errs = errors(
        "fn nick(s: Str?) [] -> Str? {\n\
         \x20   let n: Str = s ?: return None\n\
         \x20   return n\n\
         }\n\
         fn greet(s: Str?) [] -> Str {\n\
         \x20   return s ?: \"friend\"\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected a clean `?:`, got: {errs:?}");
}

/// [elvis] A pick that keeps every arm leaves nothing for the right side — the
/// same "dead scaffolding" refusal a throw-free `try` gets. A non-union subject
/// has one arm, so `?:` on it is refused the same way.
#[test]
fn a_pick_must_leave_something() {
    let one = errors("fn f(s: Str) [] -> Str {\n    return s ?: \"other\"\n}\n");
    assert!(
        one.iter().any(|e| e.contains("leave nothing for the right-hand side")),
        "expected the nothing-left refusal on a one-arm subject, got: {one:?}"
    );
    // `??:` on a `Str?` keeps both arms.
    let all = errors("fn f(s: Str?) [] -> Str? {\n    return s ??: None\n}\n");
    assert!(
        all.iter().any(|e| e.contains("leave nothing for the right-hand side")),
        "expected the nothing-left refusal when the pick keeps every arm, got: {all:?}"
    );
}

/// [placeholder] `_` is the arms a pick left behind, so it reads only in the
/// right-hand side of a pick. Outside one it is an error, not a silent
/// `Unknown`.
#[test]
fn a_placeholder_outside_a_pick_is_an_error() {
    let errs = errors("fn f() [] -> Str {\n    return _\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("right-hand side of a pick")),
        "expected the stray-placeholder error, got: {errs:?}"
    );
}

/// [elvis] [rewrap] Widening is now allowed (user decision 2026-10-11): a right
/// side outside the subject's arms joins into the result. `Int? ?: "none"` is
/// `Int | Str`.
#[test]
fn a_widening_right_side_joins() {
    let errs = errors("fn f(n: Int?) [] -> Str | Int {\n    return n ?: \"none\"\n}\n");
    assert!(errs.is_empty(), "expected a clean widening pick, got: {errs:?}");
}

/// [elvis] `_` carries the arms left behind, tags and all, so the right side
/// can route on them. A multi-arm `_` is a sub-union of the storage [rewrap].
#[test]
fn the_placeholder_carries_the_left_arms() {
    let errs = errors(
        "fn parse(t: Str) [] -> Ok Int | Err Str {\n    return err(t)\n}\n\
         fn routed(t: Str) [] -> Ok Int | Err Str {\n\
         \x20   let n: Ok Int = parse(t) ?: return _\n\
         \x20   return n\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected a clean `_` on the left arm, got: {errs:?}");
}

/// [elvis] A **backward** pick (`:?`) keeps the last arm. On `Ok Int | Err Str`
/// it picks `Err Str` and leaves `Ok Int` to `_`.
#[test]
fn a_backward_pick_keeps_the_last_arm() {
    let errs = errors(
        "fn parse(t: Str) [] -> Ok Int | Err Str {\n    return err(t)\n}\n\
         fn tail(t: Str) [] -> Ok Int | Err Str {\n\
         \x20   let e: Err Str = parse(t) :? return _\n\
         \x20   return e\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected a clean backward pick, got: {errs:?}");
}

/// [elvis] A pick may span several arms on either side (`??:` keeps two). On
/// `Ok Int | Ok Str | Err Str` the two-arm forward pick keeps `Ok Int | Ok Str`
/// and leaves `Err Str`.
#[test]
fn a_pick_may_span_several_arms() {
    let errs = errors(
        "fn f(r: Ok Int | Ok Str | Err Str) [] -> Ok Int | Ok Str | Err Str {\n\
         \x20   let kept: Ok Int | Ok Str = r ??: return _\n\
         \x20   return kept\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected a clean multi-arm pick, got: {errs:?}");
}

/// [elvis-guard] A `?:` whose right side **leaves** has proved its subject was
/// one of the picked arms, so the place narrows on the path below — the same
/// facts an `is` guard leaves [is-narrow-guard], reached from an expression.
/// Interpolation is the probe: it refuses a possibly-`None` value, so it passes
/// only if the narrowing landed.
#[test]
fn a_guarding_elvis_narrows_its_subject() {
    let errs = errors(
        "fn f(t: Str?) [] -> None {\n\
         \x20   let v: Str = t ?: return\n\
         \x20   note(t)\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected `t` to read as `Str`, got: {errs:?}");
}

/// [elvis-guard] A right side that **yields a value** proves nothing: that path
/// is the one where the subject was not picked, so the place keeps its type.
#[test]
fn a_valued_elvis_does_not_narrow() {
    let errs = errors(
        "fn f(t: Str?) [] -> None {\n\
         \x20   let v: Str = t ?: \"other\"\n\
         \x20   note(t)\n\
         }\n",
    );
    assert!(
        !errs.is_empty(),
        "expected `t` to stay optional after a valued `?:`"
    );
}

/// [elvis-guard] After a guarding pick the place reads as the picked arm, tag
/// included.
#[test]
fn a_guarding_pick_narrows_to_the_arm() {
    let errs = errors(
        "fn parse(t: Str) [] -> Ok Int | Err Str {\n    return err(t)\n}\n\
         fn f(text: Str) [] -> None {\n\
         \x20   let r = parse(text)\n\
         \x20   let n: Ok Int = r ?: return\n\
         \x20   let again: Ok Int = r\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected `r` to read as `Ok Int`, got: {errs:?}");
}

