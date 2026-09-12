//! [once-fn] `Once` as the *use*-multiplicity qualifier, and specifically its
//! generalization beyond function types (user decision 2026-09-07).
//!
//! What is left of that generalization after the reduction to `next` (R5): the
//! *factory* is gone with `Iter<T>`, so `Once` is written on function types and
//! on a type of one's own that says `canbe Once`. The rules under test are the
//! ones it always had — where it may be written, that it never drops — plus the
//! one that belongs to iteration: **driving a pass consumes it**, which now
//! holds for a hand-written pass because that is the only kind there is.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as a *std* file since only std may write `intrinsic`.
const STD_PRELUDE: &str =
    "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n\
     intrinsic fn copy<T>(value: T) [] -> T => value\n";

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

// --- where `Once` may be written -------------------------------------------

#[test]
fn once_on_a_plain_type_is_an_error_naming_the_positions() {
    let errs = errors("fn f(x: Once Int) -> None => !x {}\n");
    assert!(
        errs.iter()
            .any(|e| e.contains("`Once` applies to function types")
                && e.contains("`canbe Once`")
                && e.contains("`Int`")),
        "expected the position error naming fn types and the opt-in, got {errs:?}"
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
         fn f(t: Once Ticket) -> None => !t {}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn canbe_rejects_a_qualifier_that_is_not_an_opt_in() {
    let errs = errors("struct Ticket canbe Ok {\n    id: Int\n}\n");
    assert!(
        errs.iter().any(|e| e.contains(
            "only `Mut` and `Once` can be opted into with `canbe`"
        )),
        "expected the canbe allowlist error, got {errs:?}"
    );
}

#[test]
fn once_on_a_fn_type_still_works() {
    let errs = errors("fn run(f: Once () -> None) -> None => !f { f() }\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- [iter-protocol] hand-written passes ------------------------------------

/// The protocol as std declares it, inlined because this harness loads a
/// minimal prelude rather than the real `std/core/iterator.sv`.
const PROTOCOL: &str = r#"
qualifier Emitted<T> of T
struct Finished {}
fn emitted<T>(value: T) [] -> T as Emitted => !value { return value }
fn finished() [] -> Finished { return Finished {} }

params Yield<It, T> {
    fn next(it: Mut It) -> Emitted T | Finished => it: Mut
}

struct Countdown : Yield<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
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
/// [group-obligation]: `for` reads the `: Yield<self, T>` clause rather than
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
         fn go() -> None {{ for n in build(3) {{}} }}\n"
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
         fn go() -> None {{ for n in build(3) {{}} }}\n"
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
         fn go() -> None {{\n\
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
/// declaration is not a pass — the tie is the `: Yield<self, T>` clause, not the
/// method name — and the not-iterable error names the clause as the remedy.
#[test]
fn a_next_without_a_yield_declaration_is_not_a_pass() {
    let errs = errors(
        "qualifier Emitted<T> of T\n\
         struct Finished {}\n\
         fn emitted<T>(value: T) [] -> T as Emitted => !value { return value }\n\
         fn finished() [] -> Finished { return Finished {} }\n\
         params Yield<It, T> {\n    fn next(it: Mut It) -> Emitted T | Finished => it: Mut\n}\n\
         struct Countdown canbe Mut {\n    at: Int\n}\n\
         fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {\n\
             return finished()\n\
         }\n\
         fn build(from: Int) -> Countdown { return Countdown { at: from } }\n\
         fn go() -> None { for n in build(3) {} }\n",
    );
    assert!(
        errs.iter().any(|e| e.contains("is not iterable")
            && e.contains("has a matching `next`")
            && e.contains(": Yield<self, Int>")),
        "expected the not-iterable error naming the declaration remedy, got {errs:?}"
    );
}
