//! [is-bind-once] An `is` **binding** reads its subject twice — once for the
//! test, once for the payload — so a subject with side effects has to be
//! evaluated exactly once and shared.
//!
//! This was a defect until 2026-09-16, and the worst kind: both backends
//! emitted the subject twice, so
//!
//! ```text
//! while remove_first(queue) is Ticket next { redeem(next) }
//! ```
//!
//! called `remove_first` twice per turn and silently discarded every other
//! element — with a *linear* element, its obligation went with it, and the
//! Rust arm cloned the value the checker believed it had moved. Nothing
//! reported anything.
//!
//! The rule now: a binding `is` whose subject is **not a place** is legal as
//! the whole condition of a `while` or of an `if`'s first branch, where the
//! emitters hoist it into one temporary that the test and the binding share;
//! anywhere else it is refused, because the alternative — hoisting out of a
//! `&&` chain or an `elif` — would evaluate a subject that short-circuiting
//! says should not run.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "\
export intrinsic type Int canbe Mut
export intrinsic type Str
export intrinsic type Bool
export intrinsic fn copy<T>(value: T) [] -> T => value
";

/// A subject with a side effect, and a place to compare against.
const PRELUDE: &str = "\
struct Box canbe Mut { n: Int }

fn pull(b: Mut Box) [] -> Int? => b: Mut {
    b.n = 1
    return copy(b.n)
}
";

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
        format!("{PRELUDE}\n{src}"),
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

/// [is-bind-once] The two hoistable shapes: a `while` condition and an `if`'s
/// first branch. These are the forms a program actually wants — the drain loop
/// above all — so they stay legal and the emitters make the single evaluation
/// true.
#[test]
fn a_call_subject_is_legal_as_a_whole_condition() {
    let errs = errors(
        "\
fn go(b: Mut Box) [] -> Int => b: Mut {
    while pull(b) is Int v {
        return v
    }
    if pull(b) is Int w {
        return w
    }
    return 0
}
",
    );
    assert!(errs.is_empty(), "the hoistable shapes must stay legal: {errs:?}");
}

/// [is-bind-once] Inside a `&&` chain it is refused: hoisting would evaluate
/// the subject even where short-circuiting says it must not run, and *not*
/// hoisting is the defect this rule closed.
#[test]
fn a_call_subject_inside_a_chain_is_refused() {
    let errs = errors(
        "\
fn go(b: Mut Box) [] -> Int => b: Mut {
    if pull(b) is Int v && v > 1 {
        return v
    }
    return 0
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("has to be evaluated exactly once")
            && m.contains("inside a larger condition")),
        "expected the chain refusal: {errs:?}"
    );
}

/// [is-bind-once] …and in an `elif` condition, where there is no statement
/// position for the temporary. The diagnostic names the remedy, which is the
/// `let` the author would have written anyway.
#[test]
fn a_call_subject_in_an_elif_is_refused() {
    let errs = errors(
        "\
fn go(b: Mut Box, flag: Bool) [] -> Int => b: Mut, flag {
    if flag {
        return 0
    } elif pull(b) is Int v {
        return v
    }
    return 0
}
",
    );
    assert!(
        errs.iter().any(|m| m.contains("has to be evaluated exactly once")
            && m.contains("`elif` condition")
            && m.contains("Bind the subject first")),
        "expected the elif refusal: {errs:?}"
    );
}

/// [is-bind-once] A **place** subject is unaffected anywhere: reading a
/// variable or a field chain twice is free, so the old rendering stands and
/// none of the positions above are restricted for it.
#[test]
fn a_place_subject_is_legal_everywhere() {
    let errs = errors(
        "\
struct Holder { maybe: Int? }

fn go(h: Holder, o: Int?, flag: Bool) [] -> Int => h, o, flag {
    if o is Int a && a > 1 {
        return a
    } elif h.maybe is Int b {
        return b
    }
    while o is Int c {
        return c
    }
    return 0
}
",
    );
    assert!(errs.is_empty(), "a place subject must stay unrestricted: {errs:?}");
}

/// [is-bind-once] A subject *without* a binding is read once — by the test —
/// so it needs no hoisting and no restriction.
#[test]
fn a_call_subject_without_a_binding_is_unrestricted() {
    let errs = errors(
        "\
fn go(b: Mut Box, flag: Bool) [] -> Int => b: Mut, flag {
    if flag && pull(b) is Int {
        return 1
    }
    return 0
}
",
    );
    assert!(errs.is_empty(), "a bare `is` needs no hoist: {errs:?}");
}
