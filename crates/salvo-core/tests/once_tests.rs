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
                && e.contains("`canbe Once`")
                && e.contains("`Int`")),
        "expected the position error naming fn types, Iter<T> and the opt-in, \
         got {errs:?}"
    );
}

/// [canbe-optin] A type of one's own reaches the same place by opting in, the
/// way it opts into mutability and linearity — the author declares that using
/// the value uses it up, rather than the compiler inferring an obligation
/// (user decision 2026-09-07).
#[test]
fn canbe_once_makes_a_user_type_a_valid_position() {
    let errs = errors(
        "struct Ticket canbe Once {\n    id: Int\n}\n\
         fn f(t: Once Ticket) -> [] None {}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn canbe_rejects_a_qualifier_that_is_not_an_opt_in() {
    let errs = errors("struct Ticket canbe Ok {\n    id: Int\n}\n");
    assert!(
        errs.iter().any(|e| e.contains(
            "only `Mut`, `Linear` and `Once` can be opted into with `canbe`"
        )),
        "expected the canbe allowlist error, got {errs:?}"
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

// --- [iter-protocol] hand-written passes ------------------------------------

/// The protocol as std declares it, inlined because this harness loads a
/// minimal prelude rather than the real `std/core/iterator.sv`.
const PROTOCOL: &str = r#"
qualifier Emitted<T> of T
struct Finished {}
fn emitted<T>(value: T) [] -> [] T as Emitted { return value }
fn finished() [] -> [] Finished { return Finished {} }

params Yield<T> {
    fn next(s: Mut Self) -> [s: Mut] Emitted T | Finished
}

struct Countdown : Yield<Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}
"#;

/// [iter-protocol] [once-fn] A type with a `next` is driven directly rather
/// than through `iter` — but only if its declaration says it is a pass
/// [group-obligation]: `for` reads the `: Yield<T>` clause rather than
/// scanning overloads for a `next` and guessing (roadmap R2, user decisions
/// 2026-09-08).
/// [iter-protocol] The whole point: a hand-written pass drives a `for` loop,
/// which is what makes `zip`/`merge` — the iterators `yield` cannot express —
/// writable at all.
#[test]
fn a_hand_written_pass_drives_a_for_loop() {
    let errs = errors(&format!(
        "{PROTOCOL}\n\
         fn build(from: Int) -> Countdown {{ return Countdown {{ at: from }} }}\n\
         fn go() -> [] None {{ for n in build(3) {{}} }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// A builder may hand the pass over as `Mut` — the qualifier a mutating
/// `next` wants — and the loop drives it the same way.
#[test]
fn a_mut_built_pass_drives_a_for_loop() {
    let errs = errors(&format!(
        "{PROTOCOL}\n\
         fn build(from: Int) -> Mut Countdown {{ return Mut Countdown {{ at: from }} }}\n\
         fn go() -> [] None {{ for n in build(3) {{}} }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// Driving consumes, whatever the pass is made of: the rule is the
/// qualifier's, not `Iter`'s.
#[test]
fn driving_a_hand_written_pass_twice_is_an_error() {
    let errs = errors(&format!(
        "{PROTOCOL}\n\
         fn build(from: Int) -> Countdown {{ return Countdown {{ at: from }} }}\n\
         fn go() -> [] None {{\n\
         let p = build(3)\n\
         for n in p {{}}\n\
         for n in p {{}}\n\
         }}\n"
    ));
    assert!(
        errs.iter()
            .any(|e| e.contains("consumed (moved) by a `for` loop")),
        "expected the pass to be consumed by the first loop, got {errs:?}"
    );
}

/// [group-obligation] [iter-resolve] A matching `next` without the
/// declaration is not a pass — the tie is the `: Yield<T>` clause, not the
/// method name — and the not-iterable error names the clause as the remedy.
#[test]
fn a_next_without_a_yield_declaration_is_not_a_pass() {
    let errs = errors(
        "qualifier Emitted<T> of T\n\
         struct Finished {}\n\
         fn emitted<T>(value: T) [] -> [] T as Emitted { return value }\n\
         fn finished() [] -> [] Finished { return Finished {} }\n\
         params Yield<T> {\n    fn next(s: Mut Self) -> [s: Mut] Emitted T | Finished\n}\n\
         struct Countdown canbe Mut {\n    at: Int\n}\n\
         fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {\n\
             return finished()\n\
         }\n\
         fn build(from: Int) -> Countdown { return Countdown { at: from } }\n\
         fn go() -> [] None { for n in build(3) {} }\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("is not iterable")
            && e.contains("has a matching `next`")
            && e.contains(": Yield<Int>")),
        "expected the not-iterable error naming the declaration remedy, got {errs:?}"
    );
}
