//! The optional-or-else operator `?:` and the placeholder `_` [elvis]
//! [placeholder].
//!
//! `?:` picks the **non-`None` arms** of its subject, and its right side runs
//! only when the subject is `None`. Reserved for `T?`: a qualifier is picked by
//! naming it (step 5 of the `?` family sequence). The right side is an ordinary
//! expression, which since [expr-escape] includes `return`/`break`/`continue` —
//! so `maybe_t() ?: return None` is the form the whole sequence was for.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\nexport intrinsic type List<T> canbe Mut\nexport intrinsic fn first<T>(list: List<T>) [] -> proj[from: list] T? => list\nexport intrinsic fn to_upper(s: Str) [] -> Str => s\nexport qualifier Ok<T> of T\nexport qualifier Err<T> of T\nexport fn ok<T>(value: T) [] -> T as Ok {\n    return value\n}\nexport fn err<T>(value: T) [] -> T as Err {\n    return value\n}\n";

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

/// [elvis] The two shapes that matter: a right side that **escapes** (the
/// result is just the picked type) and one that gives a **value** (the result
/// is the join). `_` is the `None` side, so `return _` is `return None`.
#[test]
fn an_elvis_picks_the_non_none_arms() {
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

/// [elvis] Reserved for `T?`: without a `None` arm nothing can take the right
/// side, which is the same "dead scaffolding" refusal a throw-free `try` gets.
#[test]
fn an_elvis_needs_an_optional_subject() {
    let errs = errors("fn f(s: Str) [] -> Str {\n    return s ?: \"other\"\n}\n");
    assert!(
        errs.iter()
            .any(|e| e.contains("needs an optional on its left") && e.contains("no `None` arm")),
        "expected the non-optional refusal, got: {errs:?}"
    );
}

/// [placeholder] `_` is the **unpicked** side of a qualifier pick [pick], so it
/// reads only in the right-hand side of one. A plain `?:` leaves `None`, which
/// the program can already write, so there it would name a value that has a
/// name — and outside either, reading `_` is an error rather than a silent
/// `Unknown`.
#[test]
fn a_placeholder_outside_an_elvis_is_an_error() {
    let errs = errors("fn f() [] -> Str {\n    return _\n}\n");
    assert!(
        errs.iter()
            .any(|e| e.contains("unpicked") && e.contains("qualifier pick")),
        "expected the stray-placeholder error, got: {errs:?}"
    );
}

/// [elvis] A right side whose type is **not one of the subject's arms** would
/// need the picked value re-wrapped as well, and the picked value has no span of
/// its own to hang a coercion on. Refused by name for now, with `when` as the
/// remedy.
#[test]
fn a_widening_right_side_is_refused_for_now() {
    let errs = errors("fn f(n: Int?) [] -> Str | Int {\n    return n ?: \"none\"\n}\n");
    assert!(
        errs.iter().any(|e| e.contains("widens the result is not supported yet")),
        "expected the widening-right-side refusal, got: {errs:?}"
    );
}

/// [safe-call] `?.` reaches a member on the non-`None` side, and the result
/// carries a `None` arm — so a field whose own type is optional stays one arm
/// deep (the arms dedupe) and a plain field gains one.
#[test]
fn a_safe_call_re_unions_none() {
    let errs = errors(
        "struct Address {\n    city: Str,\n    zip: Str? = None\n}\n\
         struct Person {\n    home: Address? = None\n}\n\
         fn f(p: Person) [] -> Str? {\n    return p.home?.city\n}\n\
         fn g(p: Person) [] -> Str? {\n    return p.home?.zip\n}\n",
    );
    assert!(errs.is_empty(), "expected a clean `?.`, got: {errs:?}");
}

/// [safe-call] Reserved for `T?`, like `?:`: with no `None` arm the `?` asks a
/// question that has one answer.
#[test]
fn a_safe_call_needs_an_optional_receiver() {
    let errs = errors("fn f(s: Str) [] -> Str? {\n    return s?.to_upper()\n}\n");
    assert!(
        errs.iter()
            .any(|e| e.contains("needs an optional receiver") && e.contains("drop the `?`")),
        "expected the non-optional refusal, got: {errs:?}"
    );
}

/// [safe-call] [loop-while-is] The receiver must be a **place**: the form reads
/// it twice, once to test and once to reach the member, which is the same rule
/// (and the same remedy) a call subject in an `is` already has.
#[test]
fn a_safe_call_receiver_must_be_a_place() {
    let errs = errors(
        "fn id(x: Str?) [] -> Str? {\n    return x\n}\n\
         fn f(s: Str?) [] -> Str? {\n    return id(s)?.to_upper()\n}\n",
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("`?.` receiver must be a variable") && e.contains("bind the value")),
        "expected the place refusal, got: {errs:?}"
    );
}

/// [pick] The two picks: `^Ok` **lifts** the tag (the value is a plain `Int`)
/// and `Ok` keeps it (an `Ok Int`). `_` on the right is the unpicked arm with
/// its own tag — the one place the placeholder earns its keep, since `Err Str`
/// has no other spelling there.
#[test]
fn a_qualifier_pick_lifts_or_keeps() {
    let errs = errors(
        "fn parse(t: Str) [] -> Ok Int | Err Str {\n    return err(t)\n}\n\
         fn lifted(t: Str) [] -> Ok Int | Err Str {\n\
         \x20   let n: Int = parse(t) ^Ok?: return _\n\
         \x20   return ok(n)\n\
         }\n\
         fn kept(t: Str) [] -> Ok Int | Err Str {\n\
         \x20   let n: Ok Int = parse(t) Ok?: return _\n\
         \x20   return n\n\
         }\n",
    );
    assert!(errs.is_empty(), "expected clean picks, got: {errs:?}");
}

/// [pick] A pick that matches nothing can never run; one that matches
/// *everything* leaves the right-hand side dead. Both are the "dead
/// scaffolding" refusal a throw-free `try` gets [try].
#[test]
fn a_pick_must_match_something_and_leave_something() {
    let never = errors(
        "fn f(r: Ok Int | Err Str) [] -> Int {\n    return r Str?: 0\n}\n",
    );
    assert!(
        never.iter().any(|e| e.contains("can never match") || e.contains("names qualifiers")),
        "expected the no-match refusal, got: {never:?}"
    );
    let all = errors(
        "fn f(r: Ok Int | Ok Str) [] -> Int {\n    return r Ok?: 0\n}\n",
    );
    assert!(
        all.iter()
            .any(|e| e.contains("can never run") || e.contains("several arms")),
        "expected the dead-right-side refusal, got: {all:?}"
    );
}

/// [pick] First slice: one matched arm, one left. A pick spanning several arms
/// needs the arm-mapping re-wrap [let-infer] — the same one step 3's multi-arm
/// lift waits on — so it is refused by name rather than emitted wrong.
#[test]
fn a_multi_arm_pick_is_refused_for_now() {
    let errs = errors(
        "fn f(r: Ok Int | Err Str | Err Int) [] -> Int {\n    return r ^Ok?: 0\n}\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("not emitted yet") || e.contains("several arms")),
        "expected the multi-arm deferral, got: {errs:?}"
    );
}
