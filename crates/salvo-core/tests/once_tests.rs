//! [once-fn] `once` as the *use*-multiplicity qualifier, and specifically its
//! generalization beyond function types (user decision 2026-09-07).
//!
//! What is left of that generalization after the reduction to `next` (R5): the
//! *factory* is gone with `Iter<T>`, so `once` is written on function types and
//! on a type of one's own that says `canbe once`. The rules under test are the
//! ones it always had — where it may be written, that it never drops — plus the
//! one that belongs to iteration: **driving a pass consumes it**, which now
//! holds for a hand-written pass because that is the only kind there is.

use std::path::Path;

use salvo_core::{check_program, resolve, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on,
/// loaded as a *std* file since only std may write `intrinsic`.
const STD_PRELUDE: &str =
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
     export intrinsic fn copy<T>(value: T) [] -> T => value\n";

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

// --- where `once` may be written -------------------------------------------

#[test]
fn once_is_valid_on_any_type() {
    // [once-fn] D6 (user decision 2026-09-12): the fn-only/`canbe once`
    // position gate is gone — `once` is an upper bound the holder imposes
    // on itself, valid on any type.
    let errs = errors("fn f(x: once Int) -> None => !x {}\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [canbe-optin] A type of one's own reaches the same place by opting in, the
/// way it opts into mutability and linearity — the author declares that using
/// the value uses it up, rather than the compiler inferring an obligation
/// (user decision 2026-09-07).
#[test]
fn canbe_once_makes_a_user_type_a_valid_position() {
    let errs = errors(
        "struct Ticket canbe once {\n    id: Int\n}\n\
         fn f(t: once Ticket) -> None => !t {}\n",
    );
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

#[test]
fn canbe_rejects_a_qualifier_that_is_not_an_opt_in() {
    let errs = errors("struct Ticket canbe Ok {\n    id: Int\n}\n");
    assert!(
        errs.iter().any(|e| e.contains(
            "only `Mut`, `once`, `hashed` and `ordered` can be opted into with `canbe`"
        )),
        "expected the canbe allowlist error, got {errs:?}"
    );
}

#[test]
fn once_on_a_fn_type_still_works() {
    let errs = errors("fn run(f: once () -> None) -> None => !f { f() }\n");
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// --- [iter-protocol] hand-written passes ------------------------------------

/// The protocol as std declares it, inlined because this harness loads a
/// minimal prelude rather than the real `std/core/iterator.sv`.
const PROTOCOL: &str = r#"
qualifier Emitted<T> of T
struct Finished {}
fn emitted<T>(value: T) [] -> +Emitted T => !value { return value }
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
fn driving_a_hand_written_pass_twice_is_legal() {
    // [iter-drive-in-place] A named pass is driven where it lives (user
    // decision 2026-09-12 — a `for` never consumes a variable, which is
    // what lets a linear pass be bound, driven and explicitly discharged).
    // Driving an exhausted pass again is meaningful: `next` keeps
    // answering `Finished`, so the second loop runs zero times.
    let errs = errors(&format!(
        "{PROTOCOL}\n\
         fn build(from: Int) -> Countdown {{ return Countdown {{ at: from }} }}\n\
         fn go() -> None {{\n\
         let p = build(3)\n\
         for n in p {{}}\n\
         for n in p {{}}\n\
         }}\n"
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [group-obligation] [iter-resolve] A matching `next` without the
/// declaration is not a pass — the tie is the `: Yield<self, T>` clause, not the
/// method name — and the not-iterable error names the clause as the remedy.
#[test]
fn a_next_without_a_yield_declaration_is_not_a_pass() {
    let errs = errors(
        "qualifier Emitted<T> of T\n\
         struct Finished {}\n\
         fn emitted<T>(value: T) [] -> +Emitted T => !value { return value }\n\
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

// [once-fn] D6 (user decision 2026-09-12): `once` on a *data* type — the
// upper bound is the holder's self-restriction, enforcement is ordinary
// consumption, and `once T` drops to `T` at a call (a plain-typed holder
// consumes at most once anyway). On fn types nothing changes: dropping
// `once` there stays refused (see `once_where_plain_bad` shapes).
#[test]
fn once_on_a_data_type_enforces_single_use() {
    let clean = errors(
        "struct Ticket {\n    id: Int\n}\n\
         fn redeem(t: Ticket) -> None => !t {}\n\
         fn spend(t: once Ticket) -> None => !t {\n    redeem(t)\n}\n",
    );
    assert!(clean.is_empty(), "expected no errors, got {clean:?}");
    let double = errors(
        "struct Ticket {\n    id: Int\n}\n\
         fn redeem(t: Ticket) -> None => !t {}\n\
         fn spend(t: once Ticket) -> None => !t {\n    redeem(t)\n    redeem(t)\n}\n",
    );
    assert!(
        double.iter().any(|e| e.contains("consumed (moved) by an earlier call")),
        "got {double:?}"
    );
}
