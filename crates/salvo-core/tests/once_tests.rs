//! [once-fn] `Once` as the *use*-multiplicity qualifier, and specifically its
//! generalization beyond function types (user decision 2026-09-07).
//!
//! `Once Iter<T>` is a **pass** — a position in a sequence — as against a
//! plain `Iter<T>`, which is a replayable **factory**. The distinction is the
//! answer to "does an iterator function return something you may run again",
//! and it is written down per function rather than decided once for the whole
//! language (see PROGRESS.md, "Factory and pass").
//!
//! The three rules under test are the ones `Once` already had, applied to a
//! type that is not a function:
//!   * where it may be written at all (fn types and `Iter<T>`, roadmap D6);
//!   * that it never drops, so a pass does not fit a factory position;
//!   * that it restricts rather than refines, so a factory *does* fit a pass
//!     position — the inverted variance.
//! Plus the one rule that is new: driving a pass consumes it.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as a *std* file since only std may write `intrinsic`.
const STD_PRELUDE: &str =
    "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\nintrinsic type Iter<T>\n\
     intrinsic fn copy<T>(value: T) [] -> [value] T\n";

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
        .map(|d| d.message.clone())
        .collect()
}

/// A pass-returning producer and a factory-returning one, identical but for
/// the qualifier — which is the point.
const PRODUCERS: &str = r#"
fn pass_of(limit: Int) -> Once Iter<Int> {
    let i = 0
    while i < limit {
        let v = copy(i)
        i = i + 1
        yield v
    }
}

fn factory_of(limit: Int) -> Iter<Int> {
    let i = 0
    while i < limit {
        let v = copy(i)
        i = i + 1
        yield v
    }
}
"#;

fn check(body: &str) -> Vec<String> {
    errors(&format!("{PRODUCERS}\nfn use_it() -> [] None {{\n{body}\n}}\n"))
}

// --- where `Once` may be written -------------------------------------------

#[test]
fn once_on_iter_is_accepted() {
    let errs = errors(PRODUCERS);
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn once_on_a_plain_type_is_an_error_naming_the_positions() {
    let errs = errors("fn f(x: Once Int) -> [] None {}\n");
    assert!(
        errs.iter()
            .any(|e| e.contains("`Once` applies to function types and `Iter<T>`")
                && e.contains("not `Int`")),
        "expected the position error naming fn types and Iter<T>, got {errs:?}"
    );
}

#[test]
fn once_on_a_fn_type_still_works() {
    let errs = errors("fn run(f: Once () -> None) -> [] None { f() }\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- driving a pass consumes it --------------------------------------------

#[test]
fn driving_a_pass_once_is_fine() {
    let errs = check("    let p = pass_of(3)\n    for n in p {}\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn driving_a_pass_twice_is_a_consumed_use_error() {
    let errs = check("    let p = pass_of(3)\n    for n in p {}\n    for n in p {}\n");
    assert!(
        errs.iter()
            .any(|e| e.contains("consumed (moved) by a `for` loop")),
        "expected the pass to be consumed by the first loop, got {errs:?}"
    );
}

#[test]
fn driving_a_factory_twice_is_fine() {
    let errs = check("    let f = factory_of(3)\n    for n in f {}\n    for n in f {}\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The consumption is of the *value*, not of the expression: a producer called
/// twice yields two passes and neither loop sees the other.
#[test]
fn two_calls_give_two_passes() {
    let errs = check("    for n in pass_of(3) {}\n    for n in pass_of(3) {}\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- variance ---------------------------------------------------------------

#[test]
fn a_factory_fits_where_a_pass_is_expected() {
    let errs = errors(&format!(
        "{PRODUCERS}\n\
         fn drain(xs: Once Iter<Int>) -> [] None {{ for n in xs {{}} }}\n\
         fn go() -> [] None {{ drain(factory_of(3)) }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn a_pass_fits_where_a_pass_is_expected() {
    let errs = errors(&format!(
        "{PRODUCERS}\n\
         fn drain(xs: Once Iter<Int>) -> [] None {{ for n in xs {{}} }}\n\
         fn go() -> [] None {{ drain(pass_of(3)) }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// The direction that must fail: `Once` never drops, so a pass cannot stand in
/// for a factory — the callee would be free to drive it twice.
#[test]
fn a_pass_does_not_fit_where_a_factory_is_expected() {
    let errs = errors(&format!(
        "{PRODUCERS}\n\
         fn twice(xs: Iter<Int>) -> [] None {{ for n in xs {{}} for n in xs {{}} }}\n\
         fn go() -> [] None {{ twice(pass_of(3)) }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("twice(Once Iter<Int>)") || e.contains("Once Iter<Int>")),
        "expected the pass to be rejected in a factory position, got {errs:?}"
    );
}
